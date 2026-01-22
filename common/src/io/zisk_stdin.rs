use crate::io::{ZiskFileStdin, ZiskMemoryStdin, ZiskNullStdin};
use serde::Serialize;
use std::path::Path;

use anyhow::Result;

pub trait ZiskIO: Send + Sync {
    /// Read a value from the buffer.
    fn read(&mut self) -> Vec<u8>;

    /// Read a slice of bytes from the buffer.
    fn read_slice(&mut self, slice: &mut [u8]);

    /// Read bytes into the provided buffer.
    fn read_into(&mut self, buffer: &mut [u8]);

    /// Write a serialized value to the buffer.
    fn write<T: Serialize>(&mut self, data: &T);

    /// Write a slice of bytes to the buffer.
    fn write_slice(&mut self, data: &[u8]);
}

pub enum ZiskIOVariant {
    File(ZiskFileStdin),
    Null(ZiskNullStdin),
    Memory(ZiskMemoryStdin),
}

impl ZiskIO for ZiskIOVariant {
    fn read(&mut self) -> Vec<u8> {
        match self {
            ZiskIOVariant::File(file_stdin) => file_stdin.read(),
            ZiskIOVariant::Null(null_stdin) => null_stdin.read(),
            ZiskIOVariant::Memory(memory_stdin) => memory_stdin.read(),
        }
    }

    fn read_slice(&mut self, slice: &mut [u8]) {
        match self {
            ZiskIOVariant::File(file_stdin) => file_stdin.read_slice(slice),
            ZiskIOVariant::Null(null_stdin) => null_stdin.read_slice(slice),
            ZiskIOVariant::Memory(memory_stdin) => memory_stdin.read_slice(slice),
        }
    }

    fn read_into(&mut self, buffer: &mut [u8]) {
        match self {
            ZiskIOVariant::File(file_stdin) => file_stdin.read_into(buffer),
            ZiskIOVariant::Null(null_stdin) => null_stdin.read_into(buffer),
            ZiskIOVariant::Memory(memory_stdin) => memory_stdin.read_into(buffer),
        }
    }

    fn write<T: Serialize>(&mut self, data: &T) {
        match self {
            ZiskIOVariant::File(file_stdin) => file_stdin.write(data),
            ZiskIOVariant::Null(null_stdin) => null_stdin.write(data),
            ZiskIOVariant::Memory(memory_stdin) => memory_stdin.write(data),
        }
    }

    fn write_slice(&mut self, data: &[u8]) {
        match self {
            ZiskIOVariant::File(file_stdin) => file_stdin.write_slice(data),
            ZiskIOVariant::Null(null_stdin) => null_stdin.write_slice(data),
            ZiskIOVariant::Memory(memory_stdin) => memory_stdin.write_slice(data),
        }
    }
}

pub struct ZiskStdin {
    io: ZiskIOVariant,
}

impl ZiskIO for ZiskStdin {
    fn read(&mut self) -> Vec<u8> {
        self.io.read()
    }

    fn read_slice(&mut self, slice: &mut [u8]) {
        self.io.read_slice(slice)
    }

    fn read_into(&mut self, buffer: &mut [u8]) {
        self.io.read_into(buffer)
    }

    fn write<T: Serialize>(&mut self, data: &T) {
        self.io.write(data)
    }

    fn write_slice(&mut self, data: &[u8]) {
        self.io.write_slice(data)
    }
}

impl Default for ZiskStdin {
    fn default() -> Self {
        Self::new()
    }
}

impl ZiskStdin {
    /// Create new memory-based stdin
    pub fn new() -> Self {
        Self { io: ZiskIOVariant::Memory(ZiskMemoryStdin::new(Vec::new())) }
    }

    /// Create a null stdin (no input)
    pub fn null() -> Self {
        Self { io: ZiskIOVariant::Null(ZiskNullStdin) }
    }

    /// Create a file-based stdin
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self { io: ZiskIOVariant::File(ZiskFileStdin::new(path)?) })
    }

    pub fn from_vec(data: Vec<u8>) -> Self {
        Self { io: ZiskIOVariant::Memory(ZiskMemoryStdin::new(data)) }
    }
}
