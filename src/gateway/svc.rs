use std::{convert::Infallible, pin::Pin, sync::Arc};

use http_serde::http::{Request, Response};
use hyper::{body::Incoming, service::Service};

use super::{
    BoxBodyType, Middleware,
    chain::{Chain, Handler},
};

#[derive(Clone)]
pub struct Svc {
    handler: Handler,
    middlewares: Arc<Vec<Box<dyn Middleware>>>,
}

impl Svc {
    pub fn new(
        handler: Handler,
        middlewares: Arc<Vec<Box<dyn Middleware>>>,
    ) -> Self {
        Self {
            handler,
            middlewares,
        }
    }
}

impl Service<Request<Incoming>> for Svc {
    type Response = Response<BoxBodyType>;
    type Error = Infallible;
    type Future = Pin<
        Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        let handler = self.handler.clone();
        let middlewares = self.middlewares.clone();

        Box::pin(async move {
            let chain = Chain::new(middlewares.to_vec(), handler);
            let response = chain.run(req).await;
            Ok(response)
        })
    }
}
