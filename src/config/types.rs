use serde::Deserialize;

use crate::config::{
    backend::{direct::DirectLink, disk::Disk, openlist::OpenList, Backend},
    frontend::Frontend,
    general::{General, UserAgent},
};

#[derive(Deserialize)]
pub struct RawConfig {
    #[serde(rename = "General")]
    pub general: General,
    #[serde(rename = "UserAgent")]
    pub user_agent: UserAgent,
    #[serde(rename = "Frontend")]
    pub frontend: Frontend,
    #[serde(rename = "Backend")]
    pub backend: Backend,
    #[serde(rename = "Disk")]
    pub disk: Option<Disk>,
    #[serde(rename = "OpenList")]
    pub open_list: Option<OpenList>,
    #[serde(rename = "DirectLink")]
    pub direct_link: Option<DirectLink>,
}
