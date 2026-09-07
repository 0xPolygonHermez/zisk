//! Rust bindings for the ZisK assembly library (`ziskasm/zisklib/`).
//!
//! A guest program links this crate and calls its functions. Each `ziskos_*` item
//! is a raw C-ABI **stub** with a stable `#[no_mangle]` symbol and a placeholder
//! body; during transpilation (`elf2rom`) the stub's entry is redirected to the
//! matching hand-written `zisklib_*` routine in `ziskasm/zisklib/*.zisk`, so the
//! ziskasm implementation runs in the guest's place. On top of the raw stubs sit
//! ergonomic Rust wrappers (e.g. [`keccak256`]) that marshal idiomatic Rust types
//! (`&[u8]`, `[u8; 32]`) into the flat `(ptr, len, ...)` primitives the ABI
//! boundary requires.
//!
//! This is the Rust language binding; sibling directories under `ziskasm/lang/`
//! can provide equivalent bindings for other high-level languages.
//!
//! If a placeholder body ever runs, the redirect did NOT fire (a stripped ELF, a
//! missing symbol, or ziskemu/cargo-zisk built without the `ziskasm` feature), so
//! the stub FAILS HARD via [`stub_fail`] — it prints a diagnostic to the ZisK
//! stdout and then accesses address 0 to force abnormal termination — rather than
//! returning a plausible-but-wrong value. (The per-function doc comments below may
//! still describe the old sentinel return; the behavior is now the hard fault.)
//!
//! Stub rules that keep the redirect working:
//! - `#[no_mangle]` + `#[inline(never)]`: a stable symbol and a real call site the
//!   transpiler can redirect (never inlined/folded away).
//! - The body must *touch every argument* (via [`core::hint::black_box`]). The
//!   redirected routine reads its args from `a0..a7`; a stub that ignored an
//!   argument would let the optimizer elide setting up that register at the call
//!   site, leaving garbage for the real routine.
//! - Each stub body stays *distinct*: `stub_fail("<name>")` carries a unique string
//!   per stub, so identical-code folding cannot merge two same-signature stubs into
//!   one symbol (which would collapse their separate redirect entries).

#![no_std]

use core::hint::black_box;

/// Diagnostic + hard fault for a stub whose `elf2rom` redirect did not fire.
/// Prints a one-line message to the ZisK memory-mapped stdout (UART at
/// 0xA0400200, one byte per store), then accesses address 0 to force abnormal
/// termination. Reached only when the redirect did not happen (a stripped ELF, a
/// missing symbol, or ziskemu/cargo-zisk built without the `ziskasm` feature) —
/// failing hard beats returning a plausible-but-wrong value.
///
/// IMPORTANT: this is deliberately **not** `-> !`. The volatile store to address 0
/// aborts the machine at runtime, but to the compiler it is an ordinary returning
/// function (a store, not a diverge). Were it `-> !`, every stub tail-calling it
/// would be inferred `noreturn`, and a caller that can see the body — under LTO, or
/// if a stub shares a translation unit with a caller — would delete its own code
/// *after* the call. Since the redirected `.zisk` routine returns normally, that
/// would corrupt the guest. Returning `T: Default` keeps each stub non-diverging
/// (and lets the stub's tail `stub_fail(..)` supply its own return type), so no
/// caller is ever miscompiled; the returned value is never reached at runtime.
#[inline(never)]
fn stub_fail<T: Default>(name: &str) -> T {
    let uart = 0xA040_0200_usize as *mut u8; // ZisK stdout: one byte per store
    unsafe {
        for &c in b"ERROR: ziskasm stub reached without redirect: " {
            core::ptr::write_volatile(uart, c);
        }
        for &c in name.as_bytes() {
            core::ptr::write_volatile(uart, c);
        }
        for &c in b"() -- build ziskemu/cargo-zisk with --features ziskasm and do not strip the guest ELF\n" {
            core::ptr::write_volatile(uart, c);
        }
        // Access the null guard page -> abnormal termination. black_box hides the
        // null from the optimizer so it emits a real store rather than a trap/UB.
        let null = black_box(0usize) as *mut u8;
        core::ptr::write_volatile(null, 0);
    }
    black_box(T::default())
}

/// `a + b`. Implemented in ziskasm as `zisklib_add` (a demo routine). The
/// placeholder returns an obviously-wrong, argument-dependent sentinel
/// (`0xBAD00000000 + a + b`); a plain sum proves the ziskasm routine ran in its
/// place. Being argument-dependent, the optimizer cannot const-fold the call away.
#[no_mangle]
#[inline(never)]
pub extern "C" fn ziskos_add(a: u64, b: u64) -> u64 {
    let _ = black_box((a, b));
    stub_fail("ziskos_add")
}

/// `keccak256(input[0..len])` → `output[0..32]`. Raw ABI boundary redirected to
/// `ziskasm_zkvm_keccak256` (any `len`, any `input` alignment). The placeholder fills
/// `output` with a sentinel (`0xBA`), so a correct hash proves the ziskasm routine
/// ran. `black_box` on all arguments is essential (see the crate docs): otherwise
/// the optimizer would elide the `a0`/`a1` setup.
///
/// # Safety
/// `input` must point to `len` readable bytes and `output` to 32 writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ziskos_keccak(input: *const u8, len: usize, output: *mut u8) {
    let _ = black_box((input, len, output));
    stub_fail("ziskos_keccak")
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

/// `sha256(input[0..len])` → `output[0..32]`. Raw ABI boundary redirected to
/// `ziskasm_zkvm_sha256` (any `len`, any `input` alignment). Distinct sentinel byte
/// (`0x5A`) from [`ziskos_keccak`]'s `0xBA` so identical-code folding cannot merge
/// the two same-signature stubs (see the crate docs).
///
/// # Safety
/// `input` must point to `len` readable bytes and `output` to 32 writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ziskos_sha256(input: *const u8, len: usize, output: *mut u8) {
    let _ = black_box((input, len, output));
    stub_fail("ziskos_sha256")
}

/// Ergonomic Rust API over the raw [`ziskos_sha256`] boundary: the SHA2-256 digest
/// of `input` (any length, any alignment). Marshals the `&[u8]` into `(ptr, len)`
/// and returns the `[u8; 32]` buffer.
pub fn sha256(input: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    // SAFETY: `input` is a valid slice of `input.len()` bytes; `out` is 32 writable bytes.
    unsafe { ziskos_sha256(input.as_ptr(), input.len(), out.as_mut_ptr()) };
    out
}

/// BLAKE2b compression function F (RFC 7693). Raw ABI boundary redirected to
/// `ziskasm_zkvm_blake2f`: mixes message block `message[0..16]` into state
/// `state[0..8]` over `rounds` rounds with 128-bit counter `offset[0..2]` and
/// finalization flag `final_block`; `state` is updated in place. This is the
/// low-level primitive — the caller handles message blocking and padding.
/// Distinct sentinel (`0x0B2B…`); the 5-argument signature is unique anyway.
///
/// # Safety
/// `state` must point to a writable `[u64; 8]`, `message` to a readable `[u64; 16]`,
/// and `offset` to a readable `[u64; 2]`.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ziskos_blake2b_compress(
    rounds: u32,
    state: *mut u64,
    message: *const u64,
    offset: *const u64,
    final_block: u8,
) {
    let _ = black_box((rounds, state, message, offset, final_block));
    stub_fail("ziskos_blake2b_compress")
}

/// Ergonomic wrapper over [`ziskos_blake2b_compress`]: one BLAKE2b compression,
/// mixing message block `m` into state `h` over `rounds` rounds with counter `t`
/// and finalization flag `f` (`h` updated in place). Mirrors ziskos
/// `blake2b_compress`.
pub fn blake2b_compress(rounds: u32, h: &mut [u64; 8], m: &[u64; 16], t: &[u64; 2], f: bool) {
    // SAFETY: `h`/`m`/`t` are valid arrays of the required lengths (`h` writable).
    unsafe { ziskos_blake2b_compress(rounds, h.as_mut_ptr(), m.as_ptr(), t.as_ptr(), f as u8) };
}

/// `a^(-1) mod 2^256` if it exists, else "not invertible". Raw ABI boundary
/// redirected to `uint256/mul.zisk`'s `zisklib_inv256`: returns `1` and writes
/// `result[0..4]` when `a` is invertible (odd), `0` otherwise. The placeholder
/// writes a sentinel and returns an argument-dependent value so the optimizer
/// keeps the call and sets up both argument registers.
///
/// # Safety
/// `a` must point to a valid `[u64; 4]` and `result` to a writable `[u64; 4]`.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ziskos_inv256(a: *const u64, result: *mut u64) -> u64 {
    let _ = black_box((a, result));
    stub_fail("ziskos_inv256")
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

