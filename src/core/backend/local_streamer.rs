use std::{path::PathBuf, time::Instant};

use hyper::StatusCode;

use super::result::Result as AppStreamResult;

pub(crate) struct LocalStreamer;

impl LocalStreamer {
    #[allow(unused_variables)]
    pub async fn stream(
        path: PathBuf,
        range_header: Option<String>,
        start_time: Instant,
    ) -> Result<AppStreamResult, StatusCode> {
        Err(StatusCode::NOT_FOUND)
    }
}
