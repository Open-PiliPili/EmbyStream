use std::fmt;

use serde::Deserialize;

use crate::{
    config::backend::r#type::BackendType,
    util::privacy::Privacy
};

/// Configuration for the General section of the config file.
#[derive(Deserialize, Clone, Debug)]
pub struct GeneralConfig {
    pub log_level: String,
    pub backend_type: BackendType,
    pub encipher_key: String,
    pub cache_ttl_seconds: u64,
    pub api_key: String,
}

impl fmt::Display for GeneralConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let privacy = Privacy::new();
        write!(
            f,
            "GeneralConfig {{ log_level: {}, backend_type: {}, encipher_key: {}, cache_ttl_seconds: {}, api_key: {} }}",
            self.log_level,
            self.backend_type,
            privacy.desensitize(&self.encipher_key),
            self.cache_ttl_seconds,
            privacy.desensitize(&self.api_key)
        )
    }
}
