use std::{
    fs::File as StdFile,
    io::{BufReader, Error as IoError, Read, Seek, SeekFrom},
    path::PathBuf,
    sync::Arc,
};

use bytes::Bytes;
use tokio::sync::mpsc;
use tokio_stream::{Stream, wrappers::ReceiverStream};

use super::types::ContentRange;
use crate::{READ_STREAM_LOGGER_DOMAIN, error_log};

#[derive(Debug)]
pub struct ReaderStream {
    path: Arc<PathBuf>,
    content_range: ContentRange,
}

impl ReaderStream {
    pub fn new(path: impl Into<PathBuf>, content_range: ContentRange) -> Self {
        Self {
            path: Arc::new(path.into()),
            content_range,
        }
    }

    pub fn into_stream(self) -> impl Stream<Item = Result<Bytes, IoError>> {
        let (tx, rx) = mpsc::channel(self.get_optimal_channel_size());
        let path = self.path.clone();
        let content_range = self.content_range;
        let chunk_size = self.get_chunk_size_for_streaming();

        tokio::task::spawn_blocking(move || {
            if let Err(e) =
                Self::read_file_to_channel(&path, content_range, chunk_size, tx)
            {
                error_log!(
                    READ_STREAM_LOGGER_DOMAIN,
                    "Error in file streaming task: {}",
                    e
                );
            }
        });

        ReceiverStream::new(rx)
    }

    fn read_file_to_channel(
        path: &PathBuf,
        content_range: ContentRange,
        main_chunk: usize,
        tx: mpsc::Sender<Result<Bytes, IoError>>,
    ) -> Result<(), IoError> {
        let file = StdFile::open(path)?;
        let mut reader = BufReader::with_capacity(main_chunk, file);

        reader.seek(SeekFrom::Start(content_range.start))?;

        let mut limited_reader = reader.take(content_range.length());
        let mut buffer = vec![0u8; main_chunk];
        let mut is_first_read = true;

        loop {
            let read_cap = if is_first_read {
                is_first_read = false;
                const FIRST_READ_CAP: usize = 256 * 1024;
                FIRST_READ_CAP.min(main_chunk).max(1)
            } else {
                main_chunk
            };

            let bytes_read = limited_reader.read(&mut buffer[..read_cap])?;
            if bytes_read == 0 {
                break;
            }

            if tx
                .blocking_send(Ok(Bytes::copy_from_slice(
                    &buffer[..bytes_read],
                )))
                .is_err()
            {
                break;
            }
        }

        Ok(())
    }

    #[inline]
    fn get_chunk_size_for_streaming(&self) -> usize {
        disk_main_read_chunk(self.content_range.start)
    }

    #[inline]
    fn get_optimal_channel_size(&self) -> usize {
        let length = self.content_range.length();
        let chunk_size = self.get_chunk_size_for_streaming() as u64;
        (length / chunk_size)
            .try_into()
            .unwrap_or(128)
            .clamp(4, 128)
    }
}

/// Steady-state read size for Disk local streaming (PLAN-04): larger after first byte for throughput.
#[inline]
pub(crate) fn disk_main_read_chunk(range_start: u64) -> usize {
    const KB: usize = 1024;
    const MB: usize = 1024 * KB;
    if range_start > 0 { 4 * MB } else { 2 * MB }
}

#[cfg(test)]
mod tests {
    use super::disk_main_read_chunk;

    #[test]
    fn disk_main_read_chunk_from_zero_is_2mb() {
        const MB: usize = 1024 * 1024;
        assert_eq!(disk_main_read_chunk(0), 2 * MB);
    }

    #[test]
    fn disk_main_read_chunk_after_seek_is_4mb() {
        const MB: usize = 1024 * 1024;
        assert_eq!(disk_main_read_chunk(1), 4 * MB);
    }
}
