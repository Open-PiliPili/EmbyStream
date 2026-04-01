use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Clone, Copy)]
pub enum CacheKeyStrategy {
    FullUri,
    NextUpSeriesId,
    EpisodesShowId,
    UserItem,
}

#[derive(Clone, Copy)]
pub enum BodyKeyStrategy {
    Ignore,
    AutoContentType,
    RawHash,
    JsonCanonical,
    FormUrlEncodedCanonical,
}

/// Represents a single cacheable Emby API route.
///
/// ## How to add a new cacheable route
///
/// 1. Add a new `CacheableRoute` entry to the `CACHEABLE_ROUTES` array below.
/// 2. `pattern`     — Regex matching the API path (without query parameters).
/// 3. `methods`     — HTTP methods to cache (e.g. `&["GET"]` or `&["POST"]`).
/// 4. `ttl_seconds` — Cache lifetime in seconds. Recommended range: 7200–14400 (2–4 hours).
/// 5. `description` — Brief explanation of why this route needs caching.
///
/// ## Cache key strategy
///
/// - **GET requests**: key = `GET:{full URI including path + all query params}`
///   Example: `GET:/emby/Shows/NextUp?UserId=...&Limit=24&...`
///   Different query params each produce a separate cache entry.
///
/// - **POST requests**: body handling should be explicit.
///   Prefer canonical body strategies for JSON or form-urlencoded payloads
///   instead of hashing raw bytes directly, to avoid unnecessary cache miss.
///
/// - **Special case: PlaybackInfo**
///   `PlaybackInfo` is handled separately by `PlaybackInfoService`, which keeps
///   GET and POST responses isolated and preserves POST body semantics.
///
/// ## Important notes
///
/// - Only successful responses (HTTP 2xx) with `Content-Type: application/json` are cached.
/// - Media metadata can change; do not set TTL above 4 hours.
/// - Do NOT add video stream paths here — those are handled by `ForwardMiddleware`.
pub struct CacheableRoute {
    pub pattern: &'static str,
    pub methods: &'static [&'static str],
    pub ttl_seconds: u64,
    pub description: &'static str,
    pub key_strategy: CacheKeyStrategy,
    pub body_key_strategy: BodyKeyStrategy,
}

pub const CACHEABLE_ROUTES: &[CacheableRoute] = &[
    CacheableRoute {
        pattern: r"(?i)^/(?:emby/)?Users/[a-z0-9]+/Items/[0-9]+$",
        methods: &["GET"],
        ttl_seconds: 7200, // 2 hours
        description: "User item details",
        key_strategy: CacheKeyStrategy::UserItem,
        body_key_strategy: BodyKeyStrategy::Ignore,
    },
    CacheableRoute {
        pattern: r"(?i)^/(?:emby/)?Shows/NextUp$",
        methods: &["GET"],
        ttl_seconds: 7200, // 2 hours
        description: "Next-up episode for a series",
        key_strategy: CacheKeyStrategy::NextUpSeriesId,
        body_key_strategy: BodyKeyStrategy::Ignore,
    },
    CacheableRoute {
        pattern: r"(?i)^/(?:emby/)?Shows/[^/]+/Episodes$",
        methods: &["GET"],
        ttl_seconds: 7200, // 2 hours
        description: "Episode list for a series season",
        key_strategy: CacheKeyStrategy::EpisodesShowId,
        body_key_strategy: BodyKeyStrategy::Ignore,
    },
];

pub struct CompiledCacheableRoute {
    pub regex: Regex,
    pub methods: &'static [&'static str],
    pub ttl_seconds: u64,
    pub key_strategy: CacheKeyStrategy,
    pub body_key_strategy: BodyKeyStrategy,
}

pub static COMPILED_ROUTES: Lazy<Vec<CompiledCacheableRoute>> =
    Lazy::new(|| {
        CACHEABLE_ROUTES
            .iter()
            .filter_map(|route| {
                Regex::new(route.pattern).ok().map(|regex| {
                    CompiledCacheableRoute {
                        regex,
                        methods: route.methods,
                        ttl_seconds: route.ttl_seconds,
                        key_strategy: route.key_strategy,
                        body_key_strategy: route.body_key_strategy,
                    }
                })
            })
            .collect()
    });

/// Returns the matching cacheable route for the given path and method, if any.
pub fn find_cacheable_route(
    path: &str,
    method: &str,
) -> Option<&'static CompiledCacheableRoute> {
    COMPILED_ROUTES.iter().find(|route| {
        route.regex.is_match(path)
            && route.methods.iter().any(|m| m.eq_ignore_ascii_case(method))
    })
}

