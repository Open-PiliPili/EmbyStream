use std::{convert::Infallible, error::Error, net::SocketAddr, sync::Arc};

use hyper::{Request, Response, StatusCode, body, server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

use super::{
    chain::{Chain, Handler, Middleware},
    cors::Cors,
    logger::LoggerHandler,
    options::OptionsHandler,
};
use crate::gateway::{
    context::Context,
    response::{BoxBodyType, ResponseBuilder},
};
use crate::{MIDDLEWARE_LOGGER_DOMAIN, error_log};

pub struct Gateway {
    addr: String,
    handler: Option<Handler>,
    middlewares: Vec<Box<dyn Middleware>>,
}

impl Gateway {
    pub fn new(addr: &str) -> Self {
        Self {
            addr: addr.to_string(),
            handler: None,
            middlewares: Vec::new(),
        }
    }

    pub fn add_middleware(mut self, middleware: Box<dyn Middleware>) -> Self {
        self.middlewares.push(middleware);
        self
    }

    pub fn set_handler(&mut self, handler: Handler) {
        self.handler = Some(handler);
    }

    pub async fn listen(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let addr: SocketAddr = self.addr.parse()?;
        let listener = TcpListener::bind(&addr).await?;

        self.middlewares.push(Box::new(LoggerHandler));
        self.middlewares.push(Box::new(OptionsHandler));
        self.middlewares.push(Box::new(Cors));

        let handler = self.handler.clone().unwrap_or_else(|| {
            Arc::new(|_ctx: Context| -> Response<BoxBodyType> {
                ResponseBuilder::with_status_code(StatusCode::INTERNAL_SERVER_ERROR)
            })
        });

        let middlewares = Arc::new(std::mem::take(&mut self.middlewares));

        loop {
            let (stream, _) = listener.accept().await?;
            let handler = handler.clone();
            let middlewares_clone = middlewares.clone();

            tokio::spawn(async move {
                let io = TokioIo::new(stream);

                let service = service_fn(move |req: Request<body::Incoming>| {
                    let handler = handler.clone();
                    let middlewares = middlewares_clone.clone();

                    async move {
                        let chain = Chain::new(middlewares.to_vec(), handler);
                        let response = chain.run(req).await;

                        Ok::<_, Infallible>(response)
                    }
                });

                if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                    if !is_ignorable_connection_error(&err) {
                        error_log!(
                            MIDDLEWARE_LOGGER_DOMAIN,
                            "Server connection error: {:?}",
                            err
                        );
                    }
                }
            });
        }
    }
}

fn is_ignorable_connection_error(err: &dyn Error) -> bool {
    err.to_string().contains("connection closed")
}
