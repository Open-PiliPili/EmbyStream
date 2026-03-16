use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DirectLink {
    #[serde(default)]
    pub user_agent: String,
}
