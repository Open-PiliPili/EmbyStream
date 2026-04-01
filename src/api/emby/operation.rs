use crate::network::HttpMethod;

/// Enum representing specific Emby API operations.
#[derive(Debug, Clone)]
pub enum Operation {
    GetUser {
        user_id: String,
    },
    PlaybackInfo {
        item_id: String,
        media_source_id: String,
        method: HttpMethod,
        body: Option<Vec<u8>>,
        content_type: Option<String>,
    },
}
