//! BF16 dot-product system call interception (experimental)

#[cfg(zisk_guest)]
use core::arch::asm;

#[cfg(zisk_guest)]
use crate::ziskos_syscall_ret_u64;

/// The CSR port of `dot_bf16`, for callers that issue the `csrrs` inline with
/// [`crate::ziskos_syscall_ret_u64`] instead of calling [`syscall_dot_bf16`].
pub use zisk_definitions::SYSCALL_DOT_BF16_ID;

/// BF16 helpers and the reference semantics (see `zisk_definitions::dot_bf16`).
pub use zisk_definitions::{
    bf16_from_f32, bf16_to_f32, dot_bf16_fp4_block, dot_bf16_fp4_reference, dot_bf16_reference,
    DOT_BF16_FLAGS_BF16, DOT_BF16_FLAGS_MXFP4, DOT_BF16_FLAGS_NVFP4, DOT_BF16_MAX_N, DOT_BF16_NAN,
};

#[derive(Debug)]
#[repr(C)]
pub struct SyscallDotBf16Params {
    /// Address of the first vector of BF16 values (`u16` bits), 8-byte aligned.
    pub x: *const u16,
    /// Address of the second vector, 8-byte aligned.
    pub y: *const u16,
    /// Number of values in each vector: a multiple of 4, at most [`DOT_BF16_MAX_N`].
    pub n: u64,
    /// Must be [`DOT_BF16_FLAGS_BF16`] (0); [`syscall_dot_bf16_fp4`] takes the FP4 weights.
    pub flags: u64,
    /// Values per block: a multiple of 4 dividing `n`; 0 means one block of `n` values.
    pub block: u64,
    /// When not null, receives the `f32` bits of each block's result (`n / block` values),
    /// 8-byte aligned.
    pub out: *mut u32,
}

/// Executes the experimental `DotBf16` operation: `Σ x[i]·y[i]` over two vectors of `n` BF16
/// values, computed exactly and rounded once to `f32`, to nearest, ties to even. Returns the
/// result and, when `out` is not null, writes the result of each block of `block` values to it.
///
/// Departures from IEEE 754: an exact sum of zero gives +0.0, and if any input value is ±∞ or
/// NaN the result is the canonical NaN ([`DOT_BF16_NAN`]).
///
/// **Experimental:** ZisK's Rust emulator runs it, but no state machine proves it yet, so a
/// program that calls it cannot be proven.
///
/// ### Safety
///
/// `x` and `y` must point to `n` readable `u16`, `out` (if not null) to `n / block` writable
/// `u32`, all 8-byte aligned and with the length rules above met; `out` must not overlap `x`,
/// `y` or the parameter block.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_dot_bf16")]
pub unsafe extern "C" fn syscall_dot_bf16(params: &SyscallDotBf16Params) -> f32 {
    #[cfg(not(zisk_guest))]
    {
        let n = params.n as usize;
        let block = if params.block == 0 { n } else { params.block as usize };
        let x = unsafe { core::slice::from_raw_parts(params.x, n) };
        let y = unsafe { core::slice::from_raw_parts(params.y, n) };
        let out = (!params.out.is_null() && n > 0)
            .then(|| unsafe { core::slice::from_raw_parts_mut(params.out, n / block) });
        f32::from_bits(dot_bf16_reference(x, y, n, block, out))
    }
    #[cfg(zisk_guest)]
    {
        f32::from_bits(ziskos_syscall_ret_u64!(zisk_definitions::SYSCALL_DOT_BF16_ID, params) as u32)
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct SyscallDotBf16Fp4Params {
    /// Address of the FP4 weights: E2M1 values, two per byte, low nibble first, 8-byte aligned.
    pub x: *const u8,
    /// Address of the BF16 values (`u16` bits), 8-byte aligned.
    pub y: *const u16,
    /// Number of values: a multiple of the FP4 block (32 or 16), at most [`DOT_BF16_MAX_N`].
    pub n: u64,
    /// [`DOT_BF16_FLAGS_MXFP4`] (E8M0 scales, blocks of 32) or [`DOT_BF16_FLAGS_NVFP4`] (E4M3
    /// scales, blocks of 16).
    pub flags: u64,
    /// Values per block of results: a multiple of the FP4 block dividing `n`; 0 for one block.
    pub block: u64,
    /// When not null, receives the `f32` bits of each block's result, 8-byte aligned.
    pub out: *mut u32,
    /// Address of the scales, one byte per FP4 block (`n / 32` or `n / 16`), 8-byte aligned. A
    /// last, partial word is read in full; its extra bytes are ignored.
    pub scales: *const u8,
}

/// Executes the experimental `DotBf16` operation with FP4 weights: `Σ w[i]·y[i]` with
/// `w[i] = e2m1(x[i]) · scale[i / block size]` (MXFP4 or NVFP4) and BF16 `y`, computed exactly and
/// rounded once to `f32`, to nearest, ties to even. Returns the result and, when `out` is not
/// null, writes the result of each block of `block` values to it.
///
/// For NVFP4 the per-tensor `f32` scale is not included: multiply the result by it. As for two
/// BF16 vectors, a zero sum gives +0.0, and an infinite or NaN value, or a NaN scale, gives the
/// canonical NaN ([`DOT_BF16_NAN`]).
///
/// **Experimental:** ZisK's emulators run it, but no state machine proves it yet, so a program
/// that calls it cannot be proven.
///
/// ### Safety
///
/// `x` must point to `n / 2` readable bytes, `scales` to the scales rounded up to whole words,
/// `y` to `n` readable `u16`, `out` (if not null) to `n / block` writable `u32`, all 8-byte
/// aligned and with the length rules above met; `out` must not overlap the inputs or the
/// parameter block.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_dot_bf16_fp4")]
pub unsafe extern "C" fn syscall_dot_bf16_fp4(params: &SyscallDotBf16Fp4Params) -> f32 {
    #[cfg(not(zisk_guest))]
    {
        let n = params.n as usize;
        let block = if params.block == 0 { n } else { params.block as usize };
        let scale_bytes = n / dot_bf16_fp4_block(params.flags);
        let x = unsafe { core::slice::from_raw_parts(params.x, n / 2) };
        let scales = unsafe { core::slice::from_raw_parts(params.scales, scale_bytes) };
        let y = unsafe { core::slice::from_raw_parts(params.y, n) };
        let out = (!params.out.is_null() && n > 0)
            .then(|| unsafe { core::slice::from_raw_parts_mut(params.out, n / block) });
        f32::from_bits(dot_bf16_fp4_reference(params.flags, x, scales, y, n, block, out))
    }
    #[cfg(zisk_guest)]
    {
        f32::from_bits(ziskos_syscall_ret_u64!(zisk_definitions::SYSCALL_DOT_BF16_ID, params) as u32)
    }
}
