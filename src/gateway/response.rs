use bytes::Bytes;
use http_body_util::{BodyExt, combinators::BoxBody, Empty};
use hyper::{Response, StatusCode, header::{self, HeaderMap}};

use super::error::Error;

pub type BoxBodyType = BoxBody<Bytes, Error>;

pub struct ResponseBuilder;

impl ResponseBuilder {

    pub fn with_redirect(
        location: impl AsRef<str>,
        mut status: StatusCode,
        headers: Option<HeaderMap>,
    ) -> Response<BoxBodyType> {
        if !status.is_redirection() {
            status = StatusCode::FOUND;
        }

        let mut response = Response::new(
            Empty::<Bytes>::new()
                .map_err(|never| match never {})
                .boxed()
        );

        *response.status_mut() = status;

        response.headers_mut().insert(
            header::LOCATION,
            location.as_ref().parse().expect("Invalid location URI"),
        );

        if let Some(headers) = headers {
            response.headers_mut().extend(headers);
        }

        response
    }

    pub fn with_status_code(status_code: StatusCode) -> Response<BoxBodyType> {
        Response::builder()
            .status(status_code)
            .body(Self::empty())
            .expect("Failed to build empty status code response")
    }

    fn empty() -> BoxBodyType {
        Empty::new()
            .map_err(|never| match never {})
            .boxed()
    }
}
