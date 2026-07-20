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

use crate::error::{Result, StreamError};

use crate::{StreamWrite, CONNECT_DEADLINE};

#[cfg(feature = "quic")]
use crate::QuicStreamWriter;
#[cfg(unix)]
use crate::UnixSocketStreamWriter;

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
/// `flush()` only ever wait one tick on the `state` lock between polls (the bg
/// thread releases the lock while it sleeps for this interval).
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

/// Mutex-free tag mirroring `TransportKind`'s discriminant. `is_external` /
/// `is_push` are called from hot paths; caching the discriminant here lets them
/// answer without taking the `state` lock (which the bg connect thread and
/// `flush()` also contend on).
#[derive(Copy, Clone, PartialEq)]
enum TransportTag {
    Direct,
    External,
    Push,
}

// ── State ────────────────────────────────────────────────────────────────

/// All mutable lifecycle state, behind ONE mutex.
///
/// The transport, the pending buffer, and the readiness/error/generation
/// signals are deliberately kept under a single lock so that every operation
/// — `start()`, `finish()`, `flush()`, and the background connect thread —
/// sees a consistent snapshot and mutates atomically. An earlier design split
/// these across three separate mutexes (`transport`, `pending`, `live_state`)
/// and every gap between acquiring one and the next was a check-then-act race
/// (a concurrent `finish()`/`start()` tearing down or rebinding the transport
/// between a peer-connected check and the drain, a stale `start()` reopening a
/// transport `finish()` had just closed, etc.). One lock removes that entire
/// class: there is no second lock to be inconsistent with.
struct State {
    /// The transport (owned Direct writer, External marker, or Push).
    transport: TransportKind,
    /// Bytes buffered by `push_raw`, awaiting delivery on connect or `flush()`.
    pending: Vec<u8>,
    /// `true` once the peer has connected and any pre-buffered bytes have been
    /// drained (Direct), or a push sender is set (Push). `flush()` blocks on it.
    ready: bool,
    /// Monotonic counter bumped by `start()` and `finish()`. The background
    /// connect thread captures it at spawn and, on each poll iteration, bails
    /// out if it no longer matches — that is how a stale thread from a superseded
    /// `start()` is neutralized. Because it is read and bumped under the *same*
    /// (single) lock that mutates the transport, there is no window in which the
    /// generation and the transport can disagree.
    generation: u64,
    /// Set when the bg thread's start handshake (connect-poll or initial drain)
    /// failed. `flush()` returns it so waiters don't block forever after a
    /// connection timeout. Cleared by the next `start()`/`finish()`.
    last_start_error: Option<String>,
    /// Active push sender for the current job. Set/cleared in tandem with
    /// `ready` for [`TransportKind::Push`]; always `None` for other kinds.
    push_sender: Option<Box<dyn BytesPushSender>>,
}

// ── Inner shared state ─────────────────────────────────────────────────────

