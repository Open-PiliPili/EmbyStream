use std::{future::Future, pin::Pin};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

pub type HttpMockHandler = Box<
    dyn Fn(String) -> Pin<Box<dyn Future<Output = String> + Send>>
        + Send
        + Sync,
>;

pub async fn spawn_http_mock_server(handlers: Vec<HttpMockHandler>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock server");
    let addr = listener.local_addr().expect("mock server addr");

    tokio::spawn(async move {
        for handler in handlers {
            let (mut stream, _) = listener.accept().await.expect("accept");
            let mut buf = vec![0_u8; 8192];
            let read = stream.read(&mut buf).await.expect("read request");
            let request = String::from_utf8_lossy(&buf[..read]).to_string();
            let response = handler(request).await;
            stream
                .write_all(response.as_bytes())
                .await
                .expect("write response");
        }
    });

    format!("http://{}", addr)
}

pub fn http_response(status: u16, content_type: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status} OK\r\ncontent-type: {content_type}\r\n\
         content-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    )
}
