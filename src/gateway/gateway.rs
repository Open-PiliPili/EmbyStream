use std::{
    error::Error as StdError,
    fs::File,
    io::{BufReader, Error as IoError, ErrorKind as IoErrorKind},
    net::SocketAddr,
    path::Path,
    sync::Arc,
};
use std::path::PathBuf;
use hyper::{Response, StatusCode, body::Incoming, server::conn::http1};
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto as hyper_conn_auto,
};
use rustls::ServerConfig;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

use super::{
    chain::{Handler, Middleware},
    svc::Svc,
};
use crate::{
    GATEWAY_LOGGER_DOMAIN, error_log,
    gateway::{
        context::Context,
        response::{BoxBodyType, ResponseBuilder},
    },
    info_log, warn_log,
};

pub struct Gateway {
    addr: String,
    handler: Option<Handler>,
    middlewares: Vec<Box<dyn Middleware>>,
    cert_path: Option<String>,
    key_path: Option<String>,
}

impl Gateway {
    pub fn new(addr: &str) -> Self {
        Self {
            addr: addr.to_string(),
            handler: None,
            middlewares: Vec::new(),
            cert_path: None,
            key_path: None,
        }
    }

    pub fn with_tls(mut self, cert_path: Option<PathBuf>, key_path: Option<PathBuf>) -> Self {
        if let (Some(cert), Some(key)) = (cert_path, key_path) {
            if cert.exists() && key.exists() {
                info_log!(
                    GATEWAY_LOGGER_DOMAIN,
                    "SSL certificate exist, start loading cert_path={:?}, key_path={:?}",
                    cert,
                    key
                );
                self.cert_path = Some(cert.to_string_lossy().into_owned());
                self.key_path = Some(key.to_string_lossy().into_owned());
            } else {
                warn_log!(
                    GATEWAY_LOGGER_DOMAIN,
                    "SSL certificate does not exist: cert_path={:?}, key_path={:?}",
                    cert,
                    key
                )
            }
        }
        self
    }

    pub fn add_middleware(mut self, middleware: Box<dyn Middleware>) -> Self {
        self.middlewares.push(middleware);
        self
    }

    pub fn set_handler(&mut self, handler: Handler) {
        self.handler = Some(handler);
    }

    pub async fn listen(&mut self) -> Result<(), Box<dyn StdError + Send + Sync>> {
        let addr: SocketAddr = self.addr.parse()?;
        let listener = TcpListener::bind(&addr).await?;
        let handler = self.handler.clone().unwrap_or_else(Self::default_handler);
        let middlewares = Arc::new(std::mem::take(&mut self.middlewares));

        self.run_server(listener, handler, middlewares).await
    }

    async fn run_server(
        &self,
        listener: TcpListener,
        handler: Handler,
        middlewares: Arc<Vec<Box<dyn Middleware>>>,
    ) -> Result<(), Box<dyn StdError + Send + Sync>> {
        let addr = listener.local_addr()?;
        if let (Some(cert_path), Some(key_path)) = (&self.cert_path, &self.key_path) {
            match self.load_tls_config(Path::new(cert_path), Path::new(key_path)) {
                Ok(tls_config) => {
                    let tls_acceptor = TlsAcceptor::from(Arc::new(tls_config));
                    self.run_https_server(&addr, listener, handler, middlewares, tls_acceptor)
                        .await
                }
                Err(e) => {
                    warn_log!(
                        GATEWAY_LOGGER_DOMAIN,
                        "Failed to load TLS config: {}. Falling back to plain HTTP/1.1.",
                        e
                    );
                    self.run_http_server(&addr, listener, handler, middlewares)
                        .await
                }
            }
        } else {
            self.run_http_server(&addr, listener, handler, middlewares)
                .await
        }
    }

    async fn run_http_server(
        &self,
        addr: &SocketAddr,
        listener: TcpListener,
        handler: Handler,
        middlewares: Arc<Vec<Box<dyn Middleware>>>,
    ) -> Result<(), Box<dyn StdError + Send + Sync>> {
        info_log!(
            GATEWAY_LOGGER_DOMAIN,
            "Gateway listening with HTTP/1.1 on addr {}",
            addr
        );

        loop {
            let (stream, peer_addr) = listener.accept().await?;
            let service = Svc::new(handler.clone(), middlewares.clone());

            tokio::spawn(async move {
                let io = TokioIo::new(stream);
                if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                    if !Self::is_ignorable_connection_error(&err) {
                        error_log!(
                            GATEWAY_LOGGER_DOMAIN,
                            "Error serving HTTP connection from {}: {:?}",
                            peer_addr,
                            err
                        );
                    }
                }
            });
        }
    }

    async fn run_https_server(
        &self,
        addr: &SocketAddr,
        listener: TcpListener,
        handler: Handler,
        middlewares: Arc<Vec<Box<dyn Middleware>>>,
        tls_acceptor: TlsAcceptor,
    ) -> Result<(), Box<dyn StdError + Send + Sync>> {
        info_log!(
            GATEWAY_LOGGER_DOMAIN,
            "Gateway listening with TLS (H2/H1) on addr {}",
            addr
        );
        loop {
            let (stream, peer_addr) = listener.accept().await?;
            let tls_acceptor = tls_acceptor.clone();
            let service = Svc::new(handler.clone(), middlewares.clone());

            tokio::spawn(async move {
                if let Ok(tls_stream) = tls_acceptor.accept(stream).await {
                    let io = TokioIo::new(tls_stream);
                    if let Err(err) = hyper_conn_auto::Builder::new(TokioExecutor::new())
                        .serve_connection(io, service)
                        .await
                    {
                        if !Self::is_ignorable_connection_error(err.as_ref()) {
                            error_log!(
                                GATEWAY_LOGGER_DOMAIN,
                                "Error serving HTTPS connection from {}: {:?}",
                                peer_addr,
                                err
                            );
                        }
                    }
                }
            });
        }
    }

    fn load_tls_config(
        &self,
        cert_path: &Path,
        key_path: &Path,
    ) -> Result<ServerConfig, Box<dyn StdError + Send + Sync>> {
        let cert_file = File::open(cert_path)
            .map_err(|e| format!("failed to open cert file {:?}: {}", cert_path, e))?;
        let certs =
            rustls_pemfile::certs(&mut BufReader::new(cert_file)).collect::<Result<Vec<_>, _>>()?;

        let key_file = File::open(key_path)
            .map_err(|e| format!("failed to open key file {:?}: {}", key_path, e))?;
        let key = rustls_pemfile::private_key(&mut BufReader::new(key_file))?
            .ok_or("no private key found in file")?;

        let mut config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key.into())?;
        config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
        Ok(config)
    }

    fn default_handler() -> Handler {
        Arc::new(
            |_ctx: Context, _body: Option<Incoming>| -> Response<BoxBodyType> {
                ResponseBuilder::with_status_code(StatusCode::INTERNAL_SERVER_ERROR)
            },
        )
    }

    fn is_ignorable_connection_error(err: &(dyn StdError + 'static)) -> bool {
        let mut source = Some(err);
        while let Some(current_err) = source {
            if let Some(io_err) = current_err.downcast_ref::<IoError>() {
                if matches!(
                    io_err.kind(),
                    IoErrorKind::ConnectionReset | IoErrorKind::BrokenPipe
                ) {
                    return true;
                }
            }
            if let Some(hyper_err) = current_err.downcast_ref::<hyper::Error>() {
                if hyper_err.is_canceled() || hyper_err.is_closed() {
                    return true;
                }
            }
            source = current_err.source();
        }
        false
    }
}
