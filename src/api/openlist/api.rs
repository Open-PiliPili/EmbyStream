use crate::{
    api::openlist::Operation,
    network::{HttpMethod, NetworkTarget, NetworkTask},
};

/// Represents OpenList API endpoints with their respective parameters.
#[derive(Debug, Clone)]
pub struct API {
    /// The base URL for the OpenList server (e.g., "http://127.0.0.1:5244").
    url: String,
    /// The token for authenticating with the OpenList server.
    token: String,
    /// The user-agent for request alist server
    user_agent: String,
    /// The specific API operation (e.g., FsGet).
    operation: Operation,
}

impl API {
    /// Constructs a new `OpenListAPI` instance for fetching file information.
    pub fn fs_get(
        url: impl Into<String>,
        token: impl Into<String>,
        path: impl Into<String>,
        user_agent: impl Into<String>,
    ) -> Self {
        API {
            url: url.into(),
            token: token.into(),
            user_agent: user_agent.into(),
            operation: Operation::FsGet { path: path.into() },
        }
    }

    pub fn fs_link(
        url: impl Into<String>,
        token: impl Into<String>,
        path: impl Into<String>,
        user_agent: impl Into<String>,
    ) -> Self {
        API {
            url: url.into(),
            token: token.into(),
            user_agent: user_agent.into(),
            operation: Operation::FsLink { path: path.into() },
        }
    }
}

impl NetworkTarget for API {
    /// Gets the base URL for OpenList API requests.
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

    /// Gets the HTTP method for the request (always POST for OpenList fs/get).
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

    /// Gets the default headers for OpenList API requests.
    ///
    /// Includes:
    /// - Standard JSON content type headers
    /// - Authentication token
    /// - User agent string
    fn headers(&self) -> Vec<(String, String)> {
        vec![
            ("accept".into(), "application/json, text/plain, */*".into()),
            ("authorization".into(), self.token.clone()),
            ("cache-control".into(), "no-cache".into()),
            (
                "content-type".into(),
                "application/json;charset=UTF-8".into(),
            ),
            ("user-agent".into(), self.user_agent.clone()),
        ]
    }
}
