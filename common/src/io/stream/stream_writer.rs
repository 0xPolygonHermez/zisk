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

    /// Block until the stream is ready to accept writes.
    ///
    /// For transports where `open()` is non-blocking (e.g. Unix socket), the first
    /// write may fail with a "no client connected" error until the remote peer connects.
    /// Override this method to busy-wait / sleep until the peer is ready.
    ///
    /// The default implementation is a no-op, suitable for QUIC which already blocks
    /// in `open()` until the peer connects.
    fn wait_for_connection(&mut self) -> Result<()> {
        Ok(())
    }
}
