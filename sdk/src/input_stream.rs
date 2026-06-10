use anyhow::Result;
use serde::Serialize;
use zisk_common::io::ZiskStreamWriter;
use zisk_coordinator_client::{InputSender, InputSenderPushAdapter};

/// Stream transport for delivering stdin/hints data to a ZisK job.
#[derive(Clone)]
pub struct ZiskStream {
    writer: ZiskStreamWriter,
}

impl ZiskStream {
    // ── Constructors ────────────────────────────────────────────────────

    /// Unix domain socket with an auto-assigned path under `/tmp/`.
    #[cfg(unix)]
    pub fn unix() -> Self {
        let path = format!("/tmp/zisk-input-{}.sock", uuid::Uuid::new_v4());
        Self {
            writer: ZiskStreamWriter::unix_at(&path)
                .expect("failed to create UnixSocketStreamWriter"),
        }
    }

    /// Unix domain socket at an explicit path, managed externally.
    ///
    /// Use this when the socket is already bound and listening by another
    /// process (e.g. `init_hints_socket`).
    #[cfg(unix)]
    pub fn unix_external(path: &str) -> Self {
        let uri = format!("unix://{}", path);
        Self { writer: ZiskStreamWriter::unix_external(uri) }
    }

    /// Unix domain socket at an explicit path.
    ///
    /// The socket starts listening immediately so the executor can connect
    /// as soon as it launches.
    #[cfg(unix)]
    pub fn unix_at(path: &str) -> Result<Self> {
        Ok(Self { writer: ZiskStreamWriter::unix_at(path)? })
    }

    /// QUIC transport.
    ///
    /// Pass `"quic://127.0.0.1:0"` to let the OS pick a free port; the
    /// resolved address is then used as the URI so the coordinator receives
    /// the correct port.
    pub fn quic(uri: &str) -> Result<Self> {
        let addr_str = uri
            .strip_prefix("quic://")
            .ok_or_else(|| anyhow::anyhow!("QUIC URI must start with quic://"))?;
        let addr: std::net::SocketAddr = addr_str
            .parse()
            .map_err(|e| anyhow::anyhow!("invalid QUIC address '{}': {}", addr_str, e))?;
        Ok(Self { writer: ZiskStreamWriter::quic(addr)? })
    }

    /// gRPC push transport (data pushed to coordinator via `PushJobInput`).
    pub fn grpc() -> Self {
        Self { writer: ZiskStreamWriter::push("grpc://push".to_string()) }
    }

    // ── Write / flush ───────────────────────────────────────────────────

    /// Buffer a serializable value as one input record.
    ///
    /// Each call produces one length-prefixed record on the wire, recoverable
    /// by `ziskos::read::<T>()`.
    pub fn write<T: Serialize>(&self, data: &T) {
        let bytes = bincode::serde::encode_to_vec(data, bincode::config::standard())
            .expect("Failed to serialize");
        self.write_slice(&bytes);
    }

    /// Buffer raw bytes as one input record (length-prefixed, paired with
    /// `ziskos::read_slice()`).
    pub fn write_slice(&self, data: &[u8]) {
        let frame = build_frame(data);
        self.writer.push_raw(&frame);
    }

    /// Buffer raw bytes with **no record framing**.
    pub fn write_bytes(&self, data: &[u8]) {
        self.writer.push_raw(data);
    }

    /// Send all buffered bytes now. Blocks until the stream is live.
    pub fn flush(&self) -> Result<()> {
        self.writer.flush()
    }

    /// Discard all buffered (unsent) bytes.
    pub fn reset(&self) {
        self.writer.reset()
    }

    // ── Accessors ───────────────────────────────────────────────────────

    /// The transport URI (e.g. `"unix:///tmp/zisk-input-<id>.sock"`).
    pub fn uri(&self) -> &str {
        self.writer.uri()
    }

    /// Whether this stream uses gRPC push transport.
    pub(crate) fn is_grpc(&self) -> bool {
        self.writer.is_push()
    }

    // ── SDK-internal lifecycle ───────────────────────────────────────────

