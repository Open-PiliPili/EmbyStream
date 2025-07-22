//! Provides a curl-based logging plugin for network requests.
//!
//! This module implements a plugin that logs network requests in curl command format,
//! making it easy to reproduce requests for debugging or testing purposes.

use reqwest::{Error, Request, Response};

use crate::{
    NETWORK_LOGGER_DOMAIN, debug_log, error_log, network::plugin::NetworkPlugin,
};

/// A plugin that logs network requests in curl command format.
///
/// This plugin implements the `Plugin` trait and provides detailed logging of:
/// - Request details in curl command format
/// - Response status codes
/// - Error messages
pub struct CurlPlugin;

impl CurlPlugin {
    /// Logs the request details in curl command format.
    fn on_request_impl(&self, request: &Request) {
        let curl_command = CurlPlugin::request_to_curl(request);
        debug_log!(NETWORK_LOGGER_DOMAIN, "Sending request: {}", curl_command);
    }

    /// Logs the response status code.
    fn on_response_impl(&self, response: &Response) {
        debug_log!(
            NETWORK_LOGGER_DOMAIN,
            "Received response: {}",
            response.status()
        );
    }

    /// Logs any errors that occur during the request.
    fn on_error_impl(&self, error: &Error) {
        error_log!(NETWORK_LOGGER_DOMAIN, "Request occurred Error: {}", error);
    }

    /// Converts a request into a curl command string.
    ///
    /// This method generates a curl command that can be used to reproduce the request,
    /// including:
    /// - HTTP method
    /// - URL
    /// - Headers
    /// - Request body (if present)
    fn request_to_curl(request: &Request) -> String {
        let mut curl_command = String::new();
        curl_command.push_str("curl -X ");
        curl_command.push_str(request.method().as_str());
        curl_command.push_str(&format!(" '{}' ", request.url()));

        for (name, value) in request.headers() {
            if let Ok(valid_str) = value.to_str() {
                let escaped_value =
                    valid_str.replace('"', "\\\"").replace("'", "\\'");
                curl_command
                    .push_str(&format!("-H \"{name}: {escaped_value}\" "));
            }
        }

        if let Some(body) = request.body() {
            let body_str = if let Some(text) = body
                .as_bytes()
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
            {
                text.replace('\'', "\\'").replace('"', "\\\"")
            } else if let Some(chunk) = body.as_bytes() {
                format!(
                    "Binary Data ({:?})",
                    chunk
                        .iter()
                        .take(50)
                        .map(|&b| format!("{b:02X}"))
                        .collect::<Vec<_>>()
                        .join(" ")
                )
            } else {
                String::from("Unknown Content")
            };

            if !body_str.is_empty() {
                curl_command.push_str(&format!(" -d '{body_str}'"));
            }
        }

        curl_command
    }
}

impl NetworkPlugin for CurlPlugin {
    /// Logs the request details before sending.
    fn on_request(&self, request: &Request) {
        self.on_request_impl(request);
    }

    /// Logs the response details after receiving.
    fn on_response(&self, response: &Response) {
        self.on_response_impl(response);
    }

    /// Logs any errors that occur.
    fn on_error(&self, error: &Error) {
        self.on_error_impl(error);
    }
}
