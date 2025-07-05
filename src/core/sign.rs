use std::{
    collections::HashMap,
    time::Instant,
};

use serde::Deserialize;

use crate::crypto::{
    Crypto,
    CryptoInput,
    CryptoOutput,
};
use crate::{STREAM_LOGGER_DOMAIN, info_log};
use crate::backend::proxy_mode::ProxyMode;

#[derive(Debug, Deserialize)]
pub struct SignParams {
    #[serde(default)]
    pub(crate) sign: String,

    #[serde(default)]
    pub(crate) proxy_mode: ProxyMode,
}

impl Default for SignParams {
    fn default() -> Self {
        Self {
            sign: "".to_string(),
            proxy_mode: ProxyMode::default(),
        }
    }
}

pub struct Sign {
    item_id: Option<String>,
    media_source_id: Option<String>,
    expired_at: Option<u64>,
}

impl Default for Sign {
    fn default() -> Self {
        Self::new(None, None, None)
    }
}

impl Sign {
    pub fn new(
        item_id: Option<String>,
        media_source_id: Option<String>,
        expired_at: Option<u64>,
    ) -> Self {
        Self {
            item_id,
            media_source_id,
            expired_at,
        }
    }

    pub fn decrypt_with(string: impl Into<String>) -> Self {
        // TODO: implement this function later
        info_log!(
            STREAM_LOGGER_DOMAIN,
            "Ready decrypt with {}",
            string.into()
        );
        Self::default()
    }

    pub fn encrypt_self() -> String {
        // TODO: implement this function later
        "".to_string()
    }

    pub fn convert_to_dict() -> HashMap<String, String> {
        // TODO: implement this function later
        HashMap::new()
    }
}