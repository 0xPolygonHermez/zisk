//! Transport-agnostic byte-stream writer.
//!
//! `ZiskStreamWriter` owns the lifecycle and chunking concerns shared by every
//! producer that pushes bytes over a `StreamWrite` transport (Unix socket, QUIC,
//! file, future channel, etc.). It is intentionally *unframed*: callers push raw
//! bytes via [`push_raw`](ZiskStreamWriter::push_raw) and the writer chunks them
//! at u64-aligned boundaries determined by the transport's `max_message_size()`.
//!
//! # Layering
//!
//! - **Transport** (this module): opaque bytes; chunking; lifecycle (start/finish);
//!   ready-signal so `flush()` blocks until the peer is connected.
//! - **Protocol** (above this layer, owned by callers): record framing for input
//!   data, hint headers for the precompile-hints stream, etc.
//!
//! The byte stream this writer puts on the wire is exactly what the caller
//! pushed, in order, with no per-call delimiters added.
//!
//! # u64 alignment
//!
//! Every consumer that does `reinterpret_vec::<u8, u64>` will silently zero-pad a
//! non-aligned chunk and corrupt the stream. To avoid this, [`flush`] cuts each
//! intermediate wire chunk on an 8-byte boundary. The final chunk preserves any
//! trailing remainder verbatim — callers that need an aligned total are
//! responsible for pushing a multiple-of-8 byte count overall.

use std::sync::{Arc, Condvar, Mutex};

use anyhow::Result;

use crate::io::{StreamWrite, CONNECT_DEADLINE};

use crate::io::QuicStreamWriter;
#[cfg(unix)]
use crate::io::UnixSocketStreamWriter;

/// Default per-call chunk size for the [`TransportKind::Push`] arm.
///
/// The trait sender is opaque — we don't know its underlying max message size
/// (a gRPC adapter may auto-chunk internally at 3 MB; a mock impl might accept
/// anything). Picking a fixed size at this layer keeps byte-position retry
/// semantics consistent: each `send_blocking` call is one atomic unit, so a
/// failed call leaves a known unsent suffix in `pending`. 64 KB matches the
/// hint pipeline's existing flush threshold.
const PUSH_DEFAULT_CHUNK_SIZE: usize = 64 * 1024;

/// Background-connect poll interval. Short enough that `finish()` and
/// `flush()` only ever wait one tick on `transport.lock()` between polls.
const CONNECT_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(5);

// ── Push sender trait ──────────────────────────────────────────────────────

/// Sender for transports that push bytes through an external channel rather
/// than a [`StreamWrite`]. Used today by gRPC `PushJobInput` /
/// `PushJobHintsInput` streams; could host any future async push transport
/// that lives outside `zisk-common`.
///
/// All methods are blocking from the caller's point of view. Implementations
/// own any async runtime they need (typically by capturing a
/// [`tokio::runtime::Handle`] at construction).
///
/// # Atomicity
///
/// Each `send_blocking` call must be atomic: on success the bytes are
/// committed to the wire (or to a queue that will deliver them); on error
/// nothing was sent. [`ZiskStreamWriter::flush`] depends on this for its
/// byte-position retry semantics.
pub trait BytesPushSender: Send + Sync {
    /// Send one chunk of bytes. Blocks until the chunk is queued or sent.
    fn send_blocking(&self, data: Vec<u8>) -> Result<()>;

    /// Cleanly close the stream. Consumes `self` because typical impls (gRPC)
    /// need to await an RPC future. Blocks until the close completes.
    fn close_blocking(self: Box<Self>) -> Result<()>;
}

// ── Transport ──────────────────────────────────────────────────────────────

enum TransportKind {
    /// Writer owned here; opened on `start()`.
    Direct(Box<dyn StreamWrite>),
    /// Socket bound and written by another process. We only carry the URI.
    External,
    /// Async push transport (e.g. gRPC). The sender is injected after
    /// `start()` via [`ZiskStreamWriter::set_push_sender`].
    Push,
}

/// Mutex-free tag mirroring `TransportKind`'s discriminant. The bg connect
/// thread only holds `transport.lock()` for the duration of a single
/// non-blocking poll (≤ a few ms), but `is_external` / `is_push` are called
/// from hot paths where even brief contention with the bg thread is wasteful;
/// caching the discriminant here avoids the lock entirely.
#[derive(Copy, Clone, PartialEq)]
enum TransportTag {
    Direct,
    External,
    Push,
}

// ── Live state ─────────────────────────────────────────────────────────────

