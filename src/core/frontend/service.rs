use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use form_urlencoded;
use hyper::{
    StatusCode, Uri,
    header::{self, HeaderMap},
};
use reqwest::Url;
use tokio::fs::{self as TokioFS, metadata as TokioMetadata};
use tokio::sync::{Mutex as TokioMutex, OnceCell};

use super::types::{
    ForwardConfig, ForwardInfo, InfuseAuthorization, PathParams,
};
use crate::{
    AppState, FORWARD_LOGGER_DOMAIN, debug_log, error_log, info_log, warn_log,
};
use crate::{
    client::{
        PlaybackInfoRequest, PlaybackInfoService, PlaybackInfoServiceError,
    },
    core::{
        error::Error as AppForwardError, redirect_info::RedirectInfo,
        request::Request as AppForwardRequest, sign::Sign,
        sign_encryptor::SignEncryptor,
    },
    util::{StringUtil, UriExt, UriExtError},
};

const MAX_STRM_FILE_SIZE: u64 = 1024 * 1024;
const SLOW_FRONTEND_ROUTING_THRESHOLD_MS: u128 = 200;

#[async_trait]
pub trait ForwardService: Send + Sync {
    async fn handle_request(
        &self,
        request: AppForwardRequest,
        path_params: PathParams,
    ) -> Result<RedirectInfo, StatusCode>;
}

pub struct AppForwardService {
    state: Arc<AppState>,
    config: OnceCell<Arc<ForwardConfig>>,
}

