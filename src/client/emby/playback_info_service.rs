use std::{sync::Arc, time::Instant};

use hyper::Method;

use crate::{
    AppState, PLAYBACK_INFO_LOGGER_DOMAIN, api::PlaybackInfo,
    core::frontend::types::InfuseAuthorization, debug_log, info_log,
    network::HttpMethod, util::StringUtil, warn_log,
};

const SLOW_PLAYBACK_INFO_FETCH_THRESHOLD_MS: u128 = 500;
const PLAYBACK_INFO_CACHE_KEY_PREFIX: &str = "playback:info";
const PLAYBACK_INFO_METHOD_SEGMENT: &str = "method";
const PLAYBACK_INFO_ITEM_ID_SEGMENT: &str = "item_id";
const PLAYBACK_INFO_MEDIA_SOURCE_ID_SEGMENT: &str = "media_source_id";
const PLAYBACK_INFO_CONTENT_TYPE_HASH_SEGMENT: &str = "content_type_hash";
const PLAYBACK_INFO_BODY_HASH_SEGMENT: &str = "body_hash";
const PLAYBACK_INFO_ITEMS_SEGMENT: &str = "Items";
const PLAYBACK_INFO_PATH_SEGMENT: &str = "PlaybackInfo";
const PLAYBACK_INFO_MEDIA_SOURCE_ID_QUERY_KEY: &str = "MediaSourceId";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlaybackInfoRequest {
    pub item_id: String,
    pub media_source_id: String,
    pub method: HttpMethod,
    pub body: Option<Vec<u8>>,
    pub content_type: Option<String>,
}

impl PlaybackInfoRequest {
    pub fn new(
        item_id: impl Into<String>,
        media_source_id: impl Into<String>,
        method: HttpMethod,
        body: Option<Vec<u8>>,
        content_type: Option<String>,
    ) -> Self {
        Self {
            item_id: item_id.into(),
            media_source_id: media_source_id.into(),
            method,
            body,
            content_type,
        }
    }

    pub fn cache_key(&self) -> Result<String, PlaybackInfoServiceError> {
        let item_id = self.item_id.trim();
        if item_id.is_empty() {
            return Err(PlaybackInfoServiceError::InvalidItemId);
        }

        let media_source_id = self.media_source_id.trim();
        if media_source_id.is_empty() {
            return Err(PlaybackInfoServiceError::InvalidMediaSourceId);
        }

        let method = self.method.to_string().to_ascii_lowercase();
        let mut key = format!(
            "{PLAYBACK_INFO_CACHE_KEY_PREFIX}:{PLAYBACK_INFO_METHOD_SEGMENT}:{method}:{PLAYBACK_INFO_ITEM_ID_SEGMENT}:{}:{PLAYBACK_INFO_MEDIA_SOURCE_ID_SEGMENT}:{}",
            item_id.to_ascii_lowercase(),
            media_source_id.to_ascii_lowercase()
        );

        if matches!(self.method, HttpMethod::Post) {
            let body_hash = self
                .body
                .as_deref()
                .map(StringUtil::hash_bytes)
                .unwrap_or_default();
            let content_type_hash = self
                .content_type
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| StringUtil::hash_hex(&value.to_ascii_lowercase()))
                .unwrap_or_default();
            key.push_str(&format!(
                ":{PLAYBACK_INFO_CONTENT_TYPE_HASH_SEGMENT}:{content_type_hash}:{PLAYBACK_INFO_BODY_HASH_SEGMENT}:{body_hash}"
            ));
        }

