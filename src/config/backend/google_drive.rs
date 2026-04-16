use crate::oauthutil::OAuthToken;
use serde::{Deserialize, Serialize};

/// Sub-table `[BackendNode.GoogleDrive]`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct GoogleDriveConfig {
    /// Stable node identifier used for token/cache keys.
    #[serde(default)]
    pub node_uuid: String,
    /// OAuth client ID used when refreshing access tokens.
    #[serde(default)]
    pub client_id: String,
    /// OAuth client secret used when refreshing access tokens.
    #[serde(default)]
    pub client_secret: String,
    /// Preferred shared drive ID; takes precedence over `drive_name`.
    #[serde(default)]
    pub drive_id: String,
    /// Shared drive name fallback when `drive_id` is absent.
    #[serde(default)]
    pub drive_name: String,
    /// Persisted OAuth access token; may be refreshed at runtime.
    #[serde(default)]
    pub access_token: String,
    /// OAuth refresh token used to renew `access_token`.
    #[serde(default)]
    pub refresh_token: String,
    /// Preferred persisted OAuth token blob.
    #[serde(default)]
    pub token: Option<OAuthToken>,
}

impl GoogleDriveConfig {
    pub fn effective_token(&self) -> Option<OAuthToken> {
        if let Some(token) = self.token.as_ref().filter(|token| {
            token.has_access_token() || token.has_refresh_token()
        }) {
            return Some(token.clone());
        }

        if self.access_token.trim().is_empty()
            && self.refresh_token.trim().is_empty()
        {
            return None;
        }

        Some(OAuthToken {
            access_token: self.access_token.trim().to_string(),
            refresh_token: self.refresh_token.trim().to_string(),
            token_type: "Bearer".to_string(),
            expiry: None,
        })
    }

    pub fn effective_refresh_token(&self) -> Option<String> {
        self.effective_token()
            .map(|token| token.refresh_token.trim().to_string())
            .filter(|token| !token.is_empty())
    }

    pub fn apply_token(&mut self, token: OAuthToken) {
        self.access_token = token.access_token.clone();
        self.refresh_token = token.refresh_token.clone();
        self.token = Some(token);
    }
}
