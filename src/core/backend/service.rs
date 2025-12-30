use std::{borrow::Cow, path::PathBuf, sync::Arc};

use async_trait::async_trait;
use hyper::{HeaderMap, StatusCode, Uri, header};
use tokio::sync::OnceCell;

use super::{
    local_streamer::LocalStreamer,
    proxy_mode::ProxyMode,
    remote_streamer::RemoteStreamer,
    result::Result as AppStreamResult,
    source::Source,
    types::{BackendConfig, BackendRoutes},
};
use crate::backend::types::ClientInfo;
use crate::core::redirect_info::RedirectInfo;
use crate::{AppState, STREAM_LOGGER_DOMAIN, debug_log, error_log, info_log};
use crate::{
    CryptoInput, CryptoOperation, CryptoOutput,
    client::{ClientBuilder, OpenListClient},
    config::backend::types::BackendConfig as StreamBackendConfig,
    core::{
        error::Error as AppStreamError, request::Request as AppStreamRequest,
    },
    crypto::Crypto,
    network::CurlPlugin,
    sign::{Sign, SignParams},
    system::SystemInfo,
    util::{StringUtil, UriExt, resolve_fallback_video_path},
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
            .and_then(|query| {
                serde_urlencoded::from_str::<SignParams>(query).ok()
            })
            .unwrap_or_default();

        if params.sign.is_empty() {
            return Err(AppStreamError::EmptySignature);
        }

        let sign = self.decrypt(params.sign.as_str(), &params).await?;

        if !sign.is_valid() {
            return Err(AppStreamError::ExpiredStream);
        }

        let mut uri = sign.uri.clone().ok_or(AppStreamError::InvalidUri)?;

        // Get backend routes configuration (if available)
        let routes = self.state.get_backend_routes().await;

        // Determine path for routing based on match_before_rewrite setting
        let path_for_routing = if let Some(routes) = routes {
            if routes.match_before_rewrite {
                // Match routes before path rewriting
                Uri::to_path_or_url_string(&uri)
            } else {
                // Match routes after path rewriting (default)
                uri = self.rewrite_uri_if_needed(uri).await;
                Uri::to_path_or_url_string(&uri)
            }
        } else {
            // No routing configuration, use legacy behavior
            uri = self.rewrite_uri_if_needed(uri).await;
            Uri::to_path_or_url_string(&uri)
        };

        // Get backend config for this path (or use legacy config)
        let backend_config = self
            .get_backend_config_for_path_str(&path_for_routing)
            .await;

        // Use the selected backend config for OpenList processing
        uri = self
            .fetch_remote_uri_if_openlist_with_config(
                &uri,
                request.user_agent(),
                &backend_config,
            )
            .await?;

        let device_id = params.device_id;

        // Remote url
        if !Uri::is_local(&uri) {
            debug_log!(
                STREAM_LOGGER_DOMAIN,
                "Routing to remote path {:?}",
                uri
            );
            return Ok(Source::Remote {
                uri,
                mode: params.proxy_mode,
            });
        }

        // Local path
        let path = PathBuf::from(Uri::to_path_or_url_string(&uri));

        debug_log!(STREAM_LOGGER_DOMAIN, "Routing to local path {:?}", path);

        if path.exists() {
            return Ok(Source::Local { path, device_id });
        }

        debug_log!(
            STREAM_LOGGER_DOMAIN,
            "File not found at original path: {:?}, checking fallback",
            path
        );

        let fallback_path =
            self.get_fallback_path_with_config(&backend_config).await;
        match fallback_path {
            Some(fallback_path) => {
                debug_log!(
                    STREAM_LOGGER_DOMAIN,
                    "Using fallback path: {:?}",
                    fallback_path
                );
                Ok(Source::Local {
                    path: fallback_path,
                    device_id,
                })
            }
            None => {
                Err(AppStreamError::FileNotFound(path.display().to_string()))
            }
        }
    }

    /// Get backend config for a specific path (with route matching if available)
    async fn get_backend_config_for_path_str(
        &self,
        path: &str,
    ) -> Arc<BackendConfig> {
        if let Some(routes) = self.state.get_backend_routes().await {
            self.get_backend_config_for_path(path, routes).await
        } else {
            // Legacy: use single backend config
            self.get_backend_config().await
        }
    }

    /// Get backend config for a specific path using route matching
    async fn get_backend_config_for_path(
        &self,
        path: &str,
        routes: &BackendRoutes,
    ) -> Arc<BackendConfig> {
        // Find matching route based on priority setting
        let matched_route = if routes.match_priority_first {
            // First match: find the first route that matches
            routes.routes.iter().find(|route| {
                route
                    .regex
                    .get()
                    .map(|re| re.is_match(path))
                    .unwrap_or(false)
            })
        } else {
            // Last match: find the last route that matches
            routes.routes.iter().rev().find(|route| {
                route
                    .regex
                    .get()
                    .map(|re| re.is_match(path))
                    .unwrap_or(false)
            })
        };

        match matched_route {
            Some(route) => {
                debug_log!(
                    STREAM_LOGGER_DOMAIN,
                    "Route matched: pattern={}, path={}",
                    route.pattern,
                    path
                );
                Arc::new(route.backend_config.clone())
            }
            None => {
                debug_log!(
                    STREAM_LOGGER_DOMAIN,
                    "No route matched for path={}, using fallback",
                    path
                );
                Arc::new(routes.fallback.clone())
            }
        }
    }

    async fn get_fallback_path_with_config(
        &self,
        config: &BackendConfig,
    ) -> Option<PathBuf> {
        config
            .fallback_video_path
            .as_ref()
            .and_then(|fallback_path_str| {
                if fallback_path_str.is_empty() {
                    return None;
                }

                let fallback_path = PathBuf::from(fallback_path_str);
                if !fallback_path.exists() {
                    debug_log!(
                        STREAM_LOGGER_DOMAIN,
                        "Fallback path does not exist: {:?}",
                        fallback_path_str
                    );
                    return None;
                }

                Some(fallback_path)
            })
    }

    async fn decrypt(
        &self,
        sign: &str,
        params: &SignParams,
    ) -> Result<Sign, AppStreamError> {
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
            CryptoOutput::Encrypted(_) => {
                Err(AppStreamError::InvalidEncryptedSignature)
            }
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
        let original_uri_str = Uri::to_path_or_url_string(&uri);
        let path_rewrites = self.state.get_backend_path_rewrite_cache().await;

        if path_rewrites.is_empty() {
            debug_log!(
                STREAM_LOGGER_DOMAIN,
                "Backend path rewriting is empty. Skipping step."
            );
            return uri;
        }

        debug_log!(STREAM_LOGGER_DOMAIN, "Starting backend path rewrite.");

        let mut current_uri_str: Cow<str> = Cow::Borrowed(&original_uri_str);
        for path_rewrite in path_rewrites {
            if !path_rewrite.enable {
                continue;
            }
            current_uri_str =
                path_rewrite.rewrite(&current_uri_str).await.into();
        }

        let current_uri = Uri::force_from_path_or_url(&current_uri_str)
            .unwrap_or(uri.clone());

        debug_log!(
            STREAM_LOGGER_DOMAIN,
            "Backend path rewrite completed. URI before: {:?}, URI after: {:?}",
            uri,
            current_uri
        );

        current_uri
    }

    async fn fetch_remote_uri_if_openlist_with_config(
        &self,
        uri: &Uri,
        user_agent: Option<String>,
        backend_config: &BackendConfig,
    ) -> Result<Uri, AppStreamError> {
        if !Uri::is_local(uri) {
            debug_log!(
                STREAM_LOGGER_DOMAIN,
                "OpenList mode enabled: skipping backend processing for remote URI: {:?}",
                uri
            );
            return Ok(uri.clone());
        }

        let openlist_ua =
            user_agent.unwrap_or(SystemInfo::new().get_user_agent());

        let cache = self.state.get_open_list_cache().await;
        if let Some(cached_uri) =
            cache.get(&self.open_list_cache_key(uri, &openlist_ua.clone()))
        {
            debug_log!(
                STREAM_LOGGER_DOMAIN,
                "Open list cache hit: {:?}",
                cached_uri
            );
            return Ok(cached_uri);
        }

        let openlist_config = match &backend_config.backend_config {
            StreamBackendConfig::OpenList(open_list) => open_list,
            _ => return Ok(uri.clone()),
        };

        let path = Uri::to_path_or_url_string(uri);

        debug_log!(
            STREAM_LOGGER_DOMAIN,
            "Open list processing path: {:?}, user-agent: {:?}",
            path,
            openlist_ua
        );

        let openlist_client = ClientBuilder::<OpenListClient>::new()
            .with_plugin(CurlPlugin)
            .build();

        let result = openlist_client
            .fetch_file_path(
                &openlist_config.uri().to_string(),
                &openlist_config.token,
                path,
                openlist_ua.clone(),
            )
            .await;

        match result {
            Ok(new_url) => {
                let new_uri =
                    Uri::force_from_path_or_url(&new_url).map_err(|e| {
                        error_log!(
                            STREAM_LOGGER_DOMAIN,
                            "Failed to convert openlist url: {:?} to uri: {:?}",
                            new_url,
                            e
                        );
                        AppStreamError::InvalidOpenListUri(new_url.clone())
                    })?;

                cache.insert(
                    self.open_list_cache_key(uri, &openlist_ua),
                    new_uri.clone(),
                );

                debug_log!(
                    STREAM_LOGGER_DOMAIN,
                    "Successfully fetched Openlist url: {:?}",
                    new_uri
                );

                Ok(new_uri)
            }
            Err(e) => {
                error_log!(
                    STREAM_LOGGER_DOMAIN,
                    "Failed to fetch Openlist url: {:?}",
                    e
                );

                Err(AppStreamError::UnexpectedOpenListError(e.to_string()))
            }
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

                let fallback_video_path = resolve_fallback_video_path(
                    &config.fallback.video_missing_path,
                    &config.path,
                );

                Arc::new(BackendConfig {
                    crypto_key: config.general.encipher_key.clone(),
                    crypto_iv: config.general.encipher_iv.clone(),
                    backend: backend.clone(),
                    backend_config: backend_config.clone(),
                    fallback_video_path
                })
            })
            .await;

        config_arc.clone()
    }

    async fn build_redirect_info_with_config(
        &self,
        url: Uri,
        original_headers: &HeaderMap,
        backend_config: &BackendConfig,
    ) -> RedirectInfo {
        let mut final_headers = original_headers.clone();

        let user_agent = match &backend_config.backend_config {
            StreamBackendConfig::DirectLink(dirct_link) => {
                Some(Arc::new(dirct_link.user_agent.to_string()))
            }
            _ => None,
        };

        if let Some(user_agent) = user_agent {
            if !user_agent.is_empty() {
                if let Ok(parsed_header) = user_agent.parse() {
                    debug_log!(
                        STREAM_LOGGER_DOMAIN,
                        "Insert user agent {:?} to header",
                        user_agent
                    );
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

    fn decrypt_key(
        &self,
        params: &SignParams,
    ) -> Result<String, AppStreamError> {
        if params.sign.is_empty() {
            return Err(AppStreamError::InvalidEncryptedSignature);
        }

        let input = params.sign.to_lowercase();
        Ok(StringUtil::md5(&input))
    }

    fn open_list_cache_key(&self, uri: &Uri, user_agent: &str) -> String {
        let url_string = Uri::to_path_or_url_string(uri);
        let trimmed_url = url_string.trim_end();
        let input =
            format!("{}&user_agent={}", trimmed_url.to_lowercase(), user_agent);
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
            Source::Local { path, device_id } => {
                let client_info = ClientInfo::new(
                    Some(device_id),
                    request.client(),
                    request.client_ip(),
                );
                LocalStreamer::stream(
                    self.state.clone(),
                    path,
                    request.content_range(),
                    client_info,
                )
                .await
            }
            Source::Remote { uri, mode } => match mode {
                ProxyMode::Redirect => {
                    // Get backend config for redirect (use path from URI)
                    let path_for_config = Uri::to_path_or_url_string(&uri);
                    let backend_config = self
                        .get_backend_config_for_path_str(&path_for_config)
                        .await;

                    let redirect_info = self
                        .build_redirect_info_with_config(
                            uri,
                            &request.original_headers,
                            &backend_config,
                        )
                        .await;
                    Ok(AppStreamResult::Redirect(redirect_info))
                }
                ProxyMode::Proxy => {
                    // Get backend config for this request (use path from URI)
                    let path_for_config = Uri::to_path_or_url_string(&uri);
                    let backend_config = self
                        .get_backend_config_for_path_str(&path_for_config)
                        .await;

                    let user_agent = match &backend_config.backend_config {
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
                        request.client(),
                        request.client_ip(),
                    )
                    .await
                }
            },
        }
    }
}
