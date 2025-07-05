use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub enum ProxyMode {
    #[serde(rename = "proxy")]
    Proxy,
    #[serde(rename = "redirect")]
    Redirect,
}

impl Default for ProxyMode {
    fn default() -> Self {
        ProxyMode::Proxy
    }
}