impl AppForwardService {
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            state,
            config: OnceCell::new(),
        }
    }

    async fn get_emby_api_token(
        &self,
        request: &AppForwardRequest,
        use_fallback_key: bool,
    ) -> String {
        if let Some(token) = request.uri.query().and_then(|q| {
            form_urlencoded::parse(q.as_bytes())
                .find(|(k, _)| {
                    ["api_key", "X-Emby-Token"]
                        .iter()
                        .any(|&s| k.eq_ignore_ascii_case(s))
                })
                .map(|(_, v)| v.into_owned())
        }) {
            return token;
        }

        if let Some(token) = request
            .original_headers
            .get("X-Emby-Token")
            .and_then(|v| v.to_str().ok())
        {
            return token.to_owned();
        }

        if let Some(token) = request
            .original_headers
            .get("x-emby-authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(InfuseAuthorization::from_header_str)
            .and_then(|auth| {
                auth.get("MediaBrowser Token").or_else(|| auth.get("Token"))
            })
            .filter(|id| !id.is_empty())
        {
            return token.to_owned();
        }

        if use_fallback_key {
            self.get_forward_config().await.emby_api_key.clone()
        } else {
            String::new()
        }
    }

    async fn get_device_id(&self, request: &AppForwardRequest) -> String {
        if let Some(device_id) = request.uri.query().and_then(|q| {
            form_urlencoded::parse(q.as_bytes())
                .find(|(k, _)| {
                    ["DeviceId"].iter().any(|&s| k.eq_ignore_ascii_case(s))
                })
                .map(|(_, v)| v.into_owned())
        }) {
            return device_id;
        }

        if let Some(device_id) = request
            .original_headers
            .get("DeviceId")
            .and_then(|v| v.to_str().ok())
        {
            return device_id.to_owned();
        }

        if let Some(device_id) = request
            .original_headers
            .get("x-emby-authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(InfuseAuthorization::from_header_str)
            .and_then(|auth| auth.get("DeviceId"))
            .filter(|id| !id.is_empty())
        {
            return device_id.to_owned();
        }

        self.get_emby_api_token(request, false).await
    }

    async fn get_forward_info(
        &self,
        path_params: &PathParams,
        request: &AppForwardRequest,
    ) -> Result<ForwardInfo, AppForwardError> {
        let emby_token = self.get_emby_api_token(request, true).await;
        if emby_token.is_empty() {
            return Err(AppForwardError::EmptyEmbyToken);
        }

        let device_id = self.get_device_id(request).await;
        if device_id.is_empty() {
            return Err(AppForwardError::EmptyEmbyDeviceId);
        }

        let playback_info_service =
            PlaybackInfoService::new(self.state.clone());
        let playback_info = playback_info_service
            .get(
                &PlaybackInfoRequest::new(
                    path_params.item_id.clone(),
                    path_params.media_source_id.clone(),
                ),
                Some(emby_token.as_str()),
            )
            .await
            .map_err(|e| {
                error_log!(
                    FORWARD_LOGGER_DOMAIN,
                    "Failed to fetch playback info: {:?}",
                    match e {
                        PlaybackInfoServiceError::Upstream(upstream) =>
                            upstream,
                        other => anyhow::anyhow!(other.to_string()),
                    }
                );
                AppForwardError::EmbyPathRequestError
            })?;

        let forward_info = playback_info
            .find_media_source_path_by_id(&path_params.media_source_id)
            .map(|path| ForwardInfo {
                item_id: path_params.item_id.clone(),
                media_source_id: path_params.media_source_id.clone(),
                path: path.to_string(),
                device_id,
            })
            .ok_or_else(|| {
                error_log!(
                    FORWARD_LOGGER_DOMAIN,
                    "Media source not found: {}",
                    path_params.media_source_id
                );
                AppForwardError::EmbyPathParserError
            })?;

        Ok(forward_info)
    }

    async fn get_signed_uri(
        &self,
        forward_info: &ForwardInfo,
    ) -> Result<Uri, AppForwardError> {
        let sign_value = self.get_encrypt_sign(forward_info).await?;
        let config = self.get_forward_config().await;

        debug_log!(
            FORWARD_LOGGER_DOMAIN,
            "Get signed url by backend_url: {:?}",
            &config.backend_url
        );
        let mut url = Url::parse(&config.backend_url)
            .map_err(|_| AppForwardError::InvalidUri)?;

        url.query_pairs_mut()
            .append_pair("sign", &sign_value)
            .append_pair("device_id", &forward_info.device_id);

        let url_str = url.as_str();
        debug_log!(
            FORWARD_LOGGER_DOMAIN,
            "Get signed url by url str: {:?}",
            url_str
        );

        url_str.parse().map_err(|_| AppForwardError::InvalidUri)
    }

    async fn get_encrypt_sign(
        &self,
        params: &ForwardInfo,
    ) -> Result<String, AppForwardError> {
        let sign = self.get_sign(params).await?;
        debug_log!(FORWARD_LOGGER_DOMAIN, "Ready to encrypt sign: {:?}", sign);

        SignEncryptor::encrypt(&sign, &self.state).await
    }

    async fn get_sign(
        &self,
        params: &ForwardInfo,
    ) -> Result<Sign, AppForwardError> {
        let encrypt_cache = self.state.get_encrypt_cache().await;
        let cache_key = Self::encrypt_key(params)?;

        if let Some(sign) = encrypt_cache.get(&cache_key) {
            debug_log!(
                FORWARD_LOGGER_DOMAIN,
                "sign_encrypt_cache_hit key={} item_id={} media_source_id={} \
                 sign={:?}",
                cache_key,
                params.item_id,
                params.media_source_id,
                sign
            );
            return Ok(sign);
        }

        let mut path = self.reparse_if_strm(params.path.as_str()).await?;
        path = self.rewrite_if_needed(path.as_str()).await;
        debug_log!(FORWARD_LOGGER_DOMAIN, "Sign path: {:?}", path);

        let config = self.get_forward_config().await;
        let uri = self.create_uri(&mut path, config)?;

        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let expired_at = now + self.get_forward_config().await.expired_seconds;
        let sign = Sign {
            uri: Some(uri.clone()),
            expired_at: Some(expired_at),
        };

        debug_log!(
            FORWARD_LOGGER_DOMAIN,
            "sign_encrypt_ready key={} item_id={} media_source_id={} \
             sign={:?} path={:?} expired_at={:?}",
            cache_key,
            params.item_id,
            params.media_source_id,
            sign,
            uri.to_string(),
            expired_at
        );

        encrypt_cache.insert(cache_key.clone(), sign.clone());
        info_log!(
            FORWARD_LOGGER_DOMAIN,
            "sign_encrypt_cache_store key={} item_id={} media_source_id={}",
            cache_key,
            params.item_id,
            params.media_source_id
        );

        Ok(sign)
    }

    fn create_uri(
        &self,
        path: &mut String,
        config: Arc<ForwardConfig>,
    ) -> Result<Uri, AppForwardError> {
        let uri = {
            let initial_result = if config.check_file_existence {
                Uri::from_path_or_url(&path).or_else(|e| match e {
                    UriExtError::FileNotFound(original_path) => config
                        .fallback_video_path
                        .as_ref()
                        .map(|fallback_path| {
                            info_log!(
                                FORWARD_LOGGER_DOMAIN,
                                "File not found: '{}'. Using fallback: '{}'",
                                original_path,
                                fallback_path
                            );
                            Uri::from_path_or_url(fallback_path)
                        })
                        .unwrap_or(Err(UriExtError::FileNotFound(
                            original_path,
                        ))),
                    e => Err(e),
                })
            } else {
                Uri::force_from_path_or_url(&path)
            };

            initial_result.map_err(|e| match e {
                UriExtError::FileNotFound(p) => {
                    AppForwardError::FileNotFound(p)
                }
                UriExtError::InvalidUri => AppForwardError::InvalidUri,
                UriExtError::IoError(io_err) => {
                    AppForwardError::IoError(io_err)
                }
            })?
        };
        Ok(uri)
    }

    async fn reparse_if_strm(
        &self,
        path: &str,
    ) -> Result<String, AppForwardError> {
        if !path.ends_with(".strm") {
            return Ok(path.to_string());
        }

        debug_log!(FORWARD_LOGGER_DOMAIN, "Detected strm file: {}", path);

        let strm_cache = self.state.get_strm_file_cache().await;
        let strm_cache_key = Self::strm_cache_key(path)?;

        if let Some(cached_path) = strm_cache.get::<String>(&strm_cache_key) {
            debug_log!(
                FORWARD_LOGGER_DOMAIN,
                "strm_cache_hit key={} resolved_path={:?} path={}",
                strm_cache_key,
                cached_path,
                path
            );
            return Ok(cached_path);
        }

        let strm_mutex = self.strm_request_lock(&strm_cache_key);
        let result = {
            let wait_start = Instant::now();
            let _strm_guard = strm_mutex.lock().await;
            let lock_wait_ms = wait_start.elapsed().as_millis();

            if let Some(cached_path) = strm_cache.get::<String>(&strm_cache_key)
            {
                info_log!(
                    FORWARD_LOGGER_DOMAIN,
                    "strm_inflight_wait_hit key={} lock_wait_ms={} \
                     resolved_path={:?} path={}",
                    strm_cache_key,
                    lock_wait_ms,
                    cached_path,
                    path
                );
                Ok(cached_path)
            } else {
                let file_path = Path::new(path);
                let metadata = TokioMetadata(file_path).await.map_err(|e| {
                    error_log!(
                        FORWARD_LOGGER_DOMAIN,
                        "Failed to get metadata for strm file: {} (error: {})",
                        path,
                        e
                    );
                    AppForwardError::StrmFileIoError(e.to_string())
                })?;

                if metadata.len() == 0 {
                    error_log!(
                        FORWARD_LOGGER_DOMAIN,
                        "Empty strm file: {}",
                        path
                    );
                    Err(AppForwardError::EmptyStrmFile)
                } else if metadata.len() > MAX_STRM_FILE_SIZE {
                    error_log!(
                        FORWARD_LOGGER_DOMAIN,
                        "Strm file too large ({} > {}): {}",
                        metadata.len(),
                        MAX_STRM_FILE_SIZE,
                        path
                    );
                    Err(AppForwardError::StrmFileTooLarge)
                } else {
                    let content = TokioFS::read_to_string(file_path)
                        .await
                        .map_err(|e| {
                            error_log!(
                                FORWARD_LOGGER_DOMAIN,
                                "Failed to read strm file: {} (error: {})",
                                path,
                                e
                            );
                            AppForwardError::StrmFileIoError(e.to_string())
                        })?
                        .trim()
                        .to_string();

                    debug_log!(
                        FORWARD_LOGGER_DOMAIN,
                        "strm_read_complete key={} path={} content={}",
                        strm_cache_key,
                        path,
                        content
                    );

                    strm_cache.insert(strm_cache_key.clone(), content.clone());
                    info_log!(
                        FORWARD_LOGGER_DOMAIN,
                        "strm_cache_store key={} path={}",
                        strm_cache_key,
                        path
                    );
                    Ok(content)
                }
            }
        };

        AppState::cleanup_request_lock(
            &self.state.strm_request_locks,
            &strm_cache_key,
            &strm_mutex,
        );

        result
    }

    async fn rewrite_if_needed(&self, path: &str) -> String {
        let path_rewrites = self.state.get_frontend_path_rewrite_cache().await;

        if path_rewrites.is_empty() {
            debug_log!(
                FORWARD_LOGGER_DOMAIN,
                "Frontend path rewriting is empty. Skipping step."
            );
            return path.into();
        }

        debug_log!(FORWARD_LOGGER_DOMAIN, "Starting frontend path rewrite.");

        let mut current_uri_str: Cow<str> = Cow::Borrowed(path);
        for path_rewrite in path_rewrites {
            if !path_rewrite.enable {
                continue;
            }
            current_uri_str =
                path_rewrite.rewrite(&current_uri_str).await.into();
        }

        let new_uri_str = current_uri_str.into_owned();

        debug_log!(
            FORWARD_LOGGER_DOMAIN,
            "Frontend path rewrite completed. URI before: {:?}, URI after: {:?}",
            path,
            new_uri_str
        );

        match Uri::force_from_path_or_url(&new_uri_str) {
            Ok(_) => new_uri_str,
            Err(_) => path.into(),
        }
    }

    fn build_redirect_info(
        &self,
        url: Uri,
        original_headers: &HeaderMap,
    ) -> RedirectInfo {
        let mut final_headers = original_headers.clone();

        final_headers.remove(header::HOST);

        RedirectInfo {
            target_url: url,
            final_headers,
        }
    }

    fn encrypt_key(params: &ForwardInfo) -> Result<String, AppForwardError> {
        if params.item_id.is_empty() || params.media_source_id.is_empty() {
            return Err(AppForwardError::InvalidMediaSource);
        }

        let item_id = params.item_id.to_ascii_lowercase();
        let media_source_id = params.media_source_id.to_ascii_lowercase();

        Ok(format!(
            "forward:sign_encrypt:item_id:{item_id}:media_source_id:{media_source_id}"
        ))
    }

    fn strm_cache_key(path: &str) -> Result<String, AppForwardError> {
        if path.is_empty() {
            return Err(AppForwardError::InvalidStrmFile);
        }
        let path_hash = StringUtil::md5(&path.to_lowercase());
        Ok(format!("frontend:strm:path_md5:{path_hash}"))
    }

    fn strm_request_lock(&self, cache_key: &str) -> Arc<TokioMutex<()>> {
        AppState::request_lock(&self.state.strm_request_locks, cache_key)
    }

    async fn get_forward_config(&self) -> Arc<ForwardConfig> {
        let config_arc =
            self.config
                .get_or_init(|| async {
                    let config = self.state.get_config().await;

                    let frontend = config.frontend.as_ref().expect(
                        "Attempted to access frontend config, but frontend is not configured",
                    );
                    let backend = config.backend.as_ref().expect(
                        "Attempted to access backend config, but backend is not configured",
                    );

                    let fallback_video_path = Some(&config.fallback.video_missing_path)
                        .filter(|p| !p.is_empty())
                        .map(PathBuf::from)
                        .map(|path| {
                            if path.is_absolute() {
                                path
                            } else {
                                config.path.parent().unwrap_or_else(|| Path::new("")).join(path)
                            }
                        })
                        .filter(|path| path.exists())
                        .map(|path| path.to_string_lossy().into_owned());

                    let (_, ttl) = self.state.get_cache_settings().await;

                    Arc::new(ForwardConfig {
                        expired_seconds: ttl,
                        backend_url: backend.uri().to_string(),
                        crypto_key: config.general.encipher_key.clone(),
                        crypto_iv: config.general.encipher_iv.clone(),
                        emby_api_key: config.emby.token.to_string(),
                        check_file_existence: frontend.check_file_existence,
                        fallback_video_path,
                    })
                })
                .await;

        config_arc.clone()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use dashmap::DashMap;
    use tokio::sync::Mutex as TokioMutex;

    use super::AppForwardService;
    use crate::AppState;

    #[test]
    fn strm_cache_key_is_structured() {
        let key = AppForwardService::strm_cache_key("/mnt/media/episode.strm");

        assert!(key.is_ok());
        assert!(
            key.unwrap_or_default()
                .starts_with("frontend:strm:path_md5:")
        );
    }

    #[test]
    fn strm_cache_key_rejects_empty_path() {
        let key = AppForwardService::strm_cache_key("");

        assert!(key.is_err());
    }

    #[tokio::test]
    async fn strm_request_lock_reuses_same_key_mutex() {
        let locks = DashMap::<String, Arc<TokioMutex<()>>>::new();
        let key = "frontend:strm:path_md5:abc";

        let lock1 = AppState::request_lock(&locks, key);
        let lock2 = AppState::request_lock(&locks, key);

        assert!(Arc::ptr_eq(&lock1, &lock2));

        let _guard = lock1.lock().await;
        assert!(lock2.try_lock().is_err());
    }

    #[test]
    fn strm_request_lock_separates_distinct_keys() {
        let locks = DashMap::<String, Arc<TokioMutex<()>>>::new();

        let lock1 = AppState::request_lock(&locks, "key1");
        let lock2 = AppState::request_lock(&locks, "key2");

        assert!(!Arc::ptr_eq(&lock1, &lock2));
    }

    #[test]
    fn encrypt_cache_key_is_structured() {
        let params = crate::core::frontend::types::ForwardInfo {
            item_id: "Item-ABC".into(),
            media_source_id: "Media-XYZ".into(),
            path: "/tmp/demo.mkv".into(),
            device_id: "device-1".into(),
        };

        let key = AppForwardService::encrypt_key(&params);

        assert!(key.is_ok());
        assert_eq!(
            key.unwrap_or_default(),
            "forward:sign_encrypt:item_id:item-abc:media_source_id:media-xyz"
        );
    }
}

#[async_trait]
impl ForwardService for AppForwardService {
    async fn handle_request(
        &self,
        request: AppForwardRequest,
        path_params: PathParams,
    ) -> Result<RedirectInfo, StatusCode> {
        let timer = Instant::now();

        let forward_info = self
            .get_forward_info(&path_params, &request)
            .await
            .map_err(|e| {
                error_log!(
                    FORWARD_LOGGER_DOMAIN,
                    "Routing forward info error: {:?}",
                    e
                );
                StatusCode::BAD_REQUEST
            })?;

        info_log!(
            FORWARD_LOGGER_DOMAIN,
            "Start handle request forward info: {:?}",
            forward_info
        );

        let remote_uri =
            self.get_signed_uri(&forward_info)
                .await
                .map_err(|e| match e {
                    AppForwardError::FileNotFound(path) => {
                        error_log!(
                            FORWARD_LOGGER_DOMAIN,
                            "Routing forward signed uri error because of \
                        file missing: {}",
                            path
                        );
                        StatusCode::NOT_FOUND
                    }
                    _ => {
                        error_log!(
                            FORWARD_LOGGER_DOMAIN,
                            "Routing forward signed uri error: {:?}",
                            e
                        );
                        StatusCode::INTERNAL_SERVER_ERROR
                    }
                })?;

        let elapsed_ms = timer.elapsed().as_millis();
        if elapsed_ms >= SLOW_FRONTEND_ROUTING_THRESHOLD_MS {
            warn_log!(
                FORWARD_LOGGER_DOMAIN,
                "frontend_routing_slow elapsed_ms={} file_path={} \
                item_id={} media_source_id={}",
                elapsed_ms,
                forward_info.path,
                forward_info.item_id,
                forward_info.media_source_id
            );
        } else {
            info_log!(
                FORWARD_LOGGER_DOMAIN,
                "frontend_routing_complete elapsed_ms={} file_path={}",
                elapsed_ms,
                forward_info.path
            );
        }

        Ok(self.build_redirect_info(remote_uri, &request.original_headers))
    }
}
