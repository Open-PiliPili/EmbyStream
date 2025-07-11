use std::{path::PathBuf, sync::Arc};

use async_trait::async_trait;
use hyper::{HeaderMap, StatusCode, Uri, header};
use tokio::sync::OnceCell;

use super::{
    local_streamer::LocalStreamer, proxy_mode::ProxyMode, remote_streamer::RemoteStreamer,
    result::Result as AppStreamResult, source::Source, types::BackendConfig,
};
use crate::core::redirect_info::RedirectInfo;
use crate::{AppState, STREAM_LOGGER_DOMAIN, error_log, info_log};
use crate::{
    CryptoInput, CryptoOperation, CryptoOutput,
    config::backend::types::BackendConfig as StreamBackendConfig,
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
    pub config: OnceCell<Arc<BackendConfig>>,
}

impl AppStreamService {
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            state,
            config: OnceCell::new(),
        }
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
                uri,
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

        let config = self.get_backend_config().await;
        let crypto_result = Crypto::execute(
            CryptoOperation::Decrypt,
            CryptoInput::Encrypted(sign.to_string()),
            &config.crypto_key,
            &config.crypto_iv,
        )
        .map_err(AppStreamError::CommonError)?;

        match crypto_result {
            CryptoOutput::Encrypted(_) => Err(AppStreamError::InvalidEncryptedSignature),
            CryptoOutput::Dictionary(sign_map) => Ok(Sign::from_map(&sign_map)),
        }
    }

    async fn get_backend_config(&self) -> Arc<BackendConfig> {
        let config_arc = self
            .config
            .get_or_init(|| async {
                let config = self.state.get_config().await;
                let user_agent =
                    if let StreamBackendConfig::OpenList(open_list) = &config.backend_config {
                        Some(open_list.user_agent.clone())
                    } else {
                        None
                    };
                Arc::new(BackendConfig {
                    crypto_key: config.general.encipher_key.clone(),
                    crypto_iv: config.general.encipher_iv.clone(),
                    user_agent,
                })
            })
            .await;

        config_arc.clone()
    }

    async fn build_redirect_info(&self, url: Uri, original_headers: &HeaderMap) -> RedirectInfo {
        let mut final_headers = original_headers.clone();
        let config = self.get_backend_config().await;
        let user_agent = config.user_agent.clone();

        if let Some(user_agent) = user_agent {
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
            Source::Remote { uri, mode } => match mode {
                ProxyMode::Redirect => {
                    let redirect_info = self
                        .build_redirect_info(uri, &request.original_headers)
                        .await;
                    Ok(AppStreamResult::Redirect(redirect_info))
                }
                ProxyMode::Proxy => {
                    let config = self.get_backend_config().await;
                    let user_agent = config.user_agent.clone();
                    RemoteStreamer::stream(
                        self.state.clone(),
                        uri,
                        user_agent,
                        &request.original_headers,
                    )
                    .await
                }
            },
        }
    }
}
