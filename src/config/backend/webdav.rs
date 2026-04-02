use serde::{Deserialize, Serialize};

/// Sub-table `[BackendNode.WebDav]` — all fields optional for backward compatibility.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct WebDavConfig {
    /// `path_join` | `query_path` | `url_template`
    #[serde(default)]
    pub url_mode: String,
    /// Stable node identifier used by `proxy_mode = "accel_redirect"`.
    #[serde(default)]
    pub node_uuid: String,
    /// Query parameter name when `url_mode = query_path` (default in builder: `path`).
    #[serde(default)]
    pub query_param: String,
    /// Full URL template containing `{file_path}` when `url_mode = url_template`.
    #[serde(default)]
    pub url_template: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub user_agent: String,
}
