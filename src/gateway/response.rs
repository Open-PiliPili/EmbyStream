use bytes::Bytes;
use http_body_util::{BodyExt, Empty, Full, combinators::BoxBody};
use hyper::{
    Response, StatusCode,
    header::{self, HeaderMap, HeaderName, HeaderValue},
};

use super::error::Error;

pub type BoxBodyType = BoxBody<Bytes, Error>;

pub struct ResponseBuilder;

const DEFAULT_REDIRECT_STATUS: StatusCode = StatusCode::MOVED_PERMANENTLY;
const FALLBACK_STATUS_CODE: StatusCode = StatusCode::INTERNAL_SERVER_ERROR;

impl ResponseBuilder {
    pub fn with_redirect(
        location: impl AsRef<str>,
        mut status: StatusCode,
        headers: Option<HeaderMap>,
    ) -> Response<BoxBodyType> {
        if !status.is_redirection() {
            status = DEFAULT_REDIRECT_STATUS;
        }

        let mut response = Response::new(
            Empty::<Bytes>::new()
                .map_err(|never| match never {})
                .boxed(),
        );

        *response.status_mut() = status;

        if let Ok(location_value) = location.as_ref().parse() {
            response
                .headers_mut()
                .insert(header::LOCATION, location_value);
        }

        if let Some(headers) = headers {
            response.headers_mut().extend(headers);
        }

        response
    }

    pub fn with_status_code(status_code: StatusCode) -> Response<BoxBodyType> {
        Response::builder()
            .status(status_code)
            .body(Self::empty())
            .unwrap_or_else(|_| {
                let mut response = Response::new(Self::empty());
                *response.status_mut() = FALLBACK_STATUS_CODE;
                response
            })
    }

    pub fn with_headers(
        status_code: StatusCode,
        headers: HeaderMap,
    ) -> Response<BoxBodyType> {
        let mut response = Response::new(Self::empty());
        *response.status_mut() = status_code;
        *response.headers_mut() = headers;
        response
    }

    pub fn with_json(
        status_code: StatusCode,
        json: &str,
    ) -> Response<BoxBodyType> {
        let body = Full::new(Bytes::from(json.to_owned()))
            .map_err(|never| match never {})
            .boxed();

        let mut response = Response::new(body);
        *response.status_mut() = status_code;
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        response
    }

    pub fn with_bytes(
        status_code: StatusCode,
        headers: Vec<(HeaderName, HeaderValue)>,
        body_bytes: Bytes,
    ) -> Response<BoxBodyType> {
        let body = Full::new(body_bytes)
            .map_err(|never| match never {})
            .boxed();

        let mut response = Response::new(body);
        *response.status_mut() = status_code;
        for (name, value) in headers {
            response.headers_mut().append(name, value);
        }
        response
    }

    fn empty() -> BoxBodyType {
        Empty::new().map_err(|never| match never {}).boxed()
    }
}
