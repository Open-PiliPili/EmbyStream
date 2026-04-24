use std::{future::Future, sync::Arc};

use chrono::{DateTime, Duration, Utc};
use tokio::sync::Mutex as AsyncMutex;

use crate::{
    AppState,
    client::google_drive::GoogleTokenRefreshResponse,
    config::backend::BackendNode,
    debug_log, error_log, info_log,
    oauthutil::{
        TokenSnapshot, TokenSourceError, source::TokenRequest,
        store::GoogleDriveTokenStore, token::OAuthToken,
    },
};

const LOGGER_DOMAIN: &str = "GOOGLE-DRIVE-AUTH";
const REFRESH_FAILURE_BACKOFF_SECS: i64 = 30;

pub struct GoogleDriveTokenSource {
    state: Arc<AppState>,
    node: BackendNode,
}

impl GoogleDriveTokenSource {
    pub fn new(state: Arc<AppState>, node: BackendNode) -> Self {
        Self { state, node }
    }

    pub async fn token(
        &self,
        request: TokenRequest,
    ) -> Result<TokenSnapshot, TokenSourceError> {
        self.token_with_refresh(request, |refresh_token| async move {
            self.refresh_from_google(&refresh_token).await
        })
        .await
    }

    async fn token_with_refresh<RefreshFn, RefreshFut>(
        &self,
        request: TokenRequest,
        refresh_fn: RefreshFn,
    ) -> Result<TokenSnapshot, TokenSourceError>
    where
        RefreshFn: Fn(String) -> RefreshFut,
        RefreshFut: Future<Output = Result<OAuthToken, TokenSourceError>>,
    {
        let now = Utc::now();
        let node_name = self.node.name.clone();
        let (node_uuid, local_token) = self.local_token()?;

        if !request.force_refresh {
            if let Some(token) =
                self.cached_token(&node_uuid, request.min_valid_for, now)
            {
                debug_log!(
                    LOGGER_DOMAIN,
                    "google_drive_token_ready node={} node_uuid={} reason={} \
                     source=cache min_valid_for_secs={} remaining_secs={}",
                    node_name,
                    node_uuid,
                    request.reason,
                    request.min_valid_for.num_seconds(),
                    token
                        .remaining_lifetime(now)
                        .map(|value| value.num_seconds())
                        .unwrap_or_default()
                );
                return Ok(TokenSnapshot {
                    token,
                    source: "cache",
                });
            }
        }

        let lock = self.refresh_lock(&node_uuid);
        let _guard = lock.lock().await;
        let now = Utc::now();

        if !request.force_refresh {
            if let Some(token) =
                self.cached_token(&node_uuid, request.min_valid_for, now)
            {
                return Ok(TokenSnapshot {
                    token,
                    source: "cache_after_wait",
                });
            }
        }

        let config_path = {
            let config = self.state.get_config().await;
            config.path.clone()
        };

        if !request.force_refresh {
            if let Some(token) = GoogleDriveTokenStore::read(
                &config_path,
                &node_name,
                &node_uuid,
            )?
            .filter(|token| token.is_valid_for(request.min_valid_for, now))
            {
                self.update_runtime_token(&node_uuid, token.clone()).await;
                info_log!(
                    LOGGER_DOMAIN,
                    "google_drive_token_ready node={} node_uuid={} reason={} \
                     source=store remaining_secs={}",
                    node_name,
                    node_uuid,
                    request.reason,
                    token
                        .remaining_lifetime(now)
                        .map(|value| value.num_seconds())
                        .unwrap_or_default()
                );
                return Ok(TokenSnapshot {
                    token,
                    source: "store",
                });
            }
        }

        let refresh_seed =
            GoogleDriveTokenStore::read(&config_path, &node_name, &node_uuid)?
                .or(local_token);
        let refresh_token = refresh_seed
            .as_ref()
            .map(|token| token.refresh_token.trim().to_string())
            .filter(|token| !token.is_empty())
            .ok_or_else(|| TokenSourceError::MissingRefreshToken {
                node: node_name.clone(),
            })?;
        self.ensure_refresh_not_in_backoff(&node_name, &node_uuid)?;

        info_log!(
            LOGGER_DOMAIN,
            "google_drive_refresh_start node={} node_uuid={} reason={} \
             force_refresh={}",
            node_name,
            node_uuid,
            request.reason,
            request.force_refresh
        );

        let refreshed = match refresh_fn(refresh_token.clone()).await {
            Ok(token) => {
                self.clear_refresh_backoff(&node_uuid);
                token
            }
            Err(error) => {
                self.set_refresh_backoff(&node_uuid);
                return Err(error);
            }
        };
        GoogleDriveTokenStore::write(
            &config_path,
            &node_name,
            &node_uuid,
            &refreshed,
        )?;
        self.update_runtime_token(&node_uuid, refreshed.clone())
            .await;

        info_log!(
            LOGGER_DOMAIN,
            "google_drive_refresh_success node={} node_uuid={} reason={} \
             remaining_secs={}",
            node_name,
            node_uuid,
            request.reason,
            refreshed
                .remaining_lifetime(Utc::now())
                .map(|value| value.num_seconds())
                .unwrap_or_default()
        );

        Ok(TokenSnapshot {
            token: refreshed,
            source: "refresh",
        })
    }

