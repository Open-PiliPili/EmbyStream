use std::sync::Arc;

use async_trait::async_trait;
use hyper::{Response, StatusCode};

use super::{path_parser, service::ForwardService};
use crate::gateway::{
    chain::{Middleware, Next},
    context::Context,
    response::{BoxBodyType, ResponseBuilder},
};
use crate::{FORWARD_STREAMER_LOGGER_DOMAIN, info_log};

#[derive(Clone)]
pub struct ForwardMiddleware {
    path: Arc<String>,
    stream_service: Arc<dyn ForwardService>
}

impl ForwardMiddleware {
    pub fn new(path: &str, stream_service: Arc<dyn ForwardService>) -> Self {
        Self {
            path: Arc::new(path.to_string()),
            stream_service,
        }
    }
}

#[async_trait]
impl Middleware for ForwardMiddleware {
    async fn handle<'a>(&self, ctx: Context, next: Next<'a>) -> Response<BoxBodyType> {
        todo!("TODO: implement forward middleware later")
    }

    fn clone_box(&self) -> Box<dyn Middleware> {
        Box::new(self.clone())
    }
}