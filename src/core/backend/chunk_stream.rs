use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Instant
};

use bytes::Bytes;
use futures_util::Stream;
use tokio::io::{AsyncRead, ReadBuf};

use crate::{STREAM_LOGGER_DOMAIN, info_log};

pub struct AdaptiveChunkStream<R: AsyncRead + Unpin> {
    reader: R,
    initial_chunks_count: usize,
    initial_chunk_size: usize,
    standard_chunk_size: usize,
    request_start_time: Instant,
    first_chunk_sent: bool,
}

impl<R: AsyncRead + Unpin> AdaptiveChunkStream<R> {
    #[allow(dead_code)]
    pub fn new(reader: R, request_start_time: Instant) -> Self {
        Self {
            reader,
            initial_chunks_count: 10,
            initial_chunk_size: 16 * 1024,
            standard_chunk_size: 256 * 1024,
            request_start_time,
            first_chunk_sent: false,
        }
    }
}

impl<R: AsyncRead + Unpin> Stream for AdaptiveChunkStream<R> {
    type Item = std::io::Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let chunk_size = if self.initial_chunks_count > 0 {
            self.initial_chunk_size
        } else {
            self.standard_chunk_size
        };

        let mut buf = Vec::with_capacity(chunk_size);
        let mut read_buf = ReadBuf::new(&mut buf);

        match Pin::new(&mut self.reader).poll_read(cx, &mut read_buf) {
            Poll::Ready(Ok(())) => {
                let filled_bytes = read_buf.filled();
                if filled_bytes.is_empty() {
                    Poll::Ready(None)
                } else {
                    if !self.first_chunk_sent {
                        info_log!(
                            STREAM_LOGGER_DOMAIN,
                            "Time to first chunk: {:?}",
                            self.request_start_time.elapsed()
                        );
                        self.first_chunk_sent = true;
                    }

                    if self.initial_chunks_count > 0 {
                        self.initial_chunks_count -= 1;
                    }
                    Poll::Ready(Some(Ok(Bytes::from(filled_bytes.to_vec()))))
                }
            }
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
            Poll::Pending => Poll::Pending,
        }
    }
}