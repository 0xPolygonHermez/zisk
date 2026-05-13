//! I/O utilities for Zisk zkVM programs.
//!
//! This module provides a high-level API for reading inputs and committing public outputs.

#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
use crate::read_input;

use serde::{de::DeserializeOwned, Serialize};

/// Read a deserializable object from the input stream.
///
/// ### Examples
/// ```ignore
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct MyStruct {
///     a: u32,
///     b: u32,
/// }
///
/// let data: MyStruct = ziskos::io::read();
/// ```
///
/// Note: This uses zero-copy deserialization on zkvm to avoid unnecessary data copies.
pub fn read<T: DeserializeOwned>() -> T {
    let bytes = read_input_slice();
    let (val, _): (T, usize) =
        bincode::serde::decode_from_slice(bytes.as_ref(), bincode::config::standard())
            .expect("Deserialization failed");
    val
}

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
pub fn read_input_slice<'a>() -> &'a [u8] {
    crate::read_slice_zerocopy()
}

#[allow(unused)]
#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
pub fn read_input_slice() -> Box<[u8]> {
    read_input().into_boxed_slice()
}

/// Commit a serializable value to public outputs.
/// The value is serialized with bincode and written as 32-bit chunks.
pub fn commit<T: Serialize>(value: &T) {
    let bytes = bincode::serde::encode_to_vec(value, bincode::config::standard())
        .expect("Serialization failed");
    commit_slice(&bytes);
}

/// Append raw bytes to public outputs.
///
/// Successive calls append to the same byte stream; partial 32-bit output slots
/// are shared across calls.
pub fn commit_slice(buf: &[u8]) {
    // SAFETY: buf.as_ptr() is valid for buf.len() bytes by construction of &[u8].
    unsafe { crate::zisklib::zkvm_io::write_output(buf.as_ptr(), buf.len()) };
}

/// Reset the output cursor to slot 0.
pub fn write_output_reset() {
    crate::zisklib::zkvm_io::reset_output();
}