//! Rust bindings for the ZisK assembly library (`ziskasm/lib/`).
//!
//! A guest program links this crate and calls its functions. Each `ziskos_*` item
//! is a raw C-ABI **stub** with a stable `#[no_mangle]` symbol and a placeholder
//! body; during transpilation (`elf2rom`) the stub's entry is redirected to the
//! matching hand-written `zisklib_*` routine in `ziskasm/lib/*.zisk`, so the
//! ziskasm implementation runs in the guest's place. On top of the raw stubs sit
//! ergonomic Rust wrappers (e.g. [`keccak256`]) that marshal idiomatic Rust types
//! (`&[u8]`, `[u8; 32]`) into the flat `(ptr, len, ...)` primitives the ABI
//! boundary requires.
//!
//! This is the Rust language binding; sibling directories under `ziskasm/lang/`
//! can provide equivalent bindings for other high-level languages.
//!
//! Stub rules that keep the redirect working:
//! - `#[no_mangle]` + `#[inline(never)]`: a stable symbol and a real call site the
//!   transpiler can redirect (never inlined/folded away).
//! - The body must *touch every argument* (via [`core::hint::black_box`]). The
//!   redirected routine reads its args from `a0..a7`; a stub that ignored an
//!   argument would let the optimizer elide setting up that register at the call
//!   site, leaving garbage for the real routine.

#![no_std]

use core::hint::black_box;

/// `a + b`. Implemented in ziskasm as `zisklib_add` (a demo routine). The
/// placeholder returns an obviously-wrong, argument-dependent sentinel
/// (`0xBAD00000000 + a + b`); a plain sum proves the ziskasm routine ran in its
/// place. Being argument-dependent, the optimizer cannot const-fold the call away.
#[no_mangle]
#[inline(never)]
pub extern "C" fn ziskos_add(a: u64, b: u64) -> u64 {
    0xBAD_0000_0000_u64.wrapping_add(a).wrapping_add(b)
}

/// `keccak256(input[0..len])` → `output[0..32]`. Raw ABI boundary redirected to
/// `zisklib_keccak` (any `len`, any `input` alignment). The placeholder fills
/// `output` with a sentinel (`0xBA`), so a correct hash proves the ziskasm routine
/// ran. `black_box` on all arguments is essential (see the crate docs): otherwise
/// the optimizer would elide the `a0`/`a1` setup.
///
/// # Safety
/// `input` must point to `len` readable bytes and `output` to 32 writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ziskos_keccak(input: *const u8, len: usize, output: *mut u8) {
    let (_input, _len, output) = black_box((input, len, output));
    for i in 0..32usize {
        output.add(i).write(0xBA);
    }
}

/// Ergonomic Rust API over the raw [`ziskos_keccak`] boundary: the keccak256 digest
/// of `input` (any length, any alignment). Marshals the `&[u8]` into `(ptr, len)`
/// and returns the `[u8; 32]` buffer; only those flattened primitives cross into
/// ziskasm.
pub fn keccak256(input: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    // SAFETY: `input` is a valid slice of `input.len()` bytes; `out` is 32 writable bytes.
    unsafe { ziskos_keccak(input.as_ptr(), input.len(), out.as_mut_ptr()) };
    out
}

/// `a^(-1) mod 2^256` if it exists, else "not invertible". Raw ABI boundary
/// redirected to `zisklib_uint256.zisk`'s `zisklib_inv256`: returns `1` and writes
/// `result[0..4]` when `a` is invertible (odd), `0` otherwise. The placeholder
/// writes a sentinel and returns an argument-dependent value so the optimizer
/// keeps the call and sets up both argument registers.
///
/// # Safety
/// `a` must point to a valid `[u64; 4]` and `result` to a writable `[u64; 4]`.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ziskos_inv256(a: *const u64, result: *mut u64) -> u64 {
    let (a, result) = black_box((a, result));
    for i in 0..4usize {
        result.add(i).write(0x0BAD_0BAD_0BAD_0BAD);
    }
    black_box(a as u64)
}

/// Ergonomic Rust API over [`ziskos_inv256`]: the modular inverse of `a` mod
/// 2^256, or `None` if `a` is not invertible (i.e. even). Mirrors ziskos
/// `inv256`. The ziskasm routine hints the inverse and verifies `a * inv ≡ 1`
/// (mod 2^256) with the arith256 precompile before returning it.
pub fn inv256(a: &[u64; 4]) -> Option<[u64; 4]> {
    let mut result = [0u64; 4];
    // SAFETY: `a` is a `[u64; 4]`; `result` is a writable `[u64; 4]`.
    let invertible = unsafe { ziskos_inv256(a.as_ptr(), result.as_mut_ptr()) };
    (invertible != 0).then_some(result)
}
