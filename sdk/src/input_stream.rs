use std::sync::{Arc, Condvar, Mutex};

use anyhow::Result;
use bytes::Bytes;
use serde::Serialize;
use zisk_common::io::{QuicStreamWriter, StreamWrite};
use zisk_coordinator_client::InputSender;

#[cfg(unix)]
use zisk_common::io::UnixSocketStreamWriter;

// ── Transport ───────────────────────────────────────────────────────────

enum TransportKind {
    /// Unix socket / QUIC — writer owned here, opened by `start()`.
    Direct(Box<dyn StreamWrite>),
    /// gRPC push — sender injected by `set_input_sender()`.
    Grpc,
}

// ── Live state ──────────────────────────────────────────────────────────

/// Active gRPC sender for the current job.  Set by [`set_input_sender()`],
/// cleared (and awaited to completion) by [`start()`] and [`finish()`].
struct GrpcState {
    sender: InputSender,
    rt: tokio::runtime::Handle,
}

impl GrpcState {
    fn close_blocking(self) {
        let _ = match tokio::runtime::Handle::try_current() {
            Ok(_) => tokio::task::block_in_place(|| self.rt.block_on(self.sender.close())),
            Err(_) => self.rt.block_on(self.sender.close()),
        };
    }
}

/// Readiness state shared across all transport kinds.
///
/// `ready` is the condition that `flush()` waits on.  For gRPC, `ready` is
/// always in sync with `grpc.is_some()` — they are set/cleared together
/// under the same lock, so no separate `live: Mutex<bool>` is needed.
struct LiveState {
    ready: bool,
    /// gRPC-only: active sender + runtime for the current job.
    grpc: Option<GrpcState>,
}

// ── Inner shared state ──────────────────────────────────────────────────

struct Inner {
    transport: Mutex<TransportKind>,
    pending_frames: Mutex<Vec<Vec<u8>>>,
    uri: String,
    live_state: Mutex<LiveState>,
    live_cond: Condvar,
}

// ── Public API ──────────────────────────────────────────────────────────

/// Stream transport for delivering stdin/hints data to a ZisK job.
///
/// # Lifecycle
///
/// 1. **Create**: `ZiskStream::unix()` / `quic()` / `grpc()`
/// 2. **Write** (optional): `write()` / `write_slice()` buffer frames locally.
/// 3. **Start**: called by the SDK when `run()` is invoked.
///    - unix/quic: opens the socket (bind+listen), spawns a background thread
///      that waits for the peer to connect, drains buffered frames, then marks
///      the stream *live*.
///    - gRPC: `set_input_sender()` marks it live immediately.
/// 4. **Write** more data, then call `flush()` to send it.
/// 5. **Reuse**: calling `start()` again tears down the old connection and
///    re-opens for a new job.
#[derive(Clone)]
pub struct ZiskStream {
    inner: Arc<Inner>,
}

impl ZiskStream {
    // ── Constructors ────────────────────────────────────────────────────

    /// Unix domain socket with an auto-assigned path under `/tmp/`.
    #[cfg(unix)]
    pub fn unix() -> Self {
        let path = format!("/tmp/zisk-input-{}.sock", uuid::Uuid::new_v4());
        let uri = format!("unix://{}", path);
        let writer =
            UnixSocketStreamWriter::new(&path).expect("failed to create UnixSocketStreamWriter");
        Self::from_writer(Box::new(writer), uri)
    }

    /// Unix domain socket at an explicit path.
    #[cfg(unix)]
    pub fn unix_at(path: &str) -> Result<Self> {
        let uri = format!("unix://{}", path);
        let writer = UnixSocketStreamWriter::new(path)?;
        Ok(Self::from_writer(Box::new(writer), uri))
    }

    /// QUIC transport (e.g. `"quic://127.0.0.1:7001"`).
    pub fn quic(uri: &str) -> Result<Self> {
        let addr_str = uri
            .strip_prefix("quic://")
            .ok_or_else(|| anyhow::anyhow!("QUIC URI must start with quic://"))?;
        let addr: std::net::SocketAddr = addr_str
            .parse()
            .map_err(|e| anyhow::anyhow!("invalid QUIC address '{}': {}", addr_str, e))?;
        let writer = QuicStreamWriter::new(addr)?;
        Ok(Self::from_writer(Box::new(writer), uri.to_string()))
    }

