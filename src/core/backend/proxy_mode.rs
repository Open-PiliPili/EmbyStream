use serde::Deserialize;
use std::str::FromStr;

#[derive(Clone, Debug, Default, Copy, PartialEq, Deserialize)]
pub enum ProxyMode {
    #[serde(rename = "proxy")]
    #[default]
    Proxy,
    #[serde(rename = "redirect")]
    Redirect,
}

impl FromStr for ProxyMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "proxy" => Ok(ProxyMode::Proxy),
            "redirect" => Ok(ProxyMode::Redirect),
            _ => Err(()),
        }
    }
}