pub fn build_semantic_cache_key(
    route: &CompiledCacheableRoute,
    method: &str,
    path: &str,
    query: Option<&str>,
) -> String {
    let method = method.to_ascii_lowercase();
    let canonical_uri = canonical_uri_for_cache(path, query);

    match route.key_strategy {
        CacheKeyStrategy::FullUri => {
            format!("api:full_uri:method:{method}:uri:{canonical_uri}")
        }
        CacheKeyStrategy::NextUpSeriesId => {
            build_next_up_cache_key(&method, query, &canonical_uri)
        }
        CacheKeyStrategy::EpisodesShowId => {
            build_episodes_cache_key(&method, path, &canonical_uri)
        }
        CacheKeyStrategy::UserItem => {
            build_user_item_cache_key(&method, path, &canonical_uri)
        }
    }
}

fn canonical_uri_for_cache(path: &str, query: Option<&str>) -> String {
    let Some(query_str) = query.filter(|query| !query.is_empty()) else {
        return path.to_string();
    };

    let mut query_pairs: Vec<(String, String)> =
        form_urlencoded::parse(query_str.as_bytes())
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect();
    query_pairs.sort_by(|(left_key, left_value), (right_key, right_value)| {
        left_key
            .to_ascii_lowercase()
            .cmp(&right_key.to_ascii_lowercase())
            .then_with(|| left_value.cmp(right_value))
    });

    let normalized_query = form_urlencoded::Serializer::new(String::new())
        .extend_pairs(
            query_pairs
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_str())),
        )
        .finish();

    format!("{path}?{normalized_query}")
}

fn build_next_up_cache_key(
    method: &str,
    query: Option<&str>,
    fallback_uri: &str,
) -> String {
    let Some(query_str) = query else {
        return format!("api:full_uri:method:{method}:uri:{fallback_uri}");
    };

    let Some(series_id) = form_urlencoded::parse(query_str.as_bytes())
        .find(|(key, _)| key.eq_ignore_ascii_case("SeriesId"))
        .map(|(_, value)| value.into_owned())
    else {
        return format!("api:full_uri:method:{method}:uri:{fallback_uri}");
    };

    format!(
        "api:shows_nextup:method:{method}:series_id:{}",
        series_id.to_ascii_lowercase()
    )
}

fn build_episodes_cache_key(
    method: &str,
    path: &str,
    fallback_uri: &str,
) -> String {
    let segments: Vec<&str> = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();

    let Some(show_id) = segments
        .windows(3)
        .find(|window| {
            window
                .first()
                .is_some_and(|segment| segment.eq_ignore_ascii_case("Shows"))
                && window.get(2).is_some_and(|segment| {
                    segment.eq_ignore_ascii_case("Episodes")
                })
        })
        .and_then(|window| window.get(1))
    else {
        return format!("api:full_uri:method:{method}:uri:{fallback_uri}");
    };

    format!(
        "api:shows_episodes:method:{method}:show_id:{}",
        show_id.to_ascii_lowercase()
    )
}

fn build_user_item_cache_key(
    method: &str,
    path: &str,
    fallback_uri: &str,
) -> String {
    let segments: Vec<&str> = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();

    let Some((user_id, item_id)) = segments
        .windows(4)
        .find(|window| {
            window
                .first()
                .is_some_and(|segment| segment.eq_ignore_ascii_case("Users"))
                && window.get(2).is_some_and(|segment| {
                    segment.eq_ignore_ascii_case("Items")
                })
        })
        .and_then(|window| {
            let user_id = window.get(1)?;
            let item_id = window.get(3)?;
            Some((*user_id, *item_id))
        })
    else {
        return format!("api:full_uri:method:{method}:uri:{fallback_uri}");
    };

    if !user_id.chars().all(|c| c.is_ascii_alphanumeric())
        || !item_id.chars().all(|c| c.is_ascii_digit())
    {
        return format!("api:full_uri:method:{method}:uri:{fallback_uri}");
    }

    format!(
        "api:user_item:method:{method}:user_id:{}:item_id:{}",
        user_id.to_ascii_lowercase(),
        item_id.to_ascii_lowercase()
    )
}

