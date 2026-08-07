//! Guest-side stubs for functions implemented in the ZisK library
//! (`ziskasm/lib/zisklib.zisk`). Each `ziskos_*` stub has a real symbol and a
//! placeholder body so the program compiles; on the ZisK target the transpiler
//! (`elf2rom`) redirects the stub's entry to the matching hand-written
//! `zisklib_*` routine, so the ziskasm implementation runs in its place.
//!
//! Stubs are `#[unsafe(no_mangle)] extern "C"` (stable global symbol, RISC-V C
//! ABI) and `#[inline(never)]` so the call site is a real call the transpiler can
//! redirect. The placeholder returns a sentinel, so if a stub ever runs (redirect
//! failed) the result is obviously wrong. This file will grow the `ziskos_*`
//! precompile wrappers over time.

/// `a + b`. Implemented in ziskasm as `zisklib_add`. The placeholder returns an
/// obviously-wrong, argument-dependent sentinel (`0xBAD00000000 + a + b`); a plain
/// sum proves the ziskasm routine ran in its place. The body must depend on the
/// arguments so the optimizer cannot const-fold the call away (which would leave
/// no call for the transpiler to redirect, and GC the symbol).
#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn ziskos_add(a: u64, b: u64) -> u64 {
    0xBAD_0000_0000_u64.wrapping_add(a).wrapping_add(b)
}

/// `keccak256(input[0..len])` → `output[0..32]`. Implemented in ziskasm as
/// `zisklib_keccak` (current constraint: `len % 8 == 0`, `input` 8-byte aligned).
/// The placeholder fills the output with a sentinel (`0xBA`), so a correct hash
/// proves the ziskasm routine ran in its place.
///
/// `black_box` on ALL arguments is essential: the redirected `zisklib_keccak`
/// reads `input`/`len` from `a0`/`a1`, but a placeholder that ignored them would
/// let the optimizer elide setting up those argument registers at the call site
/// (it only needs `output`), leaving garbage in `a0`/`a1` for the real routine.
///
/// # Safety
/// `input` must point to `len` readable bytes and `output` to 32 writable bytes.
#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn ziskos_keccak(input: *const u8, len: usize, output: *mut u8) {
    let (_input, _len, output) = core::hint::black_box((input, len, output));
    for i in 0..32usize {
        unsafe { output.add(i).write(0xBA) };
    }
}
