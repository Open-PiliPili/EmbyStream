use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Default)]
pub struct Http2 {
    #[serde(default)]
    pub ssl_cert_file: String,
    #[serde(default)]
    pub ssl_key_file: String,
}