struct LiveState {
    /// `true` after `start()` has opened the transport, drained any
    /// pre-buffered bytes, and seen the peer connect. `flush()` blocks on this.
    ready: bool,
    /// `start()` is idempotent while this is set so two concurrent `start()`
    /// calls share one bg thread instead of racing.
    starting: bool,
    /// Monotonic counter bumped by `start()` and `finish()`. Each bg connect
    /// thread captures the value at spawn; on every loop iteration (and on
    /// final state write-back) it compares against the current value and
    /// bails out if they differ. This prevents a stale bg thread from a
    /// previous `start()` from clobbering a fresh `start()`'s `LiveState`
    /// after a `finish()` → `start()` sequence.
    start_generation: u64,
    /// Set when the bg thread's start handshake (connect-poll or initial
    /// drain) failed. `flush()` returns this so waiters don't block forever
    /// after a connection timeout. Cleared by the next successful `start()`
    /// or `finish()`.
    last_start_error: Option<String>,
    /// Active push sender for the current job. Set/cleared in tandem with
    /// `ready` for [`TransportKind::Push`]; always `None` for other kinds.
    push_sender: Option<Box<dyn BytesPushSender>>,
}

// ── Inner shared state ─────────────────────────────────────────────────────

struct Inner {
    transport: Mutex<TransportKind>,
    tag: TransportTag,
    pending: Mutex<Vec<u8>>,
    uri: String,
    live_state: Mutex<LiveState>,
    live_cond: Condvar,
}

// ── Public type ────────────────────────────────────────────────────────────

/// Buffered, transport-agnostic byte writer with start/finish lifecycle.
///
/// See module docs for layering and alignment rules.
#[derive(Clone)]
pub struct ZiskStreamWriter {
    inner: Arc<Inner>,
}

impl ZiskStreamWriter {
    // ── Constructors ───────────────────────────────────────────────────────

    /// Wrap an arbitrary `StreamWrite` transport. The `uri` is metadata used by
    /// callers (e.g. coordinator URI plumbing) and the writer itself never
    /// parses it.
    pub fn from_writer(writer: Box<dyn StreamWrite>, uri: String) -> Self {
        Self {
            inner: Arc::new(Inner {
                transport: Mutex::new(TransportKind::Direct(writer)),
                tag: TransportTag::Direct,
                pending: Mutex::new(Vec::new()),
                uri,
                live_state: Mutex::new(LiveState {
                    ready: false,
                    starting: false,
                    start_generation: 0,
                    last_start_error: None,
                    push_sender: None,
                }),
                live_cond: Condvar::new(),
            }),
        }
    }

    /// Externally-managed transport. The writer carries only the URI; pushes
    /// are buffered and `flush()` is a no-op (some other process owns the
    /// socket and writes to it directly).
    pub fn unix_external(uri: String) -> Self {
        Self {
            inner: Arc::new(Inner {
                transport: Mutex::new(TransportKind::External),
                tag: TransportTag::External,
                pending: Mutex::new(Vec::new()),
                uri,
                live_state: Mutex::new(LiveState {
                    ready: true,
                    starting: false,
                    start_generation: 0,
                    last_start_error: None,
                    push_sender: None,
                }),
                live_cond: Condvar::new(),
            }),
        }
    }

    /// Unix domain socket bound at the given path. The socket starts listening
    /// immediately; the peer can connect as soon as the path is on disk.
    #[cfg(unix)]
    pub fn unix_at(path: &str) -> Result<Self> {
        let uri = format!("unix://{}", path);
        let mut writer = UnixSocketStreamWriter::new(path)?;
        writer.open()?;
        Ok(Self::from_writer(Box::new(writer), uri))
    }

    /// QUIC transport bound at the given socket address. The resolved local
    /// address (after `:0` is replaced with an OS-assigned port) becomes the
    /// URI.
    pub fn quic(addr: std::net::SocketAddr) -> Result<Self> {
        let writer = QuicStreamWriter::new(addr)?;
        let uri = format!("quic://{}", writer.local_addr()?);
        Ok(Self::from_writer(Box::new(writer), uri))
    }

    /// Async push transport. The sender is injected later via
    /// [`set_push_sender`](Self::set_push_sender) — typically after a gRPC
    /// streaming RPC has opened. Until then, `flush()` blocks.
    pub fn push(uri: String) -> Self {
        Self {
            inner: Arc::new(Inner {
                transport: Mutex::new(TransportKind::Push),
                tag: TransportTag::Push,
                pending: Mutex::new(Vec::new()),
                uri,
                live_state: Mutex::new(LiveState {
                    ready: false,
                    starting: false,
                    start_generation: 0,
                    last_start_error: None,
                    push_sender: None,
                }),
                live_cond: Condvar::new(),
            }),
        }
    }

    // ── Accessors ──────────────────────────────────────────────────────────

    pub fn uri(&self) -> &str {
        &self.inner.uri
    }

    pub fn is_external(&self) -> bool {
        self.inner.tag == TransportTag::External
    }

    pub fn is_push(&self) -> bool {
        self.inner.tag == TransportTag::Push
    }

    /// `true` after `start()` (and, for Push, `set_push_sender`) has succeeded.
    /// Useful for callers waiting until a flush would not block; primarily for
    /// tests and observability.
    pub fn is_ready(&self) -> bool {
        self.inner.live_state.lock().unwrap().ready
    }

