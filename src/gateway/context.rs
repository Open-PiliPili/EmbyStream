use std::str;

use hyper::{HeaderMap, Method, Uri, body::Incoming};
use std::time::Instant;

pub struct Context {
    pub uri: String,
    pub path: String,
    pub method: Method,
    pub headers: HeaderMap,
    pub body: Option<Incoming>,
    pub start_time: Instant,
}

impl Context {
    pub fn new(uri: Uri, method: Method, headers: HeaderMap, body: Incoming) -> Self {
        let path = uri.path().to_string();
        let uri_str = uri.to_string();
        Self {
            uri: uri_str,
            path,
            method,
            headers,
            body: Some(body),
            start_time: Instant::now(),
        }
    }

    pub fn get_header(&self, key: &str) -> Option<&str> {
        self.headers.get(key).and_then(|v| v.to_str().ok())
    }

    pub fn take_body(&mut self) -> Option<Incoming> {
        self.body.take()
    }
}