#[cfg(test)]
mod tests {
    use super::{
        BodyKeyStrategy, CacheKeyStrategy, CompiledCacheableRoute,
        build_semantic_cache_key, find_cacheable_route,
    };
    use regex::Regex;

    fn compiled(strategy: CacheKeyStrategy) -> CompiledCacheableRoute {
        CompiledCacheableRoute {
            regex: Regex::new(".*").unwrap_or_else(|_| unreachable!()),
            methods: &["GET"],
            ttl_seconds: 1,
            key_strategy: strategy,
            body_key_strategy: BodyKeyStrategy::Ignore,
        }
    }

    #[test]
    fn next_up_semantic_key_uses_series_id_only() {
        let route = compiled(CacheKeyStrategy::NextUpSeriesId);
        let key = build_semantic_cache_key(
            &route,
            "GET",
            "/emby/Shows/NextUp",
            Some("Limit=1&SeriesId=Series-ABC_01&UserId=u1"),
        );

        assert_eq!(key, "api:shows_nextup:method:get:series_id:series-abc_01");
    }

    #[test]
    fn next_up_route_matches_exact_path_only() {
        let route = find_cacheable_route("/emby/Shows/NextUp", "GET");

        assert!(route.is_some());
    }

    #[test]
    fn next_up_route_does_not_match_extra_segments() {
        let route = find_cacheable_route("/emby/Shows/NextUp/resume", "GET");

        assert!(route.is_none());
    }

    #[test]
    fn next_up_route_does_not_match_missing_segment() {
        let route = find_cacheable_route("/emby/Shows", "GET");

        assert!(route.is_none());
    }

    #[test]
    fn episodes_semantic_key_uses_show_id_from_path_only() {
        let route = compiled(CacheKeyStrategy::EpisodesShowId);
        let key = build_semantic_cache_key(
            &route,
            "GET",
            "/emby/Shows/Show-XYZ_09/Episodes",
            Some("SeasonId=s1&UserId=u1"),
        );

        assert_eq!(key, "api:shows_episodes:method:get:show_id:show-xyz_09");
    }

    #[test]
    fn episodes_route_matches_exact_path_only() {
        let route =
            find_cacheable_route("/emby/Shows/Show-XYZ_09/Episodes", "GET");

        assert!(route.is_some());
    }

    #[test]
    fn episodes_route_does_not_match_extra_segments() {
        let route = find_cacheable_route(
            "/emby/Shows/Show-XYZ_09/Episodes/resume",
            "GET",
        );

        assert!(route.is_none());
    }

    #[test]
    fn episodes_route_does_not_match_missing_show_id() {
        let route = find_cacheable_route("/emby/Shows/Episodes", "GET");

        assert!(route.is_none());
    }

    #[test]
    fn user_item_semantic_key_uses_user_and_item_ids_only() {
        let route = compiled(CacheKeyStrategy::UserItem);
        let key = build_semantic_cache_key(
            &route,
            "GET",
            "/emby/Users/UserABC01/Items/257023",
            None,
        );

        assert_eq!(
            key,
            "api:user_item:method:get:user_id:userabc01:item_id:257023"
        );
    }

    #[test]
    fn full_uri_strategy_normalizes_query_order() {
        let route = compiled(CacheKeyStrategy::FullUri);
        let key1 = build_semantic_cache_key(
            &route,
            "GET",
            "/emby/Items",
            Some("b=2&a=1"),
        );
        let key2 = build_semantic_cache_key(
            &route,
            "GET",
            "/emby/Items",
            Some("a=1&b=2"),
        );

        assert_eq!(key1, key2);
        assert_eq!(key1, "api:full_uri:method:get:uri:/emby/Items?a=1&b=2");
    }

    #[test]
    fn user_item_route_does_not_match_extra_segments() {
        let route =
            find_cacheable_route("/emby/Users/u1/Items/i1/resume", "GET");

        assert!(route.is_none());
    }

    #[test]
    fn user_item_route_does_not_match_missing_item_id() {
        let route = find_cacheable_route("/emby/Users/u1/Items", "GET");

        assert!(route.is_none());
    }

    #[test]
    fn user_item_route_does_not_match_non_alphanumeric_user_id() {
        let route =
            find_cacheable_route("/emby/Users/user-1/Items/257023", "GET");

        assert!(route.is_none());
    }

    #[test]
    fn user_item_route_does_not_match_non_numeric_item_id() {
        let route =
            find_cacheable_route("/emby/Users/user1/Items/item257023", "GET");

        assert!(route.is_none());
    }
}
