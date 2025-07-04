use std::convert::Infallible;

use bytes::Bytes;
use http_body_util::{BodyExt, Full, combinators::BoxBody};
use hyper::{Error as HyperError, Response, StatusCode};

pub type BoxBodyType = BoxBody<Bytes, HyperError>;

pub struct ResponseBuilder;

impl ResponseBuilder {
    pub fn with_status_code(status_code: StatusCode) -> Response<BoxBodyType> {
        let full: Full<Bytes> = Full::new(Bytes::new());
        let boxed = full.map_err(|never: Infallible| match never {}).boxed();
        let expect_message = format!("Failed to build {:?} response", status_code);
        Response::builder()
            .status(status_code)
            .body(boxed)
            .expect(&expect_message)
    }

    pub fn with_text(status: StatusCode, body: impl Into<Bytes>) -> Response<BoxBodyType> {
        let full = Full::new(body.into());
        let boxed = full.map_err(|never: Infallible| match never {}).boxed();
        let mut response = Response::builder()
            .status(status)
            .body(boxed)
            .expect("Failed to build text response");
        response.headers_mut().insert(
            hyper::header::CONTENT_TYPE,
            "text/plain; charset=utf-8"
                .parse()
                .expect("Failed to parse mime type"),
        );
        response
    }

    pub fn with_json(status: StatusCode, json: impl Into<Bytes>) -> Response<BoxBodyType> {
        let full = Full::new(json.into());
        let boxed = full.map_err(|never: Infallible| match never {}).boxed();
        let mut response = Response::builder()
            .status(status)
            .body(boxed)
            .expect("Failed to build JSON response");
        response.headers_mut().insert(
            hyper::header::CONTENT_TYPE,
            "application/json"
                .parse()
                .expect("Failed to parse mime type"),
        );
        response
    }
}
