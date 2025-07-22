use serde::Deserialize;

#[derive(Clone, Debug, Default, Copy, PartialEq, Deserialize)]
pub enum ProxyMode {
    #[serde(rename = "proxy")]
    #[default]
    Proxy,
    #[serde(rename = "redirect")]
    Redirect,
}
