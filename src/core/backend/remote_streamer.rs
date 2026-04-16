use std::sync::Arc;

use chrono::Duration;
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
            url.clone(),
            client_headers,
            &user_agent,
            stream_session_id.as_str(),
        )
        .await?;
        let upstream_resp = Self::maybe_retry_google_drive_401(
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

    async fn maybe_retry_google_drive_401(
        state: Arc<AppState>,
        node: &BackendNode,
        upstream_resp: HyperResponse<Incoming>,
        url: Uri,
        headers: &HeaderMap,
        user_agent: &str,
        stream_session_id: &str,
    ) -> Result<HyperResponse<Incoming>, StatusCode> {
        if upstream_resp.status() != StatusCode::UNAUTHORIZED
            || !is_google_drive_node(node)
        {
            return Ok(upstream_resp);
        }

        let (_, body) = upstream_resp.into_parts();
        drain_incoming(body).await.map_err(|e| {
            error_log!(
                REMOTE_STREAMER_LOGGER_DOMAIN,
                "Drain googleDrive upstream body: {}",
                e
            );
            StatusCode::BAD_GATEWAY
        })?;

        info_log!(
            REMOTE_STREAMER_LOGGER_DOMAIN,
            "google_drive_upstream_401_retry node={} uri_hint={}{}",
            node.name,
            upstream_proxy::upstream_uri_hint(&url),
            upstream_proxy::stream_session_log_suffix(Some(stream_session_id)),
        );

        google_drive_auth::invalidate(&state, node);
        let auth_line = google_drive_auth::authorization_line_for_remote(
            state.clone(),
            node.clone(),
            "proxy_retry_401",
            Duration::seconds(google_drive_auth::PROXY_MIN_VALID_SECS),
        )
        .await
        .map_err(|error| {
            google_drive_auth::log_token_error("proxy_retry_401", node, &error);
            StatusCode::SERVICE_UNAVAILABLE
        })?;
        let refreshed =
            google_drive_auth::extra_headers_from_auth_line(&auth_line)
                .map_err(|_| {
                    error_log!(
                        REMOTE_STREAMER_LOGGER_DOMAIN,
                        "Invalid googleDrive auth header after refresh"
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
                "googleDrive upstream retry failed: {}",
                e
            );
            StatusCode::BAD_GATEWAY
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        sync::{Arc, Once},
    };

    use hyper::{HeaderMap, StatusCode, Uri, header};
    use rustls::crypto::aws_lc_rs;

    use super::{RemoteStreamParams, RemoteStreamer};
    use crate::{
        AppState,
        client::GoogleDriveClient,
        config::{
            backend::{BackendNode, GoogleDriveConfig},
            core::{finish_raw_config, parse_raw_config_str},
        },
        core::backend::result::Result as AppStreamResult,
        oauthutil::OAuthToken,
        test_support::{
            HttpMockHandler, http_response, spawn_http_mock_server,
        },
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

    static RUSTLS_CRYPTO_INIT: Once = Once::new();

    fn ensure_rustls_crypto_provider() {
        RUSTLS_CRYPTO_INIT.call_once(|| {
            let _ = aws_lc_rs::default_provider().install_default();
        });
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
            proxy_mode: "proxy".to_string(),
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
                drive_id: "drive-id".to_string(),
                drive_name: "pilipili".to_string(),
                access_token: "access-token".to_string(),
                refresh_token: "refresh-token".to_string(),
                token: Some(OAuthToken {
                    access_token: "access-token".to_string(),
                    refresh_token: "refresh-token".to_string(),
                    token_type: "Bearer".to_string(),
                    expiry: Some(
                        chrono::Utc::now() + chrono::Duration::hours(1),
                    ),
                }),
            }),
            webdav: None,
        }
    }

    async fn test_state_with_node(
        path: PathBuf,
        node: BackendNode,
    ) -> Arc<AppState> {
        let raw = parse_raw_config_str(MIN_FRONTEND_CONFIG).expect("parse");
        let mut config = finish_raw_config(path, raw).expect("finish");
        config.backend_nodes = vec![node];
        Arc::new(AppState::new(config).await)
    }

    fn google_drive_client_for_test(base: &str) -> Arc<GoogleDriveClient> {
        Arc::new(GoogleDriveClient::new_for_test(
            &format!("{base}/drive/v3"),
            &format!("{base}/token"),
        ))
    }

    #[tokio::test]
    async fn stream_retries_google_drive_proxy_after_401() {
        ensure_rustls_crypto_provider();
        let dir = tempfile::tempdir().expect("temp dir");
        let config_path = dir.path().join("config.toml");
        let config = r#"
[[BackendNode]]
name = "GoogleDrive"
type = "googleDrive"

[BackendNode.GoogleDrive]
node_uuid = "gd-node"
client_id = "client-id"
client_secret = "client-secret"
drive_id = "drive-id"
access_token = "access-token"
refresh_token = "refresh-token"
"#;
        std::fs::write(&config_path, config).expect("write config");

        let handlers: Vec<HttpMockHandler> = vec![
            Box::new(move |request| {
                Box::pin(async move {
                    assert!(request.starts_with("GET /media HTTP/1.1"));
                    assert!(
                        request.contains("authorization: Bearer access-token")
                    );
                    http_response(401, "text/plain", "")
                })
            }),
            Box::new(move |request| {
                Box::pin(async move {
                    assert!(request.starts_with("POST /token HTTP/1.1"));
                    http_response(
                        200,
                        "application/json",
                        r#"{"access_token":"refreshed-token","token_type":"Bearer","expires_in":3600}"#,
                    )
                })
            }),
            Box::new(move |request| {
                Box::pin(async move {
                    assert!(request.starts_with("GET /media HTTP/1.1"));
                    assert!(
                        request
                            .contains("authorization: Bearer refreshed-token")
                    );
                    http_response(206, "video/mp4", "ok")
                })
            }),
        ];
        let base = spawn_http_mock_server(handlers).await;
        let node = google_drive_node();
        let state = test_state_with_node(config_path, node.clone()).await;
        state.set_google_drive_client_for_test(google_drive_client_for_test(
            &base,
        ));

        let mut headers = HeaderMap::new();
        headers.insert(header::RANGE, "bytes=0-1".parse().expect("range"));
        let result = RemoteStreamer::stream(RemoteStreamParams {
            state,
            url: Uri::try_from(format!("{base}/media")).expect("uri"),
            user_agent: "UnitTest/1.0".to_string(),
            client_headers: &headers,
            extra_upstream_headers: Some({
                let mut extra = HeaderMap::new();
                extra.insert(
                    header::AUTHORIZATION,
                    "Bearer access-token".parse().expect("authorization"),
                );
                extra
            }),
            client: None,
            client_ip: None,
            node: &node,
            stream_session_id: "session-1".to_string(),
        })
        .await
        .expect("stream result");

        match result {
            AppStreamResult::Stream(response) => {
                assert_eq!(response.status, StatusCode::PARTIAL_CONTENT);
            }
            _ => panic!("unexpected non-stream result"),
        }
    }
}
