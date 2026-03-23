//! HTTP helpers: plugin hooks around a shared **reqwest** client (`provider`).
//!
//! Request execution uses rustls-backed reqwest; plugins are optional observers
//! (e.g. logging) and are not required for transport.

pub mod extension;
pub mod http_method;
pub mod plugin;
pub mod provider;
pub mod target;
pub mod task;

pub use extension::*;
pub use http_method::*;
pub use plugin::*;
pub use provider::*;
pub use target::*;
pub use task::*;