    /// Inject the push sender for a [`TransportKind::Push`] writer. Marks the
    /// stream live and wakes any flushers blocked on `live_cond`.
    ///
    /// Calling this on a non-Push writer is a no-op.
    pub fn set_push_sender(&self, sender: Box<dyn BytesPushSender>) {
        if !self.is_push() {
            return;
        }
        {
            let mut guard = self.inner.live_state.lock().unwrap();
            guard.push_sender = Some(sender);
            guard.ready = true;
        }
        self.inner.live_cond.notify_all();
    }

    // ── Write / flush ──────────────────────────────────────────────────────

    /// Append raw bytes to the pending buffer. Bytes are sent verbatim on the
    /// next `flush()`, in the order they were pushed.
    pub fn push_raw(&self, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        self.inner.pending.lock().unwrap().extend_from_slice(data);
    }

    /// Send all pending bytes. Blocks until the stream is live.
    ///
    /// Bytes are split into wire chunks of `max_message_size() & !7` (i.e.
    /// u64-aligned, never larger than the transport allows). On a partial-write
    /// error, the bytes that were successfully sent are dropped from the
    /// pending buffer; the unsent tail remains for the next call to retry.
    pub fn flush(&self) -> Result<()> {
        if self.is_external() {
            return Ok(());
        }

        if self.is_push() {
            return self.flush_push();
        }

        // Wait until the background `start()` thread reports the peer connected
        // and pre-buffered bytes (if any) have been drained. If the bg thread
        // recorded a startup failure, surface it instead of looping forever.
        let mut guard = self.inner.live_state.lock().unwrap();
        while !guard.ready {
            if let Some(err) = &guard.last_start_error {
                return Err(anyhow::anyhow!("ZiskStreamWriter start failed: {}", err));
            }
            let (g, _) = self
                .inner
                .live_cond
                .wait_timeout(guard, std::time::Duration::from_secs(5))
                .unwrap();
            guard = g;
        }
        drop(guard);

        let mut pending = self.inner.pending.lock().unwrap();
        if pending.is_empty() {
            return Ok(());
        }

        let mut transport = self.inner.transport.lock().unwrap();
        let TransportKind::Direct(writer) = &mut *transport else {
            // Already returned for External / Push above.
            unreachable!()
        };

        let chunk_size = aligned_chunk_size(writer.max_message_size());

        let mut sent = 0;
        while sent < pending.len() {
            let take = std::cmp::min(chunk_size, pending.len() - sent);
            match writer.write(&pending[sent..sent + take]) {
                Ok(_) => sent += take,
                Err(e) => {
                    // Drop successfully-sent bytes; leave the unsent tail.
                    pending.drain(..sent);
                    return Err(e);
                }
            }
        }
        pending.clear();
        Ok(())
    }

    /// Push-transport flush. Holds `live_state` for the duration of the
    /// chunk loop so concurrent `start()` / `finish()` / `set_push_sender()`
    /// can't tear down the sender mid-flight (matches the pre-refactor SDK
    /// gRPC behavior).
    fn flush_push(&self) -> Result<()> {
        let mut guard = self.inner.live_state.lock().unwrap();
        while !guard.ready {
            if let Some(err) = &guard.last_start_error {
                return Err(anyhow::anyhow!("ZiskStreamWriter start failed: {}", err));
            }
            guard = self.inner.live_cond.wait(guard).unwrap();
        }

        let mut pending = self.inner.pending.lock().unwrap();
        if pending.is_empty() {
            return Ok(());
        }

        let sender = guard
            .push_sender
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Push transport: ready=true but sender not set"))?;

        let chunk_size = aligned_chunk_size(PUSH_DEFAULT_CHUNK_SIZE);

        let mut sent = 0;
        while sent < pending.len() {
            let take = std::cmp::min(chunk_size, pending.len() - sent);
            match sender.send_blocking(pending[sent..sent + take].to_vec()) {
                Ok(_) => sent += take,
                Err(e) => {
                    pending.drain(..sent);
                    return Err(e);
                }
            }
        }
        pending.clear();
        Ok(())
    }

    /// Discard buffered bytes that have not yet been sent.
    pub fn reset(&self) {
        self.inner.pending.lock().unwrap().clear();
    }

    // ── Lifecycle ──────────────────────────────────────────────────────────

