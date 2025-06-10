//! Defines the plugin interface for network request/response processing.
//!
//! This module provides a trait that allows for custom processing of network requests,
//! responses, and errors through a plugin system.

use reqwest::{Error, Request, Response};

/// Defines the interface for network request/response plugins.
///
/// This trait provides methods that are called at different stages of a network request:
/// - Before the request is sent
/// - After a response is received
/// - When an error occurs
pub trait NetworkPlugin {
    /// Called before a request is sent.
    ///
    /// This method allows plugins to inspect or modify the request before it is sent.
    fn on_request(&self, request: &Request);

    /// Called after a response is received.
    ///
    /// This method allows plugins to inspect or process the response.
    fn on_response(&self, response: &Response);

    /// Called when an error occurs during the request.
    ///
    /// This method allows plugins to handle or log errors.
    fn on_error(&self, error: &Error);
}
