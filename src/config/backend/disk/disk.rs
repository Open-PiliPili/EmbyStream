use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Disk {
    pub description: String,
}