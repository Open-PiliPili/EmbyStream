use async_trait::async_trait;
use hyper::Response;

use crate::gateway::context::Context;
use super::{
    chain::{Next, Middleware},
    response::BoxBodyType
};

#[derive(Clone)]
pub struct Cors;

#[async_trait]
impl Middleware for Cors {
    async fn handle<'a>(&self, ctx: Context, next: Next<'a>) -> Response<BoxBodyType> {
        let mut response = next.run(ctx).await;

        response
            .headers_mut()
            .insert("Access-Control-Allow-Origin", "*".parse().unwrap());
        response
            .headers_mut()
            .insert("Access-Control-Allow-Methods", "GET,POST,PUT,DELETE,OPTIONS".parse().unwrap());
        response
            .headers_mut()
            .insert("Access-Control-Allow-Headers", "Content-Type,Authorization".parse().unwrap());

        response
    }

    fn clone_box(&self) -> Box<dyn Middleware> {
        Box::new(self.clone())
    }
}