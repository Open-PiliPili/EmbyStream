use std::time::Instant;

use hyper::{HeaderMap, header};

#[allow(dead_code)]
pub struct Request {
    pub sign: String,
    pub original_headers: HeaderMap,
    pub request_start_time: Instant,
}

impl Request {
    #[allow(unused_variables)]
    pub fn new(sign: String, original_headers: HeaderMap, request_start_time: Instant) -> Self {
        Self {
            sign,
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
