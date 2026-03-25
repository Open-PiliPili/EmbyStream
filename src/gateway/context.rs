use std::{collections::HashMap, str, time::Instant};

use hyper::{HeaderMap, Method, Uri, header};

pub struct Context {
    pub uri: Uri,
    pub path: String,
    pub method: Method,
    pub headers: HeaderMap,
    pub start_time: Instant,
    pub request_id: String,
}

impl Context {
    pub fn new(
        uri: Uri,
        method: Method,
        headers: HeaderMap,
        start_time: Instant,
        request_id: String,
    ) -> Self {
        let path = uri.path().to_string();
        Self {
            uri,
            path,
            method,
            headers,
            start_time,
            request_id,
        }
    }

    pub fn get_host(&self) -> Option<String> {
        self.headers
            .get(header::HOST)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string)
    }

    pub fn get_query_params(&self) -> Option<HashMap<String, String>> {
        self.uri.query().map(|query_str| {
            form_urlencoded::parse(query_str.as_bytes())
                .into_owned()
                .collect()
        })
    }

    pub fn get_header(&self, key: &str) -> Option<&str> {
        self.headers.get(key).and_then(|v| v.to_str().ok())
    }
}
