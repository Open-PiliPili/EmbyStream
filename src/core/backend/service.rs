use std::{borrow::Cow, path::PathBuf, sync::Arc};

use async_trait::async_trait;
use hyper::{StatusCode, Uri, header};

use super::{
    constants::{
        DISK_BACKEND_TYPE, STREAM_RELAY_BACKEND_TYPE,
        backend_base_url_is_empty, backend_base_url_is_local_host,
    },
    local_streamer::LocalStreamer,
    proxy_mode::ProxyMode,
    remote_streamer::{RemoteStreamParams, RemoteStreamer},
    result::Result as AppStreamResult,
    source::Source,
    webdav, webdav_auth,
};
use crate::backend::types::ClientInfo;
use crate::config::backend::BackendNode;
use crate::core::redirect_info::RedirectInfo;
use crate::{AppState, STREAM_LOGGER_DOMAIN, debug_log, error_log, info_log};
use crate::{
    client::{ClientBuilder, OpenListClient},
    core::{
        error::Error as AppStreamError, request::Request as AppStreamRequest,
    },
    sign::SignParams,
    system::SystemInfo,
    util::{StringUtil, UriExt},
};

/// Trait for handling streaming requests
///
/// Implementations of this trait process incoming streaming requests,
/// decrypt signatures, route to appropriate backends, and return streaming responses.
#[async_trait]
pub trait StreamService: Send + Sync {
    /// Handle a streaming request
    ///
    /// # Process
    /// 1. Decrypts and validates the request signature
    /// 2. Routes to local or remote source based on URI
    /// 3. Applies path rewriting if configured
    /// 4. Handles OpenList resolution if needed
    /// 5. Returns appropriate streaming response or redirect
    ///
    /// # Returns
    /// - `Ok(AppStreamResult)` on success
    /// - `Err(StatusCode)` on error
    async fn handle_request(
        &self,
        request: AppStreamRequest,
    ) -> Result<AppStreamResult, StatusCode>;
}

/// Main streaming service implementation
///
/// Handles all streaming requests, including decryption, routing, and streaming.
pub struct AppStreamService {
    pub state: Arc<AppState>,
}