/// The 256-bit numeric bounds and one, for the saturating / modular helpers below.
const MAX_256: [u64; 4] = [u64::MAX; 4];
const ZERO_256: [u64; 4] = [0; 4];
const ONE_256: [u64; 4] = [1, 0, 0, 0];

/// Raw ABI boundary redirected to `zisklib_overflowing_add256`: writes `a + b`
/// (mod 2^256) to `result` and returns the carry-out (1 on overflow, else 0).
/// Placeholder touches all args and has a side effect so the call site survives.
///
/// # Safety
/// `a`, `b` must point to valid `[u64; 4]`; `result` to a writable `[u64; 4]`.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ziskos_overflowing_add256(
    a: *const u64,
    b: *const u64,
    result: *mut u64,
) -> u64 {
    let _ = black_box((a, b, result));
    stub_fail("ziskos_overflowing_add256")
}

/// Raw ABI boundary redirected to `zisklib_overflowing_sub256`: writes `a - b`
/// (mod 2^256) to `result` and returns 1 on borrow/underflow (`a < b`), else 0.
///
/// # Safety
/// `a`, `b` must point to valid `[u64; 4]`; `result` to a writable `[u64; 4]`.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ziskos_overflowing_sub256(
    a: *const u64,
    b: *const u64,
    result: *mut u64,
) -> u64 {
    let _ = black_box((a, b, result));
    stub_fail("ziskos_overflowing_sub256")
}

// --- 256-bit addition ---------------------------------------------------------

/// `a + b` (mod 2^256) with the carry-out flag (`true` on overflow).
pub fn overflowing_add256(a: &[u64; 4], b: &[u64; 4]) -> ([u64; 4], bool) {
    let mut r = [0u64; 4];
    // SAFETY: all three are valid `[u64; 4]` (`r` writable).
    let carry = unsafe { ziskos_overflowing_add256(a.as_ptr(), b.as_ptr(), r.as_mut_ptr()) };
    (r, carry != 0)
}

/// `a + b` (mod 2^256), wrapping on overflow.
pub fn wrapping_add256(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    overflowing_add256(a, b).0
}

/// `a + b`, or `None` on overflow.
pub fn checked_add256(a: &[u64; 4], b: &[u64; 4]) -> Option<[u64; 4]> {
    let (r, overflow) = overflowing_add256(a, b);
    (!overflow).then_some(r)
}

/// `a + b`, saturating to `2^256 - 1` on overflow.
pub fn saturating_add256(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    let (r, overflow) = overflowing_add256(a, b);
    if overflow {
        MAX_256
    } else {
        r
    }
}

// --- 256-bit subtraction ------------------------------------------------------

/// `a - b` (mod 2^256) with the borrow flag (`true` on underflow, `a < b`).
pub fn overflowing_sub256(a: &[u64; 4], b: &[u64; 4]) -> ([u64; 4], bool) {
    let mut r = [0u64; 4];
    // SAFETY: all three are valid `[u64; 4]` (`r` writable).
    let borrow = unsafe { ziskos_overflowing_sub256(a.as_ptr(), b.as_ptr(), r.as_mut_ptr()) };
    (r, borrow != 0)
}

/// `a - b` (mod 2^256), wrapping on underflow.
pub fn wrapping_sub256(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    overflowing_sub256(a, b).0
}

/// `a - b`, or `None` on underflow (`a < b`).
pub fn checked_sub256(a: &[u64; 4], b: &[u64; 4]) -> Option<[u64; 4]> {
    let (r, underflow) = overflowing_sub256(a, b);
    (!underflow).then_some(r)
}

/// `a - b`, saturating to `0` on underflow.
pub fn saturating_sub256(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    let (r, underflow) = overflowing_sub256(a, b);
    if underflow {
        ZERO_256
    } else {
        r
    }
}

// --- 256-bit negation (= 0 - a) ----------------------------------------------

/// `-a` (mod 2^256) with the flag (`true` unless `a == 0`).
pub fn overflowing_neg256(a: &[u64; 4]) -> ([u64; 4], bool) {
    overflowing_sub256(&ZERO_256, a)
}

/// `-a` (mod 2^256), wrapping.
pub fn wrapping_neg256(a: &[u64; 4]) -> [u64; 4] {
    overflowing_sub256(&ZERO_256, a).0
}

/// `-a`, or `None` unless `a == 0`.
pub fn checked_neg256(a: &[u64; 4]) -> Option<[u64; 4]> {
    let (r, flag) = overflowing_sub256(&ZERO_256, a);
    (!flag).then_some(r)
}

// --- 256-bit multiplication ---------------------------------------------------

/// Raw ABI boundary redirected to `zisklib_overflowing_mul256`: writes the low 256
/// bits of `a * b` to `result` and returns 1 if the product overflows 256 bits
/// (high 256 bits != 0), else 0. Distinct sentinel body (see the ICF note above).
///
/// # Safety
/// `a`, `b` must point to valid `[u64; 4]`; `result` to a writable `[u64; 4]`.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ziskos_overflowing_mul256(
    a: *const u64,
    b: *const u64,
    result: *mut u64,
) -> u64 {
    let _ = black_box((a, b, result));
    stub_fail("ziskos_overflowing_mul256")
}

/// Low 256 bits of `a * b`, with the overflow flag (`true` if the true product
/// exceeds 256 bits).
pub fn overflowing_mul256(a: &[u64; 4], b: &[u64; 4]) -> ([u64; 4], bool) {
    let mut r = [0u64; 4];
    // SAFETY: all three are valid `[u64; 4]` (`r` writable).
    let overflow = unsafe { ziskos_overflowing_mul256(a.as_ptr(), b.as_ptr(), r.as_mut_ptr()) };
    (r, overflow != 0)
}

/// `a * b` (mod 2^256), wrapping on overflow.
pub fn wrapping_mul256(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    overflowing_mul256(a, b).0
}

/// `a * b`, or `None` on overflow.
pub fn checked_mul256(a: &[u64; 4], b: &[u64; 4]) -> Option<[u64; 4]> {
    let (r, overflow) = overflowing_mul256(a, b);
    (!overflow).then_some(r)
}

/// `a * b`, saturating to `2^256 - 1` on overflow.
pub fn saturating_mul256(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    let (r, overflow) = overflowing_mul256(a, b);
    if overflow {
        MAX_256
    } else {
        r
    }
}

// --- 256-bit squaring (= a * a) -----------------------------------------------

/// Low 256 bits of `a^2`, with the overflow flag.
pub fn overflowing_square256(a: &[u64; 4]) -> ([u64; 4], bool) {
    overflowing_mul256(a, a)
}

/// `a^2` (mod 2^256), wrapping on overflow.
pub fn wrapping_square256(a: &[u64; 4]) -> [u64; 4] {
    overflowing_mul256(a, a).0
}

/// `a^2`, or `None` on overflow.
pub fn checked_square256(a: &[u64; 4]) -> Option<[u64; 4]> {
    let (r, overflow) = overflowing_mul256(a, a);
    (!overflow).then_some(r)
}

/// `a^2`, saturating to `2^256 - 1` on overflow.
pub fn saturating_square256(a: &[u64; 4]) -> [u64; 4] {
    let (r, overflow) = overflowing_mul256(a, a);
    if overflow {
        MAX_256
    } else {
        r
    }
}

// --- 256-bit division / remainder ---------------------------------------------

/// Raw ABI boundary redirected to `zisklib_div_rem256`: writes `a / b` to `q` and
/// `a % b` to `r`. Halts on `b == 0` on-target (the `checked_*` wrappers guard
/// against that in Rust first). Distinct sentinel bodies (see the ICF note above);
/// writing two output buffers already makes this body distinct from the one-output
/// add/sub/mul stubs.
///
/// # Safety
/// `a`, `b` must point to valid `[u64; 4]`; `q`, `r` to writable `[u64; 4]`.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ziskos_div_rem256(a: *const u64, b: *const u64, q: *mut u64, r: *mut u64) {
    let _ = black_box((a, b, q, r));
    stub_fail("ziskos_div_rem256")
}