struct Inner {
    /// The single lock guarding all mutable state (see [`State`]).
    state: Mutex<State>,
    /// Signalled whenever `ready`/`last_start_error` changes, so blocked
    /// `flush()` callers wake up.
    cond: Condvar,
    /// Lock-free mirror of the transport discriminant, so `is_external()` /
    /// `is_push()` need not take `state`.
    tag: TransportTag,
    /// Immutable transport URI (metadata for callers; never parsed here).
    uri: String,
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
                state: Mutex::new(State {
                    transport: TransportKind::Direct(writer),
                    pending: Vec::new(),
                    ready: false,
                    generation: 0,
                    last_start_error: None,
                    push_sender: None,
                }),
                cond: Condvar::new(),
                tag: TransportTag::Direct,
                uri,
            }),
        }
    }

    /// Externally-managed transport. The writer carries only the URI; pushes
    /// are buffered and `flush()` is a no-op (some other process owns the
    /// socket and writes to it directly).
    pub fn unix_external(uri: String) -> Self {
        Self {
            inner: Arc::new(Inner {
                state: Mutex::new(State {
                    transport: TransportKind::External,
                    pending: Vec::new(),
                    ready: true,
                    generation: 0,
                    last_start_error: None,
                    push_sender: None,
                }),
                cond: Condvar::new(),
                tag: TransportTag::External,
                uri,
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
    #[cfg(feature = "quic")]
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
                state: Mutex::new(State {
                    transport: TransportKind::Push,
                    pending: Vec::new(),
                    ready: false,
                    generation: 0,
                    last_start_error: None,
                    push_sender: None,
                }),
                cond: Condvar::new(),
                tag: TransportTag::Push,
                uri,
            }),
        }
    }

    // ── Accessors ──────────────────────────────────────────────────────────

    /// Get the URI associated with this writer.
    pub fn uri(&self) -> &str {
        &self.inner.uri
    }

    /// Get a flag indicating whether this writer is using an externally-managed transport.
    pub fn is_external(&self) -> bool {
        self.inner.tag == TransportTag::External
    }

    /// Get a flag indicating whether this writer is using a push transport.
    pub fn is_push(&self) -> bool {
        self.inner.tag == TransportTag::Push
    }

    /// `true` after `start()` (and, for Push, `set_push_sender`) has succeeded.
    /// Useful for callers waiting until a flush would not block; primarily for
    /// tests and observability.
    pub fn is_ready(&self) -> bool {
        self.inner.state.lock().unwrap().ready
    }

    /// Inject the push sender for a [`TransportKind::Push`] writer. Marks the
    /// stream live and wakes any flushers blocked on the condvar.
    ///
    /// Calling this on a non-Push writer is a no-op.
    pub fn set_push_sender(&self, sender: Box<dyn BytesPushSender>) {
        if !self.is_push() {
            return;
        }
        {
            let mut guard = self.inner.state.lock().unwrap();
            guard.push_sender = Some(sender);
            guard.ready = true;
        }
        self.inner.cond.notify_all();
    }

    // ── Write / flush ──────────────────────────────────────────────────────

    /// Append raw bytes to the pending buffer. Bytes are sent verbatim on the
    /// next `flush()`, in the order they were pushed.
    pub fn push_raw(&self, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        self.inner.state.lock().unwrap().pending.extend_from_slice(data);
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
        // and pre-buffered bytes (if any) have been drained, then drain any
        // bytes pushed since. All of it happens under the one `state` lock, so
        // a concurrent `finish()`/`start()` cannot tear down or rebind the
        // transport between the ready-wait and the write.
        let mut guard = self.inner.state.lock().unwrap();
        while !guard.ready {
            if let Some(err) = &guard.last_start_error {
                return Err(StreamError::Transport(format!(
                    "ZiskStreamWriter start failed: {}",
                    err
                )));
            }
            guard =
                self.inner.cond.wait_timeout(guard, std::time::Duration::from_secs(5)).unwrap().0;
        }

        let state = &mut *guard;
        let TransportKind::Direct(writer) = &mut state.transport else {
            // Already returned for External / Push above.
            unreachable!()
        };
        drain_into(&mut **writer, &mut state.pending)
    }

    /// Push-transport flush. Holds `state` for the duration of the chunk loop
    /// so concurrent `start()` / `finish()` / `set_push_sender()` can't tear
    /// down the sender mid-flight (matches the pre-refactor SDK gRPC behavior).
    ///
    /// Blocks until the sender is injected via `set_push_sender()`, but only up
    /// to [`CONNECT_DEADLINE`] — the same bound the Direct path applies to the
    /// peer-connect wait. Without it a producer parked here would hang forever
    /// if the sender never arrives (e.g. the job submission that would inject it
    /// failed after `start()`), since nothing on the Push path sets
    /// `last_start_error`.
    fn flush_push(&self) -> Result<()> {
        self.flush_push_until(std::time::Instant::now() + CONNECT_DEADLINE)
    }

    /// [`flush_push`](Self::flush_push) with an explicit sender-wait deadline
    /// (parameterized so tests can exercise the timeout without waiting
    /// [`CONNECT_DEADLINE`]).
    fn flush_push_until(&self, deadline: std::time::Instant) -> Result<()> {
        let mut guard = self.inner.state.lock().unwrap();
        while !guard.ready {
            if let Some(err) = &guard.last_start_error {
                return Err(StreamError::Transport(format!(
                    "ZiskStreamWriter start failed: {}",
                    err
                )));
            }
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return Err(StreamError::Transport(format!(
                    "Push transport: timed out waiting for sender on {}",
                    self.inner.uri
                )));
            }
            guard = self.inner.cond.wait_timeout(guard, remaining).unwrap().0;
        }

        let state = &mut *guard;
        if state.pending.is_empty() {
            return Ok(());
        }

        let sender = state.push_sender.as_ref().ok_or_else(|| {
            StreamError::Transport("Push transport: ready=true but sender not set".to_string())
        })?;

        let chunk_size = aligned_chunk_size(PUSH_DEFAULT_CHUNK_SIZE);

        let mut sent = 0;
        while sent < state.pending.len() {
            let take = std::cmp::min(chunk_size, state.pending.len() - sent);
            match sender.send_blocking(state.pending[sent..sent + take].to_vec()) {
                Ok(_) => sent += take,
                Err(e) => {
                    state.pending.drain(..sent);
                    return Err(e);
                }
            }
        }
        state.pending.clear();
        Ok(())
    }

    /// Discard buffered bytes that have not yet been sent.
    pub fn reset(&self) {
        self.inner.state.lock().unwrap().pending.clear();
    }

    // ── Lifecycle ──────────────────────────────────────────────────────────

    /// Open the transport and spawn a background thread that waits for the
    /// peer to connect, drains any pre-buffered bytes, and marks the stream
    /// live. Idempotent across reuse: if the transport is already active, it
    /// is closed and reopened first.
    pub fn start(&self) -> Result<()> {
        if self.is_external() {
            // External transports are live by construction.
            let mut guard = self.inner.state.lock().unwrap();
            guard.ready = true;
            self.inner.cond.notify_all();
            return Ok(());
        }

        if self.is_push() {
            // Push: tear down any previous sender; wait for a fresh one to
            // arrive via `set_push_sender()` before going ready again.
            let old_sender = {
                let mut guard = self.inner.state.lock().unwrap();
                guard.ready = false;
                guard.push_sender.take()
            };
            if let Some(sender) = old_sender {
                let _ = sender.close_blocking();
            }
            return Ok(());
        }

        // Direct transport. Every `start()` unconditionally supersedes any
        // previous one and rebinds — there is deliberately no "a start is
        // already in flight, so no-op" fast path (such a path raced sequential
        // reuse: a prior job's bg thread could still be between delivering data
        // and clearing a flag, and a `start()` for the next job that hit that
        // flag would skip the rebind, leaving the next peer's connection to
        // queue on a stale listener whose one-shot accept thread had exited —
        // hanging forever).
        //
        // The generation bump AND the transport teardown/rebind happen in ONE
        // `state` lock hold, so the whole transition is atomic with respect to
        // any concurrent `start()`/`finish()` and with respect to a superseded
        // bg thread (which re-checks the generation under this same lock).
        let my_gen = {
            let mut guard = self.inner.state.lock().unwrap();
            guard.generation = guard.generation.wrapping_add(1);
            // Starting fresh: flushers must wait for the new bg thread to drain.
            guard.ready = false;
            guard.last_start_error = None;
            let my_gen = guard.generation;

            let TransportKind::Direct(writer) = &mut guard.transport else { unreachable!() };
            if writer.is_active() {
                let _ = writer.close();
            }
            if let Err(e) = writer.open() {
                // Surface the failure to any flusher already blocked on the
                // condvar — otherwise they'd wait forever, since `ready` never
                // flips and no bg thread will be spawned to set an error.
                guard.last_start_error = Some(e.to_string());
                self.inner.cond.notify_all();
                return Err(e);
            }
            my_gen
        };

        // Background thread: poll for the peer to connect, then drain the
        // pre-buffered bytes and mark the stream ready. It takes `state` only
        // briefly per poll (releasing it while it sleeps between polls, so
        // lifecycle callers never wait more than one poll on the lock), and on
        // connect it does the generation check, the drain, and the ready flip
        // all in a SINGLE lock hold — no concurrent `start()`/`finish()` can
        // slip in between. If a newer `start()`/`finish()` superseded us, the
        // generation no longer matches and we simply exit (that caller owns the
        // state and notifies waiters itself).
        //
        // NOTE: the drain holds `state` for its duration; for pending payloads
        // larger than the transport chunk size this spans multiple `write()`
        // calls (tens of ms on Unix, up to hundreds on QUIC), during which
        // lifecycle callers block — same as before, now under the one lock.
        let inner = Arc::clone(&self.inner);
        std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + CONNECT_DEADLINE;
            loop {
                {
                    let mut guard = inner.state.lock().unwrap();
                    if guard.generation != my_gen {
                        return; // superseded; the newer call owns state + notifies
                    }
                    let state = &mut *guard;
                    let TransportKind::Direct(writer) = &mut state.transport else {
                        state.last_start_error =
                            Some("transport closed before peer connected".to_string());
                        inner.cond.notify_all();
                        return;
                    };
                    if writer.is_client_connected() {
                        match drain_into(&mut **writer, &mut state.pending) {
                            Ok(()) => state.ready = true,
                            Err(e) => {
                                tracing::error!("ZiskStreamWriter start failed: {}", e);
                                state.last_start_error = Some(e.to_string());
                            }
                        }
                        inner.cond.notify_all();
                        return;
                    }
                    if std::time::Instant::now() >= deadline {
                        state.last_start_error =
                            Some(format!("Timed out waiting for peer to connect to {}", inner.uri));
                        inner.cond.notify_all();
                        return;
                    }
                }
                std::thread::sleep(CONNECT_POLL_INTERVAL);
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
                let mut guard = self.inner.state.lock().unwrap();
                guard.ready = false;
                guard.push_sender.take()
            };
            if let Some(sender) = old_sender {
                let res = sender.close_blocking();
                return res;
            }
            return Ok(());
        }

        // Bump the generation (invalidating any in-flight bg connect thread) and
        // close the transport in ONE lock hold, so the teardown is atomic w.r.t.
        // a concurrent `start()` and a superseded bg thread cannot reopen or
        // drain in between.
        let mut guard = self.inner.state.lock().unwrap();
        guard.ready = false;
        guard.generation = guard.generation.wrapping_add(1);
        guard.last_start_error = None;
        if let TransportKind::Direct(writer) = &mut guard.transport {
            if writer.is_active() {
                let _ = writer.close();
            }
        }
        self.inner.cond.notify_all();
        Ok(())
    }
}

