use async_trait::async_trait;
use hyper::{Response, body::Incoming};

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
    async fn handle(&self, ctx: Context, body: Option<Incoming>, next: Next) -> Response<BoxBodyType> {
        debug_log!(GATEWAY_LOGGER_DOMAIN, "Starting HTTP cors middleware...");

        let mut response = next(ctx, body).await;

        response.headers_mut().insert(
            "Access-Control-Allow-Origin",
            "*".parse()
                .expect("Failed to parse CORS Allow-Origin header"),
        );

        response.headers_mut().insert(
            "Access-Control-Allow-Methods",
            "GET,POST,PUT,DELETE,OPTIONS"
                .parse()
                .expect("Failed to parse CORS Allow-Methods header"),
        );

        response.headers_mut().insert(
            "Access-Control-Allow-Headers",
            "Content-Type,Authorization"
                .parse()
                .expect("Failed to parse CORS Allow-Headers header"),
        );

        response
    }

    fn clone_box(&self) -> Box<dyn Middleware> {
        Box::new(self.clone())
    }
}
