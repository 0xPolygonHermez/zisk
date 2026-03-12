//! A file-based implementation of ZiskStdin.
//! This module provides functionality to read input data from a file.

use anyhow::Result;
use serde::{de::DeserializeOwned, Serialize};
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

    fn read_raw_data(&self) -> std::io::Result<Vec<u8>> {
        let mut reader = self.reader.lock().unwrap();

        let mut len_bytes = [0u8; 8];
        reader.read_exact(&mut len_bytes)?;
        let len = usize::from_le_bytes(len_bytes);

        let mut data = vec![0u8; len];
        reader.read_exact(&mut data)?;

        let total_len = 8 + len;
        let padding = (8 - (total_len % 8)) % 8;
        if padding > 0 {
            let mut padding_bytes = vec![0u8; padding];
            reader.read_exact(&mut padding_bytes)?;
        }

        Ok(data)
    }
}

impl ZiskIO for ZiskFileStdin {
    fn read_raw_bytes(&self) -> Vec<u8> {
        fs::read(&self.path).expect("Could not read inputs file")
    }

    fn read_bytes(&self) -> Vec<u8> {
        self.read_raw_data().expect("Failed to read into buffer")
    }

    fn read_slice(&self, slice: &mut [u8]) {
        let data = self.read_raw_data().expect("Failed to read slice");
        assert_eq!(
            slice.len(),
            data.len(),
            "Slice length mismatch: expected {}, got {}",
            data.len(),
            slice.len()
        );
        slice.copy_from_slice(&data);
    }

    fn read<T: DeserializeOwned>(&self) -> Result<T> {
        let data = self
            .read_raw_data()
            .map_err(|e| anyhow::anyhow!("Failed to read data from file: {}", e))?;

        bincode::deserialize(&data)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize from file: {}", e))
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

    fn write_proof(&self, _proof: &[u8]) {
        // This is a read-only stdin implementation
        // Writing is not supported for file-based stdin
        panic!("Write operations are not supported for FileStdin");
    }

    fn save(&self, _path: &Path) -> Result<()> {
        // This is a read-only stdin implementation
        // Saving is not supported for file-based stdin
        panic!("Save operations are not supported for FileStdin");
    }
}
