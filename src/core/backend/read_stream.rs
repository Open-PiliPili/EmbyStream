use std::{
    fs::File as StdFile,
    io::{BufReader, Error as IoError, Read, Seek, SeekFrom},
    path::PathBuf,
    sync::Arc,
    time::Instant,
};

use bytes::Bytes;
use tokio::sync::mpsc;
use tokio_stream::{Stream, wrappers::ReceiverStream};

use super::types::ContentRange;
use crate::{READ_STREAM_LOGGER_DOMAIN, debug_log, error_log};

#[derive(Debug)]
pub struct ReaderStream {
    source: ReaderSource,
    path: Arc<PathBuf>,
    content_range: ContentRange,
}

#[derive(Debug)]
enum ReaderSource {
    Path,
    OpenedFile(StdFile),
}

impl ReaderStream {
    pub fn new(path: impl Into<PathBuf>, content_range: ContentRange) -> Self {
        Self {
            source: ReaderSource::Path,
            path: Arc::new(path.into()),
            content_range,
        }
    }

    pub fn from_opened_file(
        path: impl Into<PathBuf>,
        opened_file: StdFile,
        content_range: ContentRange,
    ) -> Self {
        Self {
            source: ReaderSource::OpenedFile(opened_file),
            path: Arc::new(path.into()),
            content_range,
        }
    }

    pub fn into_stream(self) -> impl Stream<Item = Result<Bytes, IoError>> {
        let (tx, rx) = mpsc::channel(self.get_optimal_channel_size());
        let chunk_size = self.get_chunk_size_for_streaming();
        let source = self.source;
        let path = self.path.clone();
        let content_range = self.content_range;

        tokio::task::spawn_blocking(move || {
            let result = match source {
                ReaderSource::Path => Self::read_file_to_channel(
                    &path,
                    content_range,
                    chunk_size,
                    tx,
                ),
                ReaderSource::OpenedFile(opened_file) => {
                    Self::read_opened_file_to_channel(
                        &path,
                        opened_file,
                        content_range,
                        chunk_size,
                        tx,
                    )
                }
            };
            if let Err(e) = result {
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
        Self::read_opened_file_to_channel(
            path,
            file,
            content_range,
            main_chunk,
            tx,
        )
    }

    fn read_opened_file_to_channel(
        path: &PathBuf,
        file: StdFile,
        content_range: ContentRange,
        main_chunk: usize,
        tx: mpsc::Sender<Result<Bytes, IoError>>,
    ) -> Result<(), IoError> {
        const SLOW_FIRST_IO_THRESHOLD_MS: u128 = 500;
        let mut reader = BufReader::with_capacity(main_chunk, file);

        let seek_started = Instant::now();
        reader.seek(SeekFrom::Start(content_range.start))?;
        let seek_ms = seek_started.elapsed().as_millis();

        let mut limited_reader = reader.take(content_range.length());
        let mut buffer = vec![0u8; main_chunk];
        let mut is_first_read = true;
        let mut chunks_sent = 0u64;

        loop {
            // Early exit on client disconnect to avoid wasting I/O
            if tx.is_closed() {
                debug_log!(
                    READ_STREAM_LOGGER_DOMAIN,
                    "Client disconnected, stopping read after {} chunks \
                     for path={:?}",
                    chunks_sent,
                    path
                );
                break;
            }

            let read_cap = if is_first_read {
                const FIRST_READ_CAP: usize = 256 * 1024;
                FIRST_READ_CAP.min(main_chunk).max(1)
            } else {
                main_chunk
            };

            let read_started = if is_first_read {
                Some(Instant::now())
            } else {
                None
            };
            let bytes_read = limited_reader.read(&mut buffer[..read_cap])?;
            if let Some(started) = read_started {
                let first_read_ms = started.elapsed().as_millis();
                let total_first_io_ms = seek_ms + first_read_ms;
                if total_first_io_ms >= SLOW_FIRST_IO_THRESHOLD_MS {
                    debug_log!(
                        READ_STREAM_LOGGER_DOMAIN,
                        "local_first_io_slow seek_ms={} first_read_ms={} total_first_io_ms={} bytes_read={} range_start={} path={:?}",
                        seek_ms,
                        first_read_ms,
                        total_first_io_ms,
                        bytes_read,
                        content_range.start,
                        path
                    );
                } else {
                    debug_log!(
                        READ_STREAM_LOGGER_DOMAIN,
                        "local_first_io_complete seek_ms={} first_read_ms={} total_first_io_ms={} bytes_read={} range_start={} path={:?}",
                        seek_ms,
                        first_read_ms,
                        total_first_io_ms,
                        bytes_read,
                        content_range.start,
                        path
                    );
                }
                is_first_read = false;
            }
            if bytes_read == 0 {
                break;
            }

            if tx
                .blocking_send(Ok(Bytes::copy_from_slice(
                    &buffer[..bytes_read],
                )))
                .is_err()
            {
                debug_log!(
                    READ_STREAM_LOGGER_DOMAIN,
                    "Send failed after {} chunks, client likely disconnected",
                    chunks_sent
                );
                break;
            }

            chunks_sent += 1;
        }

        Ok(())
    }

    #[inline]
    fn get_chunk_size_for_streaming(&self) -> usize {
        disk_main_read_chunk(self.content_range.start)
    }

    #[inline]
    fn get_optimal_channel_size(&self) -> usize {
        const MIN_CHANNEL_SIZE: usize = 4;
        const MAX_CHANNEL_SIZE: usize = 128;
        const DEFAULT_CHANNEL_SIZE: usize = 128;

        let length = self.content_range.length();
        let chunk_size = self.get_chunk_size_for_streaming() as u64;
        (length / chunk_size)
            .try_into()
            .unwrap_or(DEFAULT_CHANNEL_SIZE)
            .clamp(MIN_CHANNEL_SIZE, MAX_CHANNEL_SIZE)
    }
}

/// Larger chunks after initial seek for better throughput on sequential reads.
#[inline]
pub(crate) fn disk_main_read_chunk(range_start: u64) -> usize {
    const KB: usize = 1024;
    const MB: usize = 1024 * KB;
    const CHUNK_SIZE_FROM_START: usize = 2 * MB;
    const CHUNK_SIZE_AFTER_SEEK: usize = 4 * MB;

    if range_start > 0 {
        CHUNK_SIZE_AFTER_SEEK
    } else {
        CHUNK_SIZE_FROM_START
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File as StdFile};

    use futures_util::StreamExt;
    use tempfile::NamedTempFile;

    use super::{ContentRange, ReaderStream, disk_main_read_chunk};

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

    #[tokio::test]
    async fn reader_stream_reads_range_from_opened_file() {
        let temp = NamedTempFile::new().expect("temp file");
        fs::write(temp.path(), b"hello world").expect("write temp file");
        let opened_file = StdFile::open(temp.path()).expect("open temp file");

        let content_range = ContentRange {
            start: 6,
            end: 10,
            total_size: 11,
        };

        let chunks = ReaderStream::from_opened_file(
            temp.path().to_path_buf(),
            opened_file,
            content_range,
        )
        .into_stream()
        .collect::<Vec<_>>()
        .await;

        let bytes = chunks
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .expect("stream chunks")
            .concat();

        assert_eq!(bytes, b"world");
    }
}
