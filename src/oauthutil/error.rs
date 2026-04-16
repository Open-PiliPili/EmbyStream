use thiserror::Error;

#[derive(Debug, Error)]
pub enum TokenSourceError {
    #[error("googleDrive token config is missing for node '{node}'")]
    MissingGoogleDriveConfig { node: String },
    #[error("googleDrive node_uuid is missing for node '{node}'")]
    MissingNodeUuid { node: String },
    #[error("googleDrive refresh_token is empty for node '{node}'")]
    MissingRefreshToken { node: String },
    #[error("googleDrive access token is unavailable for node '{node}'")]
    MissingAccessToken { node: String },
    #[error("googleDrive token store read failed for node '{node}': {error}")]
    StoreRead { node: String, error: String },
    #[error("googleDrive token store write failed for node '{node}': {error}")]
    StoreWrite { node: String, error: String },
    #[error("googleDrive token refresh failed for node '{node}': {error}")]
    Refresh { node: String, error: String },
}
