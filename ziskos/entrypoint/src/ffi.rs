//! C ABI surface for Zisk zkVM IO.
//!
//! Exports `read_input` and `write_output` matching the proposed standardized
//! zkVM IO interface. Guest-side Rust code should use [`crate::io`] instead;

use crate::io;

/// Read the next input segment.
///
/// Writes a pointer to the segment data through `buf_ptr` and its length
/// through `buf_size`. Advances the internal input cursor — successive calls
/// return successive segments framed by the 8-byte length prefixes in the
/// input layout.
///
/// # Safety
/// `buf_ptr` and `buf_size` must point to writable memory.
#[no_mangle]
pub unsafe extern "C" fn read_input(buf_ptr: *mut *const u8, buf_size: *mut usize) {
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    {
        let s = io::read_input_slice();
        *buf_ptr = s.as_ptr();
        *buf_size = s.len();
    }
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        // Leak the boxed slice so the returned pointer outlives this call.
        let leaked: &'static [u8] = Box::leak(io::read_input_slice());
        *buf_ptr = leaked.as_ptr();
        *buf_size = leaked.len();
    }
}

/// Append `size` bytes from `output` to the public output buffer.
///
/// Successive calls byte-concatenate; the final public output is the
/// concatenation of every buffer passed to `write_output` over the lifetime
/// of the program.
///
/// # Safety
/// `output` must point to `size` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn write_output(output: *const u8, size: usize) {
    if size == 0 {
        return;
    }
    let buf = core::slice::from_raw_parts(output, size);
    io::commit_slice(buf);
}

/// Verify a Zisk proof from a guest program (recursive verification).
///
/// `proof_ptr` points to a buffer containing the proof followed by a 32-byte
/// verification key. Returns `true` if the proof is valid.
///
/// # Safety
/// `proof_ptr` must point to `proof_size` readable bytes, and `proof_size`
/// must be at least 32.
#[no_mangle]
pub unsafe extern "C" fn verify_zisk_proof(proof_ptr: *const u8, proof_size: usize) -> bool {
    let buf = core::slice::from_raw_parts(proof_ptr, proof_size);
    io::verify_zisk_proof(buf)
}
