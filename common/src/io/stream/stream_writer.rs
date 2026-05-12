use anyhow::Result;

/// Upper bound on how long any peer-connect poll loop waits before giving up.
/// Shared between [`ZiskStreamWriter`](crate::io::ZiskStreamWriter)'s background
/// connect thread and transports that drive accept inside their own `write()`
/// (currently only QUIC).
pub const CONNECT_DEADLINE: std::time::Duration = std::time::Duration::from_secs(60);

/// Core trait for stream writing operations
pub trait StreamWrite: Send + 'static {
    /// Open/initialize the stream for writing
    fn open(&mut self) -> Result<()>;

    /// Write data to the stream, returns the number of bytes written
    fn write(&mut self, item: &[u8]) -> Result<usize>;

    /// Flush any buffered data
    fn flush(&mut self) -> Result<()>;

    /// Close the stream
    fn close(&mut self) -> Result<()>;

    /// Check if the stream is currently active
    fn is_active(&self) -> bool;

    /// Non-blocking peer-connection poll. Must return quickly:
    /// `ZiskStreamWriter`'s connect thread calls this while holding
    /// `transport.lock()`. Transports with deferred accept (Unix, QUIC) MUST
    /// override; the default returns `is_active()` for transports whose
    /// `open()` synchronously establishes the connection.
    fn is_client_connected(&mut self) -> bool {
        self.is_active()
    }

    /// Maximum bytes that can be sent in a single `write()` call.
    ///
    /// `flush()` uses this to split large frames automatically so callers
    /// never need to know about transport-level size constraints.
    /// Defaults to `usize::MAX` (no limit).
    fn max_message_size(&self) -> usize {
        usize::MAX
    }
}
