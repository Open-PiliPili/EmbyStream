use std::{borrow::Cow, path::PathBuf, sync::Arc, time::Instant};

use async_trait::async_trait;
use form_urlencoded::Serializer;
use hyper::{HeaderMap, StatusCode, Uri, header};
use tokio::sync::Mutex as TokioMutex;

use super::{
    constants::{
        DISK_BACKEND_TYPE, STREAM_RELAY_BACKEND_TYPE,
        backend_base_url_is_empty, backend_base_url_is_local_host,
    },
    google_drive, google_drive_auth,
    local_streamer::LocalStreamer,
    proxy_mode::ProxyMode,
    remote_streamer::{RemoteStreamParams, RemoteStreamer},
    result::Result as AppStreamResult,
    session_id::generate_stream_session_id,
    source::Source,
    upstream_proxy, webdav, webdav_auth,
};
use crate::backend::types::ClientInfo;
use crate::config::backend::BackendNode;
use crate::core::redirect_info::{AccelRedirectInfo, RedirectInfo};
use crate::{
    AppState, STREAM_LOGGER_DOMAIN, debug_log, error_log, info_log, warn_log,
};
use crate::{
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
        let timer = Instant::now();

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

        let rewrite_start = Instant::now();
        uri = self.rewrite_uri_if_needed(uri, request).await?;
        let rewrite_ms = rewrite_start.elapsed().as_millis();

        let openlist_start = Instant::now();
        uri = self.fetch_remote_uri_if_openlist(&uri, request).await?;
        let openlist_ms = openlist_start.elapsed().as_millis();

        let device_id = params.device_id;
        let playback_session_id = params.playback_session_id;
        if playback_session_id.trim().is_empty() {
            return Err(AppStreamError::EmptyPlaybackSessionId);
        }
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

        let result = if node
            .backend_type
            .eq_ignore_ascii_case(webdav::BACKEND_TYPE)
            && Uri::is_local(&uri)
        {
            let path_str = Uri::to_path_or_url_string(&uri);
            if proxy_mode == ProxyMode::AccelRedirect {
                let node_uuid = Self::webdav_accel_redirect_node_uuid(node)?;
                let info = Self::build_webdav_accel_redirect_info(
                    &node_uuid, &path_str,
                )?;
                Ok(Source::AccelRedirect { info })
            } else {
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
                Ok(Source::Remote {
                    uri: upstream,
                    mode: proxy_mode,
                    extra_upstream_headers: None,
                })
            }
        } else if node
            .backend_type
            .eq_ignore_ascii_case(google_drive::BACKEND_TYPE)
            && Uri::is_local(&uri)
        {
            let raw_path = Uri::to_path_or_url_string(&uri);
            let resolved = self
                .resolve_google_drive_remote(node, &raw_path, proxy_mode)
                .await
                .map_err(|error| {
                    error_log!(
                        STREAM_LOGGER_DOMAIN,
                        "google_drive_route_failed node={} path={} error={}",
                        node.name,
                        raw_path,
                        error
                    );
                    AppStreamError::InvalidUri
                })?;
            Ok(resolved)
        } else if !Uri::is_local(&uri) {
            debug_log!(STREAM_LOGGER_DOMAIN, "URI is already remote: {}", uri);
            Ok(Source::Remote {
                uri,
                mode: proxy_mode,
                extra_upstream_headers: None,
            })
        } else {
            let disk =
                node.backend_type.eq_ignore_ascii_case(DISK_BACKEND_TYPE);
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
                        "StreamRelay node '{}' has loopback/empty base_url; \
                        refused to avoid redirect loops",
                        node.name
                    );
                    return Err(
                        AppStreamError::StreamRelayForbiddenLocalTarget,
                    );
                }
                if disk && remote_host {
                    error_log!(
                        STREAM_LOGGER_DOMAIN,
                        "Disk node '{}' has non-local base_url; \
                        use type StreamRelay for remote relay",
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
                Ok(Source::Remote {
                    uri: remote_uri,
                    mode: proxy_mode,
                    extra_upstream_headers: None,
                })
            } else {
                let path = PathBuf::from(Uri::to_path_or_url_string(&uri));
                debug_log!(
                    STREAM_LOGGER_DOMAIN,
                    "Routing to local path {:?}",
                    path
                );
                Ok(Source::Local {
                    path,
                    device_id,
                    playback_session_id: playback_session_id.clone(),
                })
            }
        };

        let elapsed_ms = timer.elapsed().as_millis();
        if elapsed_ms >= 100 {
            warn_log!(
                STREAM_LOGGER_DOMAIN,
                "route_with_sign_slow elapsed_ms={} rewrite_ms={} \
                 openlist_ms={} local_path_check_ms={} node={} session_id={}",
                elapsed_ms,
                rewrite_ms,
                openlist_ms,
                0,
                request
                    .node
                    .as_ref()
                    .map(|n| n.name.as_str())
                    .unwrap_or("-"),
                playback_session_id
            );
        } else {
            debug_log!(
                STREAM_LOGGER_DOMAIN,
                "route_with_sign_complete elapsed_ms={} rewrite_ms={} \
                 openlist_ms={} local_path_check_ms={} session_id={}",
                elapsed_ms,
                rewrite_ms,
                openlist_ms,
                0,
                playback_session_id
            );
        }

        result
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
        let timer = Instant::now();

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
        let node = request
            .node
            .as_ref()
            .ok_or(AppStreamError::BackendNodeNotFound)?;
        let open_list_cache_key =
            Self::open_list_cache_key(&node.uuid, uri, &openlist_ua);

        let cache = self.state.get_open_list_cache().await;
        if let Some(cached_uri) = cache.get(&open_list_cache_key) {
            let elapsed_ms = timer.elapsed().as_millis();
            debug_log!(
                STREAM_LOGGER_DOMAIN,
                "openlist_cache_hit elapsed_ms={} key={} node={} uri={:?}",
                elapsed_ms,
                open_list_cache_key,
                node.name,
                cached_uri
            );
            return Ok(cached_uri);
        }

        let openlist_config = match &node.open_list {
            Some(cfg) => cfg,
            None => {
                let elapsed_ms = timer.elapsed().as_millis();
                debug_log!(
                    STREAM_LOGGER_DOMAIN,
                    "openlist_skip_no_config elapsed_ms={}",
                    elapsed_ms
                );
                return Ok(uri.clone());
            }
        };

        let probe_mutex = self.open_list_request_lock(&open_list_cache_key);
        let result = {
            let wait_start = Instant::now();
            let _probe_guard = probe_mutex.lock().await;
            let lock_wait_ms = wait_start.elapsed().as_millis();

            if let Some(cached_uri) = cache.get(&open_list_cache_key) {
                info_log!(
                    STREAM_LOGGER_DOMAIN,
                    "openlist_inflight_wait_hit lock_wait_ms={} key={} node={} \
                     uri={:?}",
                    lock_wait_ms,
                    open_list_cache_key,
                    node.name,
                    cached_uri
                );
                Ok(cached_uri)
            } else {
                debug_log!(
                    STREAM_LOGGER_DOMAIN,
                    "openlist_fetch_start key={} node='{}' base_url='{}'",
                    open_list_cache_key,
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

                let openlist_client =
                    self.state.get_open_list_client().await.clone();

                let result = openlist_client
                    .fetch_file_path(
                        &openlist_config.base_url,
                        &openlist_config.token,
                        path,
                        openlist_ua.clone(),
                    )
                    .await;

                let elapsed_ms = timer.elapsed().as_millis();

                match result {
                    Ok(new_url) => {
                        if elapsed_ms >= 500 {
                            warn_log!(
                                STREAM_LOGGER_DOMAIN,
                                "openlist_fetch_slow elapsed_ms={} key={} node={} \
                                 url={}",
                                elapsed_ms,
                                open_list_cache_key,
                                node.name,
                                new_url
                            );
                        } else {
                            debug_log!(
                                STREAM_LOGGER_DOMAIN,
                                "openlist_fetch_complete elapsed_ms={} key={} node={} \
                                 url={}",
                                elapsed_ms,
                                open_list_cache_key,
                                node.name,
                                new_url
                            );
                        }

                        let new_uri =
                            Uri::force_from_path_or_url(&new_url).map_err(|e| {
                                error_log!(
                                    STREAM_LOGGER_DOMAIN,
                                    "Failed to convert openlist url: {:?} to uri: {:?}",
                                    new_url,
                                    e
                                );
                                AppStreamError::InvalidOpenListUri(
                                    new_url.clone(),
                                )
                            })?;

                        cache.insert(
                            open_list_cache_key.clone(),
                            new_uri.clone(),
                        );
                        info_log!(
                            STREAM_LOGGER_DOMAIN,
                            "openlist_cache_store key={} node={} uri={}",
                            open_list_cache_key,
                            node.name,
                            new_uri
                        );

                        Ok(new_uri)
                    }
                    Err(e) => {
                        error_log!(
                            STREAM_LOGGER_DOMAIN,
                            "openlist_fetch_error elapsed_ms={} key={} node={} \
                             error={:?}",
                            elapsed_ms,
                            open_list_cache_key,
                            node.name,
                            e
                        );

                        Err(AppStreamError::UnexpectedOpenListError(
                            e.to_string(),
                        ))
                    }
                }
            }
        };

        AppState::cleanup_request_lock(
            &self.state.open_list_request_locks,
            &open_list_cache_key,
            &probe_mutex,
        );

        result
    }

    async fn build_redirect_info(
        &self,
        url: Uri,
        request: &AppStreamRequest,
        extra_headers: Option<HeaderMap>,
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
        if let Some(extra_headers) = extra_headers {
            final_headers.extend(extra_headers);
        }

        Ok(RedirectInfo {
            target_url: url,
            final_headers,
        })
    }

    fn build_webdav_accel_redirect_info(
        node_uuid: &str,
        file_path: &str,
    ) -> Result<AccelRedirectInfo, AppStreamError> {
        let encoded_path = webdav::encode_path_segments(file_path);
        if encoded_path.is_empty() {
            return Err(AppStreamError::WebDavUrl(
                "empty WebDav logical file path for accel_redirect".to_string(),
            ));
        }

        Ok(AccelRedirectInfo {
            internal_path: format!(
                "{}/{}/{}",
                webdav::ACCEL_REDIRECT_PREFIX,
                node_uuid,
                encoded_path
            ),
            internal_headers: HeaderMap::new(),
        })
    }

    fn build_google_drive_accel_redirect_info(
        node_uuid: &str,
        file_id: &str,
        auth_headers: &HeaderMap,
    ) -> Result<AccelRedirectInfo, AppStreamError> {
        let encoded_file_id = webdav::encode_path_segments(file_id);
        if encoded_file_id.is_empty() {
            return Err(AppStreamError::InvalidUri);
        }

        let mut internal_path = format!(
            "{}/{}/{}",
            google_drive::ACCEL_REDIRECT_PREFIX,
            node_uuid,
            encoded_file_id
        );
        if let Some(auth_value) = auth_headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
        {
            let query = Serializer::new(String::new())
                .append_pair("token", auth_value)
                .finish();
            internal_path.push('?');
            internal_path.push_str(&query);
        }

        Ok(AccelRedirectInfo {
            internal_path,
            internal_headers: HeaderMap::new(),
        })
    }

    fn webdav_accel_redirect_node_uuid(
        node: &BackendNode,
    ) -> Result<String, AppStreamError> {
        let node_uuid = node
            .webdav
            .as_ref()
            .map(|cfg| cfg.node_uuid.trim())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                AppStreamError::WebDavUrl(format!(
                    "missing WebDav node_uuid for accel_redirect node '{}'",
                    node.name
                ))
            })?;

        Ok(node_uuid.to_string())
    }

    fn open_list_cache_key(
        node_uuid: &str,
        uri: &Uri,
        user_agent: &str,
    ) -> String {
        const OPEN_LIST_CACHE_KEY_PREFIX: &str = "backend:openlist";
        let url_string = Uri::to_path_or_url_string(uri);
        let trimmed_url = url_string.trim_end();
        let path_hash = StringUtil::hash_hex(&trimmed_url.to_lowercase());
        let ua_hash = StringUtil::hash_hex(user_agent.trim());
        format!(
            "{OPEN_LIST_CACHE_KEY_PREFIX}:node:{}:path_hash:{}:ua_hash:{}",
            node_uuid.to_ascii_lowercase(),
            path_hash,
            ua_hash
        )
    }

    fn open_list_request_lock(&self, cache_key: &str) -> Arc<TokioMutex<()>> {
        AppState::request_lock(&self.state.open_list_request_locks, cache_key)
    }

    fn google_drive_file_id_cache_key(
        node_uuid: &str,
        raw_path: &str,
    ) -> String {
        let path_hash = StringUtil::hash_hex(&raw_path.trim().to_lowercase());
        format!(
            "backend:google-drive:file-id:node:{}:path_hash:{}",
            node_uuid.trim().to_ascii_lowercase(),
            path_hash
        )
    }

    async fn resolve_google_drive_remote(
        &self,
        node: &BackendNode,
        raw_path: &str,
        proxy_mode: ProxyMode,
    ) -> Result<Source, String> {
        let cfg = node
            .google_drive
            .as_ref()
            .ok_or_else(|| "missing googleDrive config".to_string())?;
        let resolved_path =
            google_drive::resolve_google_drive_path(raw_path, cfg)
                .map_err(str::to_string)?;
        let cache_key =
            Self::google_drive_file_id_cache_key(&cfg.node_uuid, raw_path);
        let file_id_cache = self.state.get_google_drive_file_id_cache().await;
        let request_lock = AppState::request_lock(
            &self.state.google_drive_file_id_request_locks,
            &cache_key,
        );
        let guard = request_lock.lock().await;

        let file_id = if let Some(cached) =
            file_id_cache.get::<String>(&cache_key)
        {
            cached
        } else {
            let client = self.state.get_google_drive_client().await.clone();
            let access_token = google_drive_auth::current_access_token(
                &self.state.google_drive_access_token_cache,
                node,
            )
            .ok_or_else(|| "googleDrive access_token is empty".to_string())?;
            let resolved = client
                .resolve_file_id_by_path(
                    &access_token,
                    &resolved_path.lookup,
                    &resolved_path.relative_path,
                )
                .await
                .map_err(|error| error.to_string())?;
            file_id_cache.insert(cache_key.clone(), resolved.file_id.clone());
            resolved.file_id
        };
        drop(guard);
        AppState::cleanup_request_lock(
            &self.state.google_drive_file_id_request_locks,
            &cache_key,
            &request_lock,
        );

        let auth_line = google_drive_auth::authorization_line_for_remote(
            &self.state.google_drive_access_token_cache,
            node,
        )
        .ok_or_else(|| "googleDrive access_token is empty".to_string())?;
        let auth_headers =
            google_drive_auth::extra_headers_from_auth_line(&auth_line)
                .map_err(str::to_string)?;
        let client = self.state.get_google_drive_client().await.clone();
        let remote_uri: Uri = client
            .build_media_url(&file_id)
            .parse()
            .map_err(|_| "invalid googleDrive media uri".to_string())?;

        if proxy_mode == ProxyMode::AccelRedirect {
            let info = Self::build_google_drive_accel_redirect_info(
                &cfg.node_uuid,
                &file_id,
                &auth_headers,
            )
            .map_err(|error| error.to_string())?;
            return Ok(Source::AccelRedirect { info });
        }

        Ok(Source::Remote {
            uri: remote_uri,
            mode: proxy_mode,
            extra_upstream_headers: Some(auth_headers),
        })
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
        stream_session_id: Option<&str>,
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
            stream_session_id,
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

    async fn remote_extra_headers(
        &self,
        node: &BackendNode,
        uri: &Uri,
        client_headers: &HeaderMap,
        stream_session_id: Option<&str>,
        source_headers: Option<HeaderMap>,
    ) -> Result<Option<HeaderMap>, StatusCode> {
        if source_headers.is_some() {
            return Ok(source_headers);
        }
        self.webdav_proxy_auth_headers(
            node,
            uri,
            client_headers,
            stream_session_id,
        )
        .await
    }

    async fn probe_google_drive_redirect_target(
        &self,
        node: &BackendNode,
        uri: &Uri,
        extra_headers: Option<&HeaderMap>,
        user_agent: &str,
        stream_session_id: &str,
    ) -> Result<(), StatusCode> {
        if !google_drive_auth::is_google_drive_node(node) {
            return Ok(());
        }
        let auth_value = extra_headers
            .and_then(|headers| headers.get(header::AUTHORIZATION))
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        if auth_value.is_empty() {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }

        let status = upstream_proxy::probe_authorization(
            uri.clone(),
            auth_value,
            user_agent,
            Some(stream_session_id),
        )
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
        if status.is_success() {
            return Ok(());
        }

        google_drive_auth::trigger_refresh_if_needed(
            self.state.clone(),
            node.clone(),
        );
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Instant};

    use dashmap::DashMap;
    use hyper::{HeaderMap, Uri, header};
    use tokio::sync::Mutex as TokioMutex;

    use super::AppStreamService;
    use crate::{
        AppState,
        config::{
            backend::{BackendNode, GoogleDriveConfig},
            core::{finish_raw_config, parse_raw_config_str},
        },
        core::request::Request as AppStreamRequest,
    };

    const MIN_FRONTEND_CONFIG: &str = r#"
[Log]
level = "info"
prefix = ""
root_path = "./logs"

[General]
memory_mode = "middle"
stream_mode = "frontend"
encipher_key = "1234567890123456"
encipher_iv = "1234567890123456"

[Emby]
url = "http://127.0.0.1"
port = "8096"
token = "tok"

[UserAgent]
mode = "allow"
allow_ua = []
deny_ua = []

[Fallback]

[Frontend]
listen_port = 60001

[Frontend.AntiReverseProxy]
enable = false
host = ""
"#;

    async fn test_state() -> AppState {
        let raw = parse_raw_config_str(MIN_FRONTEND_CONFIG).expect("parse");
        let config =
            finish_raw_config("test.toml".into(), raw).expect("finish");
        AppState::new(config).await
    }

    fn google_drive_node() -> BackendNode {
        BackendNode {
            name: "GoogleDrive".to_string(),
            backend_type: "googleDrive".to_string(),
            pattern: String::new(),
            pattern_regex: None,
            base_url: String::new(),
            port: String::new(),
            path: String::new(),
            priority: 0,
            proxy_mode: "redirect".to_string(),
            client_speed_limit_kbs: 0,
            client_burst_speed_kbs: 0,
            path_rewrites: vec![],
            anti_reverse_proxy: Default::default(),
            path_rewriter_cache: vec![],
            uuid: "node-uuid".to_string(),
            disk: None,
            open_list: None,
            direct_link: None,
            google_drive: Some(GoogleDriveConfig {
                node_uuid: "gd-node".to_string(),
                client_id: "client-id".to_string(),
                client_secret: "client-secret".to_string(),
                drive_id: String::new(),
                drive_name: "pilipili".to_string(),
                access_token: "access-token".to_string(),
                refresh_token: "refresh-token".to_string(),
            }),
            webdav: None,
        }
    }

    #[test]
    fn open_list_cache_key_is_structured() {
        let key = AppStreamService::open_list_cache_key(
            "Node-01",
            &Uri::from_static("/mnt/media/Show/Episode01.mkv"),
            "ExampleUA/1.0",
        );

        assert!(key.starts_with("backend:openlist:node:node-01:path_hash:"));
        assert!(key.contains(":ua_hash:"));
    }

    #[test]
    fn open_list_cache_key_trims_trailing_whitespace_only() {
        let key1 = AppStreamService::open_list_cache_key(
            "node",
            &Uri::from_static("/mnt/media/file.mkv"),
            "ExampleUA/1.0",
        );
        let key2 = AppStreamService::open_list_cache_key(
            "node",
            &Uri::from_static("/mnt/media/file.mkv"),
            "ExampleUA/1.0 ",
        );

        assert_eq!(key1, key2);
    }

    #[tokio::test]
    async fn open_list_request_lock_reuses_same_key_mutex() {
        let locks = DashMap::<String, Arc<TokioMutex<()>>>::new();
        let key = "backend:openlist:node:n1:path_hash:a:ua_hash:b";

        let lock1 = AppState::request_lock(&locks, key);
        let lock2 = AppState::request_lock(&locks, key);

        assert!(Arc::ptr_eq(&lock1, &lock2));

        let _guard = lock1.lock().await;
        assert!(lock2.try_lock().is_err());
    }

    #[test]
    fn open_list_request_lock_separates_distinct_keys() {
        let locks = DashMap::<String, Arc<TokioMutex<()>>>::new();

        let lock1 = AppState::request_lock(&locks, "key1");
        let lock2 = AppState::request_lock(&locks, "key2");

        assert!(!Arc::ptr_eq(&lock1, &lock2));
    }

    #[test]
    fn google_drive_file_id_cache_key_is_structured() {
        let key = AppStreamService::google_drive_file_id_cache_key(
            "Node-01",
            "/mnt/media/pilipili/Show/Episode01.mkv",
        );

        assert!(key.starts_with(
            "backend:google-drive:file-id:node:node-01:path_hash:"
        ));
    }

    #[test]
    fn build_google_drive_accel_redirect_info_embeds_auth_in_query() {
        let mut auth_headers = HeaderMap::new();
        auth_headers.insert(
            header::AUTHORIZATION,
            "Bearer access-token".parse().expect("auth header"),
        );

        let info = AppStreamService::build_google_drive_accel_redirect_info(
            "gd-node",
            "file-id-123",
            &auth_headers,
        )
        .expect("accel redirect info");

        assert_eq!(
            info.internal_path,
            "/_origin/google-drive/gd-node/file%2Did%2D123?\
token=Bearer+access-token"
        );
        assert!(info.internal_headers.get(header::AUTHORIZATION).is_none());
        assert!(info.internal_headers.is_empty());
    }

    #[tokio::test]
    async fn build_redirect_info_injects_google_drive_auth_and_strips_host() {
        let state = Arc::new(test_state().await);
        let service = AppStreamService::new(state);
        let node = google_drive_node();

        let mut request_headers = HeaderMap::new();
        request_headers.insert(
            header::HOST,
            "gateway.local".parse().expect("host header"),
        );
        request_headers
            .insert(header::RANGE, "bytes=0-1".parse().expect("range header"));

        let request = AppStreamRequest::new(
            Uri::from_static("/stream"),
            request_headers,
            Instant::now(),
            Some(node),
        );

        let mut extra_headers = HeaderMap::new();
        extra_headers.insert(
            header::AUTHORIZATION,
            "Bearer access-token".parse().expect("auth header"),
        );

        let redirect_info = service
            .build_redirect_info(
                "https://www.googleapis.com/drive/v3/files/file-id?alt=media"
                    .parse()
                    .expect("redirect target"),
                &request,
                Some(extra_headers),
            )
            .await
            .expect("redirect info");

        assert!(redirect_info.final_headers.get(header::HOST).is_none());
        assert_eq!(
            redirect_info
                .final_headers
                .get(header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok()),
            Some("Bearer access-token")
        );
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
                Source::AccelRedirect { .. } => "AccelRedirect",
            }
        );
        info_log!(STREAM_LOGGER_DOMAIN, "Routing stream source: {:?}", source);

        match source {
            Source::Local {
                path,
                device_id,
                playback_session_id,
            } => {
                info_log!(
                    STREAM_LOGGER_DOMAIN,
                    "local_stream_context device_id={} session_id={} path={:?}",
                    device_id,
                    playback_session_id,
                    path
                );
                let client_info = ClientInfo::new(
                    Some(device_id),
                    Some(playback_session_id),
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
            Source::AccelRedirect { info } => {
                Ok(AppStreamResult::AccelRedirect(info))
            }
            Source::Remote {
                uri,
                mode,
                extra_upstream_headers,
            } => match mode {
                ProxyMode::Redirect => {
                    let stream_session_id = generate_stream_session_id();
                    let user_agent =
                        Self::resolve_upstream_user_agent(node, &request);
                    self.probe_google_drive_redirect_target(
                        node,
                        &uri,
                        extra_upstream_headers.as_ref(),
                        &user_agent,
                        stream_session_id.as_str(),
                    )
                    .await?;
                    let redirect_info = self
                        .build_redirect_info(
                            uri,
                            &request,
                            extra_upstream_headers,
                        )
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
                    let stream_session_id = generate_stream_session_id();
                    info_log!(
                        STREAM_LOGGER_DOMAIN,
                        "remote_stream_context stream_session_id={} node={} uri={}",
                        stream_session_id,
                        node.name,
                        uri
                    );
                    let extra_headers = self
                        .remote_extra_headers(
                            node,
                            &uri,
                            &request.original_headers,
                            Some(stream_session_id.as_str()),
                            extra_upstream_headers,
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
                        stream_session_id,
                    })
                    .await
                }
                ProxyMode::AccelRedirect => {
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            },
        }
    }
}
