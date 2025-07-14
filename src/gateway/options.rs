use async_trait::async_trait;
use hyper::{Response, StatusCode};

use super::{
    chain::{Middleware, Next},
    response::{BoxBodyType, ResponseBuilder},
};
use crate::gateway::context::Context;
use crate::{GATEWAY_LOGGER_DOMAIN, debug_log, error_log};

#[derive(Clone)]
pub struct OptionsMiddleware;

#[async_trait]
impl Middleware for OptionsMiddleware {
    async fn handle<'a>(&self, ctx: Context, next: Next<'a>) -> Response<BoxBodyType> {
        debug_log!(GATEWAY_LOGGER_DOMAIN, "Starting options middleware...");

        if ctx.method == hyper::Method::OPTIONS {
            error_log!(
                GATEWAY_LOGGER_DOMAIN,
                "OPTIONS request received, aborting with status 204"
            );
            return ResponseBuilder::with_status_code(StatusCode::NO_CONTENT);
        }

        next.run(ctx).await
    }

    fn clone_box(&self) -> Box<dyn Middleware> {
        Box::new(self.clone())
    }
}