    /// Prepare the transport and start waiting for a peer connection.
    ///
    /// For unix/quic: opens the socket synchronously (bind+listen) so the
    /// path is connectable immediately, then spawns a background thread
    /// that blocks on the peer connection, drains buffered bytes, and marks
    /// the stream live.
    ///
    /// For gRPC: closes the previous sender (if any) and waits for its
    /// PushJobInput RPC to finish. The new sender arrives via
    /// [`set_input_sender()`](Self::set_input_sender) after `submit_job()`.
    ///
    /// Safe to call multiple times (reuse across jobs): each call tears down
    /// the previous connection first.
    pub(crate) fn start(&self) -> Result<()> {
        self.writer.start()
    }

    /// Inject the gRPC sender obtained after job submission. Marks the stream
    /// live and wakes any flushers blocked waiting for it.
    pub(crate) fn set_input_sender(&self, sender: InputSender) {
        let adapter = InputSenderPushAdapter::new(sender);
        self.writer.set_push_sender(Box::new(adapter));
    }

    /// Close the current transport and mark the stream as not-live.
    ///
    /// For reusable streams, this is called automatically when a `JobHandle`
    /// is awaited. You only need to call it manually if you are not awaiting
    /// a handle (e.g. error paths).
    ///
    /// Any `flush()` called after `finish()` will block until the next
    /// `start()` re-opens the transport — preventing accidental writes to a
    /// completed job.
    pub fn finish(&self) -> Result<()> {
        self.writer.finish()
    }

    /// Async version of [`finish`](Self::finish). Called automatically by
    /// `JobHandle` when it is awaited.
    pub(crate) async fn finish_async(&self) -> Result<()> {
        let writer = self.writer.clone();
        tokio::task::spawn_blocking(move || writer.finish())
            .await
            .map_err(|e| anyhow::anyhow!("finish_async task panicked: {}", e))?
    }
}

// ── Frame encoding ──────────────────────────────────────────────────────

/// Produce one input record: 8-byte little-endian length + payload + zero
/// padding to an 8-byte boundary. Inverse of `ziskos::read_input()`.
fn build_frame(data: &[u8]) -> Vec<u8> {
    let data_len = data.len();
    let total_len = 8 + data_len;
    let padding = (8 - (total_len % 8)) % 8;
    let mut frame = Vec::with_capacity(total_len + padding);
    frame.extend_from_slice(&data_len.to_le_bytes());
    frame.extend_from_slice(data);
    if padding > 0 {
        frame.resize(frame.len() + padding, 0);
    }
    frame
}

