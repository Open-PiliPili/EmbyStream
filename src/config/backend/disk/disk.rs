use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Disk {
    pub listen_port: u16,
    pub base_url: String,
    pub path: String,
    pub port: String,
    pub path_replace_rule_regex: String,
}