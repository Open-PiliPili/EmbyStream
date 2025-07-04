use std::sync::Arc;

use async_trait::async_trait;
use hyper::{Response, StatusCode};

use super::{
    request::Request as AppStreamRequest, result::Result as AppStreamResult, service::StreamService,
};
use crate::gateway::{
    chain::{Middleware, Next},
    context::Context,
    response::{BoxBodyType, ResponseBuilder},
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
        if ctx.path != *self.path {
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
                    ResponseBuilder::with_redirect(
                        redirect_info.target_url.to_string().as_str(),
                        StatusCode::FOUND
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
