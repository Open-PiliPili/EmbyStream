use std::{sync::Arc, time::Duration as StdDuration};

use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use hyper::{HeaderMap, header};
use tokio::sync::Mutex as AsyncMutex;

use crate::{
    AppState,
    config::backend::{BackendNode, GoogleDriveConfig},
    debug_log, error_log, info_log,
    oauthutil::{
        GoogleDriveTokenSource, OAuthToken, TokenRequest, TokenSourceError,
    },
};

use super::google_drive::BACKEND_TYPE as GOOGLE_DRIVE_BACKEND_TYPE;

const GOOGLE_DRIVE_AUTH_LOGGER_DOMAIN: &str = "GOOGLE-DRIVE-AUTH";

pub const LOOKUP_MIN_VALID_SECS: i64 = 60;
pub const PROXY_MIN_VALID_SECS: i64 = 120;
pub const ACCEL_REDIRECT_MIN_VALID_SECS: i64 = 600;
const PRE_REFRESH_LEAD_SECS: i64 = ACCEL_REDIRECT_MIN_VALID_SECS;
const REFRESH_SCHEDULER_IDLE_SECS: i64 = 300;
const REFRESH_SCHEDULER_MIN_SLEEP_SECS: i64 = 1;

pub type GoogleDriveRefreshLocks = DashMap<String, Arc<AsyncMutex<()>>>;

pub fn is_google_drive_node(node: &BackendNode) -> bool {
    node.backend_type
        .eq_ignore_ascii_case(GOOGLE_DRIVE_BACKEND_TYPE)
}

pub fn extra_headers_from_auth_line(
    line: &str,
) -> Result<HeaderMap, &'static str> {
    let mut map = HeaderMap::new();
    let value = line
        .parse()
        .map_err(|_| "invalid authorization header value")?;
    map.insert(header::AUTHORIZATION, value);
    Ok(map)
}

pub async fn token_for_request(
    state: Arc<AppState>,
    node: BackendNode,
    reason: &'static str,
    min_valid_for: Duration,
) -> Result<OAuthToken, TokenSourceError> {
    let snapshot = GoogleDriveTokenSource::new(state, node)
        .token(TokenRequest::new(reason, min_valid_for))
        .await?;

    debug_log!(
        GOOGLE_DRIVE_AUTH_LOGGER_DOMAIN,
        "google_drive_token_selected reason={} source={}",
        reason,
        snapshot.source
    );

    Ok(snapshot.token)
}

pub async fn authorization_line_for_remote(
    state: Arc<AppState>,
    node: BackendNode,
    reason: &'static str,
    min_valid_for: Duration,
) -> Result<String, TokenSourceError> {
    let token =
        token_for_request(state, node.clone(), reason, min_valid_for).await?;
    token.authorization_header_value().ok_or_else(|| {
        TokenSourceError::MissingAccessToken {
            node: node.name.clone(),
        }
    })
}

pub async fn force_refresh(
    state: Arc<AppState>,
    node: BackendNode,
    reason: &'static str,
) -> Result<OAuthToken, TokenSourceError> {
    GoogleDriveTokenSource::new(state, node)
        .token(TokenRequest::force_refresh(reason))
        .await
        .map(|snapshot| snapshot.token)
}

pub fn invalidate(state: &Arc<AppState>, node: &BackendNode) {
    GoogleDriveTokenSource::new(state.clone(), node.clone()).invalidate();
}

fn collect_refreshable_google_drive_nodes(
    nodes: &[BackendNode],
) -> Vec<BackendNode> {
    nodes
        .iter()
        .filter_map(|node| {
            let cfg = google_drive_config(node)?;
            if cfg.node_uuid.trim().is_empty() {
                return None;
            }
            cfg.effective_refresh_token()?;
            Some(node.clone())
        })
        .collect()
}

fn google_drive_config(node: &BackendNode) -> Option<&GoogleDriveConfig> {
    if !is_google_drive_node(node) {
        return None;
    }
    node.google_drive.as_ref()
}

pub fn log_token_error(
    reason: &str,
    node: &BackendNode,
    error: &TokenSourceError,
) {
    error_log!(
        GOOGLE_DRIVE_AUTH_LOGGER_DOMAIN,
        "google_drive_token_error node={} reason={} error={}",
        node.name,
        reason,
        error
    );
}

pub fn trigger_refresh_if_needed(state: Arc<AppState>, node: BackendNode) {
    tokio::spawn(async move {
        match force_refresh(state, node.clone(), "background_refresh").await {
            Ok(_) => {
                info_log!(
                    GOOGLE_DRIVE_AUTH_LOGGER_DOMAIN,
                    "google_drive_background_refresh_succeeded node={}",
                    node.name
                );
            }
            Err(error) => {
                debug_log!(
                    GOOGLE_DRIVE_AUTH_LOGGER_DOMAIN,
                    "google_drive_background_refresh_skipped node={} error={}",
                    node.name,
                    error
                );
            }
        }
    });
}

