use std::time::Instant;

use hyper::{HeaderMap, Uri};

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
}
