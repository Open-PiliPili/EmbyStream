use once_cell::sync::Lazy;
use regex::Regex;

use super::types::PathParams;

static VIDEO_PATH_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^/(emby/)?videos/([a-zA-Z0-9_-]+)/(original|stream)(\.[a-zA-Z0-9]+)?$").unwrap()
});

pub fn parse_video_path(path: &str) -> Option<PathParams> {
    VIDEO_PATH_REGEX.captures(path).map(|caps| {
        let item_id = caps.get(2).map_or("", |m| m.as_str()).to_string();
        PathParams { item_id }
    })
}