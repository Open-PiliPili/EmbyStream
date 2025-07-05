use std::{collections::HashMap, fmt};

use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct FileResponse {
    data: FileData,
}

impl FileResponse {
    pub fn get_data(&self) -> FileData {
        self.data.clone()
    }
}

#[derive(Deserialize, Clone)]
pub struct FileData {
    raw_url: Option<String>,
}

impl FileData {
    pub fn get_raw_url(&self) -> Option<String> {
        self.raw_url.clone()
    }
}

#[derive(Deserialize)]
pub struct LinkResponse {
    data: LinkData,
}

impl LinkResponse {
    pub fn get_data(&self) -> LinkData {
        self.data.clone()
    }
}

#[derive(Deserialize, Clone, Default)]
pub struct LinkData {
    url: String,
    #[serde(default)]
    header: Option<HashMap<String, Vec<String>>>,
}

impl LinkData {
    pub fn get_url(&self) -> String {
        self.url.clone()
    }

    pub fn get_header(&self) -> Option<HashMap<String, Vec<String>>> {
        self.header.clone()
    }
}

impl fmt::Display for LinkData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AlistLinkData {{ url: {}, header: {:?} }}",
            self.url, self.header
        )
    }
}
