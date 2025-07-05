use std::{
    collections::HashMap,
    sync::Arc,
};

use futures_util::StreamExt;
use http_body_util::{BodyExt, StreamBody};
use hyper::{body::Frame, header, HeaderMap, StatusCode, Uri};

use super::{
    response::Response,
    result::Result as AppStreamResult,
};
use crate::{
    client::{ClientBuilder, DownloadClient},
    network::CurlPlugin,
};
use crate::{AppState, error_log, REMOTE_STREAMER_LOGGER_DOMAIN};

pub(crate) struct RemoteStreamer;

impl RemoteStreamer {
    #[allow(unused_variables)]
    pub async fn stream(
        state: Arc<AppState>,
        url: Uri,
        user_agent: Option<String>,
        headers: &HeaderMap,
    ) -> Result<AppStreamResult, StatusCode> {
        let client = ClientBuilder::<DownloadClient>::new()
            .with_plugin(CurlPlugin)
            .build();

        let mut headers_to_forward = headers.clone();
        headers_to_forward.remove(header::HOST);

        let forwarded_headers = Self::header_map_to_option_hashmap(headers);
        let remote_response = client
            .download(url.to_string(), user_agent, forwarded_headers)
            .await
            .map_err(|e| {
                error_log!(
                    REMOTE_STREAMER_LOGGER_DOMAIN,
                    "Failed to connect to remote stream source: {}",
                    e
                );
                StatusCode::BAD_GATEWAY
            })?;

        let response_status = remote_response.status();
        let response_headers = remote_response.headers().clone();

        let stream = remote_response
            .bytes_stream()
            .map(|res| {
                res.map(Frame::data)
                    .map_err(|e| e.into())
            });

        Ok(AppStreamResult::Stream(Response {
            status: response_status,
            headers: response_headers,
            body: BodyExt::boxed(StreamBody::new(stream)),
        }))
    }

    fn header_map_to_option_hashmap(
        headers: &HeaderMap,
    ) -> Option<HashMap<String, String>> {
        headers.iter().next().map(|_| {
            headers.iter().fold(
                HashMap::new(),
                |mut acc, (name, value)| {
                    acc.insert(
                        name.as_str().to_owned(),
                        String::from_utf8_lossy(value.as_bytes())
                            .into_owned(),
                    );
                    acc
                },
            )
        })
    }
}
