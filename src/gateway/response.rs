use bytes::Bytes;
use http_body_util::{BodyExt, Empty, Full, combinators::BoxBody};
use hyper::{
    Response, StatusCode,
    header::{self, HeaderMap},
};

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
            status = StatusCode::MOVED_PERMANENTLY;
        }

        let mut response = Response::new(
            Empty::<Bytes>::new()
                .map_err(|never| match never {})
                .boxed(),
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

    pub fn with_response<T>(
        status_code: StatusCode,
        headers: Option<HeaderMap>,
        body: Option<T>,
    ) -> Response<BoxBodyType>
    where
        T: Into<Bytes>,
    {
        let mut builder = Response::builder().status(status_code);

        if let Some(h) = headers {
            if let Some(headers_mut) = builder.headers_mut() {
                headers_mut.extend(h);
            }
        }

        let body = match body {
            Some(b) => {
                Full::new(b.into()).map_err(|never| match never {}).boxed()
            }
            None => Self::empty(),
        };

        builder.body(body).expect("Failed to build response")
    }

    pub fn with_status_code(status_code: StatusCode) -> Response<BoxBodyType> {
        Response::builder()
            .status(status_code)
            .body(Self::empty())
            .expect("Failed to build empty status code response")
    }

    fn empty() -> BoxBodyType {
        Empty::new().map_err(|never| match never {}).boxed()
    }
}
