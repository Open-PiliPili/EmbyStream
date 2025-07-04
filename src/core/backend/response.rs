use hyper::{HeaderMap, StatusCode};

use crate::gateway::response::BoxBodyType;

pub struct Response {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: BoxBodyType,
}
