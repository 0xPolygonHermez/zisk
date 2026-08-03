use crate::error::{CommonError, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::io::{Cursor, Read};
use std::path::Path;
use std::sync::{Arc, Mutex};

struct Inner {
    data: Mutex<Vec<u8>>,
    /// Frame-read offset into `data`; independent of writes.
    pos: Mutex<usize>,
}

/// The `ZiskStdin` struct provides an abstraction for handling standard input data in a flexible manner.
#[derive(Clone)]
pub struct ZiskStdin {
    inner: Arc<Inner>,
}

impl Default for ZiskStdin {
    fn default() -> Self {
        Self::new()
    }
}

impl ZiskStdin {
    /// Creates a new, empty `ZiskStdin` instance.
    pub fn new() -> Self {
        Self { inner: Arc::new(Inner { data: Mutex::new(Vec::new()), pos: Mutex::new(0) }) }
    }

    /// Creates a `ZiskStdin` instance from a vector of bytes.
    pub fn from_vec(data: Vec<u8>) -> Self {
        Self { inner: Arc::new(Inner { data: Mutex::new(data), pos: Mutex::new(0) }) }
    }

    /// Creates a `ZiskStdin` instance by reading data from a file at the specified path.
    ///
    /// # Errors
    ///
    /// Returns [`CommonError::Io`] if the file cannot be read.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let data = std::fs::read(path.as_ref()).map_err(|e| {
            CommonError::Io(format!("Failed to read input file {:?}: {}", path.as_ref(), e))
        })?;
        Ok(Self::from_vec(data))
    }

    /// Create a `ZiskStdin` from a URI string.
    /// - `None` → empty stdin
    /// - `"file://path"` → read from file
    /// - `"inline://[[1,2],[3]]"` → inline input, a JSON array of u64 arrays
    /// - No scheme → treated as a file path
    ///
    /// # Errors
    ///
    /// - [`CommonError::UnknownScheme`] if the URI carries an unrecognized scheme.
    /// - Any error from [`from_file`](Self::from_file) or [`from_inline`](Self::from_inline)
    ///   when reading the referenced input.
    pub fn from_uri<S: Into<String>>(stdin_uri: Option<S>) -> Result<ZiskStdin> {
        let Some(uri) = stdin_uri else { return Ok(ZiskStdin::new()) };
        let uri = uri.into();
        if let Some(pos) = uri.find("://") {
            let (scheme, path) = uri.split_at(pos);
            let path = &path[3..];
            match scheme {
                "file" => ZiskStdin::from_file(path),
                "inline" => ZiskStdin::from_inline(path),
                _ => Err(CommonError::UnknownScheme(scheme.to_string())),
            }
        } else {
            ZiskStdin::from_file(uri.as_str())
        }
    }

    /// Create a `ZiskStdin` from an inline JSON array of u64 arrays.
    ///
    /// Each inner array is written as one frame via [`write_slice`](Self::write_slice),
    /// so the buffer is byte-identical to a saved `input.bin`: every frame carries an
    /// 8-byte little-endian length prefix and is padded to an 8-byte boundary.
    ///
    /// Example: `"[[1,2],[3],[4,5,6]]"` produces three frames.
    ///
    /// # Errors
    ///
    /// Returns [`CommonError::Invalid`] if the input is not a valid JSON array of u64 arrays.
    pub fn from_inline(json: &str) -> Result<ZiskStdin> {
        let frames: Vec<Vec<u64>> = serde_json::from_str(json).map_err(|e| {
            CommonError::Invalid(format!(
                "inline input must be a JSON array of u64 arrays, e.g. [[1,2],[3]]; got: {json}: {e}"
            ))
        })?;
        let stdin = ZiskStdin::new();
        for frame in frames {
            let mut bytes = Vec::with_capacity(frame.len() * 8);
            for word in frame {
                bytes.extend_from_slice(&word.to_le_bytes());
            }
            stdin.write_slice(&bytes);
        }
        Ok(stdin)
    }

    /// Read the raw byte data from the `ZiskStdin` buffer.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    pub fn read_data(&self) -> Vec<u8> {
        self.inner.data.lock().unwrap().clone()
    }

    /// Borrow the raw buffer for the duration of `f`. Prefer this over
    /// [`read_data`](Self::read_data) for read-only use; inputs reach tens of MB.
    ///
    /// `f` must not call back into this `ZiskStdin`: the buffer lock is held for
    /// its duration and is not reentrant.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    pub fn with_data<R>(&self, f: impl FnOnce(&[u8]) -> R) -> R {
        f(self.inner.data.lock().unwrap().as_slice())
    }

    /// Length of the raw buffer in bytes.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    pub fn len(&self) -> usize {
        self.inner.data.lock().unwrap().len()
    }

    /// Whether the raw buffer is empty.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Read the next frame of data from the `ZiskStdin` buffer as a vector of bytes.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned or if reading the next frame fails.
    pub fn read_bytes(&self) -> Vec<u8> {
        self.read_raw().expect("Failed to read from stdin buffer")
    }

    /// Reads the next frame of data from the [`ZiskStdin`] buffer and deserializes
    /// it into a value of type `T`.
    ///
    /// # Errors
    ///
    /// - [`CommonError::Io`] if reading from the buffer fails.
    /// - [`CommonError::Deserialization`] if deserialization fails.
    pub fn read<T: DeserializeOwned>(&self) -> Result<T> {
        let data = self
            .read_raw()
            .map_err(|e| CommonError::Io(format!("Failed to read from stdin: {}", e)))?;
        bincode::serde::decode_from_slice(&data, bincode::config::standard())
            .map(|(v, _)| v)
            .map_err(|e| CommonError::Deserialization(e.to_string()))
    }

    /// Write a serializable value of type `T` to the `ZiskStdin` buffer as a new frame.
    ///
    /// # Panics
    ///
    /// Panics if `data` cannot be serialized, or if the internal mutex is poisoned.
    pub fn write<T: Serialize>(&self, data: &T) {
        let bytes = bincode::serde::encode_to_vec(data, bincode::config::standard())
            .expect("Failed to serialize");
        self.write_slice(&bytes);
    }

    /// Write a raw slice of bytes to the `ZiskStdin` buffer as a new frame, prefixed with its length and padded to an 8-byte boundary.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    pub fn write_slice(&self, data: &[u8]) {
        let data_len = data.len();
        let total_len = 8 + data_len;
        let padding = (8 - (total_len % 8)) % 8;
        let len_bytes = data_len.to_le_bytes();

        let mut buf = self.inner.data.lock().unwrap();
        buf.extend_from_slice(&len_bytes);
        buf.extend_from_slice(data);
        let padded_len = buf.len() + padding;
        buf.resize(padded_len, 0);
    }

    /// Save the `ZiskStdin` buffer to a file at the specified path.
    ///
    /// # Errors
    ///
    /// Returns [`CommonError::Io`] if the parent directory cannot be created or the file cannot be written.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                CommonError::Io(format!(
                    "failed to create parent directory {}: {e}",
                    parent.display()
                ))
            })?;
        }
        std::fs::write(path, self.inner.data.lock().unwrap().as_slice()).map_err(|e| {
            CommonError::Io(format!("failed to write stdin to {}: {e}", path.display()))
        })?;
        Ok(())
    }

    /// Reset the read cursor to the beginning.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    pub fn rewind(&self) {
        *self.inner.pos.lock().unwrap() = 0;
    }

    /// Alias for `rewind`.
    pub fn reset(&self) {
        self.rewind();
    }

    /// Clear the `ZiskStdin` buffer and reset the cursor.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    pub fn clear(&self) {
        self.inner.data.lock().unwrap().clear();
        *self.inner.pos.lock().unwrap() = 0;
    }

    /// Reads the next length-prefixed frame, advancing past its padding.
    fn read_raw(&self) -> std::io::Result<Vec<u8>> {
        let buf = self.inner.data.lock().unwrap();
        let mut pos = self.inner.pos.lock().unwrap();

        let mut cursor = Cursor::new(&buf[..]);
        cursor.set_position(*pos as u64);

        let mut len_bytes = [0u8; 8];
        cursor.read_exact(&mut len_bytes)?;
        let len = usize::from_le_bytes(len_bytes);
        let mut data = vec![0u8; len];
        cursor.read_exact(&mut data)?;
        // Padding is at most 7 bytes, so it reads into a stack buffer.
        let padding = (8 - ((8 + len) % 8)) % 8;
        if padding > 0 {
            let mut pad = [0u8; 7];
            cursor.read_exact(&mut pad[..padding])?;
        }

        // Commit only on success, so a short read leaves the reader put.
        *pos = cursor.position() as usize;
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_vec_round_trips_a_single_frame() {
        let stdin = ZiskStdin::new();
        stdin.write_slice(&[1, 2, 3]);
        // The path a saved `input.bin` takes.
        let reloaded = ZiskStdin::from_vec(stdin.read_data());
        assert_eq!(reloaded.read_bytes(), vec![1, 2, 3]);
    }

    #[test]
    fn frames_are_read_back_in_order_with_padding_skipped() {
        let stdin = ZiskStdin::new();
        stdin.write_slice(&[9]); // 1 byte  -> 7 bytes padding
        stdin.write_slice(&[1, 2, 3, 4, 5, 6, 7, 8]); // 8 bytes -> 0 padding
        stdin.write_slice(&[]); // empty frame

        assert_eq!(stdin.read_bytes(), vec![9]);
        assert_eq!(stdin.read_bytes(), vec![1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(stdin.read_bytes(), Vec::<u8>::new());
    }

    #[test]
    fn every_frame_is_eight_byte_aligned() {
        let stdin = ZiskStdin::new();
        for len in 0..24usize {
            stdin.write_slice(&vec![7u8; len]);
            assert_eq!(stdin.len() % 8, 0, "buffer unaligned after a {len}-byte frame");
        }
    }

    #[test]
    fn rewind_replays_from_the_start() {
        let stdin = ZiskStdin::new();
        stdin.write_slice(&[4, 5]);
        assert_eq!(stdin.read_bytes(), vec![4, 5]);
        stdin.rewind();
        assert_eq!(stdin.read_bytes(), vec![4, 5]);
    }

    #[test]
    fn writes_after_a_read_are_visible_without_disturbing_the_position() {
        let stdin = ZiskStdin::new();
        stdin.write_slice(&[1]);
        assert_eq!(stdin.read_bytes(), vec![1]);
        stdin.write_slice(&[2]);
        assert_eq!(stdin.read_bytes(), vec![2]);
    }

    #[test]
    fn reading_past_the_end_errors_and_leaves_the_position_put() {
        let stdin = ZiskStdin::new();
        stdin.write_slice(&[1, 2]);
        assert_eq!(stdin.read_bytes(), vec![1, 2]);

        assert!(stdin.read_raw().is_err());
        // A failed read must not consume anything.
        stdin.write_slice(&[3]);
        assert_eq!(stdin.read_bytes(), vec![3]);
    }

    #[test]
    fn clear_empties_the_buffer_and_resets_the_reader() {
        let stdin = ZiskStdin::new();
        stdin.write_slice(&[1, 2, 3]);
        stdin.clear();
        assert!(stdin.is_empty());
        assert!(stdin.read_raw().is_err());

        stdin.write_slice(&[8]);
        assert_eq!(stdin.read_bytes(), vec![8]);
    }

    #[test]
    fn with_data_sees_the_same_bytes_as_read_data() {
        let stdin = ZiskStdin::new();
        stdin.write_slice(&[1, 2, 3]);
        assert_eq!(stdin.with_data(|d| d.to_vec()), stdin.read_data());
        assert_eq!(stdin.with_data(|d| d.len()), stdin.len());
    }

    #[test]
    fn clones_share_one_buffer() {
        let stdin = ZiskStdin::new();
        let clone = stdin.clone();
        stdin.write_slice(&[42]);
        assert_eq!(clone.read_bytes(), vec![42]);
    }
}
