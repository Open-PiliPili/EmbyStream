use std::{collections::HashMap, str, time::Instant};

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

    pub fn get_scheme_and_host(&self) -> Option<String> {
        let scheme = self.uri.scheme_str()?;
        let host = self.uri.host()?.trim();

        if host.is_empty() {
            return None;
        }

        match self.uri.port_u16() {
            Some(port) => {
                let skip_port = matches!((scheme, port), ("http", 80) | ("https", 443));
                Some(if skip_port {
                    format!("{scheme}://{host}")
                } else {
                    format!("{scheme}://{host}:{port}")
                })
            }
            None => Some(format!("{scheme}://{host}")),
        }
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
