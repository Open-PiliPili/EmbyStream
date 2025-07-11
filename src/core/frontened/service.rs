use std::{
    path::Path,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use hyper::{
    StatusCode, Uri,
    header::{self, HeaderMap},
};
use once_cell::sync::OnceCell;
use reqwest::Url;
use tokio::fs::{self as TokioFS, metadata as TokioMetadata};
use url::form_urlencoded;

use super::types::{ForwardConfig, ForwardInfo, PathParams};
use crate::{AppState, CryptoInput, CryptoOperation, CryptoOutput, crypto::Crypto, sign::Sign};
use crate::{FORWARD_LOGGER_DOMAIN, error_log, info_log};
use crate::{
    client::{ClientBuilder, EmbyClient},
    core::{
        error::Error as AppForwardError, redirect_info::RedirectInfo,
        request::Request as AppForwardRequest,
    },
    network::CurlPlugin,
    util::StringUtil,
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
    config: OnceCell<ForwardConfig>,
}

impl AppForwardService {
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            state,
            config: OnceCell::new(),
        }
    }

    fn get_emby_api_token(&self, request: &AppForwardRequest) -> String {
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

        self.get_forward_config().emby_api_key.clone()
    }

    async fn get_forward_info(
        &self,
        path_params: &PathParams,
        request: &AppForwardRequest,
    ) -> Result<ForwardInfo, AppForwardError> {
        let config = self.get_forward_config();
        let emby_token = self.get_emby_api_token(request);
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

        playback_info
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
            })
    }

    async fn get_signed_uri(&self, forward_info: &ForwardInfo) -> Result<Uri, AppForwardError> {
        let sign_value = self.get_encrypt_sign(forward_info).await?;
        let config = self.get_forward_config();

        let mut url =
            Url::parse(&config.backend_base_url).map_err(|_| AppForwardError::InvalidUri)?;

        url.path_segments_mut()
            .map_err(|_| AppForwardError::InvalidUri)?
            .push(&config.backend_forward_path);

        url.query_pairs_mut()
            .append_pair("sign", &sign_value)
            .append_pair("proxy_mode", &config.proxy_mode);

        url.as_str()
            .parse()
            .map_err(|_| AppForwardError::InvalidUri)
    }

    async fn get_encrypt_sign(&self, params: &ForwardInfo) -> Result<String, AppForwardError> {
        let encrypt_map = self.get_sign(params).await?.to_map();
        let config = self.get_forward_config();
        let crypto_result = Crypto::execute(
            CryptoOperation::Encrypt,
            CryptoInput::Dictionary(encrypt_map),
            &config.crypto_key,
            &config.crypto_iv,
        )
        .map_err(AppForwardError::CommonError)?;

        match crypto_result {
            CryptoOutput::Encrypted(sign_value) => Ok(sign_value),
            CryptoOutput::Dictionary(_) => Err(AppForwardError::EncryptSignatureFailed),
        }
    }

    async fn get_sign(&self, params: &ForwardInfo) -> Result<Sign, AppForwardError> {
        let encrypt_cache = self.state.get_encrypt_cache().await;
        let cache_key = self.encrypt_key(&params)?;

        if let Some(sign) = encrypt_cache.get(&cache_key) {
            return Ok(sign);
        }

        let path = self.reparse_if_strm(params.path.as_str()).await?;
        let uri = path.parse().map_err(|_| AppForwardError::InvalidUri)?;
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

        let expired_at = now + self.get_forward_config().expired_seconds;
        let sign = Sign {
            uri: Some(uri),
            expired_at: Some(expired_at),
        };

        encrypt_cache.insert(cache_key, sign.clone());

        Ok(sign)
    }

    async fn reparse_if_strm(&self, path: &str) -> Result<String, AppForwardError> {
        if !path.ends_with(".strm") {
            return Ok(path.to_string());
        }

        let strm_cache = self.state.get_strm_file_cache().await;
        let strm_cache_key = self.strm_key(path)?;

        if let Some(cached_path) = strm_cache.get::<String>(&strm_cache_key) {
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

        strm_cache.insert(strm_cache_key, content.clone());
        Ok(content)
    }

    fn build_redirect_info(&self, url: Uri, original_headers: &HeaderMap) -> RedirectInfo {
        let mut final_headers = original_headers.clone();

        final_headers.remove(header::HOST);

        RedirectInfo {
            target_url: url,
            final_headers,
        }
    }

    fn encrypt_key(&self, params: &ForwardInfo) -> Result<String, AppForwardError> {
        if params.item_id.is_empty() || params.media_source_id.is_empty() {
            return Err(AppForwardError::InvalidMediaSource);
        }
        let input = format!("{}:{}", params.item_id, params.media_source_id).to_lowercase();
        Ok(StringUtil::md5(&input))
    }

    fn strm_key(&self, path: &str) -> Result<String, AppForwardError> {
        if path.is_empty() {
            return Err(AppForwardError::InvalidStrmFile);
        }
        let input = path.to_lowercase();
        Ok(StringUtil::md5(&input))
    }

    fn get_forward_config(&self) -> &ForwardConfig {
        todo!("implement by app state later")
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
                error_log!(FORWARD_LOGGER_DOMAIN, "Routing forward info error: {:?}", e);
                StatusCode::BAD_REQUEST
            })?;

        let remote_uri = self.get_signed_uri(&forward_info).await.map_err(|e| {
            error_log!(
                FORWARD_LOGGER_DOMAIN,
                "Routing forward signed uri error: {:?}",
                e
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        Ok(self.build_redirect_info(remote_uri, &request.original_headers))
    }
}
