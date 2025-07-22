use std::collections::HashMap;

use crate::{
    network::{HttpMethod, NetworkTarget, NetworkTask},
    system::SystemInfo,
    util::StringUtil,
};

/// Represents Emby API endpoints with their respective parameters.
#[derive(Debug, Clone)]
pub struct API {
    remote_url: String,
    user_agent: Option<String>,
    forward_headers: Option<HashMap<String, String>>,
}

impl API {
    pub fn download(
        remote_url: impl Into<String>,
        user_agent: impl Into<Option<String>>,
        forward_headers: impl Into<Option<HashMap<String, String>>>,
    ) -> Self {
        API {
            remote_url: remote_url.into(),
            user_agent: user_agent.into(),
            forward_headers: forward_headers.into(),
        }
    }
}

impl NetworkTarget for API {
    fn base_url(&self) -> String {
        self.remote_url.clone()
    }

    fn path(&self) -> String {
        "".to_string()
    }

    fn method(&self) -> HttpMethod {
        HttpMethod::Get
    }

    fn task(&self) -> NetworkTask {
        NetworkTask::RequestPlain
    }

    fn headers(&self) -> Vec<(String, String)> {
        let remote_url = StringUtil::trim_trailing_slashes(&self.remote_url);
        let mut headers = vec![
            ("accept".into(), "application/json".to_string()),
            ("origin".into(), remote_url.to_string()),
            ("referer".into(), format!("{remote_url}/")),
        ];

        if let Some(forward_headers) = &self.forward_headers {
            headers.extend(
                forward_headers.iter().map(|(k, v)| (k.into(), v.clone())),
            );
        }

        let sys_info = SystemInfo::new();
        if let Some(ua) = self.user_agent.as_deref().filter(|s| !s.is_empty()) {
            headers.push(("user-agent".into(), ua.to_string()));
        } else {
            headers.push((
                "user-agent".into(),
                sys_info.get_user_agent().to_string(),
            ));
        }

        headers
    }
}
