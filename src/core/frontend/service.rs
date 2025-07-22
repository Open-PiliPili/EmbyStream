use std::{
    path::Path,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use form_urlencoded;
use hyper::{
    StatusCode, Uri,
    header::{self, HeaderMap},
};
use reqwest::Url;
use tokio::fs::{self as TokioFS, metadata as TokioMetadata};
use tokio::sync::OnceCell;

use super::types::{ForwardConfig, ForwardInfo, PathParams};
use crate::{AppState, FORWARD_LOGGER_DOMAIN, debug_log, error_log};
use crate::{
    client::{ClientBuilder, EmbyClient},
    core::{
        error::Error as AppForwardError, redirect_info::RedirectInfo,
        request::Request as AppForwardRequest, sign::Sign,
    },
    crypto::{Crypto, CryptoInput, CryptoOperation, CryptoOutput},
    network::CurlPlugin,
    util::{PathRewriter, StringUtil, UriExt, UriExtError},
};

const MAX_STRM_FILE_SIZE: u64 = 1024 * 1024;

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

    async fn get_emby_api_token(&self, request: &AppForwardRequest) -> String {
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

        self.get_forward_config().await.emby_api_key.clone()
    }

    async fn get_forward_info(
        &self,
        path_params: &PathParams,
        request: &AppForwardRequest,
    ) -> Result<ForwardInfo, AppForwardError> {
        let forward_info_cache = self.state.get_forward_info_cache().await;
        let cache_key = self.forward_info_key(&path_params)?;
        if let Some(cached_forward_info) = forward_info_cache.get(&cache_key) {
            debug_log!(
                FORWARD_LOGGER_DOMAIN,
                "Forward info cache hit {:?}",
                cached_forward_info
            );
            return Ok(cached_forward_info);
        }

        let config = self.get_forward_config().await;
        let emby_token = self.get_emby_api_token(request).await;
        if emby_token.is_empty() {
            return Err(AppForwardError::EmptyEmbyToken);
        }

        let emby_client = ClientBuilder::<EmbyClient>::new()
            .with_plugin(CurlPlugin)
            .build();

        let playback_info = emby_client
            .playback_info(
                &config.emby_server_url,
                &emby_token,
                &path_params.item_id,
                &path_params.media_source_id,
            )
            .await
            .map_err(|e| {
                error_log!(
                    FORWARD_LOGGER_DOMAIN,
                    "Failed to fetch playback info: {:?}",
                    e
                );
                AppForwardError::EmbyPathRequestError
            })?;

        let forward_info = playback_info
            .find_media_source_path_by_id(&path_params.media_source_id)
            .map(|path| ForwardInfo {
                item_id: path_params.item_id.clone(),
                media_source_id: path_params.media_source_id.clone(),
                path: path.to_string(),
            })
            .ok_or_else(|| {
                error_log!(
                    FORWARD_LOGGER_DOMAIN,
                    "Media source not found: {}",
                    path_params.media_source_id
                );
                AppForwardError::EmbyPathParserError
            })?;

        forward_info_cache.insert(cache_key, forward_info.clone());
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
            .append_pair("proxy_mode", &config.proxy_mode);

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
        let encrypt_map = self.get_sign(params).await?.to_map();
        debug_log!(
            FORWARD_LOGGER_DOMAIN,
            "Ready to encrypt sign map: {:?}",
            encrypt_map
        );

        let config = self.get_forward_config().await;
        let crypto_result = Crypto::execute(
            CryptoOperation::Encrypt,
            CryptoInput::Dictionary(encrypt_map),
            &config.crypto_key,
            &config.crypto_iv,
        )
        .map_err(AppForwardError::CommonError)?;

        match crypto_result {
            CryptoOutput::Encrypted(sign_value) => Ok(sign_value),
            CryptoOutput::Dictionary(_) => {
                Err(AppForwardError::EncryptSignatureFailed)
            }
        }
    }

    async fn get_sign(
        &self,
        params: &ForwardInfo,
    ) -> Result<Sign, AppForwardError> {
        let encrypt_cache = self.state.get_encrypt_cache().await;
        let cache_key = self.encrypt_key(&params)?;

        if let Some(sign) = encrypt_cache.get(&cache_key) {
            debug_log!(FORWARD_LOGGER_DOMAIN, "Sign cache hit: {:?}", sign);
            return Ok(sign);
        }

        let mut path = self.reparse_if_strm(params.path.as_str()).await?;
        path = self.rewrite_if_needed(path.as_str()).await;
        debug_log!(FORWARD_LOGGER_DOMAIN, "Sign path: {:?}", path);

        let uri: Uri = Uri::from_path_or_url(path).map_err(|e| match e {
            UriExtError::FileNotFound(path) => {
                AppForwardError::FileNotFound(path)
            }
            UriExtError::InvalidUri => AppForwardError::InvalidUri,
            UriExtError::IoError(e) => AppForwardError::IoError(e),
        })?;
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

        let expired_at = now + self.get_forward_config().await.expired_seconds;
        let sign = Sign {
            uri: Some(uri.clone()),
            expired_at: Some(expired_at),
        };

        debug_log!(
            FORWARD_LOGGER_DOMAIN,
            "Successfully retrieved sign: {:?} by path: {:?}, expired_at: {:?}",
            sign,
            uri.to_string(),
            expired_at
        );

        encrypt_cache.insert(cache_key, sign.clone());

        Ok(sign)
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
        let strm_cache_key = self.strm_key(path)?;

        if let Some(cached_path) = strm_cache.get::<String>(&strm_cache_key) {
            debug_log!(
                FORWARD_LOGGER_DOMAIN,
                "Strm cache hit: {:?} by path {}",
                cached_path,
                path
            );
            return Ok(cached_path);
        }

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
            error_log!(FORWARD_LOGGER_DOMAIN, "Empty strm file: {}", path);
            return Err(AppForwardError::EmptyStrmFile);
        }

        if metadata.len() > MAX_STRM_FILE_SIZE {
            error_log!(
                FORWARD_LOGGER_DOMAIN,
                "Strm file too large ({} > {}): {}",
                metadata.len(),
                MAX_STRM_FILE_SIZE,
                path
            );
            return Err(AppForwardError::StrmFileTooLarge);
        }

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
            "Read strm file: {} (content: {})",
            path,
            content
        );

        strm_cache.insert(strm_cache_key, content.clone());
        Ok(content)
    }

    async fn rewrite_if_needed(&self, path: &str) -> String {
        let config = self.get_forward_config().await;
        if !config.path_rewrite.is_need_rewrite(path) {
            return path.to_string();
        }

        let rewriter = PathRewriter::new(
            &config.path_rewrite.pattern,
            &config.path_rewrite.replacement,
        );

        rewriter.rewrite(path).await
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

    fn encrypt_key(
        &self,
        params: &ForwardInfo,
    ) -> Result<String, AppForwardError> {
        self.md5_key(&params.item_id, &params.media_source_id)
    }

    fn forward_info_key(
        &self,
        params: &PathParams,
    ) -> Result<String, AppForwardError> {
        self.md5_key(&params.item_id, &params.media_source_id)
    }

    fn md5_key(
        &self,
        item_id: &str,
        media_source_id: &str,
    ) -> Result<String, AppForwardError> {
        if item_id.is_empty() || media_source_id.is_empty() {
            return Err(AppForwardError::InvalidMediaSource);
        }
        let input = format!("{}:{}", item_id, media_source_id).to_lowercase();
        Ok(StringUtil::md5(&input))
    }

    fn strm_key(&self, path: &str) -> Result<String, AppForwardError> {
        if path.is_empty() {
            return Err(AppForwardError::InvalidStrmFile);
        }
        let input = path.to_lowercase();
        Ok(StringUtil::md5(&input))
    }

    async fn get_forward_config(&self) -> Arc<ForwardConfig> {
        let config_arc =
            self.config
                .get_or_init(|| async {
                    let config = self.state.get_config().await;
                    let frontend = config.frontend.as_ref().expect(
                        "Attempted to access forward config, but backend is not configured",
                    );
                    let backend = config.backend.as_ref().expect(
                        "Attempted to access backend config, but backend is not configured",
                    );
                    Arc::new(ForwardConfig {
                        expired_seconds: config.general.expired_seconds,
                        proxy_mode: backend.proxy_mode.clone(),
                        backend_url: backend.uri().to_string(),
                        crypto_key: config.general.encipher_key.clone(),
                        crypto_iv: config.general.encipher_iv.clone(),
                        emby_server_url: config.general.emby_uri().to_string(),
                        emby_api_key: config.general.emby_api_key.to_string(),
                        path_rewrite: frontend.path_rewrite.clone(),
                    })
                })
                .await;

        config_arc.clone()
    }
}

#[async_trait]
impl ForwardService for AppForwardService {
    async fn handle_request(
        &self,
        request: AppForwardRequest,
        path_params: PathParams,
    ) -> Result<RedirectInfo, StatusCode> {
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

        let remote_uri = self
            .get_signed_uri(&forward_info)
            .await
            .map_err(|e| match e {
                AppForwardError::FileNotFound(path) => {
                    error_log!(
                        FORWARD_LOGGER_DOMAIN,
                        "Routing forward signed uri error because of file missing: {}",
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

        Ok(self.build_redirect_info(remote_uri, &request.original_headers))
    }
}