pub async fn schedule_google_drive_token_refreshes(state: Arc<AppState>) {
    loop {
        let nodes = {
            let config = state.get_config().await;
            collect_refreshable_google_drive_nodes(&config.backend_nodes)
        };

        if nodes.is_empty() {
            debug_log!(
                GOOGLE_DRIVE_AUTH_LOGGER_DOMAIN,
                "google_drive_refresh_scheduler_skip reason=no_nodes"
            );
            tokio::time::sleep(std_duration_from_chrono(Duration::seconds(
                REFRESH_SCHEDULER_IDLE_SECS,
            )))
            .await;
            continue;
        }

        let now = Utc::now();
        let mut due_nodes = Vec::new();
        let mut next_sleep = Duration::seconds(REFRESH_SCHEDULER_IDLE_SECS);

        for node in nodes {
            let decision = scheduled_refresh_delay(&state, &node, now);
            if decision <= Duration::zero() {
                due_nodes.push(node);
            } else if decision < next_sleep {
                next_sleep = decision;
            }
        }

        if due_nodes.is_empty() {
            debug_log!(
                GOOGLE_DRIVE_AUTH_LOGGER_DOMAIN,
                "google_drive_refresh_scheduler_sleep sleep_secs={}",
                next_sleep.num_seconds()
            );
            tokio::time::sleep(std_duration_from_chrono(next_sleep)).await;
            continue;
        }

        info_log!(
            GOOGLE_DRIVE_AUTH_LOGGER_DOMAIN,
            "google_drive_refresh_scheduler_due nodes={}",
            due_nodes.len()
        );

        for node in due_nodes {
            let result = token_for_request(
                state.clone(),
                node.clone(),
                "scheduled_prerefresh",
                Duration::seconds(PRE_REFRESH_LEAD_SECS),
            )
            .await;
            match result {
                Ok(_) => {
                    info_log!(
                        GOOGLE_DRIVE_AUTH_LOGGER_DOMAIN,
                        "google_drive_prerefresh_succeeded node={}",
                        node.name
                    );
                }
                Err(error) => {
                    debug_log!(
                        GOOGLE_DRIVE_AUTH_LOGGER_DOMAIN,
                        "google_drive_prerefresh_skipped node={} error={}",
                        node.name,
                        error
                    );
                }
            }
        }
    }
}

pub async fn prewarm_google_drive_tokens(state: Arc<AppState>) {
    let nodes = {
        let config = state.get_config().await;
        collect_refreshable_google_drive_nodes(&config.backend_nodes)
    };

    if nodes.is_empty() {
        debug_log!(
            GOOGLE_DRIVE_AUTH_LOGGER_DOMAIN,
            "google_drive_prewarm_skip reason=no_nodes"
        );
        return;
    }

    info_log!(
        GOOGLE_DRIVE_AUTH_LOGGER_DOMAIN,
        "google_drive_prewarm_start nodes={}",
        nodes.len()
    );

    for node in nodes {
        let result = token_for_request(
            state.clone(),
            node.clone(),
            "startup_prewarm",
            Duration::seconds(PROXY_MIN_VALID_SECS),
        )
        .await;
        if let Err(error) = result {
            debug_log!(
                GOOGLE_DRIVE_AUTH_LOGGER_DOMAIN,
                "google_drive_prewarm_failed node={} error={}",
                node.name,
                error
            );
        }
    }
}

fn scheduled_refresh_delay(
    state: &AppState,
    node: &BackendNode,
    now: DateTime<Utc>,
) -> Duration {
    let Some(cfg) = google_drive_config(node) else {
        return Duration::seconds(REFRESH_SCHEDULER_IDLE_SECS);
    };

    let node_uuid = cfg.node_uuid.trim();
    if node_uuid.is_empty() {
        return Duration::seconds(REFRESH_SCHEDULER_IDLE_SECS);
    }

    if let Some(backoff) = state
        .google_drive_refresh_backoff_until
        .get(node_uuid)
        .map(|value| *value.value() - now)
        .filter(|remaining| *remaining > Duration::zero())
    {
        return backoff;
    }

    let min_valid_for = Duration::seconds(PRE_REFRESH_LEAD_SECS);
    match cfg.effective_token() {
        Some(token) if token.is_valid_for(min_valid_for, now) => token
            .remaining_lifetime(now)
            .map(|remaining| remaining - min_valid_for)
            .filter(|remaining| *remaining > Duration::zero())
            .unwrap_or_else(|| {
                Duration::seconds(REFRESH_SCHEDULER_MIN_SLEEP_SECS)
            }),
        _ => Duration::zero(),
    }
}

