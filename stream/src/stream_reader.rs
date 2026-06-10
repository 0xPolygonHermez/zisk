use anyhow::Result;

/// Core trait for stream reading operations
pub trait StreamRead: Send + 'static {
    /// Open/initialize the stream for reading
    fn open(&mut self) -> Result<()>;

    /// Read the next item from the stream
    /// Returns None when the stream is finished
    fn next(&mut self) -> Result<Option<Vec<u8>>>;

    /// Close the stream
    fn close(&mut self) -> Result<()>;

    /// Check if the stream is currently active
    fn is_active(&self) -> bool;
}