/// Decode one frame produced by [`build_frame`]. Returns the payload bytes.
#[cfg(test)]
fn decode_frame(frame: &[u8]) -> Vec<u8> {
    assert!(frame.len() >= 8, "frame too short for length header");
    let len = usize::from_le_bytes(frame[..8].try_into().unwrap());
    frame[8..8 + len].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;
    use zisk_common::io::BytesPushSender;

    // ── Test helpers ────────────────────────────────────────────────────

    fn run_with_timeout<F: FnOnce() + Send + 'static>(name: &str, timeout: Duration, f: F) {
        let handle = thread::Builder::new().name(name.into()).spawn(f).unwrap();
        let deadline = std::time::Instant::now() + timeout;
        loop {
            if handle.is_finished() {
                handle.join().unwrap();
                return;
            }
            if std::time::Instant::now() > deadline {
                panic!("test '{name}' timed out after {timeout:?}");
            }
            thread::sleep(Duration::from_millis(50));
        }
    }

    const TEST_TIMEOUT: Duration = Duration::from_secs(30);

    /// Serialize tests that hit Unix-domain or QUIC sockets. Heavy default
    /// parallelism (`cargo test` uses ~num_cpus threads) causes
    /// OS-level accept-thread contention on socket teardown/rebind paths,
    /// producing flaky failures. Tests acquire this guard at entry.
    fn socket_test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        LOCK.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// A no-op push sender used to mark a gRPC stream live without a real RPC.
    struct NoopPushSender;
    impl BytesPushSender for NoopPushSender {
        fn send_blocking(&self, _data: Vec<u8>) -> Result<()> {
            Ok(())
        }
        fn close_blocking(self: Box<Self>) -> Result<()> {
            Ok(())
        }
    }

    /// A push sender that records every chunk it receives.
    struct RecordingSender(Arc<Mutex<Vec<Vec<u8>>>>);
    impl BytesPushSender for RecordingSender {
        fn send_blocking(&self, data: Vec<u8>) -> Result<()> {
            self.0.lock().unwrap().push(data);
            Ok(())
        }
        fn close_blocking(self: Box<Self>) -> Result<()> {
            Ok(())
        }
    }

    fn force_grpc_ready(stream: &ZiskStream) {
        stream.writer.set_push_sender(Box::new(NoopPushSender));
    }

    // ── Frame encoding tests ────────────────────────────────────────────

    #[test]
    fn frame_roundtrip_and_alignment() {
        let frame = build_frame(b"");
        assert_eq!(frame.len(), 8);
        assert_eq!(decode_frame(&frame), b"");

        let frame = build_frame(b"x");
        assert_eq!(frame.len(), 16); // 8 + 1 + 7 padding
        assert_eq!(decode_frame(&frame), b"x");

        let frame = build_frame(b"hello");
        assert_eq!(frame.len(), 16);
        assert_eq!(decode_frame(&frame), b"hello");

        let frame = build_frame(b"12345678");
        assert_eq!(frame.len(), 16);
        assert_eq!(decode_frame(&frame), b"12345678");

        let data = b"round-trip test data!";
        assert_eq!(decode_frame(&build_frame(data)), data.as_slice());
    }

    // ── Façade behavior unit tests ──────────────────────────────────────

    #[test]
    fn reset_clears_buffered_records() {
        let stream = ZiskStream::grpc();
        stream.write_slice(b"raw bytes");
        stream.write(&42u32);

        // Before reset: pending is non-empty. Mark ready and verify flush
        // would deliver something. (We don't actually send: we reset first.)
        stream.reset();

        // After reset: pending is empty; flush is a no-op once ready.
        force_grpc_ready(&stream);
        assert!(stream.flush().is_ok());
    }

    #[test]
    fn flush_empty_is_noop_when_live() {
        let stream = ZiskStream::grpc();
        force_grpc_ready(&stream);
        assert!(stream.flush().is_ok());
    }

    #[test]
    fn quic_rejects_bad_uri() {
        assert!(ZiskStream::quic("http://localhost:9000").is_err());
    }

    #[test]
    fn write_unframed_adds_no_length_prefix() {
        let stream = ZiskStream::grpc();
        let recorded: Arc<Mutex<Vec<Vec<u8>>>> = Arc::new(Mutex::new(Vec::new()));
        stream.writer.set_push_sender(Box::new(RecordingSender(Arc::clone(&recorded))));

        // write_slice → length-prefixed
        stream.write_slice(b"abcdefgh"); // 8 bytes payload → 16-byte frame
                                         // write_unframed → raw passthrough
        stream.write_bytes(b"01234567"); // 8 bytes literal

        stream.flush().unwrap();
        let chunks = recorded.lock().unwrap();
        let received: Vec<u8> = chunks.iter().flatten().copied().collect();

        // Layout: [8-byte len=8][abcdefgh][01234567]
        assert_eq!(received.len(), 8 + 8 + 8);
        assert_eq!(usize::from_le_bytes(received[..8].try_into().unwrap()), 8);
        assert_eq!(&received[8..16], b"abcdefgh");
        assert_eq!(&received[16..24], b"01234567");
    }

    // ── Unix socket integration tests ───────────────────────────────────

    #[cfg(unix)]
    mod unix_tests {
        use super::*;
        use zisk_common::io::{StreamRead, UnixSocketStreamReader};

        #[test]
        fn unix_write_before_start_then_flush() {
            run_with_timeout("unix_write_before_start_then_flush", TEST_TIMEOUT, || {
                let _g = socket_test_lock();
                let stream = ZiskStream::unix();
                let uri = stream.uri().to_string();
                let path = uri.strip_prefix("unix://").unwrap().to_string();

                stream.write(&42u32);
                stream.write(&99u32);

                stream.start().unwrap();

                let mut reader = UnixSocketStreamReader::new(&path).unwrap();
                // Both pre-buffered records were drained on connect — they
                // arrive concatenated in one wire message (under SOCK_SEQPACKET
                // limit).
                let msg = reader.next().unwrap().unwrap();
                let expected1 =
                    bincode::serde::encode_to_vec(42u32, bincode::config::standard()).unwrap();
                let expected2 =
                    bincode::serde::encode_to_vec(99u32, bincode::config::standard()).unwrap();
                let f1 = build_frame(&expected1);
                let f2 = build_frame(&expected2);
                let mut concat = f1;
                concat.extend_from_slice(&f2);
                assert_eq!(msg, concat);

                stream.finish().unwrap();
                reader.close().unwrap();
            });
        }

        #[test]
        fn unix_flush_after_live() {
            run_with_timeout("unix_flush_after_live", TEST_TIMEOUT, || {
                let _g = socket_test_lock();
                let stream = ZiskStream::unix();
                let path = stream.uri().strip_prefix("unix://").unwrap().to_string();

                stream.start().unwrap();
                let mut reader = UnixSocketStreamReader::new(&path).unwrap();
                reader.open().unwrap();

                // Spin until ready; outer TEST_TIMEOUT catches genuine hangs.
                while !stream.writer.is_ready() {
                    thread::sleep(Duration::from_millis(10));
                }

                stream.write_slice(b"post-live data");
                stream.flush().unwrap();

                let msg = reader.next().unwrap().unwrap();
                assert_eq!(decode_frame(&msg), b"post-live data");

                stream.finish().unwrap();
                reader.close().unwrap();
            });
        }

        #[test]
        fn unix_multiple_flush_cycles() {
            run_with_timeout("unix_multiple_flush_cycles", TEST_TIMEOUT, || {
                let _g = socket_test_lock();
                let stream = ZiskStream::unix();
                let path = stream.uri().strip_prefix("unix://").unwrap().to_string();

                stream.start().unwrap();
                let mut reader = UnixSocketStreamReader::new(&path).unwrap();
                reader.open().unwrap();

                // Spin until ready; outer TEST_TIMEOUT catches genuine hangs.
                while !stream.writer.is_ready() {
                    thread::sleep(Duration::from_millis(10));
                }

                // Flush #1: two records, sent in one wire message (coalesced).
                stream.write_slice(b"batch-1a");
                stream.write_slice(b"batch-1b");
                stream.flush().unwrap();

                // Flush #2: one record, one wire message.
                stream.write_slice(b"batch-2");
                stream.flush().unwrap();

                let m1 = reader.next().unwrap().unwrap();
                let m2 = reader.next().unwrap().unwrap();

                // m1 contains both records back-to-back. Decode in sequence.
                assert_eq!(decode_frame(&m1[..16]), b"batch-1a");
                assert_eq!(decode_frame(&m1[16..32]), b"batch-1b");
                assert_eq!(decode_frame(&m2), b"batch-2");

                stream.finish().unwrap();
                reader.close().unwrap();
            });
        }

        #[test]
        fn unix_start_reuse_across_jobs() {
            run_with_timeout("unix_start_reuse_across_jobs", TEST_TIMEOUT, || {
                let _g = socket_test_lock();
                let stream = ZiskStream::unix();
                let path = stream.uri().strip_prefix("unix://").unwrap().to_string();

                // === Job 1 ===
                stream.write(&1u32);
                stream.start().unwrap();
                let mut reader1 = UnixSocketStreamReader::new(&path).unwrap();
                let msg = reader1.next().unwrap().unwrap();
                assert_eq!(
                    decode_frame(&msg),
                    bincode::serde::encode_to_vec(1u32, bincode::config::standard()).unwrap()
                );
                stream.finish().unwrap();
                reader1.close().unwrap();

                // === Job 2 (reuse same stream) ===
                stream.write(&2u32);
                stream.start().unwrap();
                let mut reader2 = UnixSocketStreamReader::new(&path).unwrap();
                let msg = reader2.next().unwrap().unwrap();
                assert_eq!(
                    decode_frame(&msg),
                    bincode::serde::encode_to_vec(2u32, bincode::config::standard()).unwrap()
                );
                stream.finish().unwrap();
                reader2.close().unwrap();
            });
        }

        #[test]
        fn unix_finish_makes_stream_not_ready() {
            run_with_timeout("unix_finish_makes_stream_not_ready", TEST_TIMEOUT, || {
                let _g = socket_test_lock();
                let stream = ZiskStream::unix();
                let path = stream.uri().strip_prefix("unix://").unwrap().to_string();

                stream.start().unwrap();
                let mut reader = UnixSocketStreamReader::new(&path).unwrap();
                reader.open().unwrap();

                // Spin until ready; outer TEST_TIMEOUT catches genuine hangs.
                while !stream.writer.is_ready() {
                    thread::sleep(Duration::from_millis(10));
                }

                stream.finish().unwrap();
                assert!(!stream.writer.is_ready());

                reader.close().unwrap();
            });
        }

        #[test]
        fn unix_flush_blocks_until_live() {
            run_with_timeout("unix_flush_blocks_until_live", TEST_TIMEOUT, || {
                let _g = socket_test_lock();
                let stream = ZiskStream::unix();
                let path = stream.uri().strip_prefix("unix://").unwrap().to_string();

                stream.start().unwrap();

                stream.write_slice(b"blocked data");
                let stream_clone = stream.clone();
                let flushed = Arc::new(AtomicBool::new(false));
                let flushed_clone = flushed.clone();
                let flush_thread = thread::spawn(move || {
                    stream_clone.flush().unwrap();
                    flushed_clone.store(true, Ordering::Release);
                });

                thread::sleep(Duration::from_millis(100));
                assert!(!flushed.load(Ordering::Acquire), "flush should still be blocking");

                let mut reader = UnixSocketStreamReader::new(&path).unwrap();
                reader.open().unwrap();

                flush_thread.join().unwrap();
                assert!(flushed.load(Ordering::Acquire));

                let msg = reader.next().unwrap().unwrap();
                assert_eq!(decode_frame(&msg), b"blocked data");

                stream.finish().unwrap();
                reader.close().unwrap();
            });
        }

        #[test]
        fn unix_large_payload() {
            run_with_timeout("unix_large_payload", TEST_TIMEOUT, || {
                let _g = socket_test_lock();
                let stream = ZiskStream::unix();
                let path = stream.uri().strip_prefix("unix://").unwrap().to_string();

                let large_data = vec![0xABu8; 64 * 1024]; // 64 KB
                stream.write_slice(&large_data);
                stream.start().unwrap();

                let mut reader = UnixSocketStreamReader::new(&path).unwrap();

                // Frame is 64 KB + 8 bytes header (already 8-aligned, no pad).
                // SOCK_SEQPACKET caps at 128 KB so this fits in one chunk.
                let msg = reader.next().unwrap().unwrap();
                assert_eq!(decode_frame(&msg), large_data);

                stream.finish().unwrap();
                reader.close().unwrap();
            });
        }
    }

    // ── QUIC integration tests ──────────────────────────────────────────

    mod quic_tests {
        use super::*;
        use zisk_common::io::{QuicStreamReader, StreamRead};

        fn free_port() -> u16 {
            std::net::UdpSocket::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
        }

        #[test]
        fn quic_write_before_start_then_read() {
            run_with_timeout("quic_write_before_start_then_read", TEST_TIMEOUT, || {
                let _g = socket_test_lock();
                let port = free_port();
                let uri = format!("quic://127.0.0.1:{port}");
                let stream = ZiskStream::quic(&uri).unwrap();

                stream.write(&42u32);
                stream.start().unwrap();

                let mut reader =
                    QuicStreamReader::new(format!("127.0.0.1:{port}").parse().unwrap()).unwrap();
                reader.open().unwrap();

                let msg = reader.next().unwrap().unwrap();
                let expected =
                    bincode::serde::encode_to_vec(42u32, bincode::config::standard()).unwrap();
                assert_eq!(decode_frame(&msg), expected);

                stream.finish().unwrap();
                reader.close().unwrap();
            });
        }

        #[test]
        fn quic_flush_after_live() {
            run_with_timeout("quic_flush_after_live", TEST_TIMEOUT, || {
                let _g = socket_test_lock();
                let port = free_port();
                let uri = format!("quic://127.0.0.1:{port}");
                let stream = ZiskStream::quic(&uri).unwrap();

                stream.start().unwrap();
                let mut reader =
                    QuicStreamReader::new(format!("127.0.0.1:{port}").parse().unwrap()).unwrap();
                reader.open().unwrap();

                // Spin until ready; outer TEST_TIMEOUT catches genuine hangs.
                while !stream.writer.is_ready() {
                    thread::sleep(Duration::from_millis(10));
                }

                stream.write_slice(b"quic-post-live");
                stream.flush().unwrap();

                let msg = reader.next().unwrap().unwrap();
                assert_eq!(decode_frame(&msg), b"quic-post-live");

                stream.finish().unwrap();
                reader.close().unwrap();
            });
        }
    }

    // ── gRPC unit tests (no real server) ────────────────────────────────

    #[test]
    fn grpc_reset_then_flush_sends_nothing() {
        let stream = ZiskStream::grpc();
        stream.write(&1u32);
        stream.write(&2u32);
        stream.reset();

        let recorded: Arc<Mutex<Vec<Vec<u8>>>> = Arc::new(Mutex::new(Vec::new()));
        stream.writer.set_push_sender(Box::new(RecordingSender(Arc::clone(&recorded))));

        stream.flush().unwrap();
        assert!(recorded.lock().unwrap().is_empty(), "reset should drop all pending bytes");
    }

    // ── Lifecycle edge-case tests ───────────────────────────────────────

    #[cfg(unix)]
    #[test]
    fn unix_at_creates_stream_at_explicit_path() {
        let path = format!("/tmp/zisk-test-at-{}.sock", uuid::Uuid::new_v4());
        let stream = ZiskStream::unix_at(&path).unwrap();
        assert_eq!(stream.uri(), format!("unix://{path}"));
        assert!(!stream.is_grpc());
    }

    #[test]
    fn finish_without_start_is_ok() {
        let stream = ZiskStream::grpc();
        assert!(stream.finish().is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn finish_without_start_direct() {
        let stream = ZiskStream::unix();
        assert!(stream.finish().is_ok());
    }

    #[test]
    fn finish_twice_is_idempotent() {
        let stream = ZiskStream::grpc();
        force_grpc_ready(&stream);
        assert!(stream.finish().is_ok());
        assert!(stream.finish().is_ok());
        assert!(!stream.writer.is_ready());
    }

    #[cfg(unix)]
    #[test]
    fn start_while_already_live_tears_down_and_reopens() {
        run_with_timeout("start_while_already_live", TEST_TIMEOUT, || {
            let _g = socket_test_lock();
            use zisk_common::io::{StreamRead, UnixSocketStreamReader};

            let stream = ZiskStream::unix();
            let path = stream.uri().strip_prefix("unix://").unwrap().to_string();

            stream.write_slice(b"job1");
            stream.start().unwrap();
            let mut reader1 = UnixSocketStreamReader::new(&path).unwrap();
            let msg = reader1.next().unwrap().unwrap();
            assert_eq!(decode_frame(&msg), b"job1");

            // Call start() again WITHOUT calling finish() first.
            stream.write_slice(b"job2");
            stream.start().unwrap();

            let mut reader2 = UnixSocketStreamReader::new(&path).unwrap();
            reader2.open().unwrap();

            let msg = reader2.next().unwrap().unwrap();
            assert_eq!(decode_frame(&msg), b"job2");

            stream.finish().unwrap();
            reader2.close().unwrap();
        });
    }

    #[test]
    fn concurrent_writes_from_clones() {
        let stream = ZiskStream::grpc();
        let handles: Vec<_> = (0..8)
            .map(|i| {
                let s = stream.clone();
                thread::spawn(move || {
                    for j in 0..100u32 {
                        s.write(&(i * 1000 + j));
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        // 800 records × 16 bytes/frame (4-byte u32 → 12 bytes after padding +
        // 8-byte header). Verify total pending byte count.
        let recorded: Arc<Mutex<Vec<Vec<u8>>>> = Arc::new(Mutex::new(Vec::new()));
        stream.writer.set_push_sender(Box::new(RecordingSender(Arc::clone(&recorded))));
        stream.flush().unwrap();

        let total_bytes: usize = recorded.lock().unwrap().iter().map(|c| c.len()).sum();
        assert_eq!(total_bytes, 800 * 16, "no lost writes — 800 frames × 16 bytes each");
    }
}
