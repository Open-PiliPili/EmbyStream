use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct AlistResponse {
    data: AlistFileData,
}

impl AlistResponse {
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