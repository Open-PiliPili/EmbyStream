use std::fmt;

use serde::Deserialize;

use crate::config::backend::{
    openlist::Config as OpenListConfig, direct::Config as DirectLinkConfig, disk::Config as DiskConfig,
};

/// Unified backend configuration.
#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "type", content = "config")]
pub enum BackendConfig {
    Disk(DiskConfig),
    OpenList(OpenListConfig),
    DirectLink(DirectLinkConfig),
}

impl fmt::Display for BackendConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendConfig::Disk(config) => write!(f, "Disk({})", config),
            BackendConfig::OpenList(config) => write!(f, "OpenList({})", config),
            BackendConfig::DirectLink(config) => write!(f, "DirectLink({})", config),
        }
    }
}
