use std::fmt;

use serde::Deserialize;

use crate::config::{
    alist_config::AListConfig,
    direct_link_config::DirectLinkConfig,
    disk_config::DiskConfig,
};

/// Unified backend configuration.
#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "type", content = "config")]
pub enum BackendConfig {
    Disk(DiskConfig),
    AList(AListConfig),
    DirectLink(DirectLinkConfig),
}

impl fmt::Display for BackendConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendConfig::Disk(config) => write!(f, "Disk({})", config),
            BackendConfig::AList(config) => write!(f, "AList({})", config),
            BackendConfig::DirectLink(config) => write!(f, "DirectLink({})", config),
        }
    }
}