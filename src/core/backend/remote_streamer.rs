use std::sync::Arc;

use http_body_util::BodyExt;
use hyper::{
    HeaderMap, Response as HyperResponse, StatusCode, Uri, body::Incoming,
    header,
};

use super::{
    google_drive_auth, response::Response, result::Result as AppStreamResult,
    upstream_proxy, webdav::BACKEND_TYPE as WEBDAV_BACKEND_TYPE, webdav_auth,
};
use crate::{
    AppState, REMOTE_STREAMER_LOGGER_DOMAIN, config::backend::BackendNode,
    error_log, gateway::error::Error as GatewayError, info_log,
};

/// Parameters for proxying a ranged GET to an upstream HTTP(S) origin.
pub struct RemoteStreamParams<'a> {
    pub state: Arc<AppState>,
    pub url: Uri,
    pub user_agent: String,
    pub client_headers: &'a HeaderMap,
    pub extra_upstream_headers: Option<HeaderMap>,
    pub client: Option<String>,
    pub client_ip: Option<String>,
    pub node: &'a BackendNode,
    /// One UUID per proxied client request (probe + GET + optional 401 retry share this id).
    pub stream_session_id: String,
}

fn is_webdav_node(node: &BackendNode) -> bool {
    node.backend_type.eq_ignore_ascii_case(WEBDAV_BACKEND_TYPE)
}

fn is_google_drive_node(node: &BackendNode) -> bool {
    google_drive_auth::is_google_drive_node(node)
}

fn webdav_needs_auth_retry(node: &BackendNode, status: StatusCode) -> bool {
    status == StatusCode::UNAUTHORIZED
        && is_webdav_node(node)
        && node
            .webdav
            .as_ref()
            .map(|w| {
                !w.username.trim().is_empty() || !w.password.trim().is_empty()
            })
            .unwrap_or(false)
}

async fn drain_incoming(body: Incoming) -> Result<(), GatewayError> {
    let _ = BodyExt::collect(body).await?;
    Ok(())
}

pub(crate) struct RemoteStreamer;

impl RemoteStreamer {
    pub async fn stream(
        params: RemoteStreamParams<'_>,
    ) -> Result<AppStreamResult, StatusCode> {
        let RemoteStreamParams {
            state,
            url,
            user_agent,
            client_headers,
            extra_upstream_headers,
            client,
            client_ip,
            node,
            stream_session_id,
        } = params;

        if !client_headers.contains_key(header::RANGE) {
            error_log!(
                REMOTE_STREAMER_LOGGER_DOMAIN,
                "No-Range req for '{:?}' rejected. IP: {:?}, Client: {:?}",
                &url,
                client,
                client_ip
            );
            return Err(StatusCode::FORBIDDEN);
        }

        let extra_ref = extra_upstream_headers.as_ref();

        let upstream_resp = upstream_proxy::forward_get(
            url.clone(),
            client_headers,
            &user_agent,
            extra_ref,
            Some(stream_session_id.as_str()),
        )
        .await
        .map_err(|e| {
            error_log!(
                REMOTE_STREAMER_LOGGER_DOMAIN,
                "Upstream forward failed: {}",
                e
            );
            StatusCode::BAD_GATEWAY
        })?;

        let upstream_resp = Self::maybe_retry_webdav_401(
            state.clone(),
            node,
            upstream_resp,
            url,
            client_headers,
            &user_agent,
            stream_session_id.as_str(),
        )
        .await?;

        let status = upstream_resp.status();
        if !status.is_success() {
            error_log!(
                REMOTE_STREAMER_LOGGER_DOMAIN,
                "Upstream returned error status: {}",
                status
            );
            let (_, body) = upstream_resp.into_parts();
            let _ = drain_incoming(body).await;
            if status == StatusCode::UNAUTHORIZED && is_webdav_node(node) {
                return Err(StatusCode::UNAUTHORIZED);
            }
            if is_google_drive_node(node) {
                google_drive_auth::trigger_refresh_if_needed(
                    state.clone(),
                    node.clone(),
                );
                return Err(StatusCode::SERVICE_UNAVAILABLE);
            }
            return Err(StatusCode::BAD_GATEWAY);
        }

        let (response_status, response_headers, body) =
            upstream_proxy::map_upstream_to_stream_response(upstream_resp)
                .map_err(|e| {
                    error_log!(
                        REMOTE_STREAMER_LOGGER_DOMAIN,
                        "Map upstream response failed: {}",
                        e
                    );
                    StatusCode::BAD_GATEWAY
                })?;

        Ok(AppStreamResult::Stream(Response {
            status: response_status,
            headers: response_headers,
            body,
        }))
    }

    async fn maybe_retry_webdav_401(
        state: Arc<AppState>,
        node: &BackendNode,
        upstream_resp: HyperResponse<Incoming>,
        url: Uri,
        headers: &HeaderMap,
        user_agent: &str,
        stream_session_id: &str,
    ) -> Result<HyperResponse<Incoming>, StatusCode> {
        let status = upstream_resp.status();

        if !webdav_needs_auth_retry(node, status) {
            return Ok(upstream_resp);
        }

        let (_, body) = upstream_resp.into_parts();
        drain_incoming(body).await.map_err(|e| {
            error_log!(
                REMOTE_STREAMER_LOGGER_DOMAIN,
                "Drain upstream body: {}",
                e
            );
            StatusCode::BAD_GATEWAY
        })?;

        info_log!(
            REMOTE_STREAMER_LOGGER_DOMAIN,
            "webdav_upstream_401_retry webdav_upstream_401_retry=1 node={} uri_hint={}{}",
            node.name,
            upstream_proxy::upstream_uri_hint(&url),
            upstream_proxy::stream_session_log_suffix(Some(stream_session_id)),
        );

        let Some(cfg) = node.webdav.as_ref() else {
            return Err(StatusCode::UNAUTHORIZED);
        };

        webdav_auth::invalidate(
            &state.webdav_auth_cache,
            &state.webdav_auth_probe_locks,
            node,
        );

        let auth_line = match webdav_auth::authorization_header_for_proxy(
            &state.webdav_auth_cache,
            &state.webdav_auth_probe_locks,
            node,
            &url,
            cfg,
            Some(headers),
            Some(stream_session_id),
        )
        .await
        {
            Ok(Some(line)) => line,
            Ok(None) | Err(()) => return Err(StatusCode::UNAUTHORIZED),
        };

        let refreshed = webdav_auth::extra_headers_from_auth_line(&auth_line)
            .map_err(|_| {
            error_log!(
                REMOTE_STREAMER_LOGGER_DOMAIN,
                "Invalid WebDav auth header after refresh"
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        upstream_proxy::forward_get(
            url,
            headers,
            user_agent,
            Some(&refreshed),
            Some(stream_session_id),
        )
        .await
        .map_err(|e| {
            error_log!(
                REMOTE_STREAMER_LOGGER_DOMAIN,
                "Upstream retry failed: {}",
                e
            );
            StatusCode::BAD_GATEWAY
        })
    }
}
