//! A file-based implementation of FileStreamReader and FileStreamWriter.
//! This module provides functionality to read and write data from/to files.

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use super::{StreamRead, StreamWrite};

use anyhow::Result;

/// A file-based implementation of ZiskStdin that reads from a file.
pub struct FileStreamReader {
    /// The path to the input file.
    path: PathBuf,

    /// Buffered reader for the file.
    reader: Option<BufReader<File>>,

    /// Track if the file has been read already.
    has_read: bool,
}

impl FileStreamReader {
    /// Create a new FileStreamReader from a file path.
    pub fn new<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        Ok(FileStreamReader { path: path.as_ref().to_path_buf(), reader: None, has_read: false })
    }
}

impl StreamRead for FileStreamReader {
    /// Open/initialize the stream for reading
    fn open(&mut self) -> Result<()> {
        if self.is_active() {
            return Ok(());
        }

        let file = File::open(&self.path)?;
        self.reader = Some(BufReader::new(file));
        self.has_read = false;
        Ok(())
    }

    /// Reads the next item from the stream.
    ///
    /// This method does **not** stream incrementally. Instead, it repeatedly toggles
    /// between returning the full file contents and returning `None`, producing the
    /// following repeating sequence: `Some(Vec<u8>), None, Some(Vec<u8>), None, ...`
    fn next(&mut self) -> Result<Option<Vec<u8>>> {
        if self.has_read {
            self.has_read = false;
            return Ok(None);
        }

        self.has_read = true;

        // Open the file if it's not already open
        self.open()?;

        let reader = self.reader.as_mut().ok_or_else(|| {
            anyhow::anyhow!("FileStreamReader: Reader is not initialized after opening the file")
        })?;

        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;

        Ok(Some(buffer))
    }

    /// Close the stream
    fn close(&mut self) -> Result<()> {
        self.reader = None;
        Ok(())
    }

    /// Check if the stream is currently active
    fn is_active(&self) -> bool {
        self.reader.is_some()
    }
}

/// A file-based implementation of StreamWrite that writes to a file.
pub struct FileStreamWriter {
    /// The path to the output file.
    path: PathBuf,

    /// Buffered writer for the file.
    writer: Option<BufWriter<File>>,
}

impl FileStreamWriter {
    /// Create a new FileStreamWriter from a file path.
    pub fn new<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        Ok(FileStreamWriter { path: path.as_ref().to_path_buf(), writer: None })
    }
}

impl StreamWrite for FileStreamWriter {
    /// Open/initialize the stream for writing
    fn open(&mut self) -> Result<()> {
        if self.is_active() {
            return Ok(());
        }

        let file = File::create(&self.path)?;
        self.writer = Some(BufWriter::new(file));
        Ok(())
    }

    /// Write data to the stream, returns the number of bytes written
    fn write(&mut self, item: &[u8]) -> Result<usize> {
        // Open the file if it's not already open
        self.open()?;

        let writer = self.writer.as_mut().ok_or_else(|| {
            anyhow::anyhow!("FileStreamWriter: Writer is not initialized after opening the file")
        })?;

        writer.write_all(item)?;
        Ok(item.len())
    }

    /// Flush any buffered data
    fn flush(&mut self) -> Result<()> {
        if let Some(writer) = self.writer.as_mut() {
            writer.flush()?;
        }
        Ok(())
    }

    /// Close the stream
    fn close(&mut self) -> Result<()> {
        self.flush()?;
        self.writer = None;
        Ok(())
    }

    /// Check if the stream is currently active
    fn is_active(&self) -> bool {
        self.writer.is_some()
    }
}
