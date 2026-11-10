//! I/O utilities for Zisk zkVM programs.
//!
//! This module provides a high-level API for reading inputs and committing public outputs.

#[cfg(not(zisk_guest))]
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
    let bytes = read_slice();
    let (val, _): (T, usize) =
        bincode::serde::decode_from_slice(bytes.as_ref(), bincode::config::standard())
            .expect("Deserialization failed");
    val
}

/// Read the next input slice. Panics inside a fork-join task (see [`crate::par`]): the input is
/// read in order, and a task run in parallel would not see where the tasks before it left it.
#[cfg(zisk_guest)]
pub fn read_slice<'a>() -> &'a [u8] {
    crate::par::assert_outside_task("reading the input");
    crate::read_slice_zerocopy()
}

/// Read the next input slice. Panics inside a fork-join task (see [`crate::par`]).
#[allow(unused)]
#[cfg(not(zisk_guest))]
pub fn read_slice() -> Box<[u8]> {
    crate::par::assert_outside_task("reading the input");
    read_input().into_boxed_slice()
}

#[cfg(zisk_guest)]
#[deprecated(since = "1.0.0-alpha", note = "renamed to `read_slice`")]
pub fn read_input_slice<'a>() -> &'a [u8] {
    read_slice()
}

#[allow(unused)]
#[cfg(not(zisk_guest))]
#[deprecated(since = "1.0.0-alpha", note = "renamed to `read_slice`")]
pub fn read_input_slice() -> Box<[u8]> {
    read_slice()
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
/// are shared across calls. Panics inside a fork-join task (see [`crate::par`]): outputs are
/// committed in order, and a task run in parallel would lose what it committed.
pub fn commit_slice(buf: &[u8]) {
    crate::par::assert_outside_task("committing outputs");
    // SAFETY: buf.as_ptr() is valid for buf.len() bytes by construction of &[u8].
    unsafe { crate::zisklib::zkvm_io::write_output(buf.as_ptr(), buf.len()) };
}
