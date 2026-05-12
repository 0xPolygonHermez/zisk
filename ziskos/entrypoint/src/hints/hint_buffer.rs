use bytes::{Bytes, BytesMut};
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use zisk_common::{CTRL_END, CTRL_START, HINT_INPUT};

pub const DEFAULT_BUFFER_LEN: usize = 1 << 20; // 1 MiB
                                               // TODO: Set MAX_WRITE_LEN based on writer type (file or socket)
pub const MAX_WRITER_LEN: usize = 128 * 1024; // 128KB is the max write size for Unix sockets
pub const WRITE_BUFFER_FLUSH_LEN: usize = 64 * 1024; // Flush writer buffer once it exceeds 64KB
const MAX_INPUT_DATA_CHUNK: usize = 128 * 1024 - 8; // Max input data chunk size is 128KB minus 8 bytes for the header (length)
pub const HEADER_LEN: usize = 8;

// Hard cap on bytes pending in HintBuffer + in-flight to the socket. Sized
// 1000x the socket message size: enough headroom for transient consumer
// slowness, while keeping memory bounded. Hitting it means the pipeline can't
// keep up; the stream is poisoned and the job fails (order must be preserved,
// so we never drop a hint mid-stream). Hints larger than this cap bypass
// the check (single oversized hints are allowed through unconditionally).
pub const MAX_PENDING_BYTES: usize = 1000 * MAX_WRITER_LEN;

pub struct HintBuffer {
    precompiles: Mutex<HintBufferInner>,
    input_data: Mutex<HintBufferInner>,
    not_empty: Condvar,
    closed: Mutex<bool>,
    paused: Mutex<bool>,
    pending_bytes: AtomicUsize,
    overflow: AtomicBool,
    max_pending: usize,
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
        pending_bytes: AtomicUsize::new(0),
        overflow: AtomicBool::new(false),
        max_pending: MAX_PENDING_BYTES,
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

        self.pending_bytes.store(0, Ordering::Release);
        self.overflow.store(false, Ordering::Release);