// ── Drop ───────────────────────────────────────────────────────────────────

impl Drop for Inner {
    fn drop(&mut self) {
        let state = self.state.get_mut().unwrap();
        match &mut state.transport {
            TransportKind::Direct(writer) => {
                if writer.is_active() {
                    let _ = writer.close();
                }
            }
            TransportKind::Push => {
                if let Some(sender) = state.push_sender.take() {
                    let _ = sender.close_blocking();
                }
            }
            TransportKind::External => {}
        }
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────

/// Drain `pending` into a Direct `writer` in u64-aligned wire chunks.
///
/// Shared by `flush()` and the background connect thread. On a partial-write
/// error the successfully-sent prefix is dropped from `pending` and the unsent
/// tail is retained for the next attempt; on success `pending` is cleared.
fn drain_into(writer: &mut dyn StreamWrite, pending: &mut Vec<u8>) -> Result<()> {
    if pending.is_empty() {
        return Ok(());
    }
    let chunk_size = aligned_chunk_size(writer.max_message_size());
    let mut sent = 0;
    while sent < pending.len() {
        let take = std::cmp::min(chunk_size, pending.len() - sent);
        match writer.write(&pending[sent..sent + take]) {
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
        use crate::{StreamRead, UnixSocketStreamReader};

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
                    if w.inner.state.lock().unwrap().ready {
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
                while !w.inner.state.lock().unwrap().ready {
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
                while !w.inner.state.lock().unwrap().ready {
                    assert!(std::time::Instant::now() < deadline);
                    thread::sleep(Duration::from_millis(10));
                }
                assert!(w.inner.state.lock().unwrap().ready);

                w.finish().unwrap();
                assert!(!w.inner.state.lock().unwrap().ready);

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

        /// Regression for the sequential-reuse hang fixed in `start()` (see the
        /// rationale there). Reusing a writer for a new job without `finish()`
        /// between jobs — and without waiting for `is_ready()` — is the sequence
        /// that used to race the bg thread's epilogue and skip the rebind. The
        /// race is timing-dependent, so the loop repeats it to make the hang
        /// surface reliably rather than ~1-in-N.
        #[test]
        fn start_reuse_without_finish_never_hangs() {
            run_with_timeout("start_reuse_without_finish", TEST_TIMEOUT, || {
                let path = temp_path();
                let w = ZiskStreamWriter::unix_at(&path).unwrap();

                for i in 0u32..40 {
                    // Unique 8-byte, u64-aligned payload encoding the job index.
                    let payload = (i as u64).to_le_bytes();
                    w.push_raw(&payload);
                    w.start().unwrap();

                    let mut reader = UnixSocketStreamReader::new(&path).unwrap();
                    let msg = reader.next().unwrap().unwrap();
                    assert_eq!(&msg, &payload, "job {i} delivered wrong/no data");
                    reader.close().unwrap();
                }

                w.finish().unwrap();
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
                Err(StreamError::Invalid(format!("mock write failure on call {n}")))
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
        w.inner.state.lock().unwrap().ready = true;

        // 5 chunks worth (40 bytes). Writes 0 and 1 succeed (16 bytes sent),
        // write 2 fails. Pending should retain the unsent 24 bytes.
        let payload: Vec<u8> = (0..40u8).collect();
        w.push_raw(&payload);

        let err = w.flush();
        assert!(err.is_err(), "flush should propagate the mock failure");

        let guard = w.inner.state.lock().unwrap();
        let pending = &guard.pending;
        assert_eq!(pending.len(), 40 - 16, "only successfully-sent bytes drained");
        assert_eq!(&pending[..], &payload[16..], "unsent tail preserved exactly");
    }

    #[test]
    fn flush_error_with_no_progress_keeps_everything() {
        // Fail on the very first write.
        let writer = FailingWriter::new(0, 8);
        let w = ZiskStreamWriter::from_writer(Box::new(writer), "mock://test".into());
        w.inner.state.lock().unwrap().ready = true;

        let payload: Vec<u8> = (0..16u8).collect();
        w.push_raw(&payload);

        assert!(w.flush().is_err());
        let guard = w.inner.state.lock().unwrap();
        let pending = &guard.pending;
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
                return Err(StreamError::Invalid(format!("mock push failure on call {n}")));
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
    fn push_flush_times_out_if_sender_never_set() {
        // A Push flush must not hang forever when the sender is never injected
        // (e.g. the job submission that would call set_push_sender() failed after
        // start()). It waits up to the deadline, then returns an error — the same
        // bounded-wait guarantee the Direct path has via CONNECT_DEADLINE.
        let w = ZiskStreamWriter::push("grpc://test".into());
        w.push_raw(b"stranded");

        let start = std::time::Instant::now();
        let res = w.flush_push_until(start + Duration::from_millis(100));
        assert!(res.is_err(), "flush_push must time out, not hang, without a sender");
        assert!(start.elapsed() >= Duration::from_millis(100), "should wait the full deadline");
        assert!(start.elapsed() < Duration::from_secs(5), "should not block far past the deadline");
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
        let pending_len = w.inner.state.lock().unwrap().pending.len();
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
        assert!(w.inner.state.lock().unwrap().ready);

        // start() between jobs: drops the old sender (closing it) and clears ready.
        w.start().unwrap();
        assert!(!w.inner.state.lock().unwrap().ready);
        assert!(closed1.load(Ordering::Relaxed), "old sender should be closed");

        // New sender for the next job.
        let (sender2, recorded2, _) = MockPushSender::new();
        w.set_push_sender(sender2);
        w.push_raw(b"AAAAAAAA");
        w.flush().unwrap();
        assert_eq!(recorded2.lock().unwrap().concat(), b"AAAAAAAA");
    }
}
