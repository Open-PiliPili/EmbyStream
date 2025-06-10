use std::collections::HashMap;

use crate::{
    network::{HttpMethod, NetworkTarget, NetworkTask},
    system::SystemInfo,
};

pub enum EmbyAPI {
    GetUser { user_id: String },
}

impl NetworkTarget for EmbyAPI {
    fn base_url(&self) -> String {
        "".to_string()
    }

    fn path(&self) -> String {
        match self {
            EmbyAPI::GetUser { user_id, .. } => {
                format!("emby/Users/{}", user_id)
            }
        }
    }

    fn method(&self) -> HttpMethod {
        HttpMethod::Get
    }

    fn task(&self) -> NetworkTask {
        match self {
            EmbyAPI::GetUser { user_id: _ } => {
                let api_key = "".to_string();
                let mut params = HashMap::new();
                params.insert("api_key".to_string(), api_key);
                NetworkTask::RequestParameters(params)
            }
        }
    }

    fn headers(&self) -> Option<Vec<(&'static str, String)>> {
        let base_url = "".to_string();
        let sys_info = SystemInfo::new();
        Some(vec![
            ("accept", "application/json".to_string()),
            ("origin", base_url.clone()),
            ("referer", format!("{}/", base_url)),
            ("user-agent", sys_info.get_user_agent()),
        ])
    }
}
