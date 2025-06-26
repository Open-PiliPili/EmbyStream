use crate::{
    api::alist::Operation,
    network::{HttpMethod, NetworkTarget, NetworkTask},
    system::SystemInfo
};

/// Represents Alist API endpoints with their respective parameters.
#[derive(Debug, Clone)]
pub struct API {
    /// The base URL for the Alist server (e.g., "http://127.0.0.1:5244").
    url: String,
    /// The token for authenticating with the Alist server.
    token: String,
    /// The specific API operation (e.g., FsGet).
    operation: Operation,
}

impl API {
    /// Constructs a new `AlistAPI` instance for fetching file information.
    pub fn fs_get(
        url: impl Into<String>,
        token: impl Into<String>,
        path: impl Into<String>,
    ) -> Self {
        API {
            url: url.into(),
            token: token.into(),
            operation: Operation::FsGet { path: path.into() },
        }
    }

    pub fn fs_link(
        url: impl Into<String>,
        token: impl Into<String>,
        path: impl Into<String>,
    ) -> Self {
        API {
            url: url.into(),
            token: token.into(),
            operation: Operation::FsLink { path: path.into() },
        }
    }
}

impl NetworkTarget for API {
    /// Gets the base URL for Alist API requests.
    ///
    /// Ensures the URL ends with a trailing slash, if not already present.
    fn base_url(&self) -> String {
        let mut url = self.url.clone();
        if !url.ends_with('/') {
            url.push('/');
        }
        url
    }

    /// Gets the API endpoint path for the specific operation.
    fn path(&self) -> String {
        match &self.operation {
            Operation::FsGet { .. } => "api/fs/get".to_string(),
            Operation::FsLink { .. } => "api/fs/link".to_string(),
        }
    }

    /// Gets the HTTP method for the request (always POST for Alist fs/get).
    fn method(&self) -> HttpMethod {
        HttpMethod::Post
    }

    /// Converts the API operation into a network task ready for execution.
    ///
    /// # Returns
    /// A `NetworkTask` containing the JSON body with path and password.
    fn task(&self) -> NetworkTask {
        match &self.operation {
            Operation::FsGet { path } => {
                let json = serde_json::json!({
                    "path": path,
                    "password": ""
                });
                NetworkTask::RequestJson(json)
            }
            Operation::FsLink { path } => {
                let json = serde_json::json!({
                    "path": path
                });
                NetworkTask::RequestJson(json)
            }
        }
    }

    /// Gets the default headers for Alist API requests.
    ///
    /// Includes:
    /// - Standard JSON content type headers
    /// - Authentication token
    /// - User agent string
    fn headers(&self) -> Option<Vec<(&'static str, String)>> {
        let sys_info = SystemInfo::new();
        Some(vec![
            ("accept", "application/json, text/plain, */*".to_string()),
            ("authorization", self.token.clone()),
            ("cache-control", "no-cache".to_string()),
            ("content-type", "application/json;charset=UTF-8".to_string()),
            ("user-agent", sys_info.get_user_agent()),
        ])
    }
}
