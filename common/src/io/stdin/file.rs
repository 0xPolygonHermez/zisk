//! A file-based implementation of ZiskStdin.
//! This module provides functionality to read input data from a file.

use serde::Serialize;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::io::ZiskIO;

/// A file-based implementation of ZiskStdin that reads from a file.
pub struct ZiskFileStdin {
    /// The path to the input file.
    path: PathBuf,

    /// Buffered reader for the file.
    reader: Mutex<BufReader<File>>,
}

impl ZiskFileStdin {
    /// Create a new FileStdin from a file path.
    pub fn new<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let path_buf = path.as_ref().to_path_buf();
        if !path_buf.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Input file not found at {:?}", path_buf.display()),
            ));
        }

        let file = File::open(&path_buf)?;
        Ok(ZiskFileStdin { path: path_buf, reader: Mutex::new(BufReader::new(file)) })
    }
}

impl ZiskIO for ZiskFileStdin {
    fn read(&self) -> Vec<u8> {
        fs::read(&self.path).expect("Could not read inputs file")
    }

    fn read_slice(&self, slice: &mut [u8]) {
        let mut reader = self.reader.lock().unwrap();
        reader.read_exact(slice).expect("Failed to read slice");
    }

    fn read_into(&self, buffer: &mut [u8]) {
        let mut reader = self.reader.lock().unwrap();
        reader.read_exact(buffer).expect("Failed to read into buffer");
    }

    fn write<T: Serialize>(&self, _data: &T) {
        // This is a read-only stdin implementation
        // Writing is not supported for file-based stdin
        panic!("Write operations are not supported for FileStdin");
    }

    fn write_slice(&self, _data: &[u8]) {
        // This is a read-only stdin implementation
        // Writing is not supported for file-based stdin
        panic!("Write operations are not supported for FileStdin");
    }
}
