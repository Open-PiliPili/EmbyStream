use serde::{Deserialize, Serialize};

/// Sub-table `[BackendNode.GoogleDrive]`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct GoogleDriveConfig {
    /// Stable node identifier used for token/cache keys.
    #[serde(default)]
    pub node_uuid: String,
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
}
