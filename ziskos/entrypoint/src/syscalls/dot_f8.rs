//! Dot-product system call interception (experimental)

#[cfg(zisk_guest)]
use core::arch::asm;

#[cfg(zisk_guest)]
use crate::ziskos_syscall_ret_u64;

/// The CSR port of `dot_f8`, for callers that issue the `csrrs` inline with
/// [`crate::ziskos_syscall_ret_u64`] instead of calling [`syscall_dot_f8`].
pub use zisk_definitions::SYSCALL_DOT_F8_ID;

/// The element formats and their decoding (see `zisk_definitions::dot_f8`).
pub use zisk_definitions::{
    dot_f8_bits, dot_f8_formats, dot_f8_unit_shift, DOT_F8_DECODE, DOT_F8_E2M1, DOT_F8_E4M3,
    DOT_F8_E4M3_FNUZ, DOT_F8_I8, DOT_F8_MAX_N, DOT_F8_REPEAT_Y,
};

#[derive(Debug)]
#[repr(C)]
pub struct SyscallDotF8Params {
    /// Address of the first vector, 8-byte aligned.
    pub x: *const u8,
    /// Address of the second vector, 8-byte aligned.
    pub y: *const u8,
    /// Number of values in each vector, at most [`DOT_F8_MAX_N`]: a multiple of 8, or of 16
    /// when either vector is E2M1, so both vectors are whole 64-bit words.
    pub n: u64,
    /// Bits 0..8: the format of x; bits 8..16: the format of y ([`dot_f8_formats`]); bit 16:
    /// [`DOT_F8_REPEAT_Y`], `y` holds only `block` values, multiplied by every block of `x`.
    /// The other bits must be 0.
    pub formats: u64,
    /// Values per block: a divisor of `n` that is a multiple of 8 (16 with E2M1); 0 means one
    /// block of `n` values, and is not allowed with [`DOT_F8_REPEAT_Y`].
    pub block: u64,
    /// When not null, receives the sum of each block (`n / block` values), 8-byte aligned.
    pub out: *mut i64,
}

/// Executes the experimental `DotF8` operation: the exact dot product `Σ x[i]·y[i]` of two
/// vectors of `n` values, each in its own format: int8, FP8 E4M3, FP8 E4M3FNUZ or FP4 E2M1.
///
/// Every value is an integer number of units `2^-dot_f8_unit_shift(format)`, so the products and
/// sums are exact integers. Returns the sum over all `n` values in units
/// `2^-(dot_f8_unit_shift(x format) + dot_f8_unit_shift(y format))`, and, when `out` is not
/// null, writes the sum of each block of `block` values to it, in the same units. With two int8
/// vectors, the result is the plain integer dot product. With [`DOT_F8_REPEAT_Y`] in `formats`,
/// `y` holds `block` values and block `b` of `x` is multiplied by them: with `x` a matrix of
/// `n / block` rows of `block` values, `out` receives the matrix-vector product.
///
/// Proven by the `DotF8` state machine. The assembly emulator does not support it in its memory
/// operation modes yet, so proofs of programs that call it use the Rust emulator.
///
/// ### Safety
///
/// `x` and `y` must point to the bytes of `n` values (`y` of `block` values with
/// [`DOT_F8_REPEAT_Y`]), `out` (if not null) to `n / block`
/// writable `i64`, all 8-byte aligned and with the length rules above met; `out` must not
/// overlap `x`, `y` or the parameter block.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_dot_f8")]
pub unsafe extern "C" fn syscall_dot_f8(params: &SyscallDotF8Params) -> i64 {
    #[cfg(not(zisk_guest))]
    {
        let (fx, fy) = (params.formats as u8, (params.formats >> 8) as u8);
        let repeat_y = params.formats & DOT_F8_REPEAT_Y != 0;
        let n = params.n as usize;
        let block = if params.block == 0 { n } else { params.block as usize };
        let bytes = |p: *const u8, f: u8, len: usize| unsafe {
            core::slice::from_raw_parts(p, len * dot_f8_bits(f) as usize / 8)
        };
        let x = bytes(params.x, fx, n);
        let y = bytes(params.y, fy, if repeat_y { block.min(n) } else { n });
        let out = (!params.out.is_null() && n > 0)
            .then(|| unsafe { core::slice::from_raw_parts_mut(params.out, n / block) });
        zisk_definitions::dot_f8_reference_with(fx, x, fy, y, n, block.max(1), repeat_y, out)
    }
    #[cfg(zisk_guest)]
    {
        ziskos_syscall_ret_u64!(zisk_definitions::SYSCALL_DOT_F8_ID, params) as i64
    }
}
