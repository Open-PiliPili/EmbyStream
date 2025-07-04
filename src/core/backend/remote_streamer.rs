use std::sync::Arc;

use hyper::{HeaderMap, StatusCode};
use reqwest::Url;

use super::result::Result as AppStreamResult;
use crate::AppState;

pub(crate) struct RemoteStreamer;

impl RemoteStreamer {
    #[allow(unused_variables)]
    pub async fn stream(
        state: Arc<AppState>,
        url: Url,
        headers: &HeaderMap,
    ) -> Result<AppStreamResult, StatusCode> {
        Err(StatusCode::NOT_FOUND)
    }
}