    /// gRPC push transport (data pushed to coordinator via `PushJobInput`).
    pub fn grpc() -> Self {
        Self {
            inner: Arc::new(Inner {
                transport: Mutex::new(TransportKind::Grpc),
                pending_frames: Mutex::new(Vec::new()),
                uri: "grpc://push".to_string(),
                live_state: Mutex::new(LiveState { ready: false, grpc: None }),
                live_cond: Condvar::new(),
            }),
        }
    }

    fn from_writer(writer: Box<dyn StreamWrite>, uri: String) -> Self {
        Self {
            inner: Arc::new(Inner {
                transport: Mutex::new(TransportKind::Direct(writer)),
                pending_frames: Mutex::new(Vec::new()),
                uri,
                live_state: Mutex::new(LiveState { ready: false, grpc: None }),
                live_cond: Condvar::new(),
            }),
        }
    }

    // ── Write / flush ───────────────────────────────────────────────────

    /// Buffer a serializable value for later transmission.
    pub fn write<T: Serialize>(&self, data: &T) {
        let bytes = bincode::serialize(data).expect("Failed to serialize");
        self.write_slice(&bytes);
    }

    /// Buffer raw bytes for later transmission.
    pub fn write_slice(&self, data: &[u8]) {
        let frame = build_frame(data);
        self.inner.pending_frames.lock().unwrap().push(frame);
    }

