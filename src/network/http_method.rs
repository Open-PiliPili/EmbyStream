//! Defines the supported HTTP methods for network requests.
//!
//! This module provides an enum representing the standard HTTP methods
//! supported by the network system.

use std::fmt::{self, Display};

/// Represents the HTTP method to be used in a network request.
#[derive(Debug, Clone, Copy)]
pub enum HttpMethod {
    /// HTTP GET method
    Get,

    /// HTTP POST method
    Post,

    /// HTTP PUT method
    Put,

    /// HTTP DELETE method
    Delete,
}

impl Display for HttpMethod {
    /// Formats the HTTP method as a string.
    ///
    /// Returns the HTTP method name in uppercase, as per HTTP specification.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
        };
        write!(f, "{}", str)
    }
}
