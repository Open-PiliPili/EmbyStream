use std::{path::PathBuf, sync::Arc};

use async_trait::async_trait;
use hyper::{HeaderMap, StatusCode, Uri, header};
use tokio::sync::OnceCell;

use super::{
    local_streamer::LocalStreamer, proxy_mode::ProxyMode, remote_streamer::RemoteStreamer,
    result::Result as AppStreamResult, source::Source, types::BackendConfig,
};
use crate::core::redirect_info::RedirectInfo;
use crate::{AppState, STREAM_LOGGER_DOMAIN, debug_log, error_log, info_log};
use crate::{
    CryptoInput, CryptoOperation, CryptoOutput,
    client::{ClientBuilder, OpenListClient},
    config::backend::types::BackendConfig as StreamBackendConfig,
    core::{error::Error as AppStreamError, request::Request as AppStreamRequest},
    crypto::Crypto,
    network::CurlPlugin,
    sign::{Sign, SignParams},
    system::SystemInfo,
    util::{PathRewriter, StringUtil, UriExt},
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

        let mut uri = sign.uri.clone().ok_or(AppStreamError::InvalidUri)?;
        uri = self.rewrite_uri_if_needed(uri).await;
        uri = self.fetch_remote_uri_if_openlist(&uri).await?;

        if sign.is_local() {
            let local_path = PathBuf::from(Uri::to_path_or_url_string(&uri));
            debug_log!(
                STREAM_LOGGER_DOMAIN,
                "Routing to local path {:?}",
                local_path
            );
            Ok(Source::Local(local_path))
        } else {
            debug_log!(
                STREAM_LOGGER_DOMAIN,
                "Routing to remote path {:?}",
                Uri::to_path_or_url_string(&uri)
            );
            Ok(Source::Remote {
                uri,
                mode: params.proxy_mode,
            })
        }
    }

    async fn decrypt(&self, sign: &str, params: &SignParams) -> Result<Sign, AppStreamError> {
        let decrypt_cache = self.state.get_decrypt_cache().await;
        let cache_key = self.decrypt_key(params)?;

        if let Some(sign) = decrypt_cache.get(&cache_key) {
            debug_log!(STREAM_LOGGER_DOMAIN, "Sign cache hit: {:?}", sign);
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
            CryptoOutput::Dictionary(sign_map) => {
                debug_log!(
                    STREAM_LOGGER_DOMAIN,
                    "Succesfully decrypted signatures: {:?}",
                    sign_map
                );
                decrypt_cache.insert(cache_key, sign_map.clone());
                Ok(Sign::from_map(&sign_map))
            }
        }
    }

    async fn rewrite_uri_if_needed(&self, uri: Uri) -> Uri {
        let config = self.get_backend_config().await;
        let uri_str = Uri::to_path_or_url_string(&uri);
        let path_rewrite = config.backend.path_rewrite.clone();

        if !path_rewrite.is_need_rewrite(&uri_str) {
            return uri;
        }

        let rewriter = PathRewriter::new(&path_rewrite.pattern, &path_rewrite.replacement);

        let new_uri_str = rewriter.rewrite(&uri_str).await;
        new_uri_str.parse().unwrap_or(uri)
    }

    async fn fetch_remote_uri_if_openlist(&self, uri: &Uri) -> Result<Uri, AppStreamError> {
        let cache = self.state.get_open_list_cache().await;
        if let Some(cached_uri) = cache.get(&self.open_list_cache_key(uri)) {
            debug_log!(
                STREAM_LOGGER_DOMAIN,
                "Open list cache hit: {:?}",
                cached_uri
            );
            return Ok(cached_uri);
        }

        let config = self.get_backend_config().await;
        let openlist_config = match &config.backend_config {
            StreamBackendConfig::OpenList(open_list) => open_list,
            _ => return Ok(uri.clone()),
        };

        let openlist_client = ClientBuilder::<OpenListClient>::new()
            .with_plugin(CurlPlugin)
            .build();

        let result = openlist_client
            .fetch_file_path(
                Uri::to_path_or_url_string(&openlist_config.uri()),
                &openlist_config.token,
                Uri::to_path_or_url_string(&uri),
            )
            .await;

        match result {
            Ok(new_url) => {
                let new_uri: Uri = new_url.parse().map_err(|e: hyper::http::uri::InvalidUri| {
                    AppStreamError::InvalidOpenListUri(e.to_string())
                })?;
                cache.insert(self.open_list_cache_key(uri), new_uri.clone());

                Ok(new_uri)
            }
            Err(e) => Err(AppStreamError::UnexpectedOpenListError(e.to_string())),
        }
    }

    async fn get_backend_config(&self) -> Arc<BackendConfig> {
        let config_arc = self
            .config
            .get_or_init(|| async {
                let config = self.state.get_config().await;
                let backend = config
                    .backend
                    .as_ref()
                    .expect("Attempted to access backend, but backend is not configured");
                let backend_config = config.backend_config.as_ref().expect(
                    "Attempted to access backend config, but backend config is not configured",
                );
                Arc::new(BackendConfig {
                    crypto_key: config.general.encipher_key.clone(),
                    crypto_iv: config.general.encipher_iv.clone(),
                    backend: backend.clone(),
                    backend_config: backend_config.clone(),
                })
            })
            .await;

        config_arc.clone()
    }

    async fn build_redirect_info(&self, url: Uri, original_headers: &HeaderMap) -> RedirectInfo {
        let mut final_headers = original_headers.clone();
        let config = self.get_backend_config().await;
        let user_agent = match &config.backend_config {
            StreamBackendConfig::DirectLink(dirct_link) => Some(dirct_link.user_agent.to_string()),
            _ => None,
        };

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

    fn decrypt_key(&self, params: &SignParams) -> Result<String, AppStreamError> {
        if params.sign.is_empty() {
            return Err(AppStreamError::InvalidEncryptedSignature);
        }

        let input = params.sign.to_lowercase();
        Ok(StringUtil::md5(&input))
    }

    fn open_list_cache_key(&self, uri: &Uri) -> String {
        let url = Uri::to_path_or_url_string(uri).to_lowercase();
        let input = url.to_lowercase();
        StringUtil::md5(&input)
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
                    let user_agent = match &config.backend_config {
                        StreamBackendConfig::DirectLink(dirct_link) => {
                            Some(dirct_link.user_agent.to_string())
                        }
                        _ => None,
                    }
                    .unwrap_or(SystemInfo::new().get_user_agent());
                    RemoteStreamer::stream(
                        self.state.clone(),
                        uri,
                        Some(user_agent),
                        &request.original_headers,
                    )
                    .await
                }
            },
        }
    }
}
