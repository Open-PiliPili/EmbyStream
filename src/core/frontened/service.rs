use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use http_serde::http::StatusCode;
use hyper::Uri;

use super::types::{ForwardInfo, PathParams};
use crate::{AppState, CryptoInput, CryptoOperation, CryptoOutput, crypto::Crypto, sign::Sign};
use crate::{
    core::{
        error::Error as AppForwardError, redirect_info::RedirectInfo,
        request::Request as AppForwardRequest,
    },
    util::StringUtil,
};

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
}

impl AppForwardService {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    pub async fn get_signed_url(&self, forward_info: &ForwardInfo) -> Result<Uri, AppForwardError> {
        let sign_value = self.encrypt_sign(forward_info).await?;
        let proxy_mode = self.get_proxy_mode();

        let query_params = [("sign", sign_value), ("proxy_mode", proxy_mode.into())];

        let backend_base_url = self.get_backend_base_url();
        let backend_forward_path = self.get_backend_forward_path();

        let url_str = format!(
            "{}/{}?{}",
            backend_base_url,
            backend_forward_path,
            query_params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&")
        );

        url_str.parse().map_err(|_| AppForwardError::InvalidUri)
    }

    async fn encrypt_sign(&self, params: &ForwardInfo) -> Result<String, AppForwardError> {
        let encrypt_map = self.get_sign(params).await?.to_map();
        let crypto_result = Crypto::execute(
            CryptoOperation::Encrypt,
            CryptoInput::Dictionary(encrypt_map),
            "key", // TODO: Replace with real key
            "iv",  // TODO: Replace with real IV
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

        let path = self.reparse_if_strm(params.path.as_str())?;
        let uri = path.parse().map_err(|_| AppForwardError::InvalidUri)?;
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let expired_at = now + self.get_expired_seconds();

        Ok(Sign {
            uri: Some(uri),
            expired_at: Some(expired_at),
        })
    }

    fn reparse_if_strm(&self, path: &str) -> Result<String, AppForwardError> {
        // TODO: implement read strm content if strm file
        Ok(path.to_string())
    }

    fn encrypt_key(&self, params: &ForwardInfo) -> Result<String, AppForwardError> {
        if params.item_id.is_empty() || params.media_source_id.is_empty() {
            return Err(AppForwardError::InvalidMediaSource);
        }

        let input = format!("{}:{}", params.item_id, params.media_source_id).to_lowercase();

        Ok(StringUtil::md5(&input))
    }

    fn get_expired_seconds(&self) -> u64 {
        // TODO: implement by state config later
        3600
    }

    fn get_backend_base_url(&self) -> &str {
        // TODO: implement by state config later
        ""
    }

    fn get_backend_forward_path(&self) -> &str {
        // TODO: implement by state config later
        "stream"
    }

    fn get_proxy_mode(&self) -> &str {
        // TODO: implement by state config later
        "proxy"
    }
}

#[async_trait]
impl ForwardService for AppForwardService {
    async fn handle_request(
        &self,
        request: AppForwardRequest,
        path_params: PathParams,
    ) -> Result<RedirectInfo, StatusCode> {
        todo!("implement forward service later")
    }
}
