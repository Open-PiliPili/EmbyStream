use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Instant,
};

use bytes::{Bytes, BytesMut};
use futures_util::Stream;
use tokio::io::{AsyncRead, ReadBuf};

use crate::{STREAM_LOGGER_DOMAIN, debug_log};

pub struct AdaptiveChunkStream<R: AsyncRead + Unpin> {
    reader: R,
    buf: BytesMut,
    initial_chunks_count: usize,
    initial_chunk_size: usize,
    standard_chunk_size: usize,
    request_start_time: Instant,
    first_chunk_sent: bool,
}

impl<R: AsyncRead + Unpin> AdaptiveChunkStream<R> {
    pub fn new(reader: R, request_start_time: Instant) -> Self {
        let initial_chunk_size = 16 * 1024;
        Self {
            reader,
            buf: BytesMut::with_capacity(initial_chunk_size),
            initial_chunks_count: 8,
            initial_chunk_size,
            standard_chunk_size: 256 * 1024,
            request_start_time,
            first_chunk_sent: false,
        }
    }
}

impl<R: AsyncRead + Unpin> Stream for AdaptiveChunkStream<R> {
    type Item = std::io::Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().get_mut();

        let chunk_size = if this.initial_chunks_count > 0 {
            this.initial_chunk_size
        } else {
            this.standard_chunk_size
        };

        this.buf.reserve(chunk_size);
        let mut read_buf = ReadBuf::uninit(this.buf.spare_capacity_mut());

        match Pin::new(&mut this.reader).poll_read(cx, &mut read_buf) {
            Poll::Ready(Ok(())) => {
                let bytes_filled = read_buf.filled().len();

                if bytes_filled == 0 {
                    return Poll::Ready(None);
                }

                unsafe {
                    this.buf.set_len(this.buf.len() + bytes_filled);
                }

                if !this.first_chunk_sent {
                    debug_log!(
                        STREAM_LOGGER_DOMAIN,
                        "Time to first chunk: {:?}",
                        this.request_start_time.elapsed()
                    );
                    this.first_chunk_sent = true;
                }

                if this.initial_chunks_count > 0 {
                    this.initial_chunks_count -= 1;
                }

                let chunk = this.buf.split_to(bytes_filled).freeze();
                Poll::Ready(Some(Ok(chunk)))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
            Poll::Pending => Poll::Pending,
        }
    }
}