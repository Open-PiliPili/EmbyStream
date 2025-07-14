use hyper::Uri;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct OpenList {
    pub base_url: String,
    pub port: String,
    pub token: String
}

impl OpenList {

    pub fn uri(&self) -> Uri {
        let scheme = self.get_port_scheme();
        let should_show_port = !(self.port == "443" || self.port == "80");
        let clean_url = self.base_url
            .trim_start_matches("//")
            .trim_end_matches('/');

        let uri_str = if should_show_port {
            format!("{}://{}:{}", scheme, clean_url, self.port)
        } else {
            format!("{}://{}", scheme, clean_url,)
        };

        uri_str.parse().expect("Failed to parse backend URI")
    }

    fn get_port_scheme(&self) -> &str {
        if self.port == "443" {
            "https"
        } else {
            "http"
        }
    }
}