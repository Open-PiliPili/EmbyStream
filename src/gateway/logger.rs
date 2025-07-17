use async_trait::async_trait;
use hyper::{Response, body::Incoming, header};

use super::{
    chain::{Middleware, Next},
    response::BoxBodyType,
};
use crate::gateway::context::Context;
use crate::{GATEWAY_LOGGER_DOMAIN, info_log};

#[derive(Clone)]
pub struct LoggerMiddleware;

#[async_trait]
impl Middleware for LoggerMiddleware {
    async fn handle(
        &self,
        ctx: Context,
        body: Option<Incoming>,
        next: Next,
    ) -> Response<BoxBodyType> {
        info_log!(GATEWAY_LOGGER_DOMAIN, "Incoming request details:");
        info_log!(
            GATEWAY_LOGGER_DOMAIN,
            "Request scheme and host: {:?}",
            ctx.get_scheme_and_host()
        );
        info_log!(
            GATEWAY_LOGGER_DOMAIN,
            "Request query: {:?}",
            ctx.get_query_params()
        );
        info_log!(
            GATEWAY_LOGGER_DOMAIN,
            "Request method: {} path: {}",
            ctx.method,
            ctx.path
        );
        info_log!(GATEWAY_LOGGER_DOMAIN, "Request headers: {:?}", ctx.headers);

        if ctx.headers.contains_key(header::CONTENT_LENGTH) {
            info_log!(
                GATEWAY_LOGGER_DOMAIN,
                "Request contains a body (content not logged to preserve stream)"
            );
        }

        let response = next(ctx, body).await;

        info_log!(
            GATEWAY_LOGGER_DOMAIN,
            "Response status: {}",
            response.status()
        );
        info_log!(
            GATEWAY_LOGGER_DOMAIN,
            "Response headers: {:?}",
            response.headers()
        );

        response
    }

    fn clone_box(&self) -> Box<dyn Middleware> {
        Box::new(self.clone())
    }
}
