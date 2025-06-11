use std::{collections::HashMap, fmt};

use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct AlistFileResponse {
    data: AlistFileData,
}

impl AlistFileResponse {
    pub fn get_data(&self) -> AlistFileData {
        self.data.clone()
    }
}

#[derive(Deserialize, Clone)]
pub struct AlistFileData {
    raw_url: Option<String>,
}

impl AlistFileData {
    pub fn get_raw_url(&self) -> Option<String> {
        self.raw_url.clone()
    }
}

#[derive(Deserialize)]
pub struct AlistLinkResponse {
    data: AlistLinkData,
}

impl AlistLinkResponse {
    pub fn get_data(&self) -> AlistLinkData {
        self.data.clone()
    }
}

#[derive(Deserialize, Clone, Default)]
pub struct AlistLinkData {
    url: String,
    #[serde(default)]
    header: Option<HashMap<String, Vec<String>>>,
}

impl AlistLinkData {
    pub fn get_url(&self) -> String {
        self.url.clone()
    }

    pub fn get_header(&self) -> Option<HashMap<String, Vec<String>>> {
        self.header.clone()
    }
}

impl fmt::Display for AlistLinkData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AlistLinkData {{ url: {}, header: {:?} }}",
            self.url, self.header
        )
    }
}