/// `(a / b, a % b)` (Euclidean). **Panics on `b == 0`** (halts on-target).
pub fn div_rem256(a: &[u64; 4], b: &[u64; 4]) -> ([u64; 4], [u64; 4]) {
    let mut q = [0u64; 4];
    let mut r = [0u64; 4];
    // SAFETY: `a`, `b` are valid `[u64; 4]`; `q`, `r` are writable `[u64; 4]`.
    unsafe { ziskos_div_rem256(a.as_ptr(), b.as_ptr(), q.as_mut_ptr(), r.as_mut_ptr()) };
    (q, r)
}

/// `a / b`. **Panics on `b == 0`.**
pub fn wrapping_div256(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    div_rem256(a, b).0
}

/// `a % b`. **Panics on `b == 0`.**
pub fn wrapping_rem256(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    div_rem256(a, b).1
}

/// `a / b`, or `None` if `b == 0`.
pub fn checked_div256(a: &[u64; 4], b: &[u64; 4]) -> Option<[u64; 4]> {
    (b != &ZERO_256).then(|| div_rem256(a, b).0)
}

/// `a % b`, or `None` if `b == 0`.
pub fn checked_rem256(a: &[u64; 4], b: &[u64; 4]) -> Option<[u64; 4]> {
    (b != &ZERO_256).then(|| div_rem256(a, b).1)
}

/// Ceiling of `a / b` (rounds a nonzero remainder up). **Panics on `b == 0`.**
pub fn div_ceil256(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    let (q, r) = div_rem256(a, b);
    if r == ZERO_256 {
        q
    } else {
        wrapping_add256(&q, &[1, 0, 0, 0])
    }
}

// --- 256-bit modular arithmetic (arith256_mod: d = (a*b + c) mod module) -------
//
// The precompile requires `module != 0`; the wrappers short-circuit that case to
// `ZERO_256` (matching ziskos, which returns zero rather than panicking) and never
// call the stub with a zero modulus. Inputs need not be `< module`; the result is
// always reduced. Each stub carries a distinct sentinel (ICF, see the note above).

/// Raw ABI boundary redirected to `zisklib_reduce_mod256`: `result = a mod m`.
///
/// # Safety
/// `a`, `m` must point to valid `[u64; 4]`; `result` to a writable `[u64; 4]`.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ziskos_reduce_mod256(a: *const u64, m: *const u64, result: *mut u64) {
    let _ = black_box((a, m, result));
    stub_fail("ziskos_reduce_mod256")
}

/// Raw ABI boundary redirected to `zisklib_add_mod256`: `result = (a + b) mod m`.
///
/// # Safety
/// `a`, `b`, `m` must point to valid `[u64; 4]`; `result` to a writable `[u64; 4]`.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ziskos_add_mod256(
    a: *const u64,
    b: *const u64,
    m: *const u64,
    result: *mut u64,
) {
    let _ = black_box((a, b, m, result));
    stub_fail("ziskos_add_mod256")
}

/// Raw ABI boundary redirected to `zisklib_mul_mod256`: `result = (a * b) mod m`.
///
/// # Safety
/// `a`, `b`, `m` must point to valid `[u64; 4]`; `result` to a writable `[u64; 4]`.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ziskos_mul_mod256(
    a: *const u64,
    b: *const u64,
    m: *const u64,
    result: *mut u64,
) {
    let _ = black_box((a, b, m, result));
    stub_fail("ziskos_mul_mod256")
}

/// `a mod modulus` (`0` if `modulus == 0`).
pub fn reduce_mod256(a: &[u64; 4], modulus: &[u64; 4]) -> [u64; 4] {
    if modulus == &ZERO_256 {
        return ZERO_256;
    }
    let mut d = [0u64; 4];
    // SAFETY: `a`, `modulus` are valid `[u64; 4]`; `d` is a writable `[u64; 4]`.
    unsafe { ziskos_reduce_mod256(a.as_ptr(), modulus.as_ptr(), d.as_mut_ptr()) };
    d
}

/// `(a + b) mod modulus` (`0` if `modulus == 0`).
pub fn add_mod256(a: &[u64; 4], b: &[u64; 4], modulus: &[u64; 4]) -> [u64; 4] {
    if modulus == &ZERO_256 {
        return ZERO_256;
    }
    let mut d = [0u64; 4];
    // SAFETY: `a`, `b`, `modulus` are valid `[u64; 4]`; `d` is a writable `[u64; 4]`.
    unsafe { ziskos_add_mod256(a.as_ptr(), b.as_ptr(), modulus.as_ptr(), d.as_mut_ptr()) };
    d
}

/// `(a * b) mod modulus` (`0` if `modulus == 0`).
pub fn mul_mod256(a: &[u64; 4], b: &[u64; 4], modulus: &[u64; 4]) -> [u64; 4] {
    if modulus == &ZERO_256 {
        return ZERO_256;
    }
    let mut d = [0u64; 4];
    // SAFETY: `a`, `b`, `modulus` are valid `[u64; 4]`; `d` is a writable `[u64; 4]`.
    unsafe { ziskos_mul_mod256(a.as_ptr(), b.as_ptr(), modulus.as_ptr(), d.as_mut_ptr()) };
    d
}

/// `a^2 mod modulus` (`0` if `modulus == 0`).
pub fn square_mod256(a: &[u64; 4], modulus: &[u64; 4]) -> [u64; 4] {
    mul_mod256(a, a, modulus)
}

/// Raw ABI boundary redirected to `zisklib_inv_mod256`: writes `a^(-1) mod m` to
/// `result` and returns 1 if the inverse exists, else 0. On-target the routine
/// verifies whichever outcome the hint claims (the inverse, or a gcd witness that
/// none exists). Distinct sentinel (ICF, see the note above).
///
/// # Safety
/// `a`, `m` must point to valid `[u64; 4]`; `result` to a writable `[u64; 4]`.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ziskos_inv_mod256(a: *const u64, m: *const u64, result: *mut u64) -> u64 {
    let _ = black_box((a, m, result));
    stub_fail("ziskos_inv_mod256")
}

/// Modular inverse: `a^(-1) mod modulus`, or `None` if it does not exist (i.e.
/// `gcd(a, modulus) > 1`). `modulus == 0` yields `None`. The ziskasm routine hints
/// the result and verifies it (`a·inv ≡ 1 (mod m)` with `inv < m`, or a gcd witness
/// for non-existence) before returning.
pub fn inv_mod256(a: &[u64; 4], modulus: &[u64; 4]) -> Option<[u64; 4]> {
    if modulus == &ZERO_256 {
        return None;
    }
    let mut result = [0u64; 4];
    // SAFETY: `a`, `modulus` are valid `[u64; 4]`; `result` is a writable `[u64; 4]`.
    let has_inv = unsafe { ziskos_inv_mod256(a.as_ptr(), modulus.as_ptr(), result.as_mut_ptr()) };
    (has_inv != 0).then_some(result)
}

// --- 256-bit modular exponentiation -------------------------------------------

/// Raw ABI boundary redirected to `zisklib_pow_mod256`: `result = base^exp mod m`.
/// `m in {0, 1}` is handled by the wrapper. Distinct sentinel (ICF).
///
/// # Safety
/// `base`, `exp`, `m` must point to valid `[u64; 4]`; `result` to a writable one.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ziskos_pow_mod256(
    base: *const u64,
    exp: *const u64,
    m: *const u64,
    result: *mut u64,
) {
    let _ = black_box((base, exp, m, result));
    stub_fail("ziskos_pow_mod256")
}

/// `base^exp mod modulus`. `modulus in {0, 1}` yields `0` (every value is `0` mod 1).
pub fn pow_mod256(base: &[u64; 4], exp: &[u64; 4], modulus: &[u64; 4]) -> [u64; 4] {
    if modulus == &ZERO_256 || modulus == &ONE_256 {
        return ZERO_256;
    }
    let mut r = [0u64; 4];
    // SAFETY: `base`, `exp`, `modulus` are valid `[u64; 4]`; `r` is writable.
    unsafe { ziskos_pow_mod256(base.as_ptr(), exp.as_ptr(), modulus.as_ptr(), r.as_mut_ptr()) };
    r
}

// --- 256-bit exponentiation (mod 2^256) ---------------------------------------

/// Raw ABI boundary redirected to `zisklib_overflowing_pow256`: writes
/// `base^exp mod 2^256` to `result` and returns 1 if the true power exceeded 256
/// bits at any step, else 0. Distinct sentinel (ICF).
///
/// # Safety
/// `base`, `exp` must point to valid `[u64; 4]`; `result` to a writable `[u64; 4]`.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ziskos_overflowing_pow256(
    base: *const u64,
    exp: *const u64,
    result: *mut u64,
) -> u64 {
    let _ = black_box((base, exp, result));
    stub_fail("ziskos_overflowing_pow256")
}