    /// Open the transport and spawn a background thread that waits for the
    /// peer to connect, drains any pre-buffered bytes, and marks the stream
    /// live. Idempotent across reuse: if the transport is already active, it
    /// is closed and reopened first.
    pub fn start(&self) -> Result<()> {
        if self.is_external() {
            // External transports are live by construction.
            let mut guard = self.inner.live_state.lock().unwrap();
            guard.ready = true;
            self.inner.live_cond.notify_all();
            return Ok(());
        }

        if self.is_push() {
            // Push: tear down any previous sender; wait for a fresh one to
            // arrive via `set_push_sender()` before going ready again.
            let old_sender = {
                let mut guard = self.inner.live_state.lock().unwrap();
                guard.ready = false;
                guard.push_sender.take()
            };
            if let Some(sender) = old_sender {
                let _ = sender.close_blocking();
            }
            return Ok(());
        }

        // Idempotency: a concurrent `start()` becomes a no-op (`starting=true`
        // → share the in-flight bg thread). Calling `start()` on an already-
        // ready stream tears it down and rebinds.
        let my_gen = {
            let mut guard = self.inner.live_state.lock().unwrap();
            if guard.starting {
                return Ok(());
            }
            guard.starting = true;
            guard.start_generation = guard.start_generation.wrapping_add(1);
            // Starting fresh: flushers must wait for the new bg thread to drain.
            guard.ready = false;
            guard.last_start_error = None;
            guard.start_generation
        };

        // Open (or reopen) the transport synchronously so the path is bindable
        // before this function returns.
        {
            let mut transport = self.inner.transport.lock().unwrap();
            let TransportKind::Direct(writer) = &mut *transport else { unreachable!() };
            if writer.is_active() {
                let _ = writer.close();
            }
            if let Err(e) = writer.open() {
                // Surface the failure to any flusher already blocked on
                // `live_cond` — otherwise they'd wait forever, since `ready`
                // never flips and no bg thread will be spawned to set
                // `last_start_error`.
                let mut guard = self.inner.live_state.lock().unwrap();
                guard.starting = false;
                guard.last_start_error = Some(e.to_string());
                self.inner.live_cond.notify_all();
                return Err(e);
            }
        }

        // Background thread: poll for peer connection without holding the
        // transport mutex across the full wait. Each poll briefly locks
        // `transport` to ask `is_client_connected()`, then releases for 5 ms
        // — so callers contending on `transport.lock()` (e.g. `finish()`)
        // only wait the duration of one poll, not the 60 s connection
        // deadline. The loop also observes `start_generation` so a concurrent
        // `finish()` (or a re-start) tears the thread down on the next tick
        // instead of letting it spin for the remainder of the deadline.
        //
        // NOTE: once the peer connects, this thread acquires `transport` and
        // `pending` simultaneously to drain the pre-buffered bytes. For
        // pending payloads larger than the transport chunk size this can
        // span multiple `write()` calls — `finish()` blocks on
        // `transport.lock()` for the duration of the drain. In practice the
        // drain is tens of ms on Unix and can reach hundreds of ms on QUIC.
        let inner = Arc::clone(&self.inner);
        std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + CONNECT_DEADLINE;
            let result = loop {
                // Cheap check first: if the generation changed (either
                // `finish()` cleared us or another `start()` superseded us),
                // bail immediately. Without this, stale connect threads
                // accumulate across cancel/retry scenarios, each sleeping up
                // to the full 60 s deadline.
                if inner.live_state.lock().unwrap().start_generation != my_gen {
                    break Err(anyhow::anyhow!("start superseded before peer connected"));
                }
                let connected = {
                    let mut transport = inner.transport.lock().unwrap();
                    match &mut *transport {
                        TransportKind::Direct(writer) => writer.is_client_connected(),
                        // Transport was torn down (finish() raced us); abort.
                        _ => break Err(anyhow::anyhow!("transport closed before peer connected")),
                    }
                };
                if connected {
                    let mut transport = inner.transport.lock().unwrap();
                    let TransportKind::Direct(writer) = &mut *transport else {
                        break Err(anyhow::anyhow!("transport closed before drain"));
                    };
                    let mut pending = inner.pending.lock().unwrap();
                    let chunk_size = aligned_chunk_size(writer.max_message_size());
                    let mut sent = 0;
                    let mut drain_err: Option<anyhow::Error> = None;
                    while sent < pending.len() {
                        let take = std::cmp::min(chunk_size, pending.len() - sent);
                        if let Err(e) = writer.write(&pending[sent..sent + take]) {
                            drain_err = Some(e);
                            break;
                        }
                        sent += take;
                    }
                    if let Some(e) = drain_err {
                        pending.drain(..sent);
                        break Err(e);
                    }
                    pending.clear();
                    break Ok(());
                }
                if std::time::Instant::now() >= deadline {
                    break Err(anyhow::anyhow!(
                        "Timed out waiting for peer to connect to {}",
                        inner.uri
                    ));
                }
                std::thread::sleep(CONNECT_POLL_INTERVAL);
            };

            let mut guard = inner.live_state.lock().unwrap();
            // Only write back if we're still the active start. Anything else
            // means `finish()` or a fresh `start()` already took over.
            if guard.start_generation != my_gen {
                inner.live_cond.notify_all();
                return;
            }
            guard.starting = false;
            match result {
                Ok(()) => {
                    guard.ready = true;
                    inner.live_cond.notify_all();
                }
                Err(e) => {
                    tracing::error!("ZiskStreamWriter start failed: {}", e);
                    guard.last_start_error = Some(e.to_string());
                    inner.live_cond.notify_all();
                }
            }
        });

        Ok(())
    }

    /// Mark the stream not-ready and close the transport. Safe to call
    /// without a preceding `start()`. After `finish()`, any `flush()` will
    /// block until the next `start()` brings the stream live again.
    pub fn finish(&self) -> Result<()> {
        if self.is_external() {
            return Ok(());
        }

        if self.is_push() {
            let old_sender = {
                let mut guard = self.inner.live_state.lock().unwrap();
                guard.ready = false;
                guard.push_sender.take()
            };
            if let Some(sender) = old_sender {
                let res = sender.close_blocking();
                return res;
            }
            return Ok(());
        }

        {
            let mut guard = self.inner.live_state.lock().unwrap();
            guard.ready = false;
            guard.starting = false;
            // Invalidate any in-flight bg connect thread so it bails out
            // instead of clobbering a future `start()`'s LiveState.
            guard.start_generation = guard.start_generation.wrapping_add(1);
            guard.last_start_error = None;
            self.inner.live_cond.notify_all();
        }

        let mut transport = self.inner.transport.lock().unwrap();
        if let TransportKind::Direct(writer) = &mut *transport {
            if writer.is_active() {
                let _ = writer.close();
            }
        }
        Ok(())
    }
}

