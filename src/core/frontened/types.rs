#[derive(Clone, Debug)]
pub struct ForwardInfo {
    pub item_id: String,
    pub media_source_id: String,
    pub path: String
}

#[derive(Clone, Debug)]
pub struct PathParams {
    pub item_id: String,
    pub media_source_id: String,
}

#[derive(Clone, Debug)]
pub struct ForwardConfig {
    pub expired_seconds: u64,
    pub backend_base_url: String,
    pub backend_forward_path: String,
    pub proxy_mode: String,
    pub crypto_key: String,
    pub crypto_iv: String,
    pub emby_server_url: String,
    pub emby_api_key: String,
}