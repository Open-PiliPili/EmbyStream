use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Disk {
    #[serde(default)]
    pub description: String,
}