// ── Drop ───────────────────────────────────────────────────────────────────

impl Drop for Inner {
    fn drop(&mut self) {
        match self.transport.get_mut().unwrap() {
            TransportKind::Direct(writer) => {
                if writer.is_active() {
                    let _ = writer.close();
                }
            }
            TransportKind::Push => {
                if let Some(sender) = self.live_state.get_mut().unwrap().push_sender.take() {
                    let _ = sender.close_blocking();
                }
            }
            TransportKind::External => {}
        }
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────

/// Round `max` down to the largest u64-aligned chunk size, but never below 8.
#[inline]
fn aligned_chunk_size(max: usize) -> usize {
    let aligned = max & !7usize;
    if aligned == 0 {
        8
    } else {
        aligned
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::thread;
    use std::time::Duration;

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

    // ── aligned_chunk_size ─────────────────────────────────────────────────

    #[test]
    fn aligned_chunk_size_rounds_down() {
        assert_eq!(aligned_chunk_size(131_072), 131_072); // already aligned
        assert_eq!(aligned_chunk_size(131_080), 131_080);
        assert_eq!(aligned_chunk_size(131_079), 131_072); // round down
        assert_eq!(aligned_chunk_size(15), 8);
        assert_eq!(aligned_chunk_size(7), 8); // never below 8
        assert_eq!(aligned_chunk_size(0), 8);
        assert_eq!(aligned_chunk_size(usize::MAX), usize::MAX & !7);
    }

    // ── External mode ──────────────────────────────────────────────────────

    #[test]
    fn external_flush_is_noop_and_ready() {
        let w = ZiskStreamWriter::unix_external("unix:///tmp/external".into());
        assert!(w.is_external());
        assert_eq!(w.uri(), "unix:///tmp/external");
        w.push_raw(b"some bytes");
        // External flush is a no-op — pending bytes stay where they are, but
        // the call returns Ok without blocking on a connection.
        assert!(w.flush().is_ok());
        // start/finish are no-ops on the wire side
        assert!(w.start().is_ok());
        assert!(w.finish().is_ok());
    }

    // ── Unix socket integration ────────────────────────────────────────────

    #[cfg(unix)]
    mod unix_tests {
        use super::*;
        use crate::io::{StreamRead, UnixSocketStreamReader};

        fn temp_path() -> String {
            // Lightweight unique path generator (no uuid dep in common).
            let pid = std::process::id();
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            format!("/tmp/zisk-zsw-{pid}-{nanos}.sock")
        }

        #[test]
        fn push_before_start_then_flush() {
            run_with_timeout("push_before_start_then_flush", TEST_TIMEOUT, || {
                let path = temp_path();
                let w = ZiskStreamWriter::unix_at(&path).unwrap();

                // Push BEFORE start: bytes go into pending and get drained on
                // the first peer connection.
                w.push_raw(b"abcdefgh"); // 8 bytes, u64-aligned
                w.push_raw(b"01234567"); // another 8

                w.start().unwrap();

                let mut reader = UnixSocketStreamReader::new(&path).unwrap();
                // bg connect thread sees the accept once we open; pending bytes drain.
                let msg = reader.next().unwrap().unwrap();
                // Both pushes were drained before ready=true, so they arrive
                // in one wire message (under the SOCK_SEQPACKET 128 KB limit).
                assert_eq!(&msg, b"abcdefgh01234567");

                w.finish().unwrap();
                reader.close().unwrap();
            });
        }

        #[test]
        fn flush_after_live() {
            run_with_timeout("flush_after_live", TEST_TIMEOUT, || {
                let path = temp_path();
                let w = ZiskStreamWriter::unix_at(&path).unwrap();
                w.start().unwrap();

                let mut reader = UnixSocketStreamReader::new(&path).unwrap();
                reader.open().unwrap();

                // Wait for live
                let deadline = std::time::Instant::now() + Duration::from_secs(5);
                loop {
                    if w.inner.live_state.lock().unwrap().ready {
                        break;
                    }
                    assert!(std::time::Instant::now() < deadline, "stream never went live");
                    thread::sleep(Duration::from_millis(10));
                }

                w.push_raw(&[0xAB; 16]);
                w.flush().unwrap();

                let msg = reader.next().unwrap().unwrap();
                assert_eq!(msg, vec![0xAB; 16]);

                w.finish().unwrap();
                reader.close().unwrap();
            });
        }

        #[test]
        fn multiple_flush_cycles_concatenate() {
            run_with_timeout("multiple_flush_cycles_concatenate", TEST_TIMEOUT, || {
                let path = temp_path();
                let w = ZiskStreamWriter::unix_at(&path).unwrap();
                w.start().unwrap();
                let mut reader = UnixSocketStreamReader::new(&path).unwrap();
                reader.open().unwrap();

                // Wait live
                let deadline = std::time::Instant::now() + Duration::from_secs(5);
                while !w.inner.live_state.lock().unwrap().ready {
                    assert!(std::time::Instant::now() < deadline);
                    thread::sleep(Duration::from_millis(10));
                }

                w.push_raw(b"AAAAAAAA"); // 8
                w.push_raw(b"BBBBBBBB"); // 8
                w.flush().unwrap();

                w.push_raw(b"CCCCCCCC"); // 8
                w.flush().unwrap();

                // First flush sends one combined chunk (16 bytes, well under max),
                // second flush sends another. Two wire messages.
                let m1 = reader.next().unwrap().unwrap();
                let m2 = reader.next().unwrap().unwrap();
                assert_eq!(&m1, b"AAAAAAAABBBBBBBB");
                assert_eq!(&m2, b"CCCCCCCC");

                w.finish().unwrap();
                reader.close().unwrap();
            });
        }

        #[test]
        fn flush_blocks_until_live() {
            run_with_timeout("flush_blocks_until_live", TEST_TIMEOUT, || {
                let path = temp_path();
                let w = ZiskStreamWriter::unix_at(&path).unwrap();
                w.start().unwrap();

                w.push_raw(b"blocked!"); // 8 bytes
                let w_clone = w.clone();
                let flushed = Arc::new(AtomicBool::new(false));
                let flushed_clone = flushed.clone();
                let flush_thread = thread::spawn(move || {
                    w_clone.flush().unwrap();
                    flushed_clone.store(true, Ordering::Release);
                });

                thread::sleep(Duration::from_millis(100));
                assert!(!flushed.load(Ordering::Acquire), "flush should still be blocking");

                let mut reader = UnixSocketStreamReader::new(&path).unwrap();
                reader.open().unwrap();

                flush_thread.join().unwrap();
                assert!(flushed.load(Ordering::Acquire));

                let msg = reader.next().unwrap().unwrap();
                assert_eq!(&msg, b"blocked!");

                w.finish().unwrap();
                reader.close().unwrap();
            });
        }

        #[test]
        fn finish_makes_stream_not_ready() {
            run_with_timeout("finish_makes_stream_not_ready", TEST_TIMEOUT, || {
                let path = temp_path();
                let w = ZiskStreamWriter::unix_at(&path).unwrap();
                w.start().unwrap();
                let mut reader = UnixSocketStreamReader::new(&path).unwrap();
                reader.open().unwrap();

                let deadline = std::time::Instant::now() + Duration::from_secs(5);
                while !w.inner.live_state.lock().unwrap().ready {
                    assert!(std::time::Instant::now() < deadline);
                    thread::sleep(Duration::from_millis(10));
                }
                assert!(w.inner.live_state.lock().unwrap().ready);

                w.finish().unwrap();
                assert!(!w.inner.live_state.lock().unwrap().ready);

                reader.close().unwrap();
            });
        }

        #[test]
        fn start_reuse_across_jobs() {
            run_with_timeout("start_reuse_across_jobs", TEST_TIMEOUT, || {
                let path = temp_path();
                let w = ZiskStreamWriter::unix_at(&path).unwrap();

                // === Job 1 ===
                w.push_raw(b"FIRSTRUN"); // 8 bytes
                w.start().unwrap();
                let mut r1 = UnixSocketStreamReader::new(&path).unwrap();
                let msg = r1.next().unwrap().unwrap();
                assert_eq!(&msg, b"FIRSTRUN");
                w.finish().unwrap();
                r1.close().unwrap();

                // === Job 2: same writer, new transport instance ===
                w.push_raw(b"SECNDRUN");
                w.start().unwrap();
                let mut r2 = UnixSocketStreamReader::new(&path).unwrap();
                let msg = r2.next().unwrap().unwrap();
                assert_eq!(&msg, b"SECNDRUN");
                w.finish().unwrap();
                r2.close().unwrap();
            });
        }

        #[test]
        fn large_payload_chunked_at_aligned_boundaries() {
            run_with_timeout("large_payload_chunked_at_aligned_boundaries", TEST_TIMEOUT, || {
                let path = temp_path();
                let w = ZiskStreamWriter::unix_at(&path).unwrap();

                // 300 KB: must split into multiple SOCK_SEQPACKET messages
                // (limit is 128 KB). 300 KB is u64-aligned.
                let payload: Vec<u8> = (0..300 * 1024).map(|i| (i & 0xff) as u8).collect();
                w.push_raw(&payload);
                w.start().unwrap();

                let mut reader = UnixSocketStreamReader::new(&path).unwrap();

                // Reassemble across messages and verify byte-equality.
                let mut received = Vec::with_capacity(payload.len());
                while received.len() < payload.len() {
                    let msg = reader.next().unwrap().unwrap();
                    // Every intermediate chunk must be u64-aligned. The final
                    // chunk's size depends on the remainder; in this test the
                    // total is u64-aligned so all chunks are.
                    assert_eq!(msg.len() % 8, 0, "non-aligned chunk on the wire");
                    received.extend_from_slice(&msg);
                }
                assert_eq!(received, payload);

                w.finish().unwrap();
                reader.close().unwrap();
            });
        }
    }

    // ── Byte-position retry (mock writer) ──────────────────────────────────

    /// A mock writer that succeeds for the first N writes, then fails on the (N+1)th.
    struct FailingWriter {
        call_count: AtomicUsize,
        fail_on: usize,
        max_msg: usize,
        active: AtomicBool,
    }

    impl FailingWriter {
        fn new(fail_on: usize, max_msg: usize) -> Self {
            Self {
                call_count: AtomicUsize::new(0),
                fail_on,
                max_msg,
                active: AtomicBool::new(true),
            }
        }
    }

    impl StreamWrite for FailingWriter {
        fn open(&mut self) -> Result<()> {
            self.active.store(true, Ordering::Relaxed);
            Ok(())
        }
        fn write(&mut self, item: &[u8]) -> Result<usize> {
            let n = self.call_count.fetch_add(1, Ordering::Relaxed);
            if n >= self.fail_on {
                Err(anyhow::anyhow!("mock write failure on call {n}"))
            } else {
                Ok(item.len())
            }
        }
        fn flush(&mut self) -> Result<()> {
            Ok(())
        }
        fn close(&mut self) -> Result<()> {
            self.active.store(false, Ordering::Relaxed);
            Ok(())
        }
        fn is_active(&self) -> bool {
            self.active.load(Ordering::Relaxed)
        }
        fn max_message_size(&self) -> usize {
            self.max_msg
        }
    }

    #[test]
    fn flush_error_keeps_only_unsent_tail() {
        // 8-byte chunks, fail on the 3rd write.
        let writer = FailingWriter::new(2, 8);
        let w = ZiskStreamWriter::from_writer(Box::new(writer), "mock://test".into());

        // Skip the start handshake: mark live by hand.
        w.inner.live_state.lock().unwrap().ready = true;

        // 5 chunks worth (40 bytes). Writes 0 and 1 succeed (16 bytes sent),
        // write 2 fails. Pending should retain the unsent 24 bytes.
        let payload: Vec<u8> = (0..40u8).collect();
        w.push_raw(&payload);

        let err = w.flush();
        assert!(err.is_err(), "flush should propagate the mock failure");

        let pending = w.inner.pending.lock().unwrap();
        assert_eq!(pending.len(), 40 - 16, "only successfully-sent bytes drained");
        assert_eq!(&pending[..], &payload[16..], "unsent tail preserved exactly");
    }

    #[test]
    fn flush_error_with_no_progress_keeps_everything() {
        // Fail on the very first write.
        let writer = FailingWriter::new(0, 8);
        let w = ZiskStreamWriter::from_writer(Box::new(writer), "mock://test".into());
        w.inner.live_state.lock().unwrap().ready = true;

        let payload: Vec<u8> = (0..16u8).collect();
        w.push_raw(&payload);

        assert!(w.flush().is_err());
        let pending = w.inner.pending.lock().unwrap();
        assert_eq!(&pending[..], &payload[..], "no bytes consumed on first-write failure");
    }

    // ── Push transport (mock BytesPushSender) ──────────────────────────────

    type RecordedChunks = Arc<Mutex<Vec<Vec<u8>>>>;
    type ClosedFlag = Arc<AtomicBool>;

    /// Mock push sender: records every chunk and supports controlled failure.
    struct MockPushSender {
        sent: RecordedChunks,
        closed: ClosedFlag,
        fail_after: AtomicUsize,
        call_count: AtomicUsize,
    }

    impl MockPushSender {
        fn new() -> (Box<Self>, RecordedChunks, ClosedFlag) {
            let sent: RecordedChunks = Arc::new(Mutex::new(Vec::new()));
            let closed: ClosedFlag = Arc::new(AtomicBool::new(false));
            let s = Box::new(Self {
                sent: Arc::clone(&sent),
                closed: Arc::clone(&closed),
                fail_after: AtomicUsize::new(usize::MAX),
                call_count: AtomicUsize::new(0),
            });
            (s, sent, closed)
        }

        fn fail_after(self: Box<Self>, n: usize) -> Box<Self> {
            self.fail_after.store(n, Ordering::Relaxed);
            self
        }
    }

    impl BytesPushSender for MockPushSender {
        fn send_blocking(&self, data: Vec<u8>) -> Result<()> {
            let n = self.call_count.fetch_add(1, Ordering::Relaxed);
            if n >= self.fail_after.load(Ordering::Relaxed) {
                return Err(anyhow::anyhow!("mock push failure on call {n}"));
            }
            self.sent.lock().unwrap().push(data);
            Ok(())
        }

        fn close_blocking(self: Box<Self>) -> Result<()> {
            self.closed.store(true, Ordering::Relaxed);
            Ok(())
        }
    }

    #[test]
    fn push_constructor_and_accessors() {
        let w = ZiskStreamWriter::push("grpc://test".into());
        assert_eq!(w.uri(), "grpc://test");
        assert!(w.is_push());
        assert!(!w.is_external());
    }

    #[test]
    fn push_flush_sends_chunks_through_sender() {
        let w = ZiskStreamWriter::push("grpc://test".into());
        let (sender, recorded, closed) = MockPushSender::new();

        // Push bytes BEFORE the sender is set — they sit in pending until ready.
        let payload: Vec<u8> = (0..200_000u32).map(|i| (i & 0xff) as u8).collect();
        w.push_raw(&payload);

        // Inject sender → marks ready and wakes any flushers.
        w.set_push_sender(sender);

        w.flush().unwrap();

        // Verify the chunks reassemble exactly to the original payload.
        let chunks = recorded.lock().unwrap();
        let received: Vec<u8> = chunks.iter().flatten().copied().collect();
        assert_eq!(received, payload);

        // Every intermediate chunk must be u64-aligned.
        for chunk in chunks.iter().take(chunks.len().saturating_sub(1)) {
            assert_eq!(chunk.len() % 8, 0, "non-aligned chunk on push wire");
        }

        // Sender shouldn't have been closed yet.
        assert!(!closed.load(Ordering::Relaxed));

        // finish() closes the sender.
        w.finish().unwrap();
        assert!(closed.load(Ordering::Relaxed), "finish() should call close_blocking");
    }

    #[test]
    fn push_flush_blocks_until_sender_set() {
        let w = ZiskStreamWriter::push("grpc://test".into());
        w.push_raw(b"blocked!");

        let w_clone = w.clone();
        let flushed = Arc::new(AtomicBool::new(false));
        let flushed_clone = flushed.clone();
        let flush_thread = thread::spawn(move || {
            w_clone.flush().unwrap();
            flushed_clone.store(true, Ordering::Release);
        });

        thread::sleep(Duration::from_millis(50));
        assert!(!flushed.load(Ordering::Acquire), "flush should block until sender set");

        let (sender, recorded, _) = MockPushSender::new();
        w.set_push_sender(sender);

        flush_thread.join().unwrap();
        assert!(flushed.load(Ordering::Acquire));
        assert_eq!(recorded.lock().unwrap().concat(), b"blocked!");
    }

    #[test]
    fn push_flush_error_keeps_unsent_tail() {
        let w = ZiskStreamWriter::push("grpc://test".into());
        // Fail on the 3rd send — first two (64 KB each) succeed, then we lose.
        let (sender, recorded, _) = MockPushSender::new();
        let sender = sender.fail_after(2);
        w.set_push_sender(sender);

        // 200 KB → 4 chunks of 64 KB at the writer's chunk size.
        let payload = vec![0xAB_u8; 200 * 1024];
        w.push_raw(&payload);

        assert!(w.flush().is_err());

        let sent_total: usize = recorded.lock().unwrap().iter().map(|c| c.len()).sum();
        let pending_len = w.inner.pending.lock().unwrap().len();
        assert_eq!(
            sent_total + pending_len,
            payload.len(),
            "no bytes lost: sent + pending = total"
        );
        assert_eq!(sent_total, 2 * 64 * 1024, "two chunks succeeded before failure");
        assert_eq!(pending_len, payload.len() - sent_total, "remainder retained for retry");
    }

    #[test]
    fn push_start_clears_old_sender() {
        let w = ZiskStreamWriter::push("grpc://test".into());
        let (sender1, _recorded1, closed1) = MockPushSender::new();
        w.set_push_sender(sender1);
        assert!(w.inner.live_state.lock().unwrap().ready);

        // start() between jobs: drops the old sender (closing it) and clears ready.
        w.start().unwrap();
        assert!(!w.inner.live_state.lock().unwrap().ready);
        assert!(closed1.load(Ordering::Relaxed), "old sender should be closed");

        // New sender for the next job.
        let (sender2, recorded2, _) = MockPushSender::new();
        w.set_push_sender(sender2);
        w.push_raw(b"AAAAAAAA");
        w.flush().unwrap();
        assert_eq!(recorded2.lock().unwrap().concat(), b"AAAAAAAA");
    }
}
