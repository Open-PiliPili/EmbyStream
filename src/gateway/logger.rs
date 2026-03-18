use async_trait::async_trait;
use hyper::{Response, body::Incoming, header};

use super::{
    cacheable_routes::find_cacheable_route,
    chain::{Middleware, Next},
    context::Context,
    debug_paths::is_debug_path,
    response::BoxBodyType,
};
use crate::{GATEWAY_LOGGER_DOMAIN, debug_log, info_log};

macro_rules! cond_log {
    ($use_debug:expr, $domain:expr, $($args:tt)*) => {
        if $use_debug {
            debug_log!($domain, $($args)*);
        } else {
            info_log!($domain, $($args)*);
        }
    };
}

fn should_use_debug(ctx: &Context) -> bool {
    is_debug_path(&ctx.path)
        && find_cacheable_route(&ctx.path, ctx.method.as_str()).is_none()
}

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
        let use_debug = should_use_debug(&ctx);

        cond_log!(
            use_debug,
            GATEWAY_LOGGER_DOMAIN,
            "Incoming request details:"
        );
        cond_log!(
            use_debug,
            GATEWAY_LOGGER_DOMAIN,
            "Request scheme and host: {:?}",
            ctx.get_host()
        );
        cond_log!(
            use_debug,
            GATEWAY_LOGGER_DOMAIN,
            "Request query: {:?}",
            ctx.get_query_params()
        );
        cond_log!(
            use_debug,
            GATEWAY_LOGGER_DOMAIN,
            "Request method: {} path: {}",
            ctx.method,
            ctx.path
        );
        cond_log!(
            use_debug,
            GATEWAY_LOGGER_DOMAIN,
            "Request headers: {:?}",
            ctx.headers
        );

        if ctx.headers.contains_key(header::CONTENT_LENGTH) {
            debug_log!(
                GATEWAY_LOGGER_DOMAIN,
                "Request contains a body (content not logged to preserve stream)"
            );
        }

        let response = next(ctx, body).await;

        cond_log!(
            use_debug,
            GATEWAY_LOGGER_DOMAIN,
            "Response status: {}",
            response.status()
        );
        cond_log!(
            use_debug,
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
