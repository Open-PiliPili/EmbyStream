use std::sync::Arc;

use chrono::{Duration, Utc};
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

        info_log!(
            LOGGER_DOMAIN,
            "google_drive_refresh_start node={} node_uuid={} reason={} \
             force_refresh={}",
            node_name,
            node_uuid,
            request.reason,
            request.force_refresh
        );

        let refreshed = self.refresh_from_google(&refresh_token).await?;
        self.update_runtime_token(&node_uuid, refreshed.clone())
            .await;
        GoogleDriveTokenStore::write(
            &config_path,
            &node_name,
            &node_uuid,
            &refreshed,
        )?;

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
        now: chrono::DateTime<Utc>,
    ) -> Option<OAuthToken> {
        self.state
            .google_drive_token_cache
            .get(&cache_key(node_uuid))
            .map(|entry| entry.value().clone())
            .filter(|token| token.is_valid_for(min_valid_for, now))
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
    use chrono::{Duration, Utc};

    use super::token_from_refresh_response;
    use crate::client::google_drive::GoogleTokenRefreshResponse;

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
}
