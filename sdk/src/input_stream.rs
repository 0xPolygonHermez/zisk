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
