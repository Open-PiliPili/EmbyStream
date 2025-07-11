use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct DirectLink {
    pub base_url: String,
    pub port: String,
    pub user_agent: String,
    pub path_replace_rule_regex: String,
}
