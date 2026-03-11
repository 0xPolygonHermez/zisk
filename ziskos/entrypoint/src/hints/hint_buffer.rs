use bytes::{Bytes, BytesMut};
use std::io::{self, Write};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use zisk_common::{CTRL_END, CTRL_START, HINT_INPUT};

pub const DEFAULT_BUFFER_LEN: usize = 1 << 20; // 1 MiB
                                               // TODO: Set MAX_WRITE_LEN based on writer type (file or socket)
pub const MAX_WRITER_LEN: usize = 128 * 1024; // 128KB is the max write size for Unix sockets
pub const WRITE_BUFFER_FLUSH_LEN: usize = 64 * 1024; // Flush writer buffer once it exceeds 64KB
const MAX_INPUT_DATA_CHUNK: usize = 128 * 1024 - 8; // Max input data chunk size is 128KB minus 8 bytes for the header (length)
pub const HEADER_LEN: usize = 8;

pub struct HintBuffer {
    precompiles: Mutex<HintBufferInner>,
    input_data: Mutex<HintBufferInner>,
    not_empty: Condvar,
    closed: Mutex<bool>,
    paused: Mutex<bool>,
}

struct HintBufferInner {
    buf: BytesMut,
    commit_pos: usize,
}

pub struct WriteBuffer<'a> {
    hb: &'a HintBuffer,
    g: MutexGuard<'a, HintBufferInner>,
}

