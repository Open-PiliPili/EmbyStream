use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use reqwest::Url;

use super::types::ForwardInfo;
use crate::{AppState, core::error::Error as AppForwardError};
use crate::{CryptoInput, CryptoOperation, CryptoOutput, crypto::Crypto, sign::Sign};

pub struct ForwardService {
    state: Arc<AppState>,
}

impl ForwardService {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    pub async fn get_signed_url(&self, forward_info: &ForwardInfo) -> Result<Url, AppForwardError> {
        let sign_value = self.encrypt_sign(forward_info).await?;
        let proxy_mode = self.get_proxy_mode();
        let item_id = forward_info.clone().item_id;
        let media_source_id = forward_info.clone().media_source_id;

        let query_params = [
            ("sign", sign_value),
            ("item_id", item_id.into()),
            ("media_source_id", media_source_id.into()),
            ("proxy_mode", proxy_mode.into()),
        ];

        let backend_base_url = self.get_backend_base_url();
        let backend_forward_path = self.get_backend_forward_path();

        let final_url = Url::parse_with_params(
            &format!("{}/{}", backend_base_url, backend_forward_path),
            &query_params,
        )
        .map_err(|_| AppForwardError::InvalidUri)?;

        Ok(final_url)
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

        let key = format!("{}:{}", params.item_id, params.media_source_id);
        Ok(key)
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