fn std_duration_from_chrono(duration: Duration) -> StdDuration {
    let secs = duration.num_seconds().max(REFRESH_SCHEDULER_MIN_SLEEP_SECS);
    StdDuration::from_secs(secs as u64)
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone, Utc};

    use super::{
        ACCEL_REDIRECT_MIN_VALID_SECS, collect_refreshable_google_drive_nodes,
        scheduled_refresh_delay,
    };
    use crate::{
        AppState,
        config::backend::{BackendNode, GoogleDriveConfig},
        config::core::{finish_raw_config, parse_raw_config_str},
        oauthutil::OAuthToken,
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

    fn google_drive_node(
        name: &str,
        node_uuid: &str,
        refresh_token: &str,
    ) -> BackendNode {
        BackendNode {
            name: name.to_string(),
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
            uuid: String::new(),
            disk: None,
            open_list: None,
            direct_link: None,
            google_drive: Some(GoogleDriveConfig {
                node_uuid: node_uuid.to_string(),
                client_id: "client-id".to_string(),
                client_secret: "client-secret".to_string(),
                drive_id: String::new(),
                drive_name: String::new(),
                access_token: String::new(),
                refresh_token: refresh_token.to_string(),
                token: None,
            }),
            webdav: None,
        }
    }

    async fn test_state_with_node(node: BackendNode) -> AppState {
        let raw = parse_raw_config_str(MIN_FRONTEND_CONFIG).expect("parse");
        let mut config =
            finish_raw_config("test.toml".into(), raw).expect("finish");
        config.backend_nodes = vec![node];
        AppState::new(config).await
    }

    #[test]
    fn collect_refreshable_google_drive_nodes_only_keeps_valid_google_drive_nodes()
     {
        let mut token_only_refresh =
            google_drive_node("google-token", "n2", "");
        if let Some(google_drive) = token_only_refresh.google_drive.as_mut() {
            google_drive.token = Some(OAuthToken {
                access_token: String::new(),
                refresh_token: "refresh-from-blob".to_string(),
                token_type: "Bearer".to_string(),
                expiry: None,
            });
        }
        let nodes = vec![
            google_drive_node("google-1", "node-1", "refresh-1"),
            google_drive_node("google-missing-uuid", "", "refresh-2"),
            google_drive_node("google-missing-refresh", "node-3", ""),
            token_only_refresh,
            BackendNode {
                name: "disk-1".to_string(),
                backend_type: "disk".to_string(),
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
                uuid: String::new(),
                disk: None,
                open_list: None,
                direct_link: None,
                google_drive: None,
                webdav: None,
            },
        ];

        let refreshable = collect_refreshable_google_drive_nodes(&nodes);

        assert_eq!(refreshable.len(), 2);
        assert_eq!(refreshable[0].name, "google-1");
        assert_eq!(refreshable[1].name, "google-token");
    }

    #[tokio::test]
    async fn scheduled_refresh_delay_is_zero_for_missing_expiry() {
        let now = Utc
            .with_ymd_and_hms(2026, 4, 16, 12, 0, 0)
            .single()
            .expect("valid timestamp");
        let mut node = google_drive_node("google-1", "node-1", "refresh-1");
        node.google_drive.as_mut().expect("google config").token =
            Some(OAuthToken::from_refresh_parts(
                "access-token".to_string(),
                "refresh-1".to_string(),
                "Bearer".to_string(),
                None,
            ));
        let state = test_state_with_node(node.clone()).await;

        assert_eq!(
            scheduled_refresh_delay(&state, &node, now),
            Duration::zero()
        );
    }

    #[tokio::test]
    async fn scheduled_refresh_delay_waits_until_lead_window() {
        let now = Utc
            .with_ymd_and_hms(2026, 4, 16, 12, 0, 0)
            .single()
            .expect("valid timestamp");
        let mut node = google_drive_node("google-1", "node-1", "refresh-1");
        node.google_drive.as_mut().expect("google config").token =
            Some(OAuthToken::from_refresh_parts(
                "access-token".to_string(),
                "refresh-1".to_string(),
                "Bearer".to_string(),
                Some(now + Duration::seconds(1800)),
            ));
        let state = test_state_with_node(node.clone()).await;

        assert_eq!(
            scheduled_refresh_delay(&state, &node, now),
            Duration::seconds(1800 - ACCEL_REDIRECT_MIN_VALID_SECS)
        );
    }

    #[tokio::test]
    async fn scheduled_refresh_delay_respects_backoff() {
        let now = Utc
            .with_ymd_and_hms(2026, 4, 16, 12, 0, 0)
            .single()
            .expect("valid timestamp");
        let node = google_drive_node("google-1", "node-1", "refresh-1");
        let state = test_state_with_node(node.clone()).await;
        state
            .google_drive_refresh_backoff_until
            .insert("node-1".to_string(), now + Duration::seconds(42));

        assert_eq!(
            scheduled_refresh_delay(&state, &node, now),
            Duration::seconds(42)
        );
    }
}
