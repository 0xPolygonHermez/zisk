use std::io::{self, Write};
use std::sync::{Arc, Condvar, Mutex};

pub const MAX_HINT_BUFFER_LEN: usize = 1 << 20; // 1 MiB
pub const MAX_HINT_LEN: usize = 128 * 1024; // 128 KiB
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
    len_commit: usize,
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
            len_commit: 0,
            closed: false,
            paused: false,
        }),
        not_empty: Condvar::new(),
    })
}

impl HintBufferInner {
    #[inline(always)]
    fn free_space(&self) -> usize {
        MAX_HINT_BUFFER_LEN - self.len
    }

    #[inline(always)]
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

    #[inline(always)]
    fn read_bytes(&mut self, dst: &mut [u8]) -> usize {
        let hint_header = u64::from_le_bytes(self.buf[self.head..self.head + HEADER_LEN].try_into().unwrap());
        let hint_data_len = (hint_header & 0x_FFFF_FFFF) as usize;
        let pad = (8 - (hint_data_len & 7)) & 7;
        let hint_len = HEADER_LEN + hint_data_len + pad;

        assert!(hint_len <= MAX_HINT_LEN,
            "Hint length too large: max:={} bytes, hint_len={} bytes",
            MAX_HINT_LEN,
            hint_len
        );

        assert!(hint_len <= self.len_commit,
            "Not enough committed data to read hint: committed={} bytes, hint_len={} bytes",
            self.len_commit,
            hint_len
        );

        let mut out = &mut dst[..hint_len];
        while !out.is_empty() {
            let end_space = MAX_HINT_BUFFER_LEN - self.head;
            let chunk = out.len().min(end_space);

            out[..chunk].copy_from_slice(&self.buf[self.head..self.head + chunk]);
            self.head = (self.head + chunk) % MAX_HINT_BUFFER_LEN;
            self.len -= chunk;
            self.len_commit -= chunk;

            out = &mut out[chunk..];
        }

        hint_len
    }

    #[inline(always)]
    fn commit(&mut self) {
        self.len_commit = self.len;
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
        g.len_commit = 0;
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

        assert!(HEADER_LEN <= g.free_space(),
            "Not enough space to write hint header: free={} bytes, trying_to_write={}",
            g.free_space(),
            HEADER_LEN
        );

        #[cfg(zisk_hints_metrics)]
        crate::hints::metrics::inc_hint_count(hint_id);

        g.write_bytes(&header);
        //self.not_empty.notify_one();
    }

    #[inline(always)]
    pub fn write_hint_data(&self, data: *const u8, len: usize) {
        assert!(len <= MAX_HINT_BUFFER_LEN,
            "Hint data too large: {} bytes (max buffer {})", len, MAX_HINT_BUFFER_LEN);

        let mut g = self.inner.lock().unwrap();

        let payload: &[u8] = unsafe { std::slice::from_raw_parts(data, len) };

        assert!(payload.len() <= g.free_space(),
            "Not enough space to write hint data: free={} bytes, trying_to_write={} bytes",
            g.free_space(),
            payload.len()
        );

        g.write_bytes(payload);
        //self.not_empty.notify_one();
    }

    #[inline(always)]
    pub fn commit(&self) {
        let mut g = self.inner.lock().unwrap();
        g.commit();
        self.not_empty.notify_one();
    }

    fn read_blocking(&self, dst: &mut [u8]) -> io::Result<usize> {
        if dst.is_empty() {
            return Ok(0);
        }

        let mut g = self.inner.lock().unwrap();
        while g.len_commit == 0 && !g.closed {
            g = self.not_empty.wait(g).unwrap();
        }

        if g.len_commit == 0 && g.closed {
            return Ok(0);
        }

        Ok(g.read_bytes(dst))
    }

    pub fn drain_to_writer<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let mut buffer = vec![0u8; MAX_HINT_LEN];

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