    pub fn invalidate(&self) {
        if let Ok((node_uuid, _)) = self.local_token() {
            self.state
                .google_drive_token_cache
                .remove(&cache_key(&node_uuid));
            self.clear_refresh_backoff(&node_uuid);
            info_log!(
                LOGGER_DOMAIN,
                "google_drive_token_invalidated node={} node_uuid={}",
                self.node.name,
                node_uuid
            );
        }
    }

    fn local_token(
        &self,
    ) -> Result<(String, Option<OAuthToken>), TokenSourceError> {
        let cfg = self.node.google_drive.as_ref().ok_or_else(|| {
            TokenSourceError::MissingGoogleDriveConfig {
                node: self.node.name.clone(),
            }
        })?;
        let node_uuid = cfg.node_uuid.trim();
        if node_uuid.is_empty() {
            return Err(TokenSourceError::MissingNodeUuid {
                node: self.node.name.clone(),
            });
        }

        Ok((node_uuid.to_string(), cfg.effective_token()))
    }

    fn refresh_lock(&self, node_uuid: &str) -> Arc<AsyncMutex<()>> {
        self.state
            .google_drive_refresh_locks
            .entry(cache_key(node_uuid))
            .or_insert_with(|| Arc::new(AsyncMutex::new(())))
            .clone()
    }

    fn cached_token(
        &self,
        node_uuid: &str,
        min_valid_for: Duration,
        now: DateTime<Utc>,
    ) -> Option<OAuthToken> {
        self.state
            .google_drive_token_cache
            .get(&cache_key(node_uuid))
            .map(|entry| entry.value().clone())
            .filter(|token| token.is_valid_for(min_valid_for, now))
    }

    fn ensure_refresh_not_in_backoff(
        &self,
        node_name: &str,
        node_uuid: &str,
    ) -> Result<(), TokenSourceError> {
        let key = cache_key(node_uuid);
        let Some(until) = self
            .state
            .google_drive_refresh_backoff_until
            .get(&key)
            .map(|value| *value.value())
        else {
            return Ok(());
        };

        let remaining = until - Utc::now();
        if remaining > Duration::zero() {
            return Err(TokenSourceError::RefreshBackoff {
                node: node_name.to_string(),
                retry_after_secs: remaining.num_seconds(),
                reason: "recent_refresh_failure".to_string(),
            });
        }

        self.state.google_drive_refresh_backoff_until.remove(&key);
        Ok(())
    }

