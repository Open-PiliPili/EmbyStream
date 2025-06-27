use std::fmt;

use serde::Deserialize;

use crate::util::privacy::Privacy;

/// Configuration for the Frontend section of the config file.
#[derive(Deserialize, Clone, Debug)]
pub struct FrontendConfig {
    pub server_port: u16,
    pub emby_url: String,
    pub emby_api_key: Option<String>,
    pub storage_base_path: Option<String>,
}

impl fmt::Display for FrontendConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let privacy = Privacy::new();
        let emby_api_key_display = self.emby_api_key
            .as_ref()
            .map_or("None".to_string(), |key| privacy.desensitize(key));
        write!(
            f,
            "FrontendConfig {{ server_port: {}, emby_url: {}, emby_api_key: {}, storage_base_path: {} }}",
            self.server_port,
            self.emby_url,
            emby_api_key_display,
            self.storage_base_path.as_ref().map_or("None", |s| s)
        )
    }
}