/// `base^exp mod 2^256`, with the overflow flag (`true` if the true power exceeds
/// 256 bits).
pub fn overflowing_pow256(base: &[u64; 4], exp: &[u64; 4]) -> ([u64; 4], bool) {
    let mut r = [0u64; 4];
    // SAFETY: `base`, `exp` are valid `[u64; 4]`; `r` is a writable `[u64; 4]`.
    let overflow =
        unsafe { ziskos_overflowing_pow256(base.as_ptr(), exp.as_ptr(), r.as_mut_ptr()) };
    (r, overflow != 0)
}

/// `base^exp` (mod 2^256), wrapping on overflow.
pub fn wrapping_pow256(base: &[u64; 4], exp: &[u64; 4]) -> [u64; 4] {
    overflowing_pow256(base, exp).0
}

/// `base^exp`, or `None` on overflow.
pub fn checked_pow256(base: &[u64; 4], exp: &[u64; 4]) -> Option<[u64; 4]> {
    let (r, overflow) = overflowing_pow256(base, exp);
    (!overflow).then_some(r)
}

/// `base^exp`, saturating to `2^256 - 1` on overflow.
pub fn saturating_pow256(base: &[u64; 4], exp: &[u64; 4]) -> [u64; 4] {
    let (r, overflow) = overflowing_pow256(base, exp);
    if overflow {
        MAX_256
    } else {
        r
    }
}

// ===========================================================================
// Elliptic-curve routines (secp256k1 + secp256r1)
//
// Points and scalars cross the ABI as 256-bit little-endian limb arrays: a
// scalar/coordinate is `[u64; 4]` and an affine point is `[u64; 8]` = x‖y.
// Redirected to the hand-written `zisklib_*_secp256{k1,r1}` routines under
// `ziskasm/zisklib/secp256{k1,r1}/`.
// ===========================================================================

/// secp256k1 ECDSA verification, redirected to `zisklib_ecdsa_verify_secp256k1`.
/// Returns `1` iff `(r, s)` verifies over hash `z` under public key `pk`.
///
/// # Safety
/// `pk` points to 8 readable `u64`; `z`, `r`, `s` each to 4 readable `u64`.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ziskos_ecdsa_verify_secp256k1(
    pk: *const u64,
    z: *const u64,
    r: *const u64,
    s: *const u64,
) -> u64 {
    let _ = black_box((pk, z, r, s));
    stub_fail("ziskos_ecdsa_verify_secp256k1")
}

/// Ergonomic API over [`ziskos_ecdsa_verify_secp256k1`]: `true` iff the signature
/// `(r, s)` over hash `z` is valid for public key `pk` (x‖y, little-endian limbs).
pub fn secp256k1_ecdsa_verify(pk: &[u64; 8], z: &[u64; 4], r: &[u64; 4], s: &[u64; 4]) -> bool {
    // SAFETY: all pointers reference the correctly-sized local arrays.
    unsafe { ziskos_ecdsa_verify_secp256k1(pk.as_ptr(), z.as_ptr(), r.as_ptr(), s.as_ptr()) != 0 }
}

/// secp256k1 public-key recovery, redirected to `zisklib_ecdsa_recover_secp256k1`.
/// Writes the recovered public key (x‖y) to `result` and returns an error code
/// (`0` = success; nonzero = failure, see the wrapper).
///
/// # Safety
/// `r`, `s`, `z` each point to 4 readable `u64`; `result` to 8 writable `u64`.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ziskos_ecdsa_recover_secp256k1(
    r: *const u64,
    s: *const u64,
    z: *const u64,
    recid: u64,
    result: *mut u64,
) -> u64 {
    let _ = black_box((r, s, z, recid, result));
    stub_fail("ziskos_ecdsa_recover_secp256k1")
}

/// Ergonomic API over [`ziskos_ecdsa_recover_secp256k1`]: recover the public key
/// (x‖y, little-endian limbs) that produced signature `(r, s)` over hash `z` with
/// recovery id `recid`. `Ok(pk)` on success, or `Err(code)` (`1` invalid r, `2`
/// invalid s, `3` invalid recid, `4` point not on curve, `5` recovery failed).
pub fn secp256k1_ecdsa_recover(
    r: &[u64; 4],
    s: &[u64; 4],
    z: &[u64; 4],
    recid: u64,
) -> Result<[u64; 8], u64> {
    let mut pk = [0u64; 8];
    // SAFETY: `r`, `s`, `z` are `[u64; 4]`; `pk` is a writable `[u64; 8]`.
    let err = unsafe {
        ziskos_ecdsa_recover_secp256k1(r.as_ptr(), s.as_ptr(), z.as_ptr(), recid, pk.as_mut_ptr())
    };
    if err == 0 {
        Ok(pk)
    } else {
        Err(err)
    }
}

/// secp256k1 BIP-340 Schnorr verification, redirected to
/// `zisklib_schnorr_verify_secp256k1`. Returns `1` iff `(r, s)` verifies over
/// `msg` under the x-only public key `pk_x`.
///
/// # Safety
/// `pk_x`, `r`, `s` each point to 4 readable `u64`; `msg` to `msg_len` bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ziskos_schnorr_verify_secp256k1(
    pk_x: *const u64,
    r: *const u64,
    s: *const u64,
    msg: *const u8,
    msg_len: u64,
) -> u64 {
    let _ = black_box((pk_x, r, s, msg, msg_len));
    stub_fail("ziskos_schnorr_verify_secp256k1")
}

/// Ergonomic API over [`ziskos_schnorr_verify_secp256k1`]: `true` iff the BIP-340
/// signature `(r, s)` over `msg` is valid for x-only public key `pk_x` (little-
/// endian limbs). `msg` is raw bytes of any length.
pub fn secp256k1_schnorr_verify(pk_x: &[u64; 4], r: &[u64; 4], s: &[u64; 4], msg: &[u8]) -> bool {
    // SAFETY: limb arrays are `[u64; 4]`; `msg` is a valid slice of `msg.len()` bytes.
    unsafe {
        ziskos_schnorr_verify_secp256k1(
            pk_x.as_ptr(),
            r.as_ptr(),
            s.as_ptr(),
            msg.as_ptr(),
            msg.len() as u64,
        ) != 0
    }
}

/// secp256r1 (NIST P-256) ECDSA verification, redirected to
/// `zisklib_ecdsa_verify_secp256r1`. Returns `1` iff `(r, s)` verifies over hash
/// `z` under public key `pk`.
///
/// # Safety
/// `pk` points to 8 readable `u64`; `z`, `r`, `s` each to 4 readable `u64`.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ziskos_ecdsa_verify_secp256r1(
    pk: *const u64,
    z: *const u64,
    r: *const u64,
    s: *const u64,
) -> u64 {
    let _ = black_box((pk, z, r, s));
    stub_fail("ziskos_ecdsa_verify_secp256r1")
}

/// Ergonomic API over [`ziskos_ecdsa_verify_secp256r1`]: `true` iff the signature
/// `(r, s)` over hash `z` is valid for P-256 public key `pk` (x‖y, little-endian
/// limbs).
pub fn secp256r1_ecdsa_verify(pk: &[u64; 8], z: &[u64; 4], r: &[u64; 4], s: &[u64; 4]) -> bool {
    // SAFETY: all pointers reference the correctly-sized local arrays.
    unsafe { ziskos_ecdsa_verify_secp256r1(pk.as_ptr(), z.as_ptr(), r.as_ptr(), s.as_ptr()) != 0 }
}

/// BN254 (alt_bn128) optimal-ate pairing check (EIP-197 ecPairing), redirected to
/// `zisklib_pairing_check_bn254`. `g1`/`g2` are `n` points (affine, x‖y, little-
/// endian limbs; G1 = 8 u64, G2 = 16 u64). Returns a status code: `0` = the
/// pairing product is 1 (accept), `1` = it is not (reject), `2`..`6` = input
/// validation errors (see the wrapper).
///
/// # Safety
/// `g1` points to `8*n` readable `u64`, `g2` to `16*n` readable `u64`.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ziskos_pairing_check_bn254(g1: *const u64, g2: *const u64, n: u64) -> u64 {
    let _ = black_box((g1, g2, n));
    stub_fail("ziskos_pairing_check_bn254")
}

