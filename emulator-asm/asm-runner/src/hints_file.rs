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
    fn submit(&self, processed: &[u64]) -> anyhow::Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use zisk_common::io::StreamSink;

    #[test]
    fn submit_writes_u64s_little_endian_then_flushes_on_drop() {
        let path =
            std::env::temp_dir().join(format!("zisk_hintsfile_test_{}.bin", std::process::id()));
        let path_str = path.to_str().unwrap().to_string();
        let values = [1u64, 0x0203, u64::MAX];

        {
            let hf = HintsFile::new(path_str.clone()).unwrap();
            hf.submit(&values).unwrap();
        } // Drop flushes and closes.

        let mut bytes = Vec::new();
        std::fs::File::open(&path).unwrap().read_to_end(&mut bytes).unwrap();

        let expected: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
        assert_eq!(bytes, expected);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn submit_appends_across_calls() {
        let path =
            std::env::temp_dir().join(format!("zisk_hintsfile_append_{}.bin", std::process::id()));
        let path_str = path.to_str().unwrap().to_string();
        {
            let hf = HintsFile::new(path_str).unwrap();
            hf.submit(&[1u64]).unwrap();
            hf.submit(&[2u64]).unwrap();
        }
        let mut bytes = Vec::new();
        std::fs::File::open(&path).unwrap().read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes.len(), 16); // two u64s
        std::fs::remove_file(&path).ok();
    }
}
