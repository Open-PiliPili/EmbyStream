use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Disk {
    #[serde(default)]
    pub description: String,
}
