use anyhow::Result;

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

    /// Non-blocking peer-connection poll. Returns `true` once writes can
    /// proceed. The `ZiskStreamWriter` bg thread polls this with brief lock
    /// acquisitions, releasing `transport.lock()` between polls so callers
    /// like `finish()` aren't blocked behind a long wait. Override for
    /// transports where `open()` doesn't block until the peer connects.
    fn is_client_connected(&mut self) -> bool {
        true
    }

    /// Blocking variant kept for transports that need to drive accept inside
    /// `write()` (QUIC). Not used by `ZiskStreamWriter`'s bg thread anymore.
    fn wait_for_connection(&mut self) -> Result<()> {
        Ok(())
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
