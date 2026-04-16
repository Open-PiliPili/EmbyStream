use std::path::Path;

use crate::{
    config::core::{persist_google_drive_token, read_google_drive_token},
    oauthutil::{OAuthToken, TokenSourceError},
};

pub(crate) struct GoogleDriveTokenStore;

impl GoogleDriveTokenStore {
    pub(crate) fn read(
        config_path: &Path,
        node_name: &str,
        node_uuid: &str,
    ) -> Result<Option<OAuthToken>, TokenSourceError> {
        read_google_drive_token(config_path, node_uuid).map_err(|error| {
            TokenSourceError::StoreRead {
                node: node_name.to_string(),
                error: error.to_string(),
            }
        })
    }

    pub(crate) fn write(
        config_path: &Path,
        node_name: &str,
        node_uuid: &str,
        token: &OAuthToken,
    ) -> Result<(), TokenSourceError> {
        persist_google_drive_token(config_path, node_uuid, token).map_err(
            |error| TokenSourceError::StoreWrite {
                node: node_name.to_string(),
                error: error.to_string(),
            },
        )
    }
}
