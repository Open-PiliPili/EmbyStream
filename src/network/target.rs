//! Defines the target interface for network requests.
//!
//! This module provides a trait that defines the structure of a network request target,
//! including the base URL, path, HTTP method, and request task.

use super::{http_method::HttpMethod, task::NetworkTask};

/// Defines the interface for a network request target.
///
/// This trait provides methods to access the components of a network request:
/// - Base URL of the API
/// - Request path
/// - HTTP method
/// - Request task (body/parameters)
/// - Optional headers
pub trait NetworkTarget: Send + Sync {
    /// Returns the base URL of the API.
    fn base_url(&self) -> String;

    /// Returns the request path.
    fn path(&self) -> String;

    /// Returns the HTTP method to use.
    fn method(&self) -> HttpMethod;

    /// Returns the request task (body/parameters).
    fn task(&self) -> NetworkTask;

    /// Returns optional request headers.
    ///
    /// By default, returns `None`. Implementors can override this method
    /// to provide custom headers.
    fn headers(&self) -> Vec<(String, String)>;
}
