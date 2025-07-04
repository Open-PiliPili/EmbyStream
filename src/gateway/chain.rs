use std::sync::Arc;
use std::time::Instant;
use async_trait::async_trait;
use hyper::{Request, Response, body};

use crate::gateway::{context::Context, response::BoxBodyType};

pub type Handler = Arc<dyn Fn(Context) -> Response<BoxBodyType> + Send + Sync>;

#[async_trait]
pub trait Middleware: Send + Sync {
    async fn handle<'a>(&self, ctx: Context, next: Next<'a>) -> Response<BoxBodyType>;
    fn clone_box(&self) -> Box<dyn Middleware>;
}

impl Clone for Box<dyn Middleware> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

pub struct Next<'a> {
    chain: &'a [Box<dyn Middleware>],
    handler: &'a Handler,
}

impl<'a> Next<'a> {
    pub async fn run(self, ctx: Context) -> Response<BoxBodyType> {
        if let Some((current, rest)) = self.chain.split_first() {
            let next = Next {
                chain: rest,
                handler: self.handler,
            };
            current.handle(ctx, next).await
        } else {
            (self.handler)(ctx)
        }
    }
}

pub struct Chain {
    middlewares: Vec<Box<dyn Middleware>>,
    handler: Handler,
}

impl Chain {
    pub fn new(middlewares: Vec<Box<dyn Middleware>>, handler: Handler) -> Self {
        Self {
            middlewares,
            handler,
        }
    }

    pub fn add(mut self, middleware: Box<dyn Middleware>) -> Self {
        self.middlewares.push(middleware);
        self
    }

    pub async fn run(self, req: Request<body::Incoming>) -> Response<BoxBodyType> {
        let (parts, body) = req.into_parts();
        let ctx = Context::new(
            parts.uri,
            parts.method,
            parts.headers,
            Some(body),
            Instant::now()
        );

        let next = Next {
            chain: &self.middlewares,
            handler: &self.handler,
        };

        next.run(ctx).await
    }
}
