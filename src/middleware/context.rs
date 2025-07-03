use std::str;

use hyper::{HeaderMap, Method, Uri, body::Incoming};

pub struct Context {
    pub uri: String,
    pub path: String,
    pub method: Method,
    pub headers: HeaderMap,
    pub body: Option<Incoming>,
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
        }
    }

    pub fn get_header(&self, key: &str) -> Option<&str> {
        self.headers.get(key).and_then(|v| v.to_str().ok())
    }

    pub fn take_body(&mut self) -> Option<Incoming> {
        self.body.take()
    }
}