        Ok(key)
    }

    pub fn from_http_parts(
        path: &str,
        query: Option<&str>,
        method: &Method,
        body: Option<&[u8]>,
        content_type: Option<&str>,
    ) -> Result<Self, PlaybackInfoServiceError> {
        let item_id = Self::item_id_from_path(path)
            .ok_or(PlaybackInfoServiceError::InvalidItemId)?;
        let media_source_id = Self::media_source_id_from_query(query)
            .ok_or(PlaybackInfoServiceError::InvalidMediaSourceId)?;
        let method = match *method {
            Method::GET => HttpMethod::Get,
            Method::POST => HttpMethod::Post,
            _ => return Err(PlaybackInfoServiceError::UnsupportedMethod),
        };
        Ok(Self::new(
            item_id,
            media_source_id,
            method,
            body.map(|bytes| bytes.to_vec()),
            content_type.map(str::to_string),
        ))
    }

    fn item_id_from_path(path: &str) -> Option<String> {
        let segments: Vec<&str> = path
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect();

        segments
            .windows(3)
            .find(|window| {
                window.first().is_some_and(|segment| {
                    segment.eq_ignore_ascii_case(PLAYBACK_INFO_ITEMS_SEGMENT)
                }) && window.get(2).is_some_and(|segment| {
                    segment.eq_ignore_ascii_case(PLAYBACK_INFO_PATH_SEGMENT)
                })
            })
            .and_then(|window| window.get(1))
            .map(|segment| (*segment).to_string())
    }

    fn media_source_id_from_query(query: Option<&str>) -> Option<String> {
        query.and_then(|query_str| {
            form_urlencoded::parse(query_str.as_bytes())
                .find(|(key, _)| {
                    key.eq_ignore_ascii_case(
                        PLAYBACK_INFO_MEDIA_SOURCE_ID_QUERY_KEY,
                    )
                })
                .map(|(_, value)| value.into_owned())
        })
    }
}

#[derive(Debug)]
pub enum PlaybackInfoServiceError {
    InvalidItemId,
    InvalidMediaSourceId,
    UnsupportedMethod,
    EmptyApiToken,
    Upstream(anyhow::Error),
}

impl std::fmt::Display for PlaybackInfoServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidItemId => write!(f, "invalid playback info item id"),
            Self::InvalidMediaSourceId => {
                write!(f, "invalid playback info media source id")
            }
            Self::UnsupportedMethod => {
                write!(f, "unsupported playback info method")
            }
            Self::EmptyApiToken => write!(f, "empty playback info api token"),
            Self::Upstream(error) => {
                write!(f, "playback info upstream: {error}")
            }
        }
    }
}

impl std::error::Error for PlaybackInfoServiceError {}

#[derive(Clone)]
pub struct PlaybackInfoService {
    state: Arc<AppState>,
}

impl PlaybackInfoService {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    pub async fn get(
        &self,
        request: &PlaybackInfoRequest,
        api_token: Option<&str>,
    ) -> Result<PlaybackInfo, PlaybackInfoServiceError> {
        let cache_key = request.cache_key()?;
        let cache = self.state.get_playback_info_cache().await;

        if let Some(cached) = cache.get::<PlaybackInfo>(&cache_key) {
            info_log!(
                PLAYBACK_INFO_LOGGER_DOMAIN,
                "playback_info_cache_hit key={}",
                cache_key
            );
            return Ok(cached);
        }

        let lock = AppState::request_lock(
            &self.state.playback_info_request_locks,
            &cache_key,
        );

        let result = {
            let wait_start = Instant::now();
            let _guard = lock.lock().await;
            let wait_ms = wait_start.elapsed().as_millis();

            if let Some(cached) = cache.get::<PlaybackInfo>(&cache_key) {
                info_log!(
                    PLAYBACK_INFO_LOGGER_DOMAIN,
                    "playback_info_inflight_wait_hit key={} lock_wait_ms={}",
                    cache_key,
                    wait_ms
                );
                Ok(cached)
            } else {
                let token = api_token
                    .map(str::trim)
                    .filter(|token| !token.is_empty())
                    .ok_or(PlaybackInfoServiceError::EmptyApiToken)?;

                let fetch_start = Instant::now();
                let playback_info =
                    self.fetch_from_emby(request, token).await?;
                let fetch_ms = fetch_start.elapsed().as_millis();

                if fetch_ms >= SLOW_PLAYBACK_INFO_FETCH_THRESHOLD_MS {
                    warn_log!(
                        PLAYBACK_INFO_LOGGER_DOMAIN,
                        "playback_info_fetch_slow item_id={} media_source_id={} \
                         elapsed_ms={}",
                        request.item_id,
                        request.media_source_id,
                        fetch_ms
                    );
                } else {
                    debug_log!(
                        PLAYBACK_INFO_LOGGER_DOMAIN,
                        "playback_info_fetch_complete item_id={} media_source_id={} \
                         elapsed_ms={}",
                        request.item_id,
                        request.media_source_id,
                        fetch_ms
                    );
                }

                cache.insert(cache_key.clone(), playback_info.clone());
                info_log!(
                    PLAYBACK_INFO_LOGGER_DOMAIN,
                    "playback_info_cache_store key={} media_sources={}",
                    cache_key,
                    playback_info.media_sources.len()
                );

                Ok(playback_info)
            }
        };

        AppState::cleanup_request_lock(
            &self.state.playback_info_request_locks,
            &cache_key,
            &lock,
        );

        result
    }