        *self.closed.lock().unwrap() = false;
        *self.paused.lock().unwrap() = false;
        self.not_empty.notify_all();
    }

    /// Reserve `n` bytes against the pending-bytes cap.
    ///
    /// Returns true if the reservation succeeded. Hints larger than the cap
    /// bypass the check and are always admitted (single oversized hints are
    /// allowed through; they temporarily inflate pending beyond `max_pending`).
    ///
    /// Once `overflow` is set, all further reservations fail to preserve
    /// stream ordering — we never drop a hint mid-stream.
    ///
    /// Memory ordering: pending_bytes/overflow are pure counters/flags — no
    /// data payload is published through them (the BytesMut bytes are
    /// published via the precompiles/input_data mutex acquire/release).
    /// `Relaxed` is sufficient on the producer hot path.
    #[inline]
    fn try_reserve(&self, n: usize) -> bool {
        if self.overflow.load(Ordering::Relaxed) {
            return false;
        }

        // Bypass: a hint larger than the cap is allowed unconditionally.
        if n > self.max_pending {
            self.pending_bytes.fetch_add(n, Ordering::Relaxed);
            return true;
        }

        let mut current = self.pending_bytes.load(Ordering::Relaxed);
        loop {
            let next = current + n;
            if next > self.max_pending {
                self.overflow.store(true, Ordering::Relaxed);
                // Wake the drainer so it can observe the poison state and exit
                // promptly if it was waiting on not_empty.
                self.not_empty.notify_all();
                return false;
            }
            match self.pending_bytes.compare_exchange_weak(
                current,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(observed) => current = observed,
            }
        }
    }

    /// Returns true if a previous producer hit the overflow cap. Test-only;
    /// production paths surface overflow via `drain_to_writer`'s `io::Error`.
    #[cfg(test)]
    #[inline]
    pub fn has_overflowed(&self) -> bool {
        self.overflow.load(Ordering::Relaxed)
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

    /// Begin writing a precompile hint. Returns `None` when the pending-bytes
    /// cap is exceeded (poisons the stream — subsequent calls also return
    /// `None`). Callers should silently drop the hint on `None`; the host
    /// surfaces the failure via `drain_to_writer`.
    #[inline(always)]
    pub fn begin_hint(
        &self,
        hint_id: u32,
        len: usize,
        is_result: bool,
    ) -> Option<WriteBuffer<'_>> {
        let pad = (8 - (len & 7)) & 7;
        let hint_total = HEADER_LEN + len + pad;

        if !self.try_reserve(hint_total) {
            return None;
        }

        let header = ((((if is_result { 0x8000_0000u64 } else { 0 }) | hint_id as u64) << 32)
            | (len as u64))
            .to_le_bytes();

        let mut g = self.precompiles.lock().unwrap();
        g.write_bytes(&header);

        Some(WriteBuffer { hb: self, g })
    }

    #[inline(always)]
    pub fn write_hint_start(&self) {
        if let Some(w) = self.begin_hint(CTRL_START, 0, false) {
            w.commit();
        }
    }

    #[inline(always)]
    pub fn write_hint_end(&self) {
        if let Some(w) = self.begin_hint(CTRL_END, 0, false) {
            w.commit();
        }
    }

    /// Begin writing input data. `payload_len` is the size of the caller's
    /// payload; the buffer reserves 8 (inline length prefix) + payload + pad
    /// internally, mirroring `begin_hint`. Returns `None` on overflow.
    #[inline(always)]
    pub fn begin_input_data(&self, payload_len: usize) -> Option<WriteBuffer<'_>> {
        let pad = (8 - (payload_len & 7)) & 7;
        let total = 8 + payload_len + pad;
        if !self.try_reserve(total) {
            return None;
        }
        Some(WriteBuffer { hb: self, g: self.input_data.lock().unwrap() })
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
                // Treat overflow as terminal: no more producer data can land
                // (try_reserve rejects everything), so drain whatever was
                // committed before the poison and then exit. Error is raised
                // after the loop.
                let done = *self.closed.lock().unwrap()
                    || self.overflow.load(Ordering::Relaxed);

                if g.commit_pos == 0 && i.commit_pos == 0 && !done {
                    drop(i); // Release input_data lock before waiting
                    g = self.not_empty.wait(g).unwrap();
                    continue; // Re-acquire both locks in the next iteration
                }

                if g.commit_pos == 0 && i.commit_pos == 0 && done {
                    break 'drain;
                }

                break if g.commit_pos > 0 {
                    let n = g.commit_pos;
                    g.commit_pos = 0;
                    let bytes = g.buf.split_to(n).freeze();
                    self.pending_bytes.fetch_sub(n, Ordering::Relaxed);
                    bytes
                } else {
                    let n = i.commit_pos.min(MAX_INPUT_DATA_CHUNK);
                    i.commit_pos -= n;
                    let input_chunk = i.buf.split_to(n);
                    self.pending_bytes.fetch_sub(n, Ordering::Relaxed);
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

        // Final flush. UnixStream::flush is a no-op; BufWriter<File> (debug)
        // needs it. ENOBUFS retries live inside the writer itself
        // (UnixSocketStreamWriter::write polls POLLOUT).
        writer.flush()?;
        if let Some(debug_writer) = debug_writer.as_deref_mut() {
            debug_writer.flush()?;
        }

        if self.overflow.load(Ordering::Relaxed) {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "hint buffer overflowed: producer outpaced socket drain",
            ));
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a HintBuffer with a small custom cap so overflow tests don't have
    /// to allocate the production-sized 128 MiB buffer.
    fn make_buffer(cap: usize) -> Arc<HintBuffer> {
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
            closed: Mutex::new(false),
            paused: Mutex::new(false),
            pending_bytes: AtomicUsize::new(0),
            overflow: AtomicBool::new(false),
            max_pending: cap,
        })
    }

    /// Size of a precompile hint with `payload_len` bytes of data
    /// (header + payload + padding-to-8).
    fn hint_total(payload_len: usize) -> usize {
        let pad = (8 - (payload_len & 7)) & 7;
        HEADER_LEN + payload_len + pad
    }

    #[test]
    fn try_reserve_increments_pending() {
        let hb = make_buffer(1000);
        assert!(hb.try_reserve(100));
        assert_eq!(hb.pending_bytes.load(Ordering::Acquire), 100);
        assert!(hb.try_reserve(200));
        assert_eq!(hb.pending_bytes.load(Ordering::Acquire), 300);
        assert!(!hb.has_overflowed());
    }

    #[test]
    fn overflow_trips_when_exceeding_cap() {
        let hb = make_buffer(1000);
        assert!(hb.try_reserve(500));
        assert!(hb.try_reserve(500)); // exactly at cap
        assert!(!hb.try_reserve(1)); // one byte over → fail
        assert!(hb.has_overflowed());
    }

    #[test]
    fn overflow_is_sticky() {
        let hb = make_buffer(1000);
        // Trip overflow with a normal-sized (non-bypass) hint.
        assert!(hb.try_reserve(900));
        assert!(!hb.try_reserve(200)); // 900 + 200 > 1000, not bypass-sized
        assert!(hb.has_overflowed());
        // Subsequent reservations — even tiny ones — must all fail.
        assert!(!hb.try_reserve(1));
        assert!(!hb.try_reserve(0));
    }

    #[test]
    fn oversized_hint_bypasses_cap() {
        let hb = make_buffer(1000);
        assert!(hb.try_reserve(500)); // pending = 500
        // 10_000 > cap (1000) → bypass branch, unconditional success.
        assert!(hb.try_reserve(10_000));
        assert!(!hb.has_overflowed());
        assert_eq!(hb.pending_bytes.load(Ordering::Acquire), 10_500);
    }

    #[test]
    fn reset_clears_overflow_and_pending() {
        let hb = make_buffer(1000);
        // Trip a real (non-bypass) overflow: pending+req exceed cap but req ≤ cap.
        assert!(hb.try_reserve(800));
        assert!(!hb.try_reserve(300)); // 800 + 300 > 1000 → real overflow
        assert!(hb.has_overflowed());

        hb.reset();
        assert!(!hb.has_overflowed());
        assert_eq!(hb.pending_bytes.load(Ordering::Acquire), 0);
        assert!(hb.try_reserve(500)); // fresh buffer accepts again
    }

    /// Helper: write a fully-formed hint (header + payload + pad) so the bytes
    /// committed to `buf` match exactly what try_reserve reserved.
    fn push_hint(hb: &HintBuffer, hint_id: u32, payload_len: usize) -> bool {
        let Some(mut w) = hb.begin_hint(hint_id, payload_len, false) else {
            return false;
        };
        let payload = vec![0xABu8; payload_len];
        w.write_data_slice(&payload);
        let pad = (8 - (payload_len & 7)) & 7;
        if pad > 0 {
            let zeros = [0u8; 8];
            w.write_data_slice(&zeros[..pad]);
        }
        w.commit();
        true
    }

    #[test]
    fn begin_hint_returns_none_on_overflow() {
        // Cap chosen so that exactly 4 size-8 hints fit (each = 16 bytes).
        let hb = make_buffer(64);
        for _ in 0..4 {
            assert!(push_hint(&hb, 1, 8), "hint should fit");
        }
        // Pending is now 64 = cap. Next hint would exceed.
        assert!(!push_hint(&hb, 2, 8));
        assert!(hb.has_overflowed());
    }

    #[test]
    fn drain_subtracts_pending_after_write() {
        let hb = make_buffer(1000);
        let w = hb.begin_hint(42, 16, false).expect("hint should fit");
        // header(8) + payload(16) + pad(0) = 24
        assert_eq!(hb.pending_bytes.load(Ordering::Acquire), hint_total(16));
        // Write payload so the committed bytes match what try_reserve reserved.
        let payload = [0xABu8; 16];
        let mut w = w;
        w.write_data_slice(&payload);
        w.commit();
        hb.close();

        let mut sink: Vec<u8> = Vec::new();
        hb.drain_to_writer::<_, Vec<u8>>(&mut sink, None, MAX_WRITER_LEN)
            .expect("drain should succeed");

        assert_eq!(hb.pending_bytes.load(Ordering::Acquire), 0);
        assert_eq!(sink.len(), hint_total(16));
    }

    #[test]
    fn drain_returns_err_when_overflowed() {
        // Overflow alone (without close()) is enough for drain to exit and
        // surface the error — otherwise the drainer would hang on the
        // not_empty condvar when no further commits land. The committed
        // prefix is still drained: receiver sees a valid truncated prefix,
        // never a reordered stream.
        let hb = make_buffer(64);
        for _ in 0..4 {
            assert!(push_hint(&hb, 1, 8));
        }
        assert!(!push_hint(&hb, 2, 8));
        assert!(hb.has_overflowed());

        let mut sink: Vec<u8> = Vec::new();
        let err = hb
            .drain_to_writer::<_, Vec<u8>>(&mut sink, None, MAX_WRITER_LEN)
            .expect_err("drain must report overflow");
        assert!(err.to_string().contains("overflow"), "got: {}", err);
        assert_eq!(sink.len(), 4 * hint_total(8));
    }
}
