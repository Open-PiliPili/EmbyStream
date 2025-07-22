use std::{future::Future, pin::Pin, sync::Arc};

use async_trait::async_trait;
use hyper::{
    Request, Response,
    body::{self, Incoming},
};

use crate::gateway::{context::Context, response::BoxBodyType};

pub type Handler = Arc<
    dyn Fn(Context, Option<Incoming>) -> Response<BoxBodyType> + Send + Sync,
>;

type ResponseFuture =
    Pin<Box<dyn Future<Output = Response<BoxBodyType>> + Send>>;

pub type Next =
    Box<dyn FnOnce(Context, Option<Incoming>) -> ResponseFuture + Send>;

#[async_trait]
pub trait Middleware: Send + Sync {
    async fn handle(
        &self,
        ctx: Context,
        body: Option<Incoming>,
        next: Next,
    ) -> Response<BoxBodyType>;
    fn clone_box(&self) -> Box<dyn Middleware>;
}

impl Clone for Box<dyn Middleware> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

pub struct Chain {
    middlewares: Vec<Box<dyn Middleware>>,
    handler: Handler,
}

impl Chain {
    pub fn new(
        middlewares: Vec<Box<dyn Middleware>>,
        handler: Handler,
    ) -> Self {
        Self {
            middlewares,
            handler,
        }
    }

    pub fn add_middleware(mut self, middleware: Box<dyn Middleware>) -> Self {
        self.middlewares.push(middleware);
        self
    }

    pub async fn run(
        self,
        req: Request<body::Incoming>,
    ) -> Response<BoxBodyType> {
        let (parts, body) = req.into_parts();
        let ctx = Context::new(
            parts.uri,
            parts.method,
            parts.headers,
            std::time::Instant::now(),
        );

        let handler_action: Next = Box::new(move |ctx, body| {
            Box::pin(async move { (self.handler)(ctx, body) })
        });

        let chain_entry = self.middlewares.into_iter().rfold(
            handler_action,
            |next_action, middleware| {
                Box::new(move |ctx, body| {
                    Box::pin(async move {
                        middleware.handle(ctx, body, next_action).await
                    })
                })
            },
        );

        chain_entry(ctx, Some(body)).await
    }
}