    pub fn api_token_from_headers_and_query(
        headers: &hyper::HeaderMap,
        query: Option<&str>,
    ) -> Option<String> {
        query
            .and_then(Self::api_token_from_query)
            .or_else(|| Self::api_token_from_headers(headers))
    }

    async fn fetch_from_emby(
        &self,
        request: &PlaybackInfoRequest,
        api_token: &str,
    ) -> Result<PlaybackInfo, PlaybackInfoServiceError> {
        let config = self.state.get_config().await;
        let emby_server_url = config.emby.get_uri().to_string();
        let emby_client = self.state.get_emby_client().await.clone();

        emby_client
            .playback_info(emby_server_url, api_token.to_string(), request)
            .await
            .map_err(PlaybackInfoServiceError::Upstream)
    }

    fn api_token_from_query(query: &str) -> Option<String> {
        form_urlencoded::parse(query.as_bytes())
            .find(|(key, _)| {
                key.eq_ignore_ascii_case("api_key")
                    || key.eq_ignore_ascii_case("X-Emby-Token")
            })
            .map(|(_, value)| value.into_owned())
    }

    fn api_token_from_headers(headers: &hyper::HeaderMap) -> Option<String> {
        headers
            .get("X-Emby-Token")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string)
            .or_else(|| {
                headers
                    .get("x-emby-authorization")
                    .and_then(|value| value.to_str().ok())
                    .and_then(InfuseAuthorization::from_header_str)
                    .and_then(|auth| auth.get("MediaBrowser Token"))
            })
    }
}

#[cfg(test)]
mod tests {
    use hyper::Method;

    use super::PlaybackInfoRequest;
    use crate::network::HttpMethod;

    #[test]
    fn playback_info_request_parses_get_path() {
        let request = PlaybackInfoRequest::from_http_parts(
            "/emby/Items/249971/PlaybackInfo",
            Some("MediaSourceId=abc123&UserId=u1"),
            &Method::GET,
            None,
            None,
        );

        assert!(request.is_ok());
        if let Ok(request) = request {
            assert_eq!(request.item_id, "249971");
            assert_eq!(request.media_source_id, "abc123");
        }
    }

    #[test]
    fn playback_info_request_accepts_path_without_emby_prefix() {
        let request = PlaybackInfoRequest::from_http_parts(
            "/Items/249971/PlaybackInfo",
            Some("MediaSourceId=abc123"),
            &Method::GET,
            None,
            None,
        );

        assert!(request.is_ok());
        if let Ok(request) = request {
            assert_eq!(request.item_id, "249971");
            assert_eq!(request.media_source_id, "abc123");
        }
    }

    #[test]
    fn playback_info_cache_key_includes_method() {
        let request = PlaybackInfoRequest::new(
            "249971",
            "ABC123",
            HttpMethod::Get,
            None,
            None,
        );
        let cache_key = request.cache_key();

        assert!(cache_key.is_ok());
        assert_eq!(
            cache_key.unwrap_or_default(),
            "playback:info:method:get:item_id:249971:media_source_id:abc123"
        );
    }

    #[test]
    fn playback_info_cache_key_for_post_includes_body_hash() {
        let request = PlaybackInfoRequest::new(
            "249971",
            "ABC123",
            HttpMethod::Post,
            Some(br#"{"AutoOpenLiveStream":true}"#.to_vec()),
            Some("application/json".to_string()),
        );

        let cache_key = request.cache_key();

        assert!(cache_key.is_ok());
        let cache_key = cache_key.unwrap_or_default();
        assert!(cache_key.starts_with(
            "playback:info:method:post:item_id:249971:media_source_id:abc123"
        ));
        assert!(cache_key.contains(":content_type_hash:"));
        assert!(cache_key.contains(":body_hash:"));
    }
}
