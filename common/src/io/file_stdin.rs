use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use crate::io::ZiskStdin;

/// A file-based implementation of ZiskStdin that reads from a file.
pub struct ZiskFileStdin {
    path: PathBuf,
    reader: BufReader<File>,
}

impl ZiskFileStdin {
    /// Create a new FileStdin from a file path.
    pub fn new<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let path_buf = path.as_ref().to_path_buf();
        let file = File::open(&path_buf)?;
        Ok(ZiskFileStdin { path: path_buf, reader: BufReader::new(file) })
    }
}

impl ZiskStdin for ZiskFileStdin {
    fn read(&mut self) -> Vec<u8> {
        fs::read(&self.path).expect("Could not read inputs file")
    }

    fn read_slice(&mut self, slice: &mut [u8]) {
        self.reader.read_exact(slice).expect("Failed to read slice");
    }

    fn read_into(&mut self, buffer: &mut [u8]) {
        self.reader.read_exact(buffer).expect("Failed to read into buffer");
    }

    fn write_serialized(&mut self, _data: &[u8]) {
        // This is a read-only stdin implementation
        // Writing is not supported for file-based stdin
        panic!("Write operations are not supported for FileStdin");
    }

    fn write_bytes(&mut self, _data: &[u8]) {
        // This is a read-only stdin implementation
        // Writing is not supported for file-based stdin
        panic!("Write operations are not supported for FileStdin");
    }
}
