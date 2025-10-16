use std::io::{stdin, Read};

use crate::io::ZiskStdin;

/// A standard input implementation of ZiskStdin that reads from stdin.
pub struct ZiskStandardStdin {
    buffer: Vec<u8>,
    loaded: bool,
}

impl ZiskStandardStdin {
    /// Create a new ZiskStandardStdin.
    pub fn new() -> Self {
        ZiskStandardStdin { buffer: Vec::new(), loaded: false }
    }

    /// Ensure data is loaded from stdin.
    fn ensure_loaded(&mut self) {
        if !self.loaded {
            stdin().read_to_end(&mut self.buffer).expect("Failed to read from stdin");
            self.loaded = true;
        }
    }
}

impl Default for ZiskStandardStdin {
    fn default() -> Self {
        Self::new()
    }
}

impl ZiskStdin for ZiskStandardStdin {
    fn read(&mut self) -> Vec<u8> {
        self.ensure_loaded();
        self.buffer.clone()
    }

    fn read_slice(&mut self, slice: &mut [u8]) {
        self.ensure_loaded();
        let len = slice.len().min(self.buffer.len());
        slice[..len].copy_from_slice(&self.buffer[..len]);
    }

    fn read_into(&mut self, buffer: &mut [u8]) {
        self.read_slice(buffer);
    }

    fn write_serialized(&mut self, _data: &[u8]) {
        panic!("Write operations are not supported for ZiskStandardStdin");
    }

    fn write_bytes(&mut self, _data: &[u8]) {
        panic!("Write operations are not supported for ZiskStandardStdin");
    }
}
