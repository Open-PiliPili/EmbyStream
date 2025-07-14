use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct DirectLink {
    pub user_agent: String,
}
