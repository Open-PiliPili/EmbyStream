use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Frontend {
    pub listen_port: u16,
}