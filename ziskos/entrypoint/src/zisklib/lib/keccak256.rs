//! Keccak-256 hash function.

use crate::syscalls::{syscall_keccak_xor, KECCAK_XOR_LANES};

#[cfg(zisk_hints_debug)]
use core::ffi::c_char;

#[cfg(all(not(zisk_guest), zisk_hints))]
extern "C" {
    fn hint_keccak256(input_ptr: *const u8, input_len: usize);
}

#[cfg(all(not(zisk_guest), zisk_hints_debug))]
extern "C" {
    fn hint_log_c(msg: *const c_char);
}

#[cfg(zisk_hints_debug)]
pub fn hint_log<S: AsRef<str>>(msg: S) {
    // On native we call external C function to log hints, since it controls if hints are paused or not
    #[cfg(not(zisk_guest))]
    {
        extern crate alloc;
        use alloc::ffi::CString;

        if let Ok(c) = CString::new(msg.as_ref()) {
            unsafe { hint_log_c(c.as_ptr()) };
        }
    }
    // On zkvm/zisk, we can just print directly
    #[cfg(zisk_guest)]
    {
        println!("{}", msg.as_ref());
    }
}

/// Keccak-256 rate in bytes (1600 - 2*256) / 8 = 136 bytes
const KECCAK256_RATE: usize = 136;

/// Keccak-256 hash function. For reference: https://keccak.team/keccak_specs_summary.html
///
/// Every block is absorbed with one `syscall_keccak_xor` (XOR + Keccak-f), which reads the block
/// as 17 aligned 64-bit words. A full block is read in place when the input is 8-byte aligned;
/// otherwise it goes through an aligned copy, as the padded final block always does.
pub fn keccak256(input: &[u8], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u8; 32] {
    let mut state = [0u64; 25];
    let input_len = input.len();
    let aligned = (input.as_ptr() as usize) % core::mem::align_of::<u64>() == 0;

    // Absorb phase: process complete rate-sized blocks
    let mut offset = 0;
    while offset + KECCAK256_RATE <= input_len {
        let block = &input[offset..offset + KECCAK256_RATE];
        if aligned {
            // SAFETY: the block is 136 readable bytes at an 8-byte aligned address (the input is
            // aligned and the rate is a multiple of 8). Lanes are little-endian, as the target.
            unsafe {
                syscall_keccak_xor(
                    &mut state,
                    block.as_ptr() as *const [u64; KECCAK_XOR_LANES],
                    #[cfg(feature = "hints")]
                    hints,
                );
            }
        } else {
            absorb_block(
                &mut state,
                block,
                #[cfg(feature = "hints")]
                hints,
            );
        }
        offset += KECCAK256_RATE;
    }

    // Handle final block with padding
    let remaining = input_len - offset;
    let mut final_block = [0u8; KECCAK256_RATE];

    // Copy remaining bytes
    final_block[..remaining].copy_from_slice(&input[offset..]);

    // Keccak padding: append 0x01, then zeros, then 0x80 at the end of the rate
    // For Keccak-256: domain separator is 0x01
    final_block[remaining] = 0x01;
    final_block[KECCAK256_RATE - 1] |= 0x80;

    // Absorb the final padded block
    absorb_block(
        &mut state,
        &final_block,
        #[cfg(feature = "hints")]
        hints,
    );

    // Squeeze phase: extract first 32 bytes (256 bits) from state
    let mut result = [0u8; 32];
    // SAFETY: [u64; 25] and [u8; 200] have the same size (200 bytes) and [u8] has
    // alignment 1, which is compatible with any alignment. The pointer cast is valid
    // because we're reinterpreting the same memory region.
    let state_bytes: &[u8; 200] = unsafe { &*(&state as *const [u64; 25] as *const [u8; 200]) };
    result.copy_from_slice(&state_bytes[..32]);

    result
}

/// Absorbs a rate-sized block of bytes at any alignment: copies it into aligned lanes and hands
/// those to `syscall_keccak_xor`.
#[inline]
fn absorb_block(
    state: &mut [u64; 25],
    block: &[u8],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    debug_assert_eq!(block.len(), KECCAK256_RATE);
    let mut lanes = [0u64; KECCAK_XOR_LANES];
    // SAFETY: both regions are 136 bytes and do not overlap; a byte copy into the lanes is their
    // little-endian reading on the (little-endian) target.
    unsafe {
        core::ptr::copy_nonoverlapping(
            block.as_ptr(),
            lanes.as_mut_ptr() as *mut u8,
            KECCAK256_RATE,
        );
        syscall_keccak_xor(
            state,
            &lanes,
            #[cfg(feature = "hints")]
            hints,
        );
    }
}

/// C-compatible wrapper for Keccak-256 hash
///
/// # Safety
/// - `input` must point to at least `input_len` bytes
/// - `output` must point to a writable buffer of at least 32 bytes
#[inline]
pub(crate) unsafe fn keccak256_c(
    input: *const u8,
    input_len: usize,
    output: *mut u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let input_slice = core::slice::from_raw_parts(input, input_len);
    let hash = keccak256(
        input_slice,
        #[cfg(feature = "hints")]
        hints,
    );
    let output_slice = core::slice::from_raw_parts_mut(output, 32);
    output_slice.copy_from_slice(&hash);
}

/// Native keccak256 implementation for external callers
///
/// # Safety
/// - `bytes` must point to at least `len` bytes
/// - `output` must point to a writable buffer of at least 32 bytes
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_native_keccak256_c")]
pub unsafe extern "C" fn native_keccak256(bytes: *const u8, len: usize, output: *mut u8) {
    #[cfg(zisk_hints)]
    hint_keccak256(bytes, len);

    #[cfg(zisk_hints_debug)]
    {
        let input_bytes = unsafe { core::slice::from_raw_parts(bytes, len) };
        hint_log(format!("hint_keccak256 (bytes: {:?}, len: {})", input_bytes, len));
    }

    #[cfg(zisk_guest)]
    {
        keccak256_c(
            bytes,
            len,
            output,
            #[cfg(feature = "hints")]
            hints,
        );
    }

    #[cfg(not(zisk_guest))]
    {
        use tiny_keccak::{Hasher, Keccak};
        const OUT_LEN: usize = 32;

        let (input_bytes, out) = unsafe {
            let input_bytes = core::slice::from_raw_parts(bytes, len);
            let out = core::slice::from_raw_parts_mut(output, OUT_LEN);
            (input_bytes, out)
        };

        let mut hasher = Keccak::v256();
        hasher.update(input_bytes);
        hasher.finalize(out);
    }
}
