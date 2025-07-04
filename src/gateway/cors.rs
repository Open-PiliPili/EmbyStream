use async_trait::async_trait;
use hyper::Response;

use super::{
    chain::{Middleware, Next},
    response::BoxBodyType,
};
use crate::gateway::context::Context;

#[derive(Clone)]
pub struct Cors;

#[async_trait]
impl Middleware for Cors {
    async fn handle<'a>(&self, ctx: Context, next: Next<'a>) -> Response<BoxBodyType> {
        let mut response = next.run(ctx).await;

        response.headers_mut().insert(
            "Access-Control-Allow-Origin",
            "*".parse()
                .expect("Failed to parse CORS Allow-Origin header"),
        );

        response.headers_mut().insert(
            "Access-Control-Allow-Methods",
            "GET,POST,PUT,DELETE,OPTIONS"
                .parse()
                .expect("Failed to parse CORS Allow-Methods header"),
        );

        response.headers_mut().insert(
            "Access-Control-Allow-Headers",
            "Content-Type,Authorization"
                .parse()
                .expect("Failed to parse CORS Allow-Headers header"),
        );

        response
    }

    fn clone_box(&self) -> Box<dyn Middleware> {
        Box::new(self.clone())
    }
}