    /// Send all buffered frames now.  Blocks until the stream is live.
    pub fn flush(&self) -> Result<()> {
        // Wait for ready, holding the live_state lock so we can pass it
        // straight into the gRPC send path (prevents a race with start/finish).
        let mut guard = self.inner.live_state.lock().unwrap();
        while !guard.ready {
            guard = self.inner.live_cond.wait(guard).unwrap();
        }

        let frames: Vec<Vec<u8>> = self.inner.pending_frames.lock().unwrap().drain(..).collect();
        if frames.is_empty() {
            return Ok(());
        }

        if let Some(state) = guard.grpc.as_ref() {
            // gRPC: hold live_state lock for the entire send so start()/finish()
            // can't clear the sender mid-flight.
            let rt = state.rt.clone();
            let send = async {
                for frame in frames {
                    state.sender.send(Bytes::from(frame)).await?;
                }
                Ok::<(), anyhow::Error>(())
            };
            match tokio::runtime::Handle::try_current() {
                Ok(_) => tokio::task::block_in_place(|| rt.block_on(send))?,
                Err(_) => rt.block_on(send)?,
            }
        } else {
            // Direct (unix/quic): release live_state, then write under transport lock.
            drop(guard);
            let mut transport = self.inner.transport.lock().unwrap();
            let TransportKind::Direct(writer) = &mut *transport else { unreachable!() };
            for (i, frame) in frames.iter().enumerate() {
                if let Err(e) = writer.write(frame) {
                    // Re-queue unsent frames for the next start() cycle.
                    drop(transport);
                    self.inner
                        .pending_frames
                        .lock()
                        .unwrap()
                        .splice(0..0, frames[i..].iter().cloned());
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    /// Discard all buffered (unsent) frames.
    pub fn reset(&self) {
        self.inner.pending_frames.lock().unwrap().clear();
    }

    // ── Accessors ───────────────────────────────────────────────────────

    /// The transport URI (e.g. `"unix:///tmp/zisk-input-<id>.sock"`).
    pub fn uri(&self) -> &str {
        &self.inner.uri
    }

    /// Whether this stream uses gRPC push transport.
    pub(crate) fn is_grpc(&self) -> bool {
        matches!(&*self.inner.transport.lock().unwrap(), TransportKind::Grpc)
    }

    // ── SDK-internal lifecycle ───────────────────────────────────────────

    /// Prepare the transport and start waiting for a peer connection.
    ///
    /// For **unix/quic**: opens the socket synchronously (bind + listen) so
    /// the path is connectable immediately, then spawns a background thread
    /// that blocks on `wait_for_connection()`, drains buffered frames, and
    /// marks the stream *live*.
    ///
    /// For **gRPC**: closes the previous sender (if any), waits for its
    /// PushJobInput RPC to finish, then returns.  The new sender arrives
    /// via [`set_input_sender()`] after `submit_job()`.
    ///
    /// Safe to call multiple times (reuse across jobs): each call tears down
    /// the previous connection first.
    pub(crate) fn start(&self) -> Result<()> {
        if self.is_grpc() {
            let old_state = {
                let mut guard = self.inner.live_state.lock().unwrap();
                guard.ready = false;
                guard.grpc.take()
            };
            if let Some(state) = old_state {
                state.close_blocking();
            }
            return Ok(());
        }

        self.inner.live_state.lock().unwrap().ready = false;

        // Open synchronously: bind + listen.  After this returns the socket
        // path exists and readers can connect.
        {
            let mut transport = self.inner.transport.lock().unwrap();
            let TransportKind::Direct(writer) = &mut *transport else { unreachable!() };
            if writer.is_active() {
                let _ = writer.close();
            }
            writer.open()?;
        }

        // Spawn background thread: wait for connection, drain frames, go live.
        let stream = self.clone();
        std::thread::spawn(move || {
            let result = (|| -> Result<()> {
                let mut transport = stream.inner.transport.lock().unwrap();
                let TransportKind::Direct(writer) = &mut *transport else { unreachable!() };

                writer.wait_for_connection()?;

                let mut frames = stream.inner.pending_frames.lock().unwrap();
                for frame in frames.drain(..) {
                    writer.write(&frame)?;
                }
                Ok(())
            })();

            match result {
                Ok(()) => {
                    stream.inner.live_state.lock().unwrap().ready = true;
                    stream.inner.live_cond.notify_all();
                }
                Err(e) => tracing::error!("stream start failed: {}", e),
            }
        });

        Ok(())
    }

    /// Inject the gRPC sender obtained after job submission.  Marks live.
    pub(crate) fn set_input_sender(&self, sender: InputSender) {
        {
            let mut guard = self.inner.live_state.lock().unwrap();
            guard.grpc = Some(GrpcState { sender, rt: tokio::runtime::Handle::current() });
            guard.ready = true;
        }
        self.inner.live_cond.notify_all();
    }

    /// Close the current transport and mark the stream as not-live.
    ///
    /// For reusable streams, this is called automatically when a `JobHandle`
    /// is awaited.  You only need to call it manually if you are not awaiting
    /// a handle (e.g. error paths).
    ///
    /// Any `flush()` called after `finish()` will block until the next
    /// `run()` re-opens the transport — preventing accidental writes to a
    /// completed job.
    pub fn finish(&self) -> Result<()> {
        if self.is_grpc() {
            let old_state = {
                let mut guard = self.inner.live_state.lock().unwrap();
                guard.ready = false;
                guard.grpc.take()
            };
            if let Some(state) = old_state {
                state.close_blocking();
            }
        } else {
            self.inner.live_state.lock().unwrap().ready = false;
            let mut transport = self.inner.transport.lock().unwrap();
            if let TransportKind::Direct(writer) = &mut *transport {
                if writer.is_active() {
                    writer.close()?;
                }
            }
        }
        Ok(())
    }

    /// Async version of [`finish`](Self::finish).
    ///
    /// Called automatically by `JobHandle` when it is awaited.
    pub(crate) async fn finish_async(&self) -> Result<()> {
        if self.is_grpc() {
            let old_state = {
                let mut guard = self.inner.live_state.lock().unwrap();
                guard.ready = false;
                guard.grpc.take()
            };
            if let Some(state) = old_state {
                let _ = state.sender.close().await;
            }
        } else {
            self.inner.live_state.lock().unwrap().ready = false;
            let mut transport = self.inner.transport.lock().unwrap();
            if let TransportKind::Direct(writer) = &mut *transport {
                if writer.is_active() {
                    writer.close()?;
                }
            }
        }
        Ok(())
    }
}

// ── Drop ────────────────────────────────────────────────────────────────

impl Drop for Inner {
    fn drop(&mut self) {
        if let TransportKind::Direct(writer) = self.transport.get_mut().unwrap() {
            if writer.is_active() {
                let _ = writer.close();
            }
        }
    }
}

// ── Frame encoding ──────────────────────────────────────────────────────

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

/// Decode one frame produced by [`build_frame`].  Returns the payload bytes.
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
    use std::thread;
    use std::time::Duration;

    /// Run a closure on a dedicated thread with a timeout.
    /// Panics with a descriptive message if the closure doesn't finish in time.
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

    // ── Frame encoding tests ────────────────────────────────────────────

    #[test]
    fn frame_roundtrip_and_alignment() {
        // Empty
        let frame = build_frame(b"");
        assert_eq!(frame.len(), 8);
        assert_eq!(decode_frame(&frame), b"");

        // 1 byte → max padding (7 bytes)
        let frame = build_frame(b"x");
        assert_eq!(frame.len(), 16); // 8 + 1 + 7 padding
        assert_eq!(decode_frame(&frame), b"x");

        // 5 bytes → 3 bytes padding
        let frame = build_frame(b"hello");
        assert_eq!(frame.len(), 16);
        assert_eq!(decode_frame(&frame), b"hello");

        // 8 bytes → already aligned, no padding
        let frame = build_frame(b"12345678");
        assert_eq!(frame.len(), 16);
        assert_eq!(decode_frame(&frame), b"12345678");

        // Arbitrary data
        let data = b"round-trip test data!";
        assert_eq!(decode_frame(&build_frame(data)), data.as_slice());
    }

    // ── Write / reset / flush unit tests ─────────────────────────────────

    #[test]
    fn write_slice_buffers_and_reset_clears() {
        let stream = ZiskStream::grpc();
        stream.write_slice(b"raw bytes");
        assert_eq!(decode_frame(&stream.inner.pending_frames.lock().unwrap()[0]), b"raw bytes");

        stream.write(&42u32);
        assert_eq!(stream.inner.pending_frames.lock().unwrap().len(), 2);

        stream.reset();
        assert_eq!(stream.inner.pending_frames.lock().unwrap().len(), 0);
    }

    #[test]
    fn flush_empty_is_noop_when_live() {
        let stream = ZiskStream::grpc();
        stream.inner.live_state.lock().unwrap().ready = true;
        assert!(stream.flush().is_ok());
    }

    #[test]
    fn quic_rejects_bad_uri() {
        assert!(ZiskStream::quic("http://localhost:9000").is_err());
    }

    // ── Unix socket integration tests ───────────────────────────────────

    #[cfg(unix)]
    mod unix_tests {
        use super::*;
        use zisk_common::io::{StreamRead, UnixSocketStreamReader};

        #[test]
        fn unix_write_before_start_then_flush() {
            run_with_timeout("unix_write_before_start_then_flush", TEST_TIMEOUT, || {
                let stream = ZiskStream::unix();
                let uri = stream.uri().to_string();
                let path = uri.strip_prefix("unix://").unwrap().to_string();

                // Buffer data BEFORE start
                stream.write(&42u32);
                stream.write(&99u32);

                // Start opens the socket and drains buffered frames on connection
                stream.start().unwrap();

                // Connect a reader (triggers wait_for_connection)
                let mut reader = UnixSocketStreamReader::new(&path).unwrap();

                // The two pre-buffered writes should have been sent on connect
                let msg1 = reader.next().unwrap().unwrap();
                let msg2 = reader.next().unwrap().unwrap();

                let expected1 = bincode::serialize(&42u32).unwrap();
                let expected2 = bincode::serialize(&99u32).unwrap();
                assert_eq!(decode_frame(&msg1), expected1);
                assert_eq!(decode_frame(&msg2), expected2);

                stream.finish().unwrap();
                reader.close().unwrap();
            });
        }

        #[test]
        fn unix_flush_after_live() {
            run_with_timeout("unix_flush_after_live", TEST_TIMEOUT, || {
                let stream = ZiskStream::unix();
                let path = stream.uri().strip_prefix("unix://").unwrap().to_string();

                stream.start().unwrap();

                // Connect reader to make the stream go live
                let mut reader = UnixSocketStreamReader::new(&path).unwrap();
                reader.open().unwrap(); // triggers the actual socket connection

                // Wait for the stream to become live
                let deadline = std::time::Instant::now() + Duration::from_secs(5);
                loop {
                    if stream.inner.live_state.lock().unwrap().ready {
                        break;
                    }
                    assert!(std::time::Instant::now() < deadline, "stream never became live");
                    thread::sleep(Duration::from_millis(10));
                }

                // Now write + flush after live
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
                let stream = ZiskStream::unix();
                let path = stream.uri().strip_prefix("unix://").unwrap().to_string();

                stream.start().unwrap();
                let mut reader = UnixSocketStreamReader::new(&path).unwrap();
                reader.open().unwrap();

                // Wait live
                let deadline = std::time::Instant::now() + Duration::from_secs(5);
                while !stream.inner.live_state.lock().unwrap().ready {
                    assert!(std::time::Instant::now() < deadline);
                    thread::sleep(Duration::from_millis(10));
                }

                // Flush #1
                stream.write_slice(b"batch-1a");
                stream.write_slice(b"batch-1b");
                stream.flush().unwrap();

                // Flush #2
                stream.write_slice(b"batch-2");
                stream.flush().unwrap();

                let m1 = reader.next().unwrap().unwrap();
                let m2 = reader.next().unwrap().unwrap();
                let m3 = reader.next().unwrap().unwrap();
                assert_eq!(decode_frame(&m1), b"batch-1a");
                assert_eq!(decode_frame(&m2), b"batch-1b");
                assert_eq!(decode_frame(&m3), b"batch-2");

                stream.finish().unwrap();
                reader.close().unwrap();
            });
        }

        #[test]
        fn unix_start_reuse_across_jobs() {
            run_with_timeout("unix_start_reuse_across_jobs", TEST_TIMEOUT, || {
                let stream = ZiskStream::unix();
                let path = stream.uri().strip_prefix("unix://").unwrap().to_string();

                // === Job 1 ===
                stream.write(&1u32);
                stream.start().unwrap();

                let mut reader1 = UnixSocketStreamReader::new(&path).unwrap();
                let msg = reader1.next().unwrap().unwrap();
                assert_eq!(decode_frame(&msg), bincode::serialize(&1u32).unwrap());

                stream.finish().unwrap();
                reader1.close().unwrap();

                // === Job 2 (reuse same stream) ===
                stream.write(&2u32);
                stream.start().unwrap();

                let mut reader2 = UnixSocketStreamReader::new(&path).unwrap();
                let msg = reader2.next().unwrap().unwrap();
                assert_eq!(decode_frame(&msg), bincode::serialize(&2u32).unwrap());

                stream.finish().unwrap();
                reader2.close().unwrap();
            });
        }

        #[test]
        fn unix_finish_makes_stream_not_ready() {
            run_with_timeout("unix_finish_makes_stream_not_ready", TEST_TIMEOUT, || {
                let stream = ZiskStream::unix();
                let path = stream.uri().strip_prefix("unix://").unwrap().to_string();

                stream.start().unwrap();
                let mut reader = UnixSocketStreamReader::new(&path).unwrap();
                reader.open().unwrap();

                // Wait live
                let deadline = std::time::Instant::now() + Duration::from_secs(5);
                while !stream.inner.live_state.lock().unwrap().ready {
                    assert!(std::time::Instant::now() < deadline);
                    thread::sleep(Duration::from_millis(10));
                }
                assert!(stream.inner.live_state.lock().unwrap().ready);

                stream.finish().unwrap();
                assert!(!stream.inner.live_state.lock().unwrap().ready);

                reader.close().unwrap();
            });
        }

        #[test]
        fn unix_flush_blocks_until_live() {
            run_with_timeout("unix_flush_blocks_until_live", TEST_TIMEOUT, || {
                let stream = ZiskStream::unix();
                let path = stream.uri().strip_prefix("unix://").unwrap().to_string();

                stream.start().unwrap();

                // Write data and spawn a thread that flushes (will block until live)
                stream.write_slice(b"blocked data");
                let stream_clone = stream.clone();
                let flushed = Arc::new(AtomicBool::new(false));
                let flushed_clone = flushed.clone();
                let flush_thread = thread::spawn(move || {
                    stream_clone.flush().unwrap();
                    flushed_clone.store(true, Ordering::Release);
                });

                // Give the flush thread time to start blocking
                thread::sleep(Duration::from_millis(100));
                assert!(!flushed.load(Ordering::Acquire), "flush should still be blocking");

                // Connect reader → stream goes live → flush unblocks
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
                let stream = ZiskStream::unix();
                let path = stream.uri().strip_prefix("unix://").unwrap().to_string();

                let large_data = vec![0xABu8; 64 * 1024]; // 64 KB
                stream.write_slice(&large_data);
                stream.start().unwrap();

                let mut reader = UnixSocketStreamReader::new(&path).unwrap();
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

        /// Find a free port by binding to :0.
        fn free_port() -> u16 {
            std::net::UdpSocket::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
        }

        #[test]
        fn quic_write_before_start_then_read() {
            run_with_timeout("quic_write_before_start_then_read", TEST_TIMEOUT, || {
                let port = free_port();
                let uri = format!("quic://127.0.0.1:{port}");
                let stream = ZiskStream::quic(&uri).unwrap();

                // Buffer data before start
                stream.write(&42u32);
                stream.start().unwrap();

                // Connect reader
                let mut reader =
                    QuicStreamReader::new(format!("127.0.0.1:{port}").parse().unwrap()).unwrap();
                reader.open().unwrap();

                let msg = reader.next().unwrap().unwrap();
                let expected = bincode::serialize(&42u32).unwrap();
                assert_eq!(decode_frame(&msg), expected);

                stream.finish().unwrap();
                reader.close().unwrap();
            });
        }

        #[test]
        fn quic_flush_after_live() {
            run_with_timeout("quic_flush_after_live", TEST_TIMEOUT, || {
                let port = free_port();
                let uri = format!("quic://127.0.0.1:{port}");
                let stream = ZiskStream::quic(&uri).unwrap();

                stream.start().unwrap();

                let mut reader =
                    QuicStreamReader::new(format!("127.0.0.1:{port}").parse().unwrap()).unwrap();
                reader.open().unwrap();

                // Wait live
                let deadline = std::time::Instant::now() + Duration::from_secs(5);
                while !stream.inner.live_state.lock().unwrap().ready {
                    assert!(std::time::Instant::now() < deadline);
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

        #[test]
        fn quic_multiple_flush_cycles() {
            run_with_timeout("quic_multiple_flush_cycles", TEST_TIMEOUT, || {
                let port = free_port();
                let uri = format!("quic://127.0.0.1:{port}");
                let stream = ZiskStream::quic(&uri).unwrap();

                stream.start().unwrap();

                let mut reader =
                    QuicStreamReader::new(format!("127.0.0.1:{port}").parse().unwrap()).unwrap();
                reader.open().unwrap();

                // Wait live
                let deadline = std::time::Instant::now() + Duration::from_secs(5);
                while !stream.inner.live_state.lock().unwrap().ready {
                    assert!(std::time::Instant::now() < deadline);
                    thread::sleep(Duration::from_millis(10));
                }

                stream.write_slice(b"q1");
                stream.flush().unwrap();

                stream.write_slice(b"q2");
                stream.write_slice(b"q3");
                stream.flush().unwrap();

                let m1 = reader.next().unwrap().unwrap();
                let m2 = reader.next().unwrap().unwrap();
                let m3 = reader.next().unwrap().unwrap();
                assert_eq!(decode_frame(&m1), b"q1");
                assert_eq!(decode_frame(&m2), b"q2");
                assert_eq!(decode_frame(&m3), b"q3");

                stream.finish().unwrap();
                reader.close().unwrap();
            });
        }
    }

    // ── gRPC unit tests (no real server) ──────────────────────────────────

    #[test]
    fn grpc_reset_then_flush_sends_nothing() {
        let stream = ZiskStream::grpc();
        stream.write(&1u32);
        stream.write(&2u32);
        stream.reset();

        stream.inner.live_state.lock().unwrap().ready = true;

        stream.flush().unwrap();
        assert_eq!(stream.inner.pending_frames.lock().unwrap().len(), 0);
    }

    // ── Lifecycle edge-case tests ────────────────────────────────────────

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
        // finish() on a never-started stream should not panic or error
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
        stream.inner.live_state.lock().unwrap().ready = true;
        assert!(stream.finish().is_ok());
        assert!(stream.finish().is_ok());
        assert!(!stream.inner.live_state.lock().unwrap().ready);
    }

    #[cfg(unix)]
    #[test]
    fn start_while_already_live_tears_down_and_reopens() {
        run_with_timeout("start_while_already_live", TEST_TIMEOUT, || {
            use zisk_common::io::{StreamRead, UnixSocketStreamReader};

            let stream = ZiskStream::unix();
            let path = stream.uri().strip_prefix("unix://").unwrap().to_string();

            // Start job 1, connect a reader, go live
            stream.write_slice(b"job1");
            stream.start().unwrap();
            let mut reader1 = UnixSocketStreamReader::new(&path).unwrap();
            reader1.open().unwrap();

            let msg = reader1.next().unwrap().unwrap();
            assert_eq!(decode_frame(&msg), b"job1");

            // Call start() again WITHOUT calling finish() first
            stream.write_slice(b"job2");
            stream.start().unwrap();

            // Old reader should be dead; new reader connects
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

        // All 800 frames should be present (no lost writes)
        assert_eq!(stream.inner.pending_frames.lock().unwrap().len(), 800);
    }

    // ── Flush error re-queue test (mock writer) ─────────────────────────

    mod requeue_tests {
        use super::*;
        use std::sync::atomic::{AtomicUsize, Ordering};

        /// A mock writer that fails on the Nth write call.
        struct FailingWriter {
            call_count: AtomicUsize,
            fail_on: usize,
            active: bool,
        }

        impl FailingWriter {
            fn new(fail_on: usize) -> Self {
                Self { call_count: AtomicUsize::new(0), fail_on, active: true }
            }
        }

        impl StreamWrite for FailingWriter {
            fn open(&mut self) -> Result<()> {
                self.active = true;
                Ok(())
            }
            fn write(&mut self, _item: &[u8]) -> Result<usize> {
                let n = self.call_count.fetch_add(1, Ordering::Relaxed);
                if n >= self.fail_on {
                    Err(anyhow::anyhow!("mock write failure on call {n}"))
                } else {
                    Ok(_item.len())
                }
            }
            fn flush(&mut self) -> Result<()> {
                Ok(())
            }
            fn close(&mut self) -> Result<()> {
                self.active = false;
                Ok(())
            }
            fn is_active(&self) -> bool {
                self.active
            }
        }

        #[test]
        fn flush_error_requeues_unsent_frames() {
            // Writer that succeeds on first write, fails on second
            let writer = FailingWriter::new(1);
            let stream = ZiskStream::from_writer(Box::new(writer), "mock://test".into());

            // Buffer 3 frames
            stream.write_slice(b"frame-0");
            stream.write_slice(b"frame-1");
            stream.write_slice(b"frame-2");

            // Manually mark live (skip start() — no real socket)
            stream.inner.live_state.lock().unwrap().ready = true;

            // Flush should fail on frame-1
            let result = stream.flush();
            assert!(result.is_err());

            // Frames 1 and 2 should be re-queued
            let pending = stream.inner.pending_frames.lock().unwrap();
            assert_eq!(pending.len(), 2, "unsent frames should be re-queued");
            assert_eq!(decode_frame(&pending[0]), b"frame-1");
            assert_eq!(decode_frame(&pending[1]), b"frame-2");
        }

        #[test]
        fn flush_error_requeues_all_when_first_write_fails() {
            let writer = FailingWriter::new(0); // fail immediately
            let stream = ZiskStream::from_writer(Box::new(writer), "mock://test".into());

            stream.write_slice(b"a");
            stream.write_slice(b"b");

            stream.inner.live_state.lock().unwrap().ready = true;

            assert!(stream.flush().is_err());

            let pending = stream.inner.pending_frames.lock().unwrap();
            assert_eq!(pending.len(), 2, "all frames re-queued on first-write failure");
            assert_eq!(decode_frame(&pending[0]), b"a");
            assert_eq!(decode_frame(&pending[1]), b"b");
        }
    }
}
