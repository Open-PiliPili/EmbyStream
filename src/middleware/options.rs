use async_trait::async_trait;
use hyper::{Response, StatusCode};

use super::{
    chain::{Middleware, Next},
    response::{BoxBodyType, ResponseBuilder},
};
use crate::middleware::context::Context;
use crate::{MIDDLEWARE_LOGGER_DOMAIN, error_log};

#[derive(Clone)]
pub struct OptionsHandler;

#[async_trait]
impl Middleware for OptionsHandler {
    async fn handle<'a>(&self, ctx: Context, next: Next<'a>) -> Response<BoxBodyType> {
        if ctx.method == hyper::Method::OPTIONS {
            error_log!(
                MIDDLEWARE_LOGGER_DOMAIN,
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
