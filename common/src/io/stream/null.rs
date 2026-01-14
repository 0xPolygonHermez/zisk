use super::StreamRead;

use anyhow::Result;

pub struct NullStreamReader {
    active: bool,
}

impl Default for NullStreamReader {
    fn default() -> Self {
        NullStreamReader::new()
    }
}

impl NullStreamReader {
    /// Create a new NullStreamReader
    pub fn new() -> Self {
        NullStreamReader { active: false }
    }
}

impl StreamRead for NullStreamReader {
    /// Open/initialize the stream for reading
    fn open(&mut self) -> Result<()> {
        self.active = true;
        Ok(())
    }

    /// Read the next item from the stream
    fn next(&mut self) -> Result<Option<Vec<u8>>> {
        Ok(None)
    }

    /// Close the stream
    fn close(&mut self) -> Result<()> {
        self.active = false;
        Ok(())
    }

    /// Check if the stream is currently active
    fn is_active(&self) -> bool {
        self.active
    }
}
