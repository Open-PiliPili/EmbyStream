use std::collections::HashMap;

use crate::{
    api::emby::Operation,
    network::{HttpMethod, NetworkTarget, NetworkTask},
    system::SystemInfo,
    util::StringUtil,
};

/// Represents Emby API endpoints with their respective parameters.
#[derive(Debug, Clone)]
pub struct API {
    /// The base URL for the Emby API.
    base_url: String,
    /// The API key for authenticating with the Emby API.
    api_key: String,
    /// The specific API operation (GetUser or PlaybackInfo).
    operation: Operation,
}

impl API {
    /// Constructs an EmbyAPI instance for the GetUser endpoint.
    pub fn get_user(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        user_id: impl Into<String>,
    ) -> Self {
        API {
            base_url: base_url.into(),
            api_key: api_key.into(),
            operation: Operation::GetUser {
                user_id: user_id.into(),
            },
        }
    }

    /// Constructs an EmbyAPI instance for the PlaybackInfo endpoint.
    pub fn playback_info(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        item_id: impl Into<String>,
        media_source_id: impl Into<String>,
    ) -> Self {
        API {
            base_url: base_url.into(),
            api_key: api_key.into(),
            operation: Operation::PlaybackInfo {
                item_id: item_id.into(),
                media_source_id: media_source_id.into(),
            },
        }
    }
}

impl NetworkTarget for API {
    fn base_url(&self) -> String {
        self.base_url.clone()
    }

    fn path(&self) -> String {
        match &self.operation {
            Operation::GetUser { user_id } => format!("emby/Users/{user_id}"),
            Operation::PlaybackInfo { item_id, .. } => {
                format!("emby/Items/{item_id}/PlaybackInfo")
            }
        }
    }

    fn method(&self) -> HttpMethod {
        HttpMethod::Get
    }

    fn task(&self) -> NetworkTask {
        let mut params = HashMap::new();
        params.insert("api_key".to_string(), self.api_key.clone());
        match &self.operation {
            Operation::GetUser { .. } => NetworkTask::RequestParameters(params),
            Operation::PlaybackInfo {
                media_source_id, ..
            } => {
                if !media_source_id.is_empty() {
                    params.insert(
                        "MediaSourceId".to_string(),
                        media_source_id.clone(),
                    );
                }
                NetworkTask::RequestParameters(params)
            }
        }
    }

    fn headers(&self) -> Vec<(String, String)> {
        let sys_info = SystemInfo::new();
        let base_url =
            StringUtil::trim_trailing_slashes(&self.base_url).to_string();
        vec![
            ("accept".into(), "application/json".into()),
            ("origin".into(), base_url.clone()),
            ("referer".into(), format!("{base_url}/")),
            ("user-agent".into(), sys_info.get_user_agent()),
        ]
    }
}
