use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Instant,
};

use bytes::{Bytes, BytesMut};
use futures_util::Stream;
use tokio::io::{AsyncRead, ReadBuf};

use crate::{STREAM_LOGGER_DOMAIN, debug_log};

const KB: usize = 1024;
const MB: u64 = 1_000_000;
const CHUNK_128KB: usize = 128 * KB;
const CHUNK_256KB: usize = 256 * KB;
const CHUNK_512KB: usize = 512 * KB;

const FILE_SIZE_300MB: u64 = 300 * MB;
const FILE_SIZE_500MB: u64 = 500 * MB;
const FILE_SIZE_1000MB: u64 = 1_000 * MB;
const FILE_SIZE_4000MB: u64 = 4_000 * MB;

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
    pub fn new(reader: R, file_length: u64, request_start_time: Instant) -> Self {
        let (initial_chunk_size, initial_chunks_count, standard_chunk_size) =
            Self::get_chunk_sizes(file_length);

        Self {
            reader,
            buf: BytesMut::with_capacity(initial_chunk_size),
            initial_chunks_count,
            initial_chunk_size,
            standard_chunk_size,
            request_start_time,
            first_chunk_sent: false,
        }
    }

    fn get_chunk_sizes(file_length: u64) -> (usize, usize, usize) {
        match file_length {
            // 0 ..< 300MB
            0..FILE_SIZE_300MB => (CHUNK_128KB, 1, CHUNK_256KB),
            // 300 ..< 500MB
            FILE_SIZE_300MB..FILE_SIZE_500MB => (CHUNK_256KB, 1, CHUNK_256KB),
            // 500 ..< 1000MB
            FILE_SIZE_500MB..FILE_SIZE_1000MB => (CHUNK_256KB, 2, CHUNK_512KB),
            // 1000 ..< 4000MB
            FILE_SIZE_1000MB..FILE_SIZE_4000MB => (CHUNK_512KB, 1, CHUNK_512KB),
            // 4000MB and above
            _ => (CHUNK_512KB, 2, CHUNK_512KB),
        }
    }
}

impl<R: AsyncRead + Unpin> Stream for AdaptiveChunkStream<R> {
    type Item = std::io::Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = &mut *self;

        if !this.buf.is_empty() {
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

            let chunk = this.buf.split().freeze();
            return Poll::Ready(Some(Ok(chunk)));
        }

        let chunk_size = if this.initial_chunks_count > 0 {
            this.initial_chunk_size
        } else {
            this.standard_chunk_size
        };

        this.buf.reserve(chunk_size);

        let original_len = this.buf.len();
        let mut read_buf = ReadBuf::uninit(this.buf.spare_capacity_mut());

        match Pin::new(&mut this.reader).poll_read(cx, &mut read_buf) {
            Poll::Ready(Ok(())) => {
                let bytes_filled = read_buf.filled().len();

                if bytes_filled == 0 {
                    return Poll::Ready(None);
                }

                unsafe {
                    this.buf.set_len(original_len + bytes_filled);
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

                let chunk = this.buf.split().freeze();
                Poll::Ready(Some(Ok(chunk)))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
            Poll::Pending => Poll::Pending,
        }
    }
}
