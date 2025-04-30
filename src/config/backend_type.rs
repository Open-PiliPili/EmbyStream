use std::fmt;

use serde::Deserialize;

/// Represents the backend type for media streaming.
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub enum BackendType {
    #[serde(rename = "disk")]
    Disk,
    #[serde(rename = "direct_link")]
    DirectLink,
    #[serde(rename = "alist")]
    AList,
}

impl fmt::Display for BackendType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendType::Disk => write!(f, "disk"),
            BackendType::DirectLink => write!(f, "direct_link"),
            BackendType::AList => write!(f, "alist"),
        }
    }
}