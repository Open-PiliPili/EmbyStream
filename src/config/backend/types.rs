use serde::Deserialize;

use super::{
    direct::direct::DirectLink, disk::disk::Disk, openlist::openlist::OpenList,
};

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "backend_type", content = "settings")]
pub enum BackendConfig {
    Disk(Disk),
    OpenList(OpenList),
    DirectLink(DirectLink),
}
