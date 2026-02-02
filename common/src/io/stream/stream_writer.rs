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
}
