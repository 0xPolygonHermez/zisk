//! HintsFile is responsible for writing precompile processed hints to a file.
//!
//! It implements the StreamSink trait to receive processed hints and write them to a file.

use anyhow::Result;
use std::fs::File;
use std::io::Write;
use std::sync::Mutex;
use zisk_common::io::StreamSink;

/// HintsFile struct manages the writing of processed precompile hints to a file.
pub struct HintsFile {
    file: Mutex<File>,
}

unsafe impl Send for HintsFile {}
unsafe impl Sync for HintsFile {}

impl HintsFile {
    /// Create a new HintsFile with the given filename.
    ///
    /// # Arguments
    /// * `filename` - Path to the file where hints will be written.
    ///
    /// # Returns
    /// A new `HintsFile` instance.
    pub fn new(filename: String) -> Result<Self> {
        let file = File::create(&filename)?;
        Ok(Self { file: Mutex::new(file) })
    }
}

impl StreamSink for HintsFile {
    /// Writes processed precompile hints to the file.
    ///
    /// # Arguments
    /// * `processed` - A vector of processed precompile hints as u64 values.
    ///
    /// # Returns
    /// * `Ok(())` - If hints were successfully written to the file
    /// * `Err` - If writing to the file fails
    #[inline]
    fn submit(&self, processed: Vec<u64>) -> anyhow::Result<()> {
        let mut file = self.file.lock().unwrap();

        // Write each u64 as 8 bytes (little-endian)
        for value in processed {
            file.write_all(&value.to_le_bytes())?;
        }

        // Flush to ensure data is written immediately
        file.flush()?;

        Ok(())
    }
}

impl Drop for HintsFile {
    fn drop(&mut self) {
        // File is automatically closed when dropped
        // We can ensure final flush here
        if let Ok(mut file) = self.file.lock() {
            let _ = file.flush();
        }
    }
}
