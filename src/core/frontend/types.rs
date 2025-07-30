#[derive(Clone, Debug)]
pub struct ForwardInfo {
    pub item_id: String,
    pub media_source_id: String,
    pub path: String,
    pub device_id: String,
}

#[derive(Clone, Debug)]
pub struct PathParams {
    pub item_id: String,
    pub media_source_id: String,
}

#[derive(Clone, Debug)]
pub struct ForwardConfig {
    pub expired_seconds: u64,
    pub backend_url: String,
    pub proxy_mode: String,
    pub crypto_key: String,
    pub crypto_iv: String,
    pub emby_server_url: String,
    pub emby_api_key: String,
    pub fallback_video_path: Option<String>,
}
