//! I/O utilities for Zisk zkVM programs.
//!
//! This module provides a high-level API for reading inputs and committing public outputs.

use crate::{read_input, set_output};
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
    bincode::deserialize(&bytes).expect("Deserialization failed")
}

/// Read raw bytes from the input stream.
///
/// ### Examples
/// ```ignore
/// let data: Vec<u8> = ziskos::io::read_vec();
/// ```
pub fn read_vec() -> Vec<u8> {
    read_input()
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

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
pub fn read_proof<'a>() -> &'a [u8] {
    crate::read_slice_zerocopy()
}

#[allow(unused)]
#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
pub fn read_proof() -> Box<[u8]> {
    read_input().into_boxed_slice()
}

/// Commit a serializable value to public outputs.
/// The value is serialized with bincode and written as 32-bit chunks.
pub fn commit<T: Serialize>(value: &T) {
    let bytes = bincode::serialize(value).expect("Serialization failed");
    write(&bytes);
}

/// Write raw bytes to public outputs.
/// Bytes are written as 32-bit little-endian values.
pub fn write(buf: &[u8]) {
    let chunks = buf.len().div_ceil(4);

    for i in 0..chunks {
        let start = i * 4;
        let end = (start + 4).min(buf.len());
        let mut bytes = [0u8; 4];
        bytes[..end - start].copy_from_slice(&buf[start..end]);
        let val = u32::from_le_bytes(bytes);
        set_output(i, val);
    }
}

pub fn verify_zisk_proof(zisk_proof: &[u8]) -> bool {
    let (proof, vk) = zisk_proof.split_at(zisk_proof.len() - 32);
    zisk_verifier::verify_vadcop_final_proof(proof, vk)
}
