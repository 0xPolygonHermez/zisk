//! I/O utilities for Zisk zkVM programs.
//!
//! This module provides a high-level API for reading inputs and committing public outputs.

#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
use crate::read_input;
use crate::set_output;
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
    let bytes = bincode::serialize(value).expect("Serialization failed");
    commit_slice(&bytes);
}

/// Append raw bytes to public outputs.
///
/// Each call occupies a fresh run of 32-bit slots starting at the current
/// cursor; if `buf.len()` is not a multiple of 4 the trailing slot is
/// zero-padded. Successive calls advance the cursor — they do not share a
/// partial word, so up to 3 bytes per call may be wasted to padding.
pub fn commit_slice(buf: &[u8]) {
    let chunks = buf.len().div_ceil(4);
    let base = unsafe { OUTPUT_SLOT };

    for i in 0..chunks {
        let start = i * 4;
        let end = (start + 4).min(buf.len());
        let mut bytes = [0u8; 4];
        bytes[..end - start].copy_from_slice(&buf[start..end]);
        set_output(base + i, u32::from_le_bytes(bytes));
    }

    unsafe { OUTPUT_SLOT = base + chunks };
}

static mut OUTPUT_SLOT: usize = 0;

/// Reset the output cursor to slot 0.
pub fn write_output_reset() {
    unsafe { OUTPUT_SLOT = 0 };
}

pub fn verify_zisk_proof(zisk_proof: &[u8]) -> bool {
    let (proof, vk) = zisk_proof.split_at(zisk_proof.len() - 32);
    zisk_verifier::verify_vadcop_final_proof(proof, vk)
}
