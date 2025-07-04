use std::convert::Infallible;

use bytes::Bytes;
use http_body_util::{BodyExt, Full, combinators::BoxBody};
use hyper::{Response, StatusCode, header};

use super::error::Error;

pub type BoxBodyType = BoxBody<Bytes, Error>;

pub struct ResponseBuilder;

impl ResponseBuilder {
    pub fn from_error(err: &Error) -> Response<BoxBodyType> {
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .body(Self::from_bytes(err.to_string()))
            .unwrap()
    }

    pub fn with_redirect(
        location: impl AsRef<str>,
        mut status: StatusCode
    ) -> Response<BoxBodyType> {
        if !status.is_redirection() {
            status = StatusCode::FOUND
        }
        let mut response = Response::new(Self::empty());

        *response.status_mut() = status;
        response.headers_mut().insert(
            header::LOCATION,
            location.as_ref().parse().expect("Invalid location URI"),
        );

        response
    }

    pub fn with_status_code(status_code: StatusCode) -> Response<BoxBodyType> {
        Response::builder()
            .status(status_code)
            .body(Self::empty())
            .expect("Failed to build empty status code response")
    }

    pub fn with_text(status: StatusCode, body: impl Into<Bytes>) -> Response<BoxBodyType> {
        Response::builder()
            .status(status)
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .body(Self::from_bytes(body))
            .expect("Failed to build text response")
    }

    pub fn with_json(status: StatusCode, json: impl Into<Bytes>) -> Response<BoxBodyType> {
        Response::builder()
            .status(status)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Self::from_bytes(json))
            .expect("Failed to build JSON response")
    }

    fn empty() -> BoxBodyType {
        http_body_util::Empty::new()
            .map_err(|never| match never {})
            .boxed()
    }

    fn from_bytes(bytes: impl Into<Bytes>) -> BoxBodyType {
        Full::new(bytes.into())
            .map_err(|never: Infallible| match never {})
            .boxed()
    }
}
