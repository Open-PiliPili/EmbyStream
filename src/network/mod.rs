//! A flexible and extensible network request handling system.
//!
//! This module provides a plugin-based architecture for making HTTP requests with the following features:
//! - Support for different HTTP methods
//! - Plugin system for request/response processing
//! - Curl-based implementation
//! - Task-based request handling
//!
pub mod curl_plugin;
pub mod extension;
pub mod http_method;
pub mod plugin;
pub mod provider;
pub mod target;
pub mod task;

pub use curl_plugin::*;
pub use extension::*;
pub use http_method::*;
pub use plugin::*;
pub use provider::*;
pub use target::*;
pub use task::*;
