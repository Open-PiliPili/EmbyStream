use std::sync::Arc;

use async_trait::async_trait;
use hyper::{Response, StatusCode};

use super::{result::Result as AppStreamResult, service::StreamService};
use crate::{GATEWAY_LOGGER_DOMAIN, REMOTE_STREAMER_LOGGER_DOMAIN, debug_log, info_log};
use crate::{
    core::request::Request as AppStreamRequest,
    gateway::{
        chain::{Middleware, Next},
        context::Context,
        response::{BoxBodyType, ResponseBuilder},
    },
};

#[derive(Clone)]
pub struct StreamMiddleware {
    path: Arc<String>,
    stream_service: Arc<dyn StreamService>,
}
impl StreamMiddleware {
    pub fn new(path: &str, stream_service: Arc<dyn StreamService>) -> Self {
        Self {
            path: Arc::new(path.to_string()),
            stream_service,
        }
    }
}

#[async_trait]
impl Middleware for StreamMiddleware {
    async fn handle<'a>(&self, ctx: Context, next: Next<'a>) -> Response<BoxBodyType> {
        debug_log!(GATEWAY_LOGGER_DOMAIN, "Starting stream middleware...");

        let request_path = {
            let path = ctx.path.clone().to_lowercase();
            path.trim_start_matches('/')
                .trim_end_matches('/')
                .to_string()
        };

        let expected_path = {
            let path = self.path.clone().to_lowercase();
            path.trim_start_matches('/')
                .trim_end_matches('/')
                .to_string()
        };

        if expected_path != request_path {
            debug_log!(
                REMOTE_STREAMER_LOGGER_DOMAIN,
                "Ctx path: {} doesn't match path {}!",
                ctx.path,
                self.path
            );
            return next.run(ctx).await;
        }

        let stream_request = AppStreamRequest {
            uri: ctx.uri,
            original_headers: ctx.headers,
            request_start_time: ctx.start_time,
        };

        let result = self.stream_service.handle_request(stream_request).await;

        match result {
            Ok(service_result) => match service_result {
                AppStreamResult::Stream(stream_response) => {
                    let mut response = Response::builder()
                        .status(stream_response.status)
                        .body(stream_response.body)
                        .expect("Failed to build stream response");
                    *response.headers_mut() = stream_response.headers;
                    response
                }
                AppStreamResult::Redirect(redirect_info) => {
                    info_log!(
                        REMOTE_STREAMER_LOGGER_DOMAIN,
                        "Redirecting backend to {:?}",
                        redirect_info.target_url
                    );
                    debug_log!(
                        REMOTE_STREAMER_LOGGER_DOMAIN,
                        "Redirecting backend headers {:?}",
                        redirect_info.final_headers.clone()
                    );
                    ResponseBuilder::with_redirect(
                        redirect_info.target_url.to_string().as_str(),
                        StatusCode::FOUND,
                        Some(redirect_info.final_headers),
                    )
                }
            },
            Err(status_code) => ResponseBuilder::with_status_code(status_code),
        }
    }

    fn clone_box(&self) -> Box<dyn Middleware> {
        Box::new(self.clone())
    }
}
