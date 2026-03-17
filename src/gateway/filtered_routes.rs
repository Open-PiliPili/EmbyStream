use once_cell::sync::Lazy;
use regex::Regex;

/// Developer-maintained list of URL path patterns that require User-Agent filtering.
///
/// Only requests matching these patterns will be checked against UA allow/deny rules.
/// All other paths (web UI, API, static assets, etc.) pass through without UA filtering.
///
/// ## How to add a new filtered path
///
/// 1. Add a new pattern string to the `UA_FILTERED_PATTERNS` array below.
/// 2. The pattern is a regex matched against the request URL path (without query params).
/// 3. Typically these are video stream paths that should only be accessed by known players.
const UA_FILTERED_PATTERNS: &[&str] = &[
    // Normal video stream: /videos/{itemId}/stream.mkv, /videos/{itemId}/original, /emby/videos/{itemId}/{name}.m3u8
    r"(?i)^/(?:emby/)?videos/[a-zA-Z0-9_-]+(?:/(?:original|stream)(?:\.[a-zA-Z0-9]+)?|/[a-zA-Z0-9_-]+\.m3u8)$",
    // HLS segments: /videos/{itemId}/hls/{segment}.ts, /videos/{itemId}/hls1/{name}.m3u8
    r"(?i)^/(?:emby/)?videos/[a-zA-Z0-9_-]+/hls\d*/[^/]+(?:/\d+)?\.(?:ts|m3u8)$",
];

pub static COMPILED_UA_FILTERS: Lazy<Vec<Regex>> = Lazy::new(|| {
    UA_FILTERED_PATTERNS
        .iter()
        .filter_map(|p| Regex::new(p).ok())
        .collect()
});
