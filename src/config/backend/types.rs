use serde::Deserialize;

use super::{disk::disk::Disk, openlist::openlist::OpenList, direct::direct::DirectLink};

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "backend_type", content = "settings")]
pub enum BackendConfig {
    Disk(Disk),
    OpenList(OpenList),
    DirectLink(DirectLink),
}