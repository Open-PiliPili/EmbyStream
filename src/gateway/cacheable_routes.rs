use once_cell::sync::Lazy;
use regex::Regex;

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
/// ## Cache key strategy (precise matching — different params produce different entries)
///
/// - **GET requests**: key = `GET:{full URI including path + all query params}`
///   Example: `GET:/emby/Shows/NextUp?UserId=...&Limit=24&...`
///   Different query params each produce a separate cache entry.
///
/// - **POST requests**: key = `POST:{full URI including path + all query params}:{MD5 of request body}`
///   Example: `POST:/emby/Items/251044/PlaybackInfo?MediaSourceId=...&UserId=...:a3f2b8c1...`
///   The POST body (e.g. `DeviceProfile`) differs across device types (iPhone vs Android TV)
///   and directly affects Emby's transcoding compatibility response, so it must be part of the key.
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
}

pub const CACHEABLE_ROUTES: &[CacheableRoute] = &[
    CacheableRoute {
        pattern: r"(?i)^/(?:emby/)?Items/[^/]+/PlaybackInfo",
        methods: &["POST"],
        ttl_seconds: 7200, // 2 hours
        description: "Playback info — Emby processing takes ~1400ms",
    },
    CacheableRoute {
        pattern: r"(?i)^/(?:emby/)?Shows/NextUp",
        methods: &["GET"],
        ttl_seconds: 7200, // 2 hours
        description: "Next-up episode for a series",
    },
    CacheableRoute {
        pattern: r"(?i)^/(?:emby/)?Shows/[^/]+/Episodes",
        methods: &["GET"],
        ttl_seconds: 7200, // 2 hours
        description: "Episode list for a series season",
    },
];

pub struct CompiledCacheableRoute {
    pub regex: Regex,
    pub methods: &'static [&'static str],
    pub ttl_seconds: u64,
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
