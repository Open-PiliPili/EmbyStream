use std::time::Instant;

use hyper::{HeaderMap, Uri, header};

pub struct Request {
    pub uri: Uri,
    pub original_headers: HeaderMap,
    pub request_start_time: Instant,
}

impl Request {
    pub fn new(
        uri: Uri,
        original_headers: HeaderMap,
        request_start_time: Instant,
    ) -> Self {
        Self {
            uri,
            original_headers,
            request_start_time,
        }
    }

    pub(crate) fn content_range(&self) -> Option<String> {
        self.original_headers
            .get(header::RANGE)
            .and_then(|v| v.to_str().ok())
            .map(String::from)
    }
}
