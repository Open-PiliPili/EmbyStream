use std::{str, time::Instant};

use hyper::{HeaderMap, Method, Uri};

pub struct Context {
    pub uri: Uri,
    pub path: String,
    pub method: Method,
    pub headers: HeaderMap,
    pub start_time: Instant,
}

impl Context {
    pub fn new(uri: Uri, method: Method, headers: HeaderMap, start_time: Instant) -> Self {
        let path = uri.path().to_string();
        Self {
            uri,
            path,
            method,
            headers,
            start_time,
        }
    }

    pub fn get_header(&self, key: &str) -> Option<&str> {
        self.headers.get(key).and_then(|v| v.to_str().ok())
    }
}
