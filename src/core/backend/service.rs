use std::{path::PathBuf, sync::Arc};

use async_trait::async_trait;
use hyper::{HeaderMap, StatusCode, Uri, header};

use super::{
    local_streamer::LocalStreamer, proxy_mode::ProxyMode, remote_streamer::RemoteStreamer,
    result::Result as AppStreamResult, source::Source,
};
use crate::core::redirect_info::RedirectInfo;
use crate::{AppState, STREAM_LOGGER_DOMAIN, error_log, info_log};
use crate::{
    CryptoInput, CryptoOperation, CryptoOutput,
    core::{error::Error as AppStreamError, request::Request as AppStreamRequest},
    crypto::Crypto,
    sign::{Sign, SignParams},
    util::StringUtil,
};

#[async_trait]
pub trait StreamService: Send + Sync {
    async fn handle_request(
        &self,
        request: AppStreamRequest,
    ) -> Result<AppStreamResult, StatusCode>;
}

pub struct AppStreamService {
    pub state: Arc<AppState>,
    pub user_agent: Option<String>,
}

impl AppStreamService {
    pub fn new(state: Arc<AppState>, user_agent: Option<String>) -> Self {
        Self { state, user_agent }
    }

    async fn decrypt_and_route(
        &self,
        request: &AppStreamRequest,
    ) -> Result<Source, AppStreamError> {
        let params = request
            .uri
            .query()
            .and_then(|query| serde_urlencoded::from_str::<SignParams>(query).ok())
            .unwrap_or_default();

        if params.sign.is_empty() {
            return Err(AppStreamError::EmptySignature);
        }

        let sign = self.decrypt(params.sign.as_str(), &params).await?;

        if !sign.is_valid() {
            return Err(AppStreamError::ExpiredStream);
        }

        let uri = sign.uri.clone().ok_or(AppStreamError::InvalidUri)?;

        if sign.is_local() {
            Ok(Source::Local(PathBuf::from(uri.to_string())))
        } else {
            Ok(Source::Remote {
                url: uri,
                mode: params.proxy_mode,
            })
        }
    }

    fn decrypt_key(&self, params: &SignParams) -> Result<String, AppStreamError> {
        if params.sign.is_empty() {
            return Err(AppStreamError::InvalidEncryptedSignature);
        }

        let input = params.sign.to_lowercase();
        Ok(StringUtil::md5(&input))
    }

    async fn decrypt(&self, sign: &str, params: &SignParams) -> Result<Sign, AppStreamError> {
        let decrypt_cache = self.state.get_decrypt_cache().await;

        let cache_key = self.decrypt_key(params)?;

        if let Some(sign) = decrypt_cache.get(&cache_key) {
            return Ok(sign);
        }

        let crypto_result = Crypto::execute(
            CryptoOperation::Decrypt,
            CryptoInput::Encrypted(sign.to_string()),
            "key", // TODO: Replace with real key
            "iv",  // TODO: Replace with real IV
        )
        .map_err(AppStreamError::CommonError)?;

        match crypto_result {
            CryptoOutput::Encrypted(_) => Err(AppStreamError::InvalidEncryptedSignature),
            CryptoOutput::Dictionary(sign_map) => Ok(Sign::from_map(&sign_map)),
        }
    }

    fn build_redirect_info(&self, url: Uri, original_headers: &HeaderMap) -> RedirectInfo {
        let mut final_headers = original_headers.clone();

        if let Some(user_agent) = &self.user_agent {
            if !user_agent.is_empty() {
                if let Ok(parsed_header) = user_agent.parse() {
                    final_headers.insert(header::USER_AGENT, parsed_header);
                }
            }
        }

        final_headers.remove(header::HOST);

        RedirectInfo {
            target_url: url,
            final_headers,
        }
    }
}

#[async_trait]
impl StreamService for AppStreamService {
    async fn handle_request(
        &self,
        request: AppStreamRequest,
    ) -> Result<AppStreamResult, StatusCode> {
        let source = self.decrypt_and_route(&request).await.map_err(|e| {
            error_log!("Routing stream error: {:?}", e);
            StatusCode::BAD_REQUEST
        })?;
        info_log!(STREAM_LOGGER_DOMAIN, "Routing stream source: {:?}", source);

        match source {
            Source::Local(path) => {
                LocalStreamer::stream(
                    self.state.clone(),
                    path,
                    request.content_range(),
                    request.request_start_time,
                )
                .await
            }
            Source::Remote { url, mode } => match mode {
                ProxyMode::Redirect => {
                    let redirect_info = self.build_redirect_info(url, &request.original_headers);
                    Ok(AppStreamResult::Redirect(redirect_info))
                }
                ProxyMode::Proxy => {
                    RemoteStreamer::stream(
                        self.state.clone(),
                        url,
                        self.user_agent.clone(),
                        &request.original_headers,
                    )
                    .await
                }
            },
        }
    }
}
