use hyper::Uri;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct General {
    pub log_level: String,
    pub mermory_mode: String,
    pub expired_seconds: u64,
    pub backend_type: String,
    pub encipher_key: String,
    pub encipher_iv: String,
    pub emby_url: String,
    pub emby_port: String,
    pub emby_api_key: String,
}

impl General {

    pub fn emby_uri(&self) -> Uri {
        let scheme = self.get_port_scheme();
        let should_show_port = !(self.emby_port == "443" || self.emby_port == "80");
        let clean_url = self.emby_url.trim_start_matches("//");

        let uri_str = if should_show_port {
            format!("{}://{}:{}", scheme, clean_url, self.emby_port)
        } else {
            format!("{}://{}", scheme, clean_url)
        };

        uri_str.parse().expect("Failed to parse backend URI")
    }

    fn get_port_scheme(&self) -> &str {
        if self.emby_port == "443" { "https" } else { "http" }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct UserAgent {
    pub mode: String,
    pub allow_ua: Vec<String>,
    pub deny_ua: Vec<String>,
}

impl UserAgent {
    pub fn is_allow_mode(&self) -> bool {
        self.mode == "allow"
    }
}