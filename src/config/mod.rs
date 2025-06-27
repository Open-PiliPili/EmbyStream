pub mod backend;
pub mod config;
pub mod frontened;
pub mod general;

pub use backend::{
    BackendConfig, BackendType, alist::Config as AlistConfig, direct::Config as DirectLinkConfig,
    disk::Config as DiskConfig,
};
pub use frontened::FrontendConfig;
pub use general::GeneralConfig;
