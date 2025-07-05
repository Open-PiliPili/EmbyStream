use std::{
    path::PathBuf,
    sync::Arc,
};

use async_trait::async_trait;
use hyper::{HeaderMap, StatusCode, header, Uri};
use serde_urlencoded as urlencoded;

use super::{
    error::Error as AppStreamError, local_streamer::LocalStreamer, proxy_mode::ProxyMode,
    redirect_info::RedirectInfo, remote_streamer::RemoteStreamer,
    request::Request as AppStreamRequest, result::Result as AppStreamResult, source::Source,
};
use crate::sign::{SignParams, Sign};
use crate::{AppState, STREAM_LOGGER_DOMAIN, info_log};

#[async_trait]
pub trait StreamService: Send + Sync {
    async fn handle_request(
        &self,
        request: AppStreamRequest,
    ) -> Result<AppStreamResult, StatusCode>;
}

pub struct AppStreamService {
    pub state: Arc<AppState>,
    pub user_agent: Option<String>,
}

impl AppStreamService {
    pub fn new(state: Arc<AppState>, user_agent: Option<String>) -> Self {
        Self { state, user_agent }
    }

    fn decrypt_and_route(&self, request: &AppStreamRequest) -> Result<Source, AppStreamError> {
        let params = request.uri.query()
            .and_then(|query| serde_urlencoded::from_str::<SignParams>(query).ok())
            .unwrap_or_default();

        if params.sign.is_empty() {
            return Err(AppStreamError::InvalidSignature);
        }

        // TODO: parse sign later
        let _ = Sign::decrypt_with(params.sign);

        if request.is_local() {
            Ok(Source::Local(PathBuf::from(request.uri.to_string())))
        } else {
            Ok(Source::Remote {
                url: request.uri.clone(),
                mode: params.proxy_mode,
            })
        }
    }

    fn build_redirect_info(&self, url: Uri, original_headers: &HeaderMap) -> RedirectInfo {
        let mut final_headers = original_headers.clone();

        if let Some(user_agent) = &self.user_agent {
            if !user_agent.is_empty() {
                if let Ok(parsed_header) = user_agent.parse() {
                    final_headers.insert(header::USER_AGENT, parsed_header);
                }
            }
        }

        final_headers.remove(header::HOST);

        RedirectInfo {
            target_url: url,
            final_headers,
        }
    }
}

#[async_trait]
impl StreamService for AppStreamService {
    async fn handle_request(
        &self,
        request: AppStreamRequest,
    ) -> Result<AppStreamResult, StatusCode> {
        let source = self
            .decrypt_and_route(&request)
            .map_err(|_| StatusCode::UNAUTHORIZED)?;
        info_log!(STREAM_LOGGER_DOMAIN, "Routing stream source: {:?}", source);

        match source {
            Source::Local(path) => {
                LocalStreamer::stream(
                    self.state.clone(),
                    path,
                    request.content_range(),
                    request.request_start_time,
                )
                    .await
            }
            Source::Remote { url, mode } => match mode {
                ProxyMode::Redirect => {
                    let redirect_info = self.build_redirect_info(url, &request.original_headers);
                    Ok(AppStreamResult::Redirect(redirect_info))
                }
                ProxyMode::Proxy => {
                    RemoteStreamer::stream(
                        self.state.clone(),
                        url,
                        self.user_agent.clone(),
                        &request.original_headers,
                    ).await
                }
            },
        }
    }
}