    async fn refresh_from_google(
        &self,
        refresh_token: &str,
    ) -> Result<OAuthToken, TokenSourceError> {
        let cfg = self.node.google_drive.as_ref().ok_or_else(|| {
            TokenSourceError::MissingGoogleDriveConfig {
                node: self.node.name.clone(),
            }
        })?;
        let client = self.state.get_google_drive_client().await.clone();
        let refreshed = client
            .refresh_access_token(
                &cfg.client_id,
                &cfg.client_secret,
                refresh_token,
            )
            .await
            .map_err(|error| {
                error_log!(
                    LOGGER_DOMAIN,
                    "google_drive_refresh_failed node={} error={}",
                    self.node.name,
                    error
                );
                TokenSourceError::Refresh {
                    node: self.node.name.clone(),
                    error: error.to_string(),
                }
            })?;

        Ok(token_from_refresh_response(refreshed, refresh_token))
    }

    async fn update_runtime_token(&self, node_uuid: &str, token: OAuthToken) {
        self.state
            .google_drive_token_cache
            .insert(cache_key(node_uuid), token.clone());

        let mut config = self.state.config.write().await;
        for backend_node in &mut config.backend_nodes {
            let Some(google_drive) = backend_node.google_drive.as_mut() else {
                continue;
            };
            if google_drive.node_uuid.trim() != node_uuid {
                continue;
            }
            google_drive.apply_token(token.clone());
            break;
        }
    }

    fn set_refresh_backoff(&self, node_uuid: &str) {
        self.state.google_drive_refresh_backoff_until.insert(
            cache_key(node_uuid),
            Utc::now() + Duration::seconds(REFRESH_FAILURE_BACKOFF_SECS),
        );
    }

    fn clear_refresh_backoff(&self, node_uuid: &str) {
        self.state
            .google_drive_refresh_backoff_until
            .remove(&cache_key(node_uuid));
    }
}

fn cache_key(node_uuid: &str) -> String {
    format!(
        "google-drive-token:{}",
        node_uuid.trim().to_ascii_lowercase()
    )
}