/// Ergonomic API over [`ziskos_pairing_check_bn254`]: returns the raw status code
/// for the BN254 pairing check over `n` pairs (`g1[i]`, `g2[i]`). `0` accepts
/// (∏ e(g1ᵢ, g2ᵢ) == 1); `1` rejects; `2` G1 not canonical; `3` G1 not on curve;
/// `4` G2 not canonical; `5` G2 not on curve; `6` G2 not in subgroup.
pub fn bn254_pairing_check(g1: &[[u64; 8]], g2: &[[u64; 16]]) -> u64 {
    assert_eq!(g1.len(), g2.len(), "g1 and g2 must have the same number of points");
    // SAFETY: `g1`/`g2` are contiguous arrays of `len` points of 8/16 u64 each.
    unsafe {
        ziskos_pairing_check_bn254(
            g1.as_ptr() as *const u64,
            g2.as_ptr() as *const u64,
            g1.len() as u64,
        )
    }
}

/// BLS12-381 optimal-ate pairing check (EIP-2537), redirected to
/// `zisklib_pairing_check_bls12_381`. `g1`/`g2` are `n` points (affine, x‖y,
/// little-endian limbs; G1 = 12 u64, G2 = 24 u64). Returns a status code: `0` =
/// the pairing product is 1 (accept), `1` = it is not (reject), `2`..`7` = input
/// validation errors (see the wrapper).
///
/// # Safety
/// `g1` points to `12*n` readable `u64`, `g2` to `24*n` readable `u64`.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ziskos_pairing_check_bls12_381(
    g1: *const u64,
    g2: *const u64,
    n: u64,
) -> u64 {
    let _ = black_box((g1, g2, n));
    stub_fail("ziskos_pairing_check_bls12_381")
}

/// Ergonomic API over [`ziskos_pairing_check_bls12_381`]: returns the raw status
/// code for the BLS12-381 pairing check over `n` pairs (`g1[i]`, `g2[i]`). `0`
/// accepts (∏ e(g1ᵢ, g2ᵢ) == 1); `1` rejects; `2` G1 not canonical; `3` G1 not
/// on curve; `4` G1 not in subgroup; `5` G2 not canonical; `6` G2 not on curve;
/// `7` G2 not in subgroup.
pub fn bls12_381_pairing_check(g1: &[[u64; 12]], g2: &[[u64; 24]]) -> u64 {
    assert_eq!(g1.len(), g2.len(), "g1 and g2 must have the same number of points");
    // SAFETY: `g1`/`g2` are contiguous arrays of `len` points of 12/24 u64 each.
    unsafe {
        ziskos_pairing_check_bls12_381(
            g1.as_ptr() as *const u64,
            g2.as_ptr() as *const u64,
            g1.len() as u64,
        )
    }
}

/// BLS12-381 map field element Fp → G1 (EIP-2537 MAP_FP_TO_G1), redirected to
/// `zisklib_map_to_curve_g1_bls12_381`. Writes the resulting G1 point (12 u64,
/// x‖y little-endian) to `result`; returns `0` on success, `1` if `u ≥ p`.
///
/// # Safety
/// `u` points to 6 readable `u64`, `result` to 12 writable `u64`.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ziskos_map_to_curve_g1_bls12_381(u: *const u64, result: *mut u64) -> u64 {
    let _ = black_box((u, result));
    stub_fail("ziskos_map_to_curve_g1_bls12_381")
}

/// Ergonomic API over [`ziskos_map_to_curve_g1_bls12_381`]: maps `u ∈ Fp` to a
/// G1 point. Returns `Ok(point)` (12 u64, x‖y) or `Err(1)` when `u ≥ p`.
pub fn bls12_381_map_to_curve_g1(u: &[u64; 6]) -> Result<[u64; 12], u64> {
    let mut point = [0u64; 12];
    // SAFETY: `u` is 6 u64, `point` is 12 u64.
    let status = unsafe { ziskos_map_to_curve_g1_bls12_381(u.as_ptr(), point.as_mut_ptr()) };
    if status == 0 {
        Ok(point)
    } else {
        Err(status)
    }
}

/// BLS12-381 map field element Fp2 → G2 (EIP-2537 MAP_FP2_TO_G2), redirected to
/// `zisklib_map_to_curve_g2_bls12_381`. Writes the resulting G2 point (24 u64,
/// x‖y little-endian, each Fp2) to `result`; returns `0` on success, `1` if
/// either coordinate of `u` is `≥ p`.
///
/// # Safety
/// `u` points to 12 readable `u64`, `result` to 24 writable `u64`.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ziskos_map_to_curve_g2_bls12_381(u: *const u64, result: *mut u64) -> u64 {
    let _ = black_box((u, result));
    stub_fail("ziskos_map_to_curve_g2_bls12_381")
}

/// Ergonomic API over [`ziskos_map_to_curve_g2_bls12_381`]: maps `u ∈ Fp2` to a
/// G2 point. Returns `Ok(point)` (24 u64, x‖y each Fp2) or `Err(1)` when either
/// coordinate of `u` is `≥ p`.
pub fn bls12_381_map_to_curve_g2(u: &[u64; 12]) -> Result<[u64; 24], u64> {
    let mut point = [0u64; 24];
    // SAFETY: `u` is 12 u64, `point` is 24 u64.
    let status = unsafe { ziskos_map_to_curve_g2_bls12_381(u.as_ptr(), point.as_mut_ptr()) };
    if status == 0 {
        Ok(point)
    } else {
        Err(status)
    }
}

/// BLS12-381 hash-to-curve to G2 (RFC 9380, suite BLS12381G2_XMD:SHA-256_SSWU_RO_),
/// redirected to `zisklib_hash_to_curve_g2_bls12_381`. Hashes `msg` under domain
/// tag `dst` to a G2 point (24 u64, x‖y each Fp2) written to `result`.
///
/// # Safety
/// `msg`/`dst` are readable for `msg_len`/`dst_len` bytes; `result` is 24 writable u64.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ziskos_hash_to_curve_g2_bls12_381(
    msg: *const u8,
    msg_len: u64,
    dst: *const u8,
    dst_len: u64,
    result: *mut u64,
) {
    let _ = black_box((msg, msg_len, dst, dst_len, result));
    stub_fail("ziskos_hash_to_curve_g2_bls12_381")
}

/// Ergonomic API over [`ziskos_hash_to_curve_g2_bls12_381`]: returns the G2 point
/// (24 u64, x‖y each Fp2) that `msg` hashes to under domain-separation tag `dst`.
pub fn bls12_381_hash_to_curve_g2(msg: &[u8], dst: &[u8]) -> [u64; 24] {
    let mut point = [0u64; 24];
    // SAFETY: slices are valid for their lengths; `point` is 24 u64.
    unsafe {
        ziskos_hash_to_curve_g2_bls12_381(
            msg.as_ptr(),
            msg.len() as u64,
            dst.as_ptr(),
            dst.len() as u64,
            point.as_mut_ptr(),
        );
    }
    point
}

/// BLS12-381 signature verification (minimal-pubkey-size / G2 signatures, basic
/// scheme, DST `BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_NUL_`), redirected to
/// `zisklib_bls_verify_bls12_381`. `pk` is a 48-byte compressed G1 public key,
/// `sig` a 96-byte compressed G2 signature. Returns `true` iff the signature is
/// valid for `msg`.
///
/// # Safety
/// `pk` points to 48 bytes, `sig` to 96 bytes, `msg` to `msg_len` bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ziskos_bls_verify_bls12_381(
    pk: *const u8,
    msg: *const u8,
    msg_len: u64,
    sig: *const u8,
) -> u64 {
    let _ = black_box((pk, msg, msg_len, sig));
    stub_fail("ziskos_bls_verify_bls12_381")
}

/// Ergonomic API over [`ziskos_bls_verify_bls12_381`]: verifies a BLS signature
/// (48-byte compressed G1 `pk`, 96-byte compressed G2 `sig`) over `msg`.
pub fn bls12_381_verify(pk: &[u8; 48], msg: &[u8], sig: &[u8; 96]) -> bool {
    // SAFETY: `pk`/`sig` are fixed-size; `msg` is valid for its length.
    let r = unsafe {
        ziskos_bls_verify_bls12_381(pk.as_ptr(), msg.as_ptr(), msg.len() as u64, sig.as_ptr())
    };
    r == 1
}

/// BLS12-381 KZG proof verification (EIP-4844 point evaluation), redirected to
/// `zisklib_verify_kzg_proof_bls12_381`. Checks that a polynomial committed to by
/// `commitment` (48-byte compressed G1) evaluates to `y` at `z` with the given
/// `proof` (48-byte compressed G1). `z`/`y` are 32-byte big-endian field scalars.
/// Returns `true` iff the proof is valid.
///
/// # Safety
/// `z`/`y` point to 32 bytes each; `commitment`/`proof` to 48 bytes each.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ziskos_verify_kzg_proof_bls12_381(
    z: *const u8,
    y: *const u8,
    commitment: *const u8,
    proof: *const u8,
) -> u64 {
    let _ = black_box((z, y, commitment, proof));
    stub_fail("ziskos_verify_kzg_proof_bls12_381")
}

