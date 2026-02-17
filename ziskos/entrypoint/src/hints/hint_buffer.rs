use bytes::{Bytes, BytesMut};
use std::io::{self, Write};
use std::sync::{Arc, Condvar, Mutex};

pub const DEFAULT_BUFFER_LEN: usize = 1 << 20; // 1 MiB
                                               // TODO: Set MAX_WRITE_LEN based on writer type (file or socket)
pub const MAX_WRITER_LEN: usize = 128 * 1024; // 128KB is the max write size for Unix sockets
pub const HEADER_LEN: usize = 8;

pub struct HintBuffer {
    inner: Mutex<HintBufferInner>,
    not_empty: Condvar,
}

struct HintBufferInner {
    buf: BytesMut,
    commit_pos: usize,
    closed: bool,
    paused: bool,
    // counter: u64,
}

pub fn build_hint_buffer() -> Arc<HintBuffer> {
    Arc::new(HintBuffer {
        inner: Mutex::new(HintBufferInner {
            buf: BytesMut::with_capacity(DEFAULT_BUFFER_LEN),
            commit_pos: 0,
            closed: true,
            paused: false,
            // counter: 0,
        }),
        not_empty: Condvar::new(),
    })
}

impl HintBufferInner {
    #[inline(always)]
    fn write_bytes(&mut self, src: &[u8]) {
        self.buf.extend_from_slice(src);
    }

    #[inline(always)]
    fn commit(&mut self) {
        self.commit_pos = self.buf.len();
    }
}

impl HintBuffer {
    pub fn close(&self) {
        let mut g = self.inner.lock().unwrap();
        g.closed = true;
        self.not_empty.notify_all();
    }

    pub fn reset(&self) {
        let mut g = self.inner.lock().unwrap();
        g.buf.clear();
        g.commit_pos = 0;
        g.closed = false;
        g.paused = false;
        // g.counter = 0;
        self.not_empty.notify_all();
    }

    #[inline(always)]
    pub fn pause(&self) {
        self.inner.lock().unwrap().paused = true;
    }

    #[inline(always)]
    pub fn resume(&self) {
        self.inner.lock().unwrap().paused = false;
    }

    #[inline(always)]
    pub fn is_paused(&self) -> bool {
        let g = self.inner.lock().unwrap();
        g.paused
    }

    #[inline(always)]
    pub fn is_enabled(&self) -> bool {
        let g = self.inner.lock().unwrap();
        !g.paused && !g.closed
    }

    #[inline(always)]
    pub fn write_hint_header(&self, hint_id: u32, len: usize, is_result: bool) {
        let header = ((((if is_result { 0x8000_0000u64 } else { 0 }) | hint_id as u64) << 32)
            | (len as u64))
            .to_le_bytes();

        let mut g = self.inner.lock().unwrap();

        g.write_bytes(&header);

        // g.counter += 1;

        // if g.counter ==  32672 {
        //     panic!("Hint counter reached");
        // }
    }

    #[inline(always)]
    pub fn write_hint_data(&self, data: *const u8, len: usize) {
        let payload = unsafe { std::slice::from_raw_parts(data, len) };
        self.inner.lock().unwrap().write_bytes(payload);
    }

    #[inline(always)]
    pub fn commit(&self) {
        let mut g = self.inner.lock().unwrap();
        g.commit();
        self.not_empty.notify_one();
    }

    pub fn drain_to_writer<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        loop {
            // Get chunk of hints to write from HintBuffer(under lock)
            let chunk: Bytes = {
                let mut g = self.inner.lock().unwrap();

                while g.commit_pos == 0 && !g.closed {
                    g = self.not_empty.wait(g).unwrap();
                }

                if g.commit_pos == 0 && g.closed {
                    return Ok(());
                }

                let n = g.commit_pos;
                g.commit_pos = 0;
                g.buf.split_to(n).freeze()
            };

            // Write hints from chunk without holding the lock
            let mut chunk_pos = 0usize;
            let chunk_len = chunk.len();
            let chunk_base = chunk.as_ptr();

            // Start and end of the write buffer (MAX_WRITER_LEN)
            let mut buf_start = 0usize;
            let mut buf_end = 0usize;

            while chunk_pos < chunk_len {
                let hint_header = unsafe {
                    let header_bytes = core::slice::from_raw_parts(chunk_base.add(chunk_pos), 8);
                    u64::from_le_bytes(header_bytes.try_into().unwrap())
                };

                #[cfg(zisk_hints_metrics)]
                {
                    let hint_id = (hint_header >> 32) as u32 & 0x7FFF_FFFF;
                    crate::hints::metrics::inc_hint_count(hint_id);
                }

                let hint_data_len = (hint_header & 0xFFFF_FFFF) as usize;
                let pad = (8 - (hint_data_len & 7)) & 7;
                let hint_len = HEADER_LEN + hint_data_len + pad;

                // If adding this hint exceeds max write size, flush current write buffer
                if buf_end - buf_start + hint_len > MAX_WRITER_LEN {
                    let buf: &[u8] = unsafe {
                        core::slice::from_raw_parts(chunk_base.add(buf_start), buf_end - buf_start)
                    };
                    writer.write_all(buf)?;

                    // Reset write buffer
                    buf_start = chunk_pos;
                    buf_end = chunk_pos;
                }

                // If single hint exceeds MAX_WRITER_LEN, write it in chunks
                if hint_len > MAX_WRITER_LEN {
                    let mut hint_pos = 0usize;
                    while hint_pos < hint_len {
                        let chunk_size = std::cmp::min(MAX_WRITER_LEN, hint_len - hint_pos);
                        let hint_bytes: &[u8] = unsafe {
                            core::slice::from_raw_parts(
                                chunk_base.add(chunk_pos + hint_pos),
                                chunk_size,
                            )
                        };

                        writer.write_all(hint_bytes)?;

                        hint_pos += chunk_size;
                    }
                    // Advance to next hint
                    chunk_pos += hint_len;
                    // Reset write buffer
                    buf_start = chunk_pos;
                    buf_end = chunk_pos;
                } else {
                    // Accumulate current hint into write buffer
                    buf_end += hint_len;
                    // Advance to next hint
                    chunk_pos += hint_len;
                }
            }

            // Flush any remaining data in write buffer
            if buf_end > buf_start {
                let buf: &[u8] = unsafe {
                    core::slice::from_raw_parts(chunk_base.add(buf_start), buf_end - buf_start)
                };
                writer.write_all(buf)?;
            }
        }
    }
}