pub fn build_hint_buffer() -> Arc<HintBuffer> {
    Arc::new(HintBuffer {
        precompiles: Mutex::new(HintBufferInner {
            buf: BytesMut::with_capacity(DEFAULT_BUFFER_LEN),
            commit_pos: 0,
        }),
        input_data: Mutex::new(HintBufferInner {
            buf: BytesMut::with_capacity(DEFAULT_BUFFER_LEN),
            commit_pos: 0,
        }),
        not_empty: Condvar::new(),
        closed: Mutex::new(true),
        paused: Mutex::new(false),
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
        *self.closed.lock().unwrap() = true;
        self.not_empty.notify_all();
    }

    pub fn reset(&self) {
        let mut g = self.precompiles.lock().unwrap();
        g.buf.clear();
        g.commit_pos = 0;
        let mut i = self.input_data.lock().unwrap();
        i.buf.clear();
        i.commit_pos = 0;

        *self.closed.lock().unwrap() = false;
        *self.paused.lock().unwrap() = false;
        self.not_empty.notify_all();
    }

    #[inline(always)]
    pub fn pause(&self) {
        *self.paused.lock().unwrap() = true;
    }

    #[inline(always)]
    pub fn resume(&self) {
        *self.paused.lock().unwrap() = false;
    }

    #[inline(always)]
    pub fn is_paused(&self) -> bool {
        *self.paused.lock().unwrap()
    }

    #[inline(always)]
    pub fn is_enabled(&self) -> bool {
        let paused = *self.paused.lock().unwrap();
        let closed = *self.closed.lock().unwrap();
        !paused && !closed
    }

    #[inline(always)]
    pub fn begin_hint(&self, hint_id: u32, len: usize, is_result: bool) -> WriteBuffer<'_> {
        let header = ((((if is_result { 0x8000_0000u64 } else { 0 }) | hint_id as u64) << 32)
            | (len as u64))
            .to_le_bytes();

        let mut g = self.precompiles.lock().unwrap();
        g.write_bytes(&header);

        WriteBuffer { hb: self, g }
    }

    #[inline(always)]
    pub fn write_hint_start(&self) {
        let w = self.begin_hint(CTRL_START, 0, false);
        w.commit();
    }

    #[inline(always)]
    pub fn write_hint_end(&self) {
        let w = self.begin_hint(CTRL_END, 0, false);
        w.commit();
    }

    #[inline(always)]
    pub fn begin_input_data(&self) -> WriteBuffer<'_> {
        WriteBuffer { hb: self, g: self.input_data.lock().unwrap() }
    }

    pub fn drain_to_writer<W, D>(
        &self,
        writer: &mut W,
        mut debug_writer: Option<&mut D>,
        write_flush_threshold: usize,
    ) -> io::Result<()>
    where
        W: Write + ?Sized,
        D: Write + ?Sized,
    {
        // Write hints from the buffer to the writer and optionally to a debug writer
        let mut write_all = |buf: &[u8]| -> io::Result<()> {
            writer.write_all(buf)?;

            if let Some(debug_writer) = debug_writer.as_deref_mut() {
                debug_writer.write_all(buf)?;
            }

            Ok(())
        };

        fn flush_write_buf<F>(write_all: &mut F, buf: &mut Vec<u8>) -> io::Result<()>
        where
            F: FnMut(&[u8]) -> io::Result<()>,
        {
            if buf.is_empty() {
                return Ok(());
            }

            debug_assert!(buf.len() <= MAX_WRITER_LEN);
            write_all(buf)?;
            buf.clear();

            Ok(())
        }

        let mut flush_threshold = std::cmp::min(write_flush_threshold, MAX_WRITER_LEN);
        flush_threshold = flush_threshold.max(1);

        let mut write_buf = Vec::with_capacity(flush_threshold);
        'drain: loop {
            // Get chunk of hints to write from HintBuffer (under lock)
            let chunk: Bytes = loop {
                let mut g = self.precompiles.lock().unwrap();
                let mut i = self.input_data.lock().unwrap();
                let closed = *self.closed.lock().unwrap();

                if g.commit_pos == 0 && i.commit_pos == 0 && !closed {
                    drop(i); // Release input_data lock before waiting
                    g = self.not_empty.wait(g).unwrap();
                    continue; // Re-acquire both locks in the next iteration
                }

                if g.commit_pos == 0 && i.commit_pos == 0 && closed {
                    break 'drain;
                }

                break if g.commit_pos > 0 {
                    let n = g.commit_pos;
                    g.commit_pos = 0;
                    g.buf.split_to(n).freeze()
                } else {
                    let n = i.commit_pos.min(MAX_INPUT_DATA_CHUNK);
                    i.commit_pos -= n;
                    let input_chunk = i.buf.split_to(n);
                    let header = (((HINT_INPUT as u64) << 32) | n as u64).to_le_bytes();
                    let mut chunk = BytesMut::with_capacity(HEADER_LEN + n);
                    chunk.extend_from_slice(&header);
                    chunk.unsplit(input_chunk);
                    chunk.freeze()
                };
            };

            // Write hints from chunk without holding the lock
            let mut chunk_pos = 0usize;
            let chunk_len = chunk.len();
            let chunk_base = chunk.as_ptr();

            while chunk_pos < chunk_len {
                let hint_header = unsafe {
                    let header_bytes = core::slice::from_raw_parts(chunk_base.add(chunk_pos), 8);
                    u64::from_le_bytes(header_bytes.try_into().unwrap())
                };

                let hint_data_len = (hint_header & 0xFFFF_FFFF) as usize;
                let pad = (8 - (hint_data_len & 7)) & 7;
                let hint_len = HEADER_LEN + hint_data_len + pad;

                #[cfg(zisk_hints_metrics)]
                {
                    use std::hint;

                    let hint_id = (hint_header >> 32) as u32 & 0x7FFF_FFFF;
                    crate::hints::metrics::inc_hint_count(hint_id, hint_len as u64);
                }

                // If single hint exceeds MAX_WRITER_LEN, write it in chunks directly
                if hint_len > MAX_WRITER_LEN {
                    flush_write_buf(&mut write_all, &mut write_buf)?;

                    let mut hint_pos = 0usize;
                    while hint_pos < hint_len {
                        let chunk_size = std::cmp::min(MAX_WRITER_LEN, hint_len - hint_pos);
                        let hint_bytes: &[u8] = unsafe {
                            core::slice::from_raw_parts(
                                chunk_base.add(chunk_pos + hint_pos),
                                chunk_size,
                            )
                        };

                        write_all(hint_bytes)?;

                        hint_pos += chunk_size;
                    }

                    chunk_pos += hint_len;
                    continue;
                }

                let hint_bytes: &[u8] =
                    unsafe { core::slice::from_raw_parts(chunk_base.add(chunk_pos), hint_len) };

                if write_buf.len() + hint_len > MAX_WRITER_LEN {
                    flush_write_buf(&mut write_all, &mut write_buf)?;
                }

                write_buf.extend_from_slice(hint_bytes);

                chunk_pos += hint_len;
            }

            if write_buf.len() >= flush_threshold {
                flush_write_buf(&mut write_all, &mut write_buf)?;
            }
        }

        flush_write_buf(&mut write_all, &mut write_buf)?;

        // Flush the writer and debug writer at the end
        writer.flush()?;
        if let Some(debug_writer) = debug_writer.as_deref_mut() {
            debug_writer.flush()?;
        }

        Ok(())
    }
}

impl<'a> WriteBuffer<'a> {
    #[inline(always)]
    pub fn write_data_ptr(&mut self, data: *const u8, len: usize) {
        if len == 0 {
            return;
        }
        debug_assert!(!data.is_null(), "write_data_ptr called with null data pointer");
        let payload = unsafe { std::slice::from_raw_parts(data, len) };
        self.g.write_bytes(payload);
    }

    #[inline(always)]
    pub fn write_data_slice(&mut self, payload: &[u8]) {
        if payload.is_empty() {
            return;
        }
        self.g.write_bytes(payload);
    }

    #[inline(always)]
    pub fn commit(mut self) {
        self.g.commit();

        drop(self.g);
        self.hb.not_empty.notify_one();
    }
}
