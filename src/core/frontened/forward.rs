use std::sync::Arc;

use async_trait::async_trait;
use hyper::{Response, StatusCode};

use super::{path_parser, service::ForwardService};
use crate::{FORWARD_LOGGER_DOMAIN, info_log, debug_log};
use crate::{
    core::request::Request as AppForwardRequest,
    gateway::{
        chain::{Middleware, Next},
        context::Context,
        response::{BoxBodyType, ResponseBuilder},
    },
};

#[derive(Clone)]
pub struct ForwardMiddleware {
    path: Arc<String>,
    forward_service: Arc<dyn ForwardService>,
}

impl ForwardMiddleware {
    pub fn new(path: &str, forward_service: Arc<dyn ForwardService>) -> Self {
        Self {
            path: Arc::new(path.to_string()),
            forward_service,
        }
    }
}

#[async_trait]
impl Middleware for ForwardMiddleware {
    async fn handle<'a>(&self, ctx: Context, next: Next<'a>) -> Response<BoxBodyType> {
        let Some(path_params) = path_parser::parse_video_path(&ctx.path) else {
            return next.run(ctx).await;
        };

        let forward_request = AppForwardRequest {
            uri: ctx.uri,
            original_headers: ctx.headers,
            request_start_time: ctx.start_time,
        };

        let result = self
            .forward_service
            .handle_request(forward_request, path_params)
            .await;

        match result {
            Ok(redirect_info) => {
                info_log!(
                    FORWARD_LOGGER_DOMAIN,
                    "Redirecting forward to {:?}",
                    redirect_info.target_url
                );
                debug_log!(
                    FORWARD_LOGGER_DOMAIN,
                    "Redirecting forward headers {:?}",
                    redirect_info.final_headers.clone()
                );
                ResponseBuilder::with_redirect(
                    redirect_info.target_url.to_string(),
                    StatusCode::FOUND,
                    Some(redirect_info.final_headers),
                )
            }
            Err(status_code) => ResponseBuilder::with_status_code(status_code),
        }
    }

    fn clone_box(&self) -> Box<dyn Middleware> {
        Box::new(self.clone())
    }
}
