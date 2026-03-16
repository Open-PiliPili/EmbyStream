use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct Http2 {
    #[serde(default)]
    pub ssl_cert_file: String,
    #[serde(default)]
    pub ssl_key_file: String,
}