/// Ergonomic API over [`ziskos_verify_kzg_proof_bls12_381`]: verifies an EIP-4844
/// KZG evaluation proof that the polynomial behind `commitment` takes value `y`
/// at point `z`, given `proof`. `z`/`y` are 32-byte big-endian scalars;
/// `commitment`/`proof` are 48-byte compressed G1 points.
pub fn bls12_381_verify_kzg_proof(
    z: &[u8; 32],
    y: &[u8; 32],
    commitment: &[u8; 48],
    proof: &[u8; 48],
) -> bool {
    // SAFETY: all inputs are fixed-size arrays of the documented lengths.
    let r = unsafe {
        ziskos_verify_kzg_proof_bls12_381(
            z.as_ptr(),
            y.as_ptr(),
            commitment.as_ptr(),
            proof.as_ptr(),
        )
    };
    r == 1
}

/// EIP-198 modular exponentiation `base^exp mod modulus` over little-endian u64
/// limb arrays, redirected to `zisklib_modexp_u64_c`. Handles arbitrary-precision
/// operands (radix-2^256 with hint-verified division). Writes the result limbs to
/// `result` (little-endian) and returns the number of u64 limbs written.
///
/// # Safety
/// `base`/`exp`/`modulus` point to their respective `*_len` readable u64s; `result`
/// must be writable for at least `modulus_len.next_multiple_of(4)` u64s.
#[allow(clippy::too_many_arguments)]
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ziskos_modexp_u64_c(
    base: *const u64,
    base_len: usize,
    exp: *const u64,
    exp_len: usize,
    modulus: *const u64,
    modulus_len: usize,
    result: *mut u64,
) -> usize {
    let _ = black_box((base, base_len, exp, exp_len, modulus, modulus_len, result));
    stub_fail("ziskos_modexp_u64_c")
}

/// Ergonomic API over [`ziskos_modexp_u64_c`]: computes `base^exp mod modulus`
/// where all operands are little-endian u64 limb slices. Writes the result limbs
/// to `result` and returns the number of limbs written (edge cases and single-U256
/// moduli return 4; larger moduli return `ceil(modulus_len/4) * 4`).
pub fn modexp_u64(base: &[u64], exp: &[u64], modulus: &[u64], result: &mut [u64]) -> usize {
    // SAFETY: all slices are valid for their lengths; `result` holds the output.
    unsafe {
        ziskos_modexp_u64_c(
            base.as_ptr(),
            base.len(),
            exp.as_ptr(),
            exp.len(),
            modulus.as_ptr(),
            modulus.len(),
            result.as_mut_ptr(),
        )
    }
}

// ============================================================================
// EF zkVM-accelerator ABI (zkvm_accelerators.h) — Rust stubs. Same model as the
// C stubs (ziskasm/lang/c/src/zkvm_stubs.c): each `zkvm_*` is redirected by
// elf2rom DIRECTLY to the native `ziskasm_zkvm_*` .zisk routine (single call, no
// wrapper). A guest links EITHER these OR the portable `zkvm-interface` impl of
// the same standard symbols — never both. Byte structs cross as raw pointers
// (ABI-identical). Return: 0 = ZKVM_EOK, -1 = ZKVM_EFAIL.
// ============================================================================

/// `zkvm_keccak256(data, len, output)` — redirected to `ziskasm_zkvm_keccak256`.
///
/// # Safety
/// `data` points to `len` readable bytes; `output` to 32 writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_keccak256(data: *const u8, len: usize, output: *mut u8) -> i32 {
    let _ = black_box((data, len, output));
    stub_fail("zkvm_keccak256")
}

/// `zkvm_sha256(data, len, output)` — redirected to the shared `ziskasm_zkvm_sha256`.
///
/// # Safety
/// `data` points to `len` readable bytes; `output` to 32 writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_sha256(data: *const u8, len: usize, output: *mut u8) -> i32 {
    let _ = black_box((data, len, output));
    stub_fail("zkvm_sha256")
}

/// `zkvm_keccak_f1600(state)` — redirected to `ziskasm_zkvm_keccak_f1600`. Applies
/// the Keccak-f[1600] permutation in place to the raw 25-word `state`. Returns
/// 0 = ZKVM_EOK.
///
/// # Safety
/// `state` must point to a writable `[u64; 25]`.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_keccak_f1600(state: *mut u64) -> i32 {
    let _ = black_box((state,));
    stub_fail("zkvm_keccak_f1600")
}

/// `zkvm_secp256k1_verify(msg, sig, pubkey, verified)` — redirected to
/// `ziskasm_zkvm_secp256k1_verify`. msg=32B, sig=64B (r||s), pubkey=64B (x||y),
/// all big-endian; `verified` is written 0/1. Returns 0 = ZKVM_EOK.
///
/// # Safety
/// `msg`/`sig`/`pubkey` point to 32/64/64 readable bytes; `verified` is writable.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_secp256k1_verify(
    msg: *const u8,
    sig: *const u8,
    pubkey: *const u8,
    verified: *mut u8,
) -> i32 {
    let _ = black_box((msg, sig, pubkey, verified));
    stub_fail("zkvm_secp256k1_verify")
}

/// `zkvm_secp256k1_ecrecover(msg, sig, recid, output)` — redirected to
/// `ziskasm_zkvm_secp256k1_ecrecover`. output=64B (x||y, BE). 0=EOK, -1=EFAIL.
/// # Safety
/// `msg`/`sig` point to 32/64 readable bytes; `output` to 64 writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_secp256k1_ecrecover(
    msg: *const u8,
    sig: *const u8,
    recid: u8,
    output: *mut u8,
) -> i32 {
    let _ = black_box((msg, sig, recid, output));
    stub_fail("zkvm_secp256k1_ecrecover")
}

/// `zkvm_secp256r1_verify(msg, sig, pubkey, verified)` — redirected to
/// `ziskasm_zkvm_secp256r1_verify`. Same shape as the secp256k1 variant.
/// # Safety
/// `msg`/`sig`/`pubkey` point to 32/64/64 readable bytes; `verified` writable.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_secp256r1_verify(
    msg: *const u8,
    sig: *const u8,
    pubkey: *const u8,
    verified: *mut u8,
) -> i32 {
    let _ = black_box((msg, sig, pubkey, verified));
    stub_fail("zkvm_secp256r1_verify")
}

/// `zkvm_blake2f(rounds, h, m, t, f)` — redirected to `ziskasm_zkvm_blake2f`.
/// h=64B (updated in place), m=128B, t=16B, all little-endian. 0=EOK.
/// # Safety
/// `h` is 64 writable bytes; `m`/`t` are 128/16 readable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_blake2f(
    rounds: u32,
    h: *mut u8,
    m: *const u8,
    t: *const u8,
    f: u8,
) -> i32 {
    let _ = black_box((rounds, h, m, t, f));
    stub_fail("zkvm_blake2f")
}

/// `zkvm_modexp(base, base_len, exp, exp_len, mod, mod_len, output)` (EIP-198) —
/// redirected to `ziskasm_zkvm_modexp`. All operands are big-endian byte arrays
/// of arbitrary length; `output` receives `mod_len` big-endian bytes. 0=EOK.
/// # Safety
/// Each pointer/len pair describes a readable byte range; `output` is `mod_len`
/// writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_modexp(
    base: *const u8,
    base_len: usize,
    exp: *const u8,
    exp_len: usize,
    modulus: *const u8,
    mod_len: usize,
    output: *mut u8,
) -> i32 {
    let _ = black_box((base, base_len, exp, exp_len, modulus, mod_len, output));
    stub_fail("zkvm_modexp")
}

/// `zkvm_bn254_g1_add(p1, p2, result)` — redirected to `ziskasm_zkvm_bn254_g1_add`.
/// G1 points are 64 big-endian bytes (x‖y). 0=EOK, -1=EFAIL (not in field / off curve).
/// # Safety
/// `p1`/`p2` are 64 readable bytes; `result` is 64 writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_bn254_g1_add(p1: *const u8, p2: *const u8, result: *mut u8) -> i32 {
    let _ = black_box((p1, p2, result));
    stub_fail("zkvm_bn254_g1_add")
}

