use std::time::Instant;

use hyper::{HeaderMap, Uri, header};

use crate::config::backend::BackendNode;
use crate::core::sign::Sign;

pub struct Request {
    pub uri: Uri,
    pub original_headers: HeaderMap,
    pub request_start_time: Instant,
    pub node: Option<BackendNode>,
    pub sign: Option<Sign>,
}

impl Request {
    pub fn new(
        uri: Uri,
        original_headers: HeaderMap,
        request_start_time: Instant,
        node: Option<BackendNode>,
    ) -> Self {
        Self {
            uri,
            original_headers,
            request_start_time,
            node,
            sign: None,
        }
    }

    pub(crate) fn content_range(&self) -> Option<String> {
        self.original_headers
            .get(header::RANGE)
            .and_then(|v| v.to_str().ok())
            .map(String::from)
    }

    pub(crate) fn client(&self) -> Option<String> {
        self.original_headers
            .get("client")
            .or_else(|| self.original_headers.get(header::USER_AGENT))
            .and_then(|v| v.to_str().ok())
            .map(String::from)
    }

    pub(crate) fn user_agent(&self) -> Option<String> {
        self.original_headers
            .get(header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .map(String::from)
    }

    pub(crate) fn client_ip(&self) -> Option<String> {
        self.original_headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| {
                s.split(',').next().map(str::trim).map(str::to_string)
            })
            .or_else(|| {
                self.original_headers
                    .get("x-real-ip")
                    .and_then(|v| v.to_str().ok().map(str::to_string))
            })
    }
}
