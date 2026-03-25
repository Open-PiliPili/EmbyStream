use std::time::Instant;

use hyper::{Method, Uri};

use super::context::Context;

#[test]
fn context_stores_request_id() {
    let uri: Uri = "http://localhost/test".parse().expect("valid uri");
    let ctx = Context::new(
        uri,
        Method::GET,
        hyper::HeaderMap::new(),
        Instant::now(),
        "test-req-123".to_string(),
    );

    assert_eq!(ctx.request_id, "test-req-123");
}

#[test]
fn context_request_id_is_unique() {
    let uri1: Uri = "http://localhost/test1".parse().expect("valid uri");
    let uri2: Uri = "http://localhost/test2".parse().expect("valid uri");

    let ctx1 = Context::new(
        uri1,
        Method::GET,
        hyper::HeaderMap::new(),
        Instant::now(),
        "req-1".to_string(),
    );

    let ctx2 = Context::new(
        uri2,
        Method::GET,
        hyper::HeaderMap::new(),
        Instant::now(),
        "req-2".to_string(),
    );

    assert_ne!(ctx1.request_id, ctx2.request_id);
}

#[test]
fn context_tracks_start_time() {
    let uri: Uri = "http://localhost/test".parse().expect("valid uri");
    let start = Instant::now();
    let ctx = Context::new(
        uri,
        Method::GET,
        hyper::HeaderMap::new(),
        start,
        "test-req".to_string(),
    );

    let elapsed = ctx.start_time.elapsed();
    assert!(elapsed.as_millis() < 100);
}

#[test]
fn context_clones_request_id() {
    let uri: Uri = "http://localhost/test".parse().expect("valid uri");
    let request_id = "test-req-456".to_string();
    let ctx = Context::new(
        uri,
        Method::GET,
        hyper::HeaderMap::new(),
        Instant::now(),
        request_id.clone(),
    );

    assert_eq!(ctx.request_id, request_id);
    assert_eq!(ctx.request_id.len(), request_id.len());
}