impl AppStreamService {
    /// Create a new AppStreamService instance
    ///
    /// # Arguments
    /// * `state` - Shared application state containing configuration and caches
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    async fn route_with_sign(
        &self,
        request: &AppStreamRequest,
    ) -> Result<Source, AppStreamError> {
        let sign = request
            .sign
            .as_ref()
            .ok_or(AppStreamError::EmptySignature)?;

        let params = request
            .uri
            .query()
            .and_then(|query| {
                serde_urlencoded::from_str::<SignParams>(query).ok()
            })
            .unwrap_or_default();

        let mut uri = sign.uri.clone().ok_or(AppStreamError::InvalidUri)?;
        debug_log!(STREAM_LOGGER_DOMAIN, "Original URI from sign: {}", uri);

        uri = self.rewrite_uri_if_needed(uri, request).await?;
        uri = self.fetch_remote_uri_if_openlist(&uri, request).await?;

        let device_id = params.device_id;
        let node = request
            .node
            .as_ref()
            .ok_or(AppStreamError::BackendNodeNotFound)?;
        let proxy_mode =
            node.proxy_mode.parse::<ProxyMode>().unwrap_or_default();

        debug_log!(
            STREAM_LOGGER_DOMAIN,
            "Using node '{}' with proxy_mode: {:?}",
            node.name,
            proxy_mode
        );

        if node.backend_type.eq_ignore_ascii_case(webdav::BACKEND_TYPE)
            && Uri::is_local(&uri)
        {
            let path_str = Uri::to_path_or_url_string(&uri);
            let upstream = webdav::build_upstream_uri(
                node,
                &path_str,
                node.webdav.as_ref(),
            )
            .map_err(|e| AppStreamError::WebDavUrl(e.to_string()))?;
            debug_log!(
                STREAM_LOGGER_DOMAIN,
                "WebDav upstream URI: {}",
                upstream
            );
            return Ok(Source::Remote {
                uri: upstream,
                mode: proxy_mode,
            });
        }

        if !Uri::is_local(&uri) {
            debug_log!(STREAM_LOGGER_DOMAIN, "URI is already remote: {}", uri);
            return Ok(Source::Remote {
                uri,
                mode: proxy_mode,
            });
        }

        let disk = node.backend_type.eq_ignore_ascii_case(DISK_BACKEND_TYPE);
        let stream_relay = node
            .backend_type
            .eq_ignore_ascii_case(STREAM_RELAY_BACKEND_TYPE);
        let remote_host = Self::node_has_remote_stream_base(node);

        if stream_relay || remote_host {
            if stream_relay
                && (backend_base_url_is_empty(&node.base_url)
                    || backend_base_url_is_local_host(&node.base_url))
            {
                error_log!(
                    STREAM_LOGGER_DOMAIN,
                    "StreamRelay node '{}' has loopback/empty base_url; refused to avoid redirect loops",
                    node.name
                );
                return Err(AppStreamError::StreamRelayForbiddenLocalTarget);
            }
            if disk && remote_host {
                error_log!(
                    STREAM_LOGGER_DOMAIN,
                    "Disk node '{}' has non-local base_url; use type StreamRelay for remote relay",
                    node.name
                );
                return Err(AppStreamError::DiskRemoteNotSupported);
            }
            let remote_uri =
                Self::build_node_remote_uri(node, request.uri.query())?;
            if stream_relay {
                info_log!(
                    STREAM_LOGGER_DOMAIN,
                    "StreamRelay node '{}': forwarding signed request to {}",
                    node.name,
                    remote_uri
                );
            } else {
                info_log!(
                    STREAM_LOGGER_DOMAIN,
                    "Node '{}' points to remote server, forwarding to: {}",
                    node.name,
                    remote_uri
                );
            }
            return Ok(Source::Remote {
                uri: remote_uri,
                mode: proxy_mode,
            });
        }

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

        let fallback_path = self.get_fallback_path().await;
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

    /// Non-empty `base_url` that is not a loopback placeholder — use node's stream URL for relay.
    fn node_has_remote_stream_base(node: &BackendNode) -> bool {
        !backend_base_url_is_empty(&node.base_url)
            && !backend_base_url_is_local_host(&node.base_url)
    }

    fn build_node_remote_uri(
        node: &BackendNode,
        original_query: Option<&str>,
    ) -> Result<Uri, AppStreamError> {
        let base = node.uri().to_string();
        let full = match original_query {
            Some(q) if !q.is_empty() => format!("{}?{}", base, q),
            _ => base,
        };
        full.parse().map_err(|_| AppStreamError::InvalidUri)
    }

    async fn get_fallback_path(&self) -> Option<PathBuf> {
        let config = self.state.get_config().await;

        let fallback_path_str = &config.fallback.video_missing_path;
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
    }

    async fn rewrite_uri_if_needed(
        &self,
        uri: Uri,
        request: &AppStreamRequest,
    ) -> Result<Uri, AppStreamError> {
        let original_uri_str = Uri::to_path_or_url_string(&uri);
        let node = request
            .node
            .as_ref()
            .ok_or(AppStreamError::BackendNodeNotFound)?;

        let path_rewriters = &node.path_rewriter_cache;
        if path_rewriters.is_empty() {
            debug_log!(
                STREAM_LOGGER_DOMAIN,
                "Backend path rewriting is empty. Skipping step."
            );
            return Ok(uri);
        }

        debug_log!(STREAM_LOGGER_DOMAIN, "Starting backend path rewrite.");
        debug_log!(
            STREAM_LOGGER_DOMAIN,
            "Original URI: '{}', Rewrite rules count: {}",
            original_uri_str,
            path_rewriters.len()
        );

        let mut current_uri_str: Cow<str> = Cow::Borrowed(&original_uri_str);

        for (idx, rewriter) in path_rewriters.iter().enumerate() {
            if !rewriter.enable {
                debug_log!(
                    STREAM_LOGGER_DOMAIN,
                    "  Rule #{}: DISABLED",
                    idx + 1
                );
                continue;
            }

            let before = current_uri_str.clone();
            let outcome = rewriter.rewrite(&current_uri_str).await;
            let changed = before != outcome;

            debug_log!(
                STREAM_LOGGER_DOMAIN,
                "  Rule #{}: {} → {} [{}]",
                idx + 1,
                before,
                outcome,
                if changed { "APPLIED" } else { "NO CHANGE" }
            );

            current_uri_str = Cow::Owned(outcome);
        }

        let current_uri = Uri::force_from_path_or_url(&current_uri_str)
            .unwrap_or(uri.clone());

        debug_log!(
            STREAM_LOGGER_DOMAIN,
            "Backend path rewrite completed. \
            URI before: '{}', URI after: '{}'",
            uri,
            current_uri
        );

        Ok(current_uri)
    }

    async fn fetch_remote_uri_if_openlist(
        &self,
        uri: &Uri,
        request: &AppStreamRequest,
    ) -> Result<Uri, AppStreamError> {
        if !Uri::is_local(uri) {
            debug_log!(
                STREAM_LOGGER_DOMAIN,
                "OpenList mode enabled: \
                skipping backend processing for remote URI: {:?}",
                uri
            );
            return Ok(uri.clone());
        }

        let user_agent = request.user_agent();
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

        let node = request
            .node
            .as_ref()
            .ok_or(AppStreamError::BackendNodeNotFound)?;
        let openlist_config = match &node.open_list {
            Some(cfg) => cfg,
            None => return Ok(uri.clone()),
        };

        debug_log!(
            STREAM_LOGGER_DOMAIN,
            "Processing OpenList for node '{}': base_url='{}'",
            node.name,
            openlist_config.base_url
        );

        let path = Uri::to_path_or_url_string(uri);
        debug_log!(
            STREAM_LOGGER_DOMAIN,
            "Open list processing path: {:?}, user-agent: {:?}",
            path,
            openlist_ua
        );

        let openlist_client = ClientBuilder::<OpenListClient>::new().build();

        let result = openlist_client
            .fetch_file_path(
                &openlist_config.base_url,
                &openlist_config.token,
                path,
                openlist_ua.clone(),
            )
            .await;

        match result {
            Ok(new_url) => {
                debug_log!(
                    STREAM_LOGGER_DOMAIN,
                    "✓ OpenList resolved: '{}' → '{}'",
                    uri,
                    new_url
                );
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

    async fn build_redirect_info(
        &self,
        url: Uri,
        request: &AppStreamRequest,
    ) -> Result<RedirectInfo, AppStreamError> {
        let mut final_headers = request.original_headers.clone();

        let node = request
            .node
            .as_ref()
            .ok_or(AppStreamError::BackendNodeNotFound)?;
        let has_client_ua = final_headers
            .get(header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);

        if !has_client_ua {
            let ua = node
                .webdav
                .as_ref()
                .filter(|w| !w.user_agent.trim().is_empty())
                .map(|w| w.user_agent.trim().to_string())
                .or_else(|| {
                    node.direct_link.as_ref().and_then(|link| {
                        if link.user_agent.is_empty() {
                            None
                        } else {
                            Some(link.user_agent.to_string())
                        }
                    })
                })
                .unwrap_or_else(|| SystemInfo::new().get_user_agent());
            if let Ok(parsed_header) = ua.parse() {
                debug_log!(
                    STREAM_LOGGER_DOMAIN,
                    "Insert user agent {:?} to redirect headers",
                    ua
                );
                final_headers.insert(header::USER_AGENT, parsed_header);
            }
        }

        final_headers.remove(header::HOST);

        Ok(RedirectInfo {
            target_url: url,
            final_headers,
        })
    }

    fn open_list_cache_key(&self, uri: &Uri, user_agent: &str) -> String {
        let url_string = Uri::to_path_or_url_string(uri);
        let trimmed_url = url_string.trim_end();
        let input =
            format!("{}&user_agent={}", trimmed_url.to_lowercase(), user_agent);
        StringUtil::md5(&input)
    }

    fn resolve_upstream_user_agent(
        node: &BackendNode,
        request: &AppStreamRequest,
    ) -> String {
        if node.backend_type.eq_ignore_ascii_case(webdav::BACKEND_TYPE) {
            if let Some(ua) = request.user_agent() {
                let t = ua.trim();
                if !t.is_empty() {
                    return t.to_string();
                }
            }
            if let Some(w) = &node.webdav {
                let t = w.user_agent.trim();
                if !t.is_empty() {
                    return t.to_string();
                }
            }
            return SystemInfo::new().get_user_agent();
        }
        if let Some(d) = &node.direct_link {
            if !d.user_agent.is_empty() {
                return d.user_agent.to_string();
            }
        }
        request
            .user_agent()
            .unwrap_or_else(|| SystemInfo::new().get_user_agent())
    }

    async fn webdav_proxy_auth_headers(
        &self,
        node: &BackendNode,
        uri: &Uri,
        client_headers: &hyper::HeaderMap,
    ) -> Result<Option<hyper::HeaderMap>, StatusCode> {
        if !node.backend_type.eq_ignore_ascii_case(webdav::BACKEND_TYPE) {
            return Ok(None);
        }
        let Some(cfg) = node.webdav.as_ref() else {
            return Ok(None);
        };
        if !webdav_auth::credentials_configured(cfg) {
            return Ok(None);
        }

        match webdav_auth::authorization_header_for_proxy(
            &self.state.webdav_auth_cache,
            &self.state.webdav_auth_probe_locks,
            node,
            uri,
            cfg,
            Some(client_headers),
        )
        .await
        {
            Ok(Some(line)) => webdav_auth::extra_headers_from_auth_line(&line)
                .map(Some)
                .map_err(|_| {
                    error_log!(
                        STREAM_LOGGER_DOMAIN,
                        "Invalid WebDav Authorization header value"
                    );
                    StatusCode::INTERNAL_SERVER_ERROR
                }),
            Ok(None) => Ok(None),
            Err(()) => Err(StatusCode::UNAUTHORIZED),
        }
    }
}

#[async_trait]
impl StreamService for AppStreamService {
    async fn handle_request(
        &self,
        request: AppStreamRequest,
    ) -> Result<AppStreamResult, StatusCode> {
        let source = self.route_with_sign(&request).await.map_err(|e| {
            error_log!(STREAM_LOGGER_DOMAIN, "Routing stream error: {:?}", e);
            StatusCode::BAD_REQUEST
        })?;

        let node = request.node.as_ref().ok_or_else(|| {
            error_log!(
                STREAM_LOGGER_DOMAIN,
                "Backend node not found in request"
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        let node_uuid = &node.uuid;

        debug_log!(
            STREAM_LOGGER_DOMAIN,
            "==== Routing completed for node '{}' (uuid={}): source type = {} ====",
            node.name,
            node_uuid,
            match &source {
                Source::Local { .. } => "Local",
                Source::Remote { .. } => "Remote",
            }
        );
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
                    node_uuid,
                )
                .await
            }
            Source::Remote { uri, mode } => match mode {
                ProxyMode::Redirect => {
                    let redirect_info = self
                        .build_redirect_info(uri, &request)
                        .await
                        .map_err(|e| {
                            error_log!(
                                STREAM_LOGGER_DOMAIN,
                                "Failed to build redirect info: {:?}",
                                e
                            );
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?;
                    Ok(AppStreamResult::Redirect(redirect_info))
                }
                ProxyMode::Proxy => {
                    let user_agent =
                        Self::resolve_upstream_user_agent(node, &request);
                    let extra_headers = self
                        .webdav_proxy_auth_headers(
                            node,
                            &uri,
                            &request.original_headers,
                        )
                        .await?;

                    RemoteStreamer::stream(RemoteStreamParams {
                        state: self.state.clone(),
                        url: uri,
                        user_agent,
                        client_headers: &request.original_headers,
                        extra_upstream_headers: extra_headers,
                        client: request.client(),
                        client_ip: request.client_ip(),
                        node,
                    })
                    .await
                }
            },
        }
    }
}
