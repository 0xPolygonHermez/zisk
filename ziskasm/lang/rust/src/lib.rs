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
/// `zisklib_keccak` (current constraint: `len % 8 == 0`, `input` 8-byte aligned).
/// The placeholder fills `output` with a sentinel (`0xBA`), so a correct hash
/// proves the ziskasm routine ran. `black_box` on all arguments is essential (see
/// the crate docs): otherwise the optimizer would elide the `a0`/`a1` setup.
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
/// of `input`. Marshals the `&[u8]` into `(ptr, len)` and returns the `[u8; 32]`
/// buffer; only those flattened primitives cross into ziskasm.
///
/// Current constraint inherited from `zisklib_keccak`: `input.len() % 8 == 0` and
/// `input` 8-byte aligned. (A future version can hide this by copying into an
/// aligned, length-padded scratch buffer.)
pub fn keccak256(input: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    // SAFETY: `input` is a valid slice of `input.len()` bytes; `out` is 32 writable bytes.
    unsafe { ziskos_keccak(input.as_ptr(), input.len(), out.as_mut_ptr()) };
    out
}
