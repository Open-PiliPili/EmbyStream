use async_trait::async_trait;
use hyper::{Response, body::Incoming, header::HeaderValue};

use super::{
    chain::{Middleware, Next},
    response::BoxBodyType,
};
use crate::gateway::context::Context;
use crate::{GATEWAY_LOGGER_DOMAIN, debug_log};

#[derive(Clone)]
pub struct CorsMiddleware;

#[async_trait]
impl Middleware for CorsMiddleware {
    async fn handle(
        &self,
        ctx: Context,
        body: Option<Incoming>,
        next: Next,
    ) -> Response<BoxBodyType> {
        debug_log!(GATEWAY_LOGGER_DOMAIN, "Starting HTTP cors middleware...");

        let mut response = next(ctx, body).await;

        response.headers_mut().insert(
            "Access-Control-Allow-Origin",
            HeaderValue::from_static("*"),
        );

        response.headers_mut().insert(
            "Access-Control-Allow-Methods",
            HeaderValue::from_static("GET,POST,PUT,DELETE,OPTIONS"),
        );

        response.headers_mut().insert(
            "Access-Control-Allow-Headers",
            HeaderValue::from_static("Content-Type,Authorization"),
        );

        response
    }

    fn clone_box(&self) -> Box<dyn Middleware> {
        Box::new(self.clone())
    }
}
