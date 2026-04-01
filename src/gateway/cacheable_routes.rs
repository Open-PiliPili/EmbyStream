use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Clone, Copy)]
pub enum CacheKeyStrategy {
    FullUri,
}

#[derive(Clone, Copy)]
pub enum BodyKeyStrategy {
    Ignore,
    AutoContentType,
    RawHash,
    JsonCanonical,
    FormUrlEncodedCanonical,
}

const IGNORED_QUERY_KEYS: &[&str] = &["UserId"];

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
/// - **GET requests**: key = normalized full URI including all query params.
///   Example: `GET:/emby/Shows/NextUp?UserId=...&Limit=24&...`
///   Different query params each produce a separate cache entry.
/// - Query normalization rules apply to every cacheable route:
///   ignore `UserId`, lowercase query keys for comparison and serialization,
///   then sort by key and value so equivalent queries share the same cache key.
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
        key_strategy: CacheKeyStrategy::FullUri,
        body_key_strategy: BodyKeyStrategy::Ignore,
    },
    CacheableRoute {
        pattern: r"(?i)^/(?:emby/)?Shows/NextUp$",
        methods: &["GET"],
        ttl_seconds: 7200, // 2 hours
        description: "Next-up episode for a series",
        key_strategy: CacheKeyStrategy::FullUri,
        body_key_strategy: BodyKeyStrategy::Ignore,
    },
    CacheableRoute {
        pattern: r"(?i)^/(?:emby/)?Shows/[^/]+/Episodes$",
        methods: &["GET"],
        ttl_seconds: 7200, // 2 hours
        description: "Episode list for a series season",
        key_strategy: CacheKeyStrategy::FullUri,
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
    }
}

fn canonical_uri_for_cache(path: &str, query: Option<&str>) -> String {
    let Some(query_str) = query.filter(|query| !query.is_empty()) else {
        return path.to_string();
    };

    // Normalize query params for every cacheable route:
    // drop ignored keys, compare keys case-insensitively by lowering them,
    // then sort pairs to avoid cache misses caused by query order.
    let mut query_pairs: Vec<(String, String)> =
        form_urlencoded::parse(query_str.as_bytes())
            .filter_map(|(key, value)| {
                if IGNORED_QUERY_KEYS
                    .iter()
                    .any(|ignored| key.eq_ignore_ascii_case(ignored))
                {
                    return None;
                }

                Some((key.to_ascii_lowercase(), value.into_owned()))
            })
            .collect();
    if query_pairs.is_empty() {
        return path.to_string();
    }
    query_pairs.sort_by(|(left_key, left_value), (right_key, right_value)| {
        left_key
            .cmp(right_key)
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
    fn next_up_semantic_key_uses_full_uri() {
        let route = compiled(CacheKeyStrategy::FullUri);
        let key = build_semantic_cache_key(
            &route,
            "GET",
            "/emby/Shows/NextUp",
            Some("Limit=1&SeriesId=Series-ABC_01&UserId=u1"),
        );

        assert_eq!(
            key,
            "api:full_uri:method:get:uri:/emby/Shows/NextUp?limit=1&seriesid=Series-ABC_01"
        );
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
    fn episodes_semantic_key_distinguishes_season_id() {
        let route = compiled(CacheKeyStrategy::FullUri);
        let key1 = build_semantic_cache_key(
            &route,
            "GET",
            "/emby/Shows/Show-XYZ_09/Episodes",
            Some("SeasonId=s1&UserId=u1"),
        );
        let key = build_semantic_cache_key(
            &route,
            "GET",
            "/emby/Shows/Show-XYZ_09/Episodes",
            Some("SeasonId=s2&UserId=u1"),
        );

        assert_ne!(key1, key);
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
    fn user_item_semantic_key_uses_full_uri() {
        let route = compiled(CacheKeyStrategy::FullUri);
        let key = build_semantic_cache_key(
            &route,
            "GET",
            "/emby/Users/UserABC01/Items/257023",
            Some("Fields=Path%2COverview&UserId=u1"),
        );

        assert_eq!(
            key,
            "api:full_uri:method:get:uri:/emby/Users/UserABC01/Items/257023?fields=Path%2COverview"
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
    fn full_uri_strategy_ignores_user_id() {
        let route = compiled(CacheKeyStrategy::FullUri);
        let key1 = build_semantic_cache_key(
            &route,
            "GET",
            "/emby/Shows/49619/Episodes",
            Some("SeasonId=49708&UserId=u1"),
        );
        let key2 = build_semantic_cache_key(
            &route,
            "GET",
            "/emby/Shows/49619/Episodes",
            Some("UserId=u2&SeasonId=49708"),
        );

        assert_eq!(key1, key2);
        assert_eq!(
            key1,
            "api:full_uri:method:get:uri:/emby/Shows/49619/Episodes?seasonid=49708"
        );
    }

    #[test]
    fn full_uri_strategy_treats_query_keys_case_insensitively() {
        let route = compiled(CacheKeyStrategy::FullUri);
        let key1 = build_semantic_cache_key(
            &route,
            "GET",
            "/emby/Shows/49619/Episodes",
            Some("SeasonId=49708&Fields=A"),
        );
        let key2 = build_semantic_cache_key(
            &route,
            "GET",
            "/emby/Shows/49619/Episodes",
            Some("seasonid=49708&fields=A"),
        );

        assert_eq!(key1, key2);
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