/// `zkvm_bn254_g1_mul(point, scalar, result)` — redirected to `ziskasm_zkvm_bn254_g1_mul`.
/// `point` = 64 BE bytes (x‖y), `scalar` = 32 BE bytes. 0=EOK, -1=EFAIL.
/// # Safety
/// `point` is 64 readable bytes, `scalar` 32 readable bytes, `result` 64 writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_bn254_g1_mul(
    point: *const u8,
    scalar: *const u8,
    result: *mut u8,
) -> i32 {
    let _ = black_box((point, scalar, result));
    stub_fail("zkvm_bn254_g1_mul")
}

/// `zkvm_bn254_pairing(pairs, num_pairs, verified)` — redirected to
/// `ziskasm_zkvm_bn254_pairing`. `pairs` = num_pairs × 192 BE bytes (G1 64 ‖ G2 128).
/// Sets `*verified` and returns 0=EOK, -1=EFAIL (invalid input).
/// # Safety
/// `pairs` is `num_pairs*192` readable bytes; `verified` is a writable bool.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_bn254_pairing(
    pairs: *const u8,
    num_pairs: usize,
    verified: *mut bool,
) -> i32 {
    let _ = black_box((pairs, num_pairs, verified));
    stub_fail("zkvm_bn254_pairing")
}

// ---- BLS12-381 (EIP-2537) + KZG (EIP-4844) stubs -----------------------------
// All operands are packed big-endian bytes (Fp=48, G1=96, G2=192, scalar=32);
// each redirects to the matching ziskasm_zkvm_* .zisk routine. 0=EOK, -1=EFAIL.

/// # Safety
/// `p1`/`p2` are 96 readable bytes; `result` 96 writable.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_bls12_g1_add(p1: *const u8, p2: *const u8, result: *mut u8) -> i32 {
    let _ = black_box((p1, p2, result));
    stub_fail("zkvm_bls12_g1_add")
}
/// # Safety
/// `pairs` is `num_pairs*128` readable bytes; `result` 96 writable.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_bls12_g1_msm(
    pairs: *const u8,
    num_pairs: usize,
    result: *mut u8,
) -> i32 {
    let _ = black_box((pairs, num_pairs, result));
    stub_fail("zkvm_bls12_g1_msm")
}
/// # Safety
/// `p1`/`p2` are 192 readable bytes; `result` 192 writable.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_bls12_g2_add(p1: *const u8, p2: *const u8, result: *mut u8) -> i32 {
    let _ = black_box((p1, p2, result));
    stub_fail("zkvm_bls12_g2_add")
}
/// # Safety
/// `pairs` is `num_pairs*224` readable bytes; `result` 192 writable.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_bls12_g2_msm(
    pairs: *const u8,
    num_pairs: usize,
    result: *mut u8,
) -> i32 {
    let _ = black_box((pairs, num_pairs, result));
    stub_fail("zkvm_bls12_g2_msm")
}
/// # Safety
/// `pairs` is `num_pairs*288` readable bytes; `verified` a writable bool.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_bls12_pairing(
    pairs: *const u8,
    num_pairs: usize,
    verified: *mut bool,
) -> i32 {
    let _ = black_box((pairs, num_pairs, verified));
    stub_fail("zkvm_bls12_pairing")
}
/// # Safety
/// `field_element` 48 readable bytes; `result` 96 writable.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_bls12_map_fp_to_g1(field_element: *const u8, result: *mut u8) -> i32 {
    let _ = black_box((field_element, result));
    stub_fail("zkvm_bls12_map_fp_to_g1")
}
/// # Safety
/// `field_element` 96 readable bytes; `result` 192 writable.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_bls12_map_fp2_to_g2(
    field_element: *const u8,
    result: *mut u8,
) -> i32 {
    let _ = black_box((field_element, result));
    stub_fail("zkvm_bls12_map_fp2_to_g2")
}
/// `zkvm_kzg_point_eval(commitment, z, y, proof, verified)` (EIP-4844) —
/// redirected to `ziskasm_zkvm_kzg_point_eval`. commitment/proof are 48-byte
/// compressed G1; z/y are 32-byte BE field elements. Always 0=EOK; `*verified` set.
/// # Safety
/// `commitment`/`proof` 48 readable bytes; `z`/`y` 32 readable bytes; `verified` writable.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_kzg_point_eval(
    commitment: *const u8,
    z: *const u8,
    y: *const u8,
    proof: *const u8,
    verified: *mut bool,
) -> i32 {
    let _ = black_box((commitment, z, y, proof, verified));
    stub_fail("zkvm_kzg_point_eval")
}

/// `zkvm_ripemd160(data, len, output)` — redirected to `ziskasm_zkvm_ripemd160`.
/// Writes 32 bytes: [0..12]=0, [12..32]=the 20-byte digest (each word LE). 0=EOK.
/// # Safety
/// `data` is `len` readable bytes; `output` is 32 writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_ripemd160(data: *const u8, len: usize, output: *mut u8) -> i32 {
    let _ = black_box((data, len, output));
    stub_fail("zkvm_ripemd160")
}

// ---- U256 EVM-word arithmetic (zkvm_u256.h) stubs ----------------------------

/// `zkvm_u256_add(...)` — redirected to `ziskasm_zkvm_u256_add`. Big-endian
/// 32-byte operands. Returns 0 = ZKVM_EOK.
/// # Safety
/// All pointers reference 32 readable/writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_u256_add(a: *const u8, b: *const u8, result: *mut u8) -> i32 {
    let _ = black_box((a, b, result));
    stub_fail("zkvm_u256_add")
}

/// `zkvm_u256_sub(...)` — redirected to `ziskasm_zkvm_u256_sub`. Big-endian
/// 32-byte operands. Returns 0 = ZKVM_EOK.
/// # Safety
/// All pointers reference 32 readable/writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_u256_sub(a: *const u8, b: *const u8, result: *mut u8) -> i32 {
    let _ = black_box((a, b, result));
    stub_fail("zkvm_u256_sub")
}

/// `zkvm_u256_mul(...)` — redirected to `ziskasm_zkvm_u256_mul`. Big-endian
/// 32-byte operands. Returns 0 = ZKVM_EOK.
/// # Safety
/// All pointers reference 32 readable/writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_u256_mul(a: *const u8, b: *const u8, result: *mut u8) -> i32 {
    let _ = black_box((a, b, result));
    stub_fail("zkvm_u256_mul")
}

/// `zkvm_u256_div(...)` — redirected to `ziskasm_zkvm_u256_div`. Big-endian
/// 32-byte operands. Returns 0 = ZKVM_EOK.
/// # Safety
/// All pointers reference 32 readable/writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_u256_div(a: *const u8, b: *const u8, quotient: *mut u8) -> i32 {
    let _ = black_box((a, b, quotient));
    stub_fail("zkvm_u256_div")
}

/// `zkvm_u256_mod(...)` — redirected to `ziskasm_zkvm_u256_mod`. Big-endian
/// 32-byte operands. Returns 0 = ZKVM_EOK.
/// # Safety
/// All pointers reference 32 readable/writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_u256_mod(a: *const u8, b: *const u8, remainder: *mut u8) -> i32 {
    let _ = black_box((a, b, remainder));
    stub_fail("zkvm_u256_mod")
}

/// `zkvm_u256_divmod(...)` — redirected to `ziskasm_zkvm_u256_divmod`. Big-endian
/// 32-byte operands. Returns 0 = ZKVM_EOK.
/// # Safety
/// All pointers reference 32 readable/writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_u256_divmod(
    a: *const u8,
    b: *const u8,
    quotient: *mut u8,
    remainder: *mut u8,
) -> i32 {
    let _ = black_box((a, b, quotient, remainder));
    stub_fail("zkvm_u256_divmod")
}

/// `zkvm_u256_addmod(...)` — redirected to `ziskasm_zkvm_u256_addmod`. Big-endian
/// 32-byte operands. Returns 0 = ZKVM_EOK.
/// # Safety
/// All pointers reference 32 readable/writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_u256_addmod(
    a: *const u8,
    b: *const u8,
    n: *const u8,
    result: *mut u8,
) -> i32 {
    let _ = black_box((a, b, n, result));
    stub_fail("zkvm_u256_addmod")
}

