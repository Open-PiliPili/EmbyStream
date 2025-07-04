use std::{path::PathBuf, sync::Arc, time::Instant};

use hyper::StatusCode;

use super::result::Result as AppStreamResult;
use crate::AppState;

pub(crate) struct LocalStreamer;

impl LocalStreamer {
    #[allow(unused_variables)]
    pub async fn stream(
        state: Arc<AppState>,
        path: PathBuf,
        range_header: Option<String>,
        start_time: Instant,
    ) -> Result<AppStreamResult, StatusCode> {
        Err(StatusCode::NOT_FOUND)
    }
}
