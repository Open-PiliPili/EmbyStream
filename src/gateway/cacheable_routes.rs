use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Clone, Copy)]
pub enum CacheKeyStrategy {
    FullUri,
    NextUpSeriesId,
    EpisodesShowId,
    UserItem,
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
/// - **POST requests**: key = `POST:{full URI including path + all query params}:{MD5 of request body}`
///   Example: `POST:/emby/Items/.../Action?...:a3f2b8c1...`
///
/// - **Special case: PlaybackInfo**
///   `PlaybackInfo` is normalized and shared by `PlaybackInfoService` using
///   `item_id + media_source_id` as the semantic cache key across GET and POST.
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
}

pub const CACHEABLE_ROUTES: &[CacheableRoute] = &[
    CacheableRoute {
        pattern: r"(?i)^/(?:emby/)?Users/[a-z0-9]+/Items/[0-9]+$",
        methods: &["GET"],
        ttl_seconds: 7200, // 2 hours
        description: "User item details",
        key_strategy: CacheKeyStrategy::UserItem,
    },
    CacheableRoute {
        pattern: r"(?i)^/(?:emby/)?Shows/NextUp$",
        methods: &["GET"],
        ttl_seconds: 7200, // 2 hours
        description: "Next-up episode for a series",
        key_strategy: CacheKeyStrategy::NextUpSeriesId,
    },
    CacheableRoute {
        pattern: r"(?i)^/(?:emby/)?Shows/[^/]+/Episodes$",
        methods: &["GET"],
        ttl_seconds: 7200, // 2 hours
        description: "Episode list for a series season",
        key_strategy: CacheKeyStrategy::EpisodesShowId,
    },
];

pub struct CompiledCacheableRoute {
    pub regex: Regex,
    pub methods: &'static [&'static str],
    pub ttl_seconds: u64,
    pub key_strategy: CacheKeyStrategy,
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
    fallback_uri: &str,
) -> String {
    match route.key_strategy {
        CacheKeyStrategy::FullUri => format!("{method}:{fallback_uri}"),
        CacheKeyStrategy::NextUpSeriesId => {
            build_next_up_cache_key(method, query, fallback_uri)
        }
        CacheKeyStrategy::EpisodesShowId => {
            build_episodes_cache_key(method, path, fallback_uri)
        }
        CacheKeyStrategy::UserItem => {
            build_user_item_cache_key(method, path, fallback_uri)
        }
    }
}

fn build_next_up_cache_key(
    method: &str,
    query: Option<&str>,
    fallback_uri: &str,
) -> String {
    let Some(query_str) = query else {
        return format!("{method}:{fallback_uri}");
    };

    let Some(series_id) = form_urlencoded::parse(query_str.as_bytes())
        .find(|(key, _)| key.eq_ignore_ascii_case("SeriesId"))
        .map(|(_, value)| value.into_owned())
    else {
        return format!("{method}:{fallback_uri}");
    };

    format!(
        "{method}:shows_nextup:series_id:{}",
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
        return format!("{method}:{fallback_uri}");
    };

    format!(
        "{method}:shows_episodes:show_id:{}",
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
        return format!("{method}:{fallback_uri}");
    };

    if !user_id.chars().all(|c| c.is_ascii_alphanumeric())
        || !item_id.chars().all(|c| c.is_ascii_digit())
    {
        return format!("{method}:{fallback_uri}");
    }

    format!(
        "{method}:user_item:user_id:{}:item_id:{}",
        user_id.to_ascii_lowercase(),
        item_id.to_ascii_lowercase()
    )
}

#[cfg(test)]
mod tests {
    use super::{
        CacheKeyStrategy, CompiledCacheableRoute, build_semantic_cache_key,
        find_cacheable_route,
    };
    use regex::Regex;

    fn compiled(strategy: CacheKeyStrategy) -> CompiledCacheableRoute {
        CompiledCacheableRoute {
            regex: Regex::new(".*").unwrap_or_else(|_| unreachable!()),
            methods: &["GET"],
            ttl_seconds: 1,
            key_strategy: strategy,
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
            "/emby/Shows/NextUp?Limit=1&SeriesId=Series-ABC_01&UserId=u1",
        );

        assert_eq!(key, "GET:shows_nextup:series_id:series-abc_01");
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
            "/emby/Shows/Show-XYZ_09/Episodes?SeasonId=s1&UserId=u1",
        );

        assert_eq!(key, "GET:shows_episodes:show_id:show-xyz_09");
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
            "/emby/Users/UserABC01/Items/257023",
        );

        assert_eq!(key, "GET:user_item:user_id:userabc01:item_id:257023");
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