/// `zkvm_u256_mulmod(...)` — redirected to `ziskasm_zkvm_u256_mulmod`. Big-endian
/// 32-byte operands. Returns 0 = ZKVM_EOK.
/// # Safety
/// All pointers reference 32 readable/writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_u256_mulmod(
    a: *const u8,
    b: *const u8,
    n: *const u8,
    result: *mut u8,
) -> i32 {
    let _ = black_box((a, b, n, result));
    stub_fail("zkvm_u256_mulmod")
}

/// `zkvm_u256_exp(...)` — redirected to `ziskasm_zkvm_u256_exp`. Big-endian
/// 32-byte operands. Returns 0 = ZKVM_EOK.
/// # Safety
/// All pointers reference 32 readable/writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_u256_exp(
    base: *const u8,
    exponent: *const u8,
    result: *mut u8,
) -> i32 {
    let _ = black_box((base, exponent, result));
    stub_fail("zkvm_u256_exp")
}

/// `zkvm_u256_sdiv(...)` — redirected to `ziskasm_zkvm_u256_sdiv`. Big-endian
/// 32-byte operands. Returns 0 = ZKVM_EOK.
/// # Safety
/// All pointers reference 32 readable/writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_u256_sdiv(a: *const u8, b: *const u8, quotient: *mut u8) -> i32 {
    let _ = black_box((a, b, quotient));
    stub_fail("zkvm_u256_sdiv")
}

/// `zkvm_u256_smod(...)` — redirected to `ziskasm_zkvm_u256_smod`. Big-endian
/// 32-byte operands. Returns 0 = ZKVM_EOK.
/// # Safety
/// All pointers reference 32 readable/writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_u256_smod(a: *const u8, b: *const u8, remainder: *mut u8) -> i32 {
    let _ = black_box((a, b, remainder));
    stub_fail("zkvm_u256_smod")
}

/// `zkvm_u256_sdivmod(...)` — redirected to `ziskasm_zkvm_u256_sdivmod`. Big-endian
/// 32-byte operands. Returns 0 = ZKVM_EOK.
/// # Safety
/// All pointers reference 32 readable/writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_u256_sdivmod(
    a: *const u8,
    b: *const u8,
    quotient: *mut u8,
    remainder: *mut u8,
) -> i32 {
    let _ = black_box((a, b, quotient, remainder));
    stub_fail("zkvm_u256_sdivmod")
}

/// `zkvm_u256_lt(...)` — redirected to `ziskasm_zkvm_u256_lt`. Big-endian
/// 32-byte operands. Returns 0 = ZKVM_EOK.
/// # Safety
/// All pointers reference 32 readable/writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_u256_lt(a: *const u8, b: *const u8, result: *mut u8) -> i32 {
    let _ = black_box((a, b, result));
    stub_fail("zkvm_u256_lt")
}

/// `zkvm_u256_gt(...)` — redirected to `ziskasm_zkvm_u256_gt`. Big-endian
/// 32-byte operands. Returns 0 = ZKVM_EOK.
/// # Safety
/// All pointers reference 32 readable/writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_u256_gt(a: *const u8, b: *const u8, result: *mut u8) -> i32 {
    let _ = black_box((a, b, result));
    stub_fail("zkvm_u256_gt")
}

/// `zkvm_u256_slt(...)` — redirected to `ziskasm_zkvm_u256_slt`. Big-endian
/// 32-byte operands. Returns 0 = ZKVM_EOK.
/// # Safety
/// All pointers reference 32 readable/writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_u256_slt(a: *const u8, b: *const u8, result: *mut u8) -> i32 {
    let _ = black_box((a, b, result));
    stub_fail("zkvm_u256_slt")
}

/// `zkvm_u256_sgt(...)` — redirected to `ziskasm_zkvm_u256_sgt`. Big-endian
/// 32-byte operands. Returns 0 = ZKVM_EOK.
/// # Safety
/// All pointers reference 32 readable/writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_u256_sgt(a: *const u8, b: *const u8, result: *mut u8) -> i32 {
    let _ = black_box((a, b, result));
    stub_fail("zkvm_u256_sgt")
}

/// `zkvm_u256_eq(...)` — redirected to `ziskasm_zkvm_u256_eq`. Big-endian
/// 32-byte operands. Returns 0 = ZKVM_EOK.
/// # Safety
/// All pointers reference 32 readable/writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_u256_eq(a: *const u8, b: *const u8, result: *mut u8) -> i32 {
    let _ = black_box((a, b, result));
    stub_fail("zkvm_u256_eq")
}

/// `zkvm_u256_iszero(...)` — redirected to `ziskasm_zkvm_u256_iszero`. Big-endian
/// 32-byte operands. Returns 0 = ZKVM_EOK.
/// # Safety
/// All pointers reference 32 readable/writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_u256_iszero(a: *const u8, result: *mut u8) -> i32 {
    let _ = black_box((a, result));
    stub_fail("zkvm_u256_iszero")
}

/// `zkvm_u256_and(...)` — redirected to `ziskasm_zkvm_u256_and`. Big-endian
/// 32-byte operands. Returns 0 = ZKVM_EOK.
/// # Safety
/// All pointers reference 32 readable/writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_u256_and(a: *const u8, b: *const u8, result: *mut u8) -> i32 {
    let _ = black_box((a, b, result));
    stub_fail("zkvm_u256_and")
}

/// `zkvm_u256_or(...)` — redirected to `ziskasm_zkvm_u256_or`. Big-endian
/// 32-byte operands. Returns 0 = ZKVM_EOK.
/// # Safety
/// All pointers reference 32 readable/writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_u256_or(a: *const u8, b: *const u8, result: *mut u8) -> i32 {
    let _ = black_box((a, b, result));
    stub_fail("zkvm_u256_or")
}

/// `zkvm_u256_xor(...)` — redirected to `ziskasm_zkvm_u256_xor`. Big-endian
/// 32-byte operands. Returns 0 = ZKVM_EOK.
/// # Safety
/// All pointers reference 32 readable/writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_u256_xor(a: *const u8, b: *const u8, result: *mut u8) -> i32 {
    let _ = black_box((a, b, result));
    stub_fail("zkvm_u256_xor")
}

/// `zkvm_u256_not(...)` — redirected to `ziskasm_zkvm_u256_not`. Big-endian
/// 32-byte operands. Returns 0 = ZKVM_EOK.
/// # Safety
/// All pointers reference 32 readable/writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_u256_not(a: *const u8, result: *mut u8) -> i32 {
    let _ = black_box((a, result));
    stub_fail("zkvm_u256_not")
}

/// `zkvm_u256_byte(...)` — redirected to `ziskasm_zkvm_u256_byte`. Big-endian
/// 32-byte operands. Returns 0 = ZKVM_EOK.
/// # Safety
/// All pointers reference 32 readable/writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_u256_byte(i: *const u8, a: *const u8, result: *mut u8) -> i32 {
    let _ = black_box((i, a, result));
    stub_fail("zkvm_u256_byte")
}

/// `zkvm_u256_shl(...)` — redirected to `ziskasm_zkvm_u256_shl`. Big-endian
/// 32-byte operands. Returns 0 = ZKVM_EOK.
/// # Safety
/// All pointers reference 32 readable/writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_u256_shl(shift: *const u8, value: *const u8, result: *mut u8) -> i32 {
    let _ = black_box((shift, value, result));
    stub_fail("zkvm_u256_shl")
}

/// `zkvm_u256_shr(...)` — redirected to `ziskasm_zkvm_u256_shr`. Big-endian
/// 32-byte operands. Returns 0 = ZKVM_EOK.
/// # Safety
/// All pointers reference 32 readable/writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_u256_shr(shift: *const u8, value: *const u8, result: *mut u8) -> i32 {
    let _ = black_box((shift, value, result));
    stub_fail("zkvm_u256_shr")
}

/// `zkvm_u256_sar(...)` — redirected to `ziskasm_zkvm_u256_sar`. Big-endian
/// 32-byte operands. Returns 0 = ZKVM_EOK.
/// # Safety
/// All pointers reference 32 readable/writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_u256_sar(shift: *const u8, value: *const u8, result: *mut u8) -> i32 {
    let _ = black_box((shift, value, result));
    stub_fail("zkvm_u256_sar")
}

/// `zkvm_u256_signextend(...)` — redirected to `ziskasm_zkvm_u256_signextend`. Big-endian
/// 32-byte operands. Returns 0 = ZKVM_EOK.
/// # Safety
/// All pointers reference 32 readable/writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn zkvm_u256_signextend(
    b: *const u8,
    value: *const u8,
    result: *mut u8,
) -> i32 {
    let _ = black_box((b, value, result));
    stub_fail("zkvm_u256_signextend")
}