fn token_from_refresh_response(
    refreshed: GoogleTokenRefreshResponse,
    refresh_token: &str,
) -> OAuthToken {
    let expiry = refreshed
        .expires_in
        .and_then(|seconds| {
            chrono::Duration::from_std(std::time::Duration::from_secs(seconds))
                .ok()
        })
        .map(|duration| Utc::now() + duration);

    OAuthToken::from_refresh_parts(
        refreshed.access_token,
        refresh_token.to_string(),
        refreshed.token_type,
        expiry,
    )
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };

    use chrono::{DateTime, Duration, TimeZone, Utc};

    use super::{GoogleDriveTokenSource, token_from_refresh_response};
    use crate::{
        AppState,
        client::google_drive::GoogleTokenRefreshResponse,
        config::{
            backend::{BackendNode, GoogleDriveConfig},
            core::{finish_raw_config, parse_raw_config_str},
        },
        oauthutil::{OAuthToken, TokenRequest, TokenSourceError},
    };

    const GOOGLE_FRONTEND_CONFIG: &str = r#"
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

    fn google_node(
        node_uuid: &str,
        access_token: &str,
        refresh_token: &str,
        expiry: Option<DateTime<Utc>>,
    ) -> BackendNode {
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
            uuid: "runtime-node".to_string(),
            disk: None,
            open_list: None,
            direct_link: None,
            google_drive: Some(GoogleDriveConfig {
                node_uuid: node_uuid.to_string(),
                client_id: "client-id".to_string(),
                client_secret: "client-secret".to_string(),
                drive_id: "drive-id".to_string(),
                drive_name: "drive-name".to_string(),
                access_token: access_token.to_string(),
                refresh_token: refresh_token.to_string(),
                token: Some(OAuthToken {
                    access_token: access_token.to_string(),
                    refresh_token: refresh_token.to_string(),
                    token_type: "Bearer".to_string(),
                    expiry,
                }),
            }),
            webdav: None,
        }
    }

    async fn test_state_with_google_node(
        node: BackendNode,
        path: PathBuf,
    ) -> Arc<AppState> {
        let raw = parse_raw_config_str(GOOGLE_FRONTEND_CONFIG).expect("parse");
        let mut config = finish_raw_config(path, raw).expect("finish");
        config.backend_nodes = vec![node];
        Arc::new(AppState::new(config).await)
    }

    #[test]
    fn refresh_response_maps_expiry_and_refresh_token() {
        let now = Utc::now();
        let token = token_from_refresh_response(
            GoogleTokenRefreshResponse {
                access_token: "access-token".to_string(),
                token_type: "Bearer".to_string(),
                expires_in: Some(3600),
            },
            "refresh-token",
        );

        assert_eq!(token.refresh_token, "refresh-token");
        assert!(token.expiry.is_some());
        assert!(
            token
                .expiry
                .map(|expiry| expiry > now + Duration::minutes(50))
                .unwrap_or(false)
        );
    }

    #[tokio::test]
    async fn token_uses_store_reread_when_shared_store_is_newer() {
        let dir = tempfile::tempdir().expect("temp dir");
        let config_path = dir.path().join("config.toml");
        let expiry = (Utc::now() + Duration::minutes(30))
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let persisted = format!(
            r#"
[[BackendNode]]
name = "GoogleDrive"
type = "googleDrive"

[BackendNode.GoogleDrive]
node_uuid = "node-1"
client_id = "client-id"
client_secret = "client-secret"
drive_id = "drive-id"
access_token = "persisted-access"
refresh_token = "persisted-refresh"
token = {{ access_token = "persisted-access",
          refresh_token = "persisted-refresh",
          token_type = "Bearer",
          expiry = {expiry} }}
"#,
            expiry = expiry
        );
        std::fs::write(&config_path, persisted).expect("write config");

        let stale_node =
            google_node("node-1", "stale-access", "refresh-1", None);
        let state =
            test_state_with_google_node(stale_node.clone(), config_path).await;
        let source = GoogleDriveTokenSource::new(state, stale_node);
        let token = source
            .token_with_refresh(
                TokenRequest::new("reread_store", Duration::seconds(60)),
                |_| async {
                    panic!("refresh should not run when store token is valid")
                },
            )
            .await
            .expect("token");

        assert_eq!(token.source, "store");
        assert_eq!(token.token.access_token, "persisted-access");
    }

    #[tokio::test]
    async fn token_singleflight_runs_refresh_only_once() {
        let dir = tempfile::tempdir().expect("temp dir");
        let config_path = dir.path().join("config.toml");
        let initial = r#"
[[BackendNode]]
name = "GoogleDrive"
type = "googleDrive"

[BackendNode.GoogleDrive]
node_uuid = "node-1"
client_id = "client-id"
client_secret = "client-secret"
drive_id = "drive-id"
access_token = "stale-access"
refresh_token = "refresh-1"
"#;
        std::fs::write(&config_path, initial).expect("write config");

        let node = google_node("node-1", "stale-access", "refresh-1", None);
        let state =
            test_state_with_google_node(node.clone(), config_path).await;
        let counter = Arc::new(AtomicUsize::new(0));
        let mut tasks = Vec::new();

        for _ in 0..8 {
            let state = state.clone();
            let node = node.clone();
            let counter = counter.clone();
            tasks.push(tokio::spawn(async move {
                let source = GoogleDriveTokenSource::new(state, node);
                source
                    .token_with_refresh(
                        TokenRequest::new(
                            "singleflight",
                            Duration::seconds(60),
                        ),
                        move |refresh_token| {
                            let counter = counter.clone();
                            let refresh_token = refresh_token.to_string();
                            async move {
                                counter.fetch_add(1, Ordering::SeqCst);
                                tokio::time::sleep(
                                    std::time::Duration::from_millis(25),
                                )
                                .await;
                                Ok(OAuthToken::from_refresh_parts(
                                    "fresh-access".to_string(),
                                    refresh_token,
                                    "Bearer".to_string(),
                                    Some(Utc::now() + Duration::minutes(30)),
                                ))
                            }
                        },
                    )
                    .await
                    .expect("token")
            }));
        }

        let results = futures_util::future::join_all(tasks).await;
        for result in results {
            let snapshot = result.expect("join");
            assert_eq!(snapshot.token.access_token, "fresh-access");
        }
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn token_multi_instance_reuses_store_token_from_other_instance() {
        let dir = tempfile::tempdir().expect("temp dir");
        let config_path = dir.path().join("config.toml");
        let initial = r#"
[[BackendNode]]
name = "GoogleDrive"
type = "googleDrive"

[BackendNode.GoogleDrive]
node_uuid = "node-1"
client_id = "client-id"
client_secret = "client-secret"
drive_id = "drive-id"
access_token = "stale-access"
refresh_token = "refresh-1"
"#;
        std::fs::write(&config_path, initial).expect("write config");

        let node = google_node("node-1", "stale-access", "refresh-1", None);
        let state_a =
            test_state_with_google_node(node.clone(), config_path.clone())
                .await;
        let state_b =
            test_state_with_google_node(node.clone(), config_path.clone())
                .await;
        let refresh_counter = Arc::new(AtomicUsize::new(0));

        GoogleDriveTokenSource::new(state_a, node.clone())
            .token_with_refresh(
                TokenRequest::new("instance_a", Duration::seconds(60)),
                {
                    let refresh_counter = refresh_counter.clone();
                    move |refresh_token| {
                        let refresh_counter = refresh_counter.clone();
                        let refresh_token = refresh_token.to_string();
                        async move {
                            refresh_counter.fetch_add(1, Ordering::SeqCst);
                            Ok(OAuthToken::from_refresh_parts(
                                "shared-access".to_string(),
                                refresh_token,
                                "Bearer".to_string(),
                                Some(Utc::now() + Duration::minutes(30)),
                            ))
                        }
                    }
                },
            )
            .await
            .expect("instance a token");

        let snapshot = GoogleDriveTokenSource::new(state_b, node)
            .token_with_refresh(
                TokenRequest::new("instance_b", Duration::seconds(60)),
                |_| async {
                    panic!("instance b should reuse refreshed store token")
                },
            )
            .await
            .expect("instance b token");

        assert_eq!(snapshot.source, "store");
        assert_eq!(snapshot.token.access_token, "shared-access");
        assert_eq!(refresh_counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn token_enforces_accel_redirect_min_validity() {
        let dir = tempfile::tempdir().expect("temp dir");
        let config_path = dir.path().join("config.toml");
        let initial = r#"
[[BackendNode]]
name = "GoogleDrive"
type = "googleDrive"

[BackendNode.GoogleDrive]
node_uuid = "node-1"
client_id = "client-id"
client_secret = "client-secret"
drive_id = "drive-id"
access_token = "near-expiry"
refresh_token = "refresh-1"
"#;
        std::fs::write(&config_path, initial).expect("write config");

        let node = google_node(
            "node-1",
            "near-expiry",
            "refresh-1",
            Some(Utc::now() + Duration::minutes(5)),
        );
        let state =
            test_state_with_google_node(node.clone(), config_path).await;
        let snapshot = GoogleDriveTokenSource::new(state, node)
            .token_with_refresh(
                TokenRequest::new("accel_redirect", Duration::minutes(10)),
                |refresh_token| {
                    let refresh_token = refresh_token.to_string();
                    async move {
                        Ok(OAuthToken::from_refresh_parts(
                            "refreshed-for-accel".to_string(),
                            refresh_token,
                            "Bearer".to_string(),
                            Some(Utc::now() + Duration::minutes(30)),
                        ))
                    }
                },
            )
            .await
            .expect("token");

        assert_eq!(snapshot.source, "refresh");
        assert_eq!(snapshot.token.access_token, "refreshed-for-accel");
    }

    #[tokio::test]
    async fn token_does_not_update_runtime_cache_when_persist_fails() {
        #[cfg(unix)]
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("temp dir");
        let config_path = dir.path().join("config.toml");
        let initial = r#"
[[BackendNode]]
name = "GoogleDrive"
type = "googleDrive"

[BackendNode.GoogleDrive]
node_uuid = "node-1"
client_id = "client-id"
client_secret = "client-secret"
drive_id = "drive-id"
access_token = "stale-access"
refresh_token = "refresh-1"
"#;
        fs::write(&config_path, initial).expect("write config");
        #[cfg(unix)]
        {
            let mut permissions = fs::metadata(dir.path())
                .expect("dir metadata")
                .permissions();
            permissions.set_mode(0o555);
            fs::set_permissions(dir.path(), permissions)
                .expect("set readonly dir");
        }

        let node = google_node("node-1", "stale-access", "refresh-1", None);
        let state =
            test_state_with_google_node(node.clone(), config_path).await;
        let source = GoogleDriveTokenSource::new(state.clone(), node);

        let error = source
            .token_with_refresh(
                TokenRequest::new("persist_failure", Duration::seconds(60)),
                |refresh_token| {
                    let refresh_token = refresh_token.to_string();
                    async move {
                        Ok(OAuthToken::from_refresh_parts(
                            "fresh-access".to_string(),
                            refresh_token,
                            "Bearer".to_string(),
                            Some(Utc::now() + Duration::minutes(30)),
                        ))
                    }
                },
            )
            .await
            .expect_err("persist failure should fail");

        assert!(matches!(
            error,
            TokenSourceError::StoreWrite { .. }
                | TokenSourceError::StoreRead { .. }
        ));
        assert!(state.google_drive_token_cache.is_empty());

        #[cfg(unix)]
        {
            let mut permissions = fs::metadata(dir.path())
                .expect("dir metadata")
                .permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(dir.path(), permissions)
                .expect("restore dir permissions");
        }
    }

    #[tokio::test]
    async fn refresh_failure_sets_backoff_until_next_retry_window() {
        let dir = tempfile::tempdir().expect("temp dir");
        let config_path = dir.path().join("config.toml");
        let initial = r#"
[[BackendNode]]
name = "GoogleDrive"
type = "googleDrive"

[BackendNode.GoogleDrive]
node_uuid = "node-1"
client_id = "client-id"
client_secret = "client-secret"
drive_id = "drive-id"
access_token = "stale-access"
refresh_token = "refresh-1"
"#;
        std::fs::write(&config_path, initial).expect("write config");

        let node = google_node("node-1", "stale-access", "refresh-1", None);
        let state =
            test_state_with_google_node(node.clone(), config_path).await;
        let source = GoogleDriveTokenSource::new(state.clone(), node);

        let first = source
            .token_with_refresh(
                TokenRequest::new("backoff_first", Duration::seconds(60)),
                |_| async {
                    Err(TokenSourceError::Refresh {
                        node: "GoogleDrive".to_string(),
                        error: "boom".to_string(),
                    })
                },
            )
            .await
            .expect_err("first refresh should fail");
        assert!(matches!(first, TokenSourceError::Refresh { .. }));

        let second = source
            .token_with_refresh(
                TokenRequest::new("backoff_second", Duration::seconds(60)),
                |_| async {
                    panic!("backoff should block second refresh attempt")
                },
            )
            .await
            .expect_err("second refresh should back off");
        assert!(matches!(second, TokenSourceError::RefreshBackoff { .. }));
    }

    #[test]
    fn token_blob_helper_supports_explicit_timestamp() {
        let expiry = Utc
            .with_ymd_and_hms(2026, 4, 16, 12, 0, 0)
            .single()
            .expect("expiry");
        let node = google_node("node-1", "access", "refresh", Some(expiry));
        let token = node
            .google_drive
            .as_ref()
            .and_then(|config| config.effective_token())
            .expect("effective token");

        assert_eq!(token.expiry, Some(expiry));
    }
}
