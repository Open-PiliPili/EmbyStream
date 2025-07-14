use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct OpenList {
    pub base_url: String,
    pub port: String,
    pub token: String,
    pub user_agent: String,
}