use async_trait::async_trait;
use hyper::{Response, header};

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
    async fn handle<'a>(&self, ctx: Context, next: Next<'a>) -> Response<BoxBodyType> {
        info_log!(GATEWAY_LOGGER_DOMAIN, "{}", "\n\n");
        info_log!(GATEWAY_LOGGER_DOMAIN, "Incoming request details:");
        info_log!(GATEWAY_LOGGER_DOMAIN, "Request Headers: {:?}", ctx.headers);
        info_log!(
            GATEWAY_LOGGER_DOMAIN,
            "Request Method: {} path: {}",
            ctx.method,
            ctx.path
        );

        if ctx.headers.contains_key(header::CONTENT_LENGTH) {
            info_log!(
                GATEWAY_LOGGER_DOMAIN,
                "Request contains a body (content not logged to preserve stream)"
            );
        }

        let response = next.run(ctx).await;

        info_log!(
            GATEWAY_LOGGER_DOMAIN,
            "Response Status: {}",
            response.status()
        );
        info_log!(
            GATEWAY_LOGGER_DOMAIN,
            "Response Headers: {:?}",
            response.headers()
        );

        response
    }

    fn clone_box(&self) -> Box<dyn Middleware> {
        Box::new(self.clone())
    }
}
