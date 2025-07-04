use hyper::{HeaderMap, StatusCode};
use reqwest::Url;

use super::{
    result::Result as AppStreamResult
};

pub(crate) struct RemoteStreamer;

impl RemoteStreamer {
    #[allow(unused_variables)]
    pub async fn stream(
        url: Url,
        headers: &HeaderMap,
    ) -> Result<AppStreamResult, StatusCode> {
        Err(StatusCode::NOT_FOUND)
    }
}