use std::io::{self, Write};
use std::sync::{Arc, Condvar, Mutex};

pub const MAX_HINT_BUFFER_LEN: usize = 1 << 20; // 1 MiB
pub const HEADER_LEN: usize = 8;

pub struct HintBuffer {
    inner: Mutex<HintBufferInner>,
    not_empty: Condvar,
}

struct HintBufferInner {
    buf: [u8; MAX_HINT_BUFFER_LEN],
    head: usize,
    tail: usize,
    len: usize,
    closed: bool,
    paused: bool,
}

pub fn build_hint_buffer() -> Arc<HintBuffer> {
    Arc::new(HintBuffer {
        inner: Mutex::new(HintBufferInner {
            buf: [0u8; MAX_HINT_BUFFER_LEN],
            head: 0,
            tail: 0,
            len: 0,
            closed: false,
            paused: false,
        }),
        not_empty: Condvar::new(),
    })
}

impl HintBufferInner {
    #[inline]
    fn free_space(&self) -> usize {
        MAX_HINT_BUFFER_LEN - self.len
    }

    #[inline]
    fn write_bytes(&mut self, src: &[u8]) {
        let mut remaining = src;
        while !remaining.is_empty() {
            let end_space = MAX_HINT_BUFFER_LEN - self.tail;
            let chunk = remaining.len().min(end_space);

            self.buf[self.tail..self.tail + chunk].copy_from_slice(&remaining[..chunk]);
            self.tail = (self.tail + chunk) % MAX_HINT_BUFFER_LEN;
            self.len += chunk;

            remaining = &remaining[chunk..];
        }
    }

    #[inline]
    fn read_bytes(&mut self, dst: &mut [u8]) -> usize {
        let to_read = dst.len().min(self.len);
        if to_read == 0 {
            return 0;
        }

        let mut out = &mut dst[..to_read];
        while !out.is_empty() {
            let end_space = MAX_HINT_BUFFER_LEN - self.head;
            let chunk = out.len().min(end_space);

            out[..chunk].copy_from_slice(&self.buf[self.head..self.head + chunk]);
            self.head = (self.head + chunk) % MAX_HINT_BUFFER_LEN;
            self.len -= chunk;

            out = &mut out[chunk..];
        }

        to_read
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
        g.head = 0;
        g.tail = 0;
        g.len = 0;
        g.closed = false;
        g.paused = false;

        self.not_empty.notify_all();
    }

    #[inline(always)]
    pub fn pause(&self) {
        let mut g = self.inner.lock().unwrap();
        g.paused = true;
    }

    #[inline(always)]
    pub fn resume(&self) {
        let mut g = self.inner.lock().unwrap();
        g.paused = false;
    }

    #[inline(always)]
    pub fn is_closed(&self) -> bool {
        let g = self.inner.lock().unwrap();
        g.closed
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

        if HEADER_LEN > g.free_space() {
            panic!(
                "Hint buffer overflow: capacity={} used={} free={} trying_to_write={}",
                MAX_HINT_BUFFER_LEN,
                g.len,
                g.free_space(),
                HEADER_LEN
            );
        }

        #[cfg(zisk_hints_metrics)]
        crate::hints::metrics::inc_hint_count(hint_id);

        g.write_bytes(&header);
        self.not_empty.notify_one();
    }

    #[inline(always)]
    pub fn write_hint_data(&self, data: *const u8, len: usize) {
        if len > MAX_HINT_BUFFER_LEN {
            panic!("Hint data too large: {} bytes (max buffer {})", len, MAX_HINT_BUFFER_LEN);
        }

        let mut g = self.inner.lock().unwrap();

        let payload: &[u8] = unsafe { std::slice::from_raw_parts(data, len) };

        if payload.len() > g.free_space() {
            panic!(
                "Hint buffer overflow: capacity={} used={} free={} trying_to_write={}",
                MAX_HINT_BUFFER_LEN,
                g.len,
                g.free_space(),
                payload.len()
            );
        }

        g.write_bytes(payload);
        self.not_empty.notify_one();
    }

    fn read_blocking(&self, dst: &mut [u8]) -> io::Result<usize> {
        if dst.is_empty() {
            return Ok(0);
        }

        let mut g = self.inner.lock().unwrap();
        while g.len == 0 && !g.closed {
            g = self.not_empty.wait(g).unwrap();
        }

        if g.len == 0 && g.closed {
            return Ok(0);
        }

        Ok(g.read_bytes(dst))
    }

    pub fn drain_to_writer<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let mut buffer = vec![0u8; 64 * 1024];

        loop {
            let n = self.read_blocking(&mut buffer)?;
            if n == 0 {
                break; // closed and empty
            }

            writer.write_all(&buffer[..n])?;
        }

        Ok(())
    }
}
