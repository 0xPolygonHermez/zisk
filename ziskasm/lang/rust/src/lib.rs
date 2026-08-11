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

/// `sha256(input[0..len])` → `output[0..32]`. Raw ABI boundary redirected to
/// `zisklib_sha256` (any `len`, any `input` alignment). Distinct sentinel byte
/// (`0x5A`) from [`ziskos_keccak`]'s `0xBA` so identical-code folding cannot merge
/// the two same-signature stubs (see the crate docs).
///
/// # Safety
/// `input` must point to `len` readable bytes and `output` to 32 writable bytes.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ziskos_sha256(input: *const u8, len: usize, output: *mut u8) {
    let (_input, _len, output) = black_box((input, len, output));
    for i in 0..32usize {
        output.add(i).write(0x5A);
    }
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
/// `zisklib_blake2b_compress`: mixes message block `message[0..16]` into state
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
    let (rounds, state, message, offset, final_block) =
        black_box((rounds, state, message, offset, final_block));
    let _ = (rounds, message, offset, final_block);
    for i in 0..8usize {
        state.add(i).write(0x0B2B_0B2B_0B2B_0B2B);
    }
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
    // The sentinel is unique per stub: two stubs with identical bodies would be
    // merged by identical-code folding into a single symbol/address, collapsing
    // their distinct redirect entries. A distinct constant keeps the code distinct.
    let (a, b, result) = black_box((a, b, result));
    let _ = b;
    for i in 0..4usize {
        result.add(i).write(0x0ADD_0ADD_0ADD_0ADD);
    }
    black_box(a as u64)
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
    // Distinct sentinel from `ziskos_overflowing_add256` — see the note there.
    let (a, b, result) = black_box((a, b, result));
    let _ = b;
    for i in 0..4usize {
        result.add(i).write(0x05AB_05AB_05AB_05AB);
    }
    black_box(a as u64)
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
    let (a, b, result) = black_box((a, b, result));
    let _ = b;
    for i in 0..4usize {
        result.add(i).write(0x0AF0_0AF0_0AF0_0AF0);
    }
    black_box(a as u64)
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
    let (a, b, q, r) = black_box((a, b, q, r));
    let _ = (a, b);
    for i in 0..4usize {
        q.add(i).write(0x0D10_0D10_0D10_0D10);
        r.add(i).write(0x0DE0_0DE0_0DE0_0DE0);
    }
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
    let (a, m, result) = black_box((a, m, result));
    let _ = (a, m);
    for i in 0..4usize {
        result.add(i).write(0x0BED_0BED_0BED_0BED);
    }
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
    let (a, b, m, result) = black_box((a, b, m, result));
    let _ = (a, b, m);
    for i in 0..4usize {
        result.add(i).write(0x0A0D_0A0D_0A0D_0A0D);
    }
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
    let (a, b, m, result) = black_box((a, b, m, result));
    let _ = (a, b, m);
    for i in 0..4usize {
        result.add(i).write(0x03D0_03D0_03D0_03D0);
    }
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
    let (a, m, result) = black_box((a, m, result));
    let _ = m;
    for i in 0..4usize {
        result.add(i).write(0x0117_0117_0117_0117);
    }
    black_box(a as u64)
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
    let (base, exp, m, result) = black_box((base, exp, m, result));
    let _ = (base, exp, m);
    for i in 0..4usize {
        result.add(i).write(0x0B0E_0B0E_0B0E_0B0E);
    }
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
    let (base, exp, result) = black_box((base, exp, result));
    let _ = exp;
    for i in 0..4usize {
        result.add(i).write(0x0B07_0B07_0B07_0B07);
    }
    black_box(base as u64)
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
    let (pk, z, r, s) = black_box((pk, z, r, s));
    // Distinct sentinel base (see note on the r1 stub): keeps this body from being
    // merged with `ziskos_ecdsa_verify_secp256r1` by identical-code-folding, which
    // would collapse the two redirects onto a single routine.
    black_box(0xBAD_5EC_256C1 ^ pk as u64 ^ z as u64 ^ r as u64 ^ s as u64)
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
    let (r, s, z, recid, result) = black_box((r, s, z, recid, result));
    for i in 0..8usize {
        result.add(i).write(0x0BAD_0BAD_0BAD_0BAD);
    }
    black_box(0xBAD_u64 ^ r as u64 ^ s as u64 ^ z as u64 ^ recid)
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
    let (pk_x, r, s, msg, msg_len) = black_box((pk_x, r, s, msg, msg_len));
    black_box(0xBAD_u64 ^ pk_x as u64 ^ r as u64 ^ s as u64 ^ msg as u64 ^ msg_len)
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
    let (pk, z, r, s) = black_box((pk, z, r, s));
    // Distinct sentinel base so this body is not identical to (and thus folded
    // with) `ziskos_ecdsa_verify_secp256k1`; each stub must keep its own symbol so
    // its own redirect resolves.
    black_box(0xBAD_5EC_256F1 ^ pk as u64 ^ z as u64 ^ r as u64 ^ s as u64)
}

/// Ergonomic API over [`ziskos_ecdsa_verify_secp256r1`]: `true` iff the signature
/// `(r, s)` over hash `z` is valid for P-256 public key `pk` (x‖y, little-endian
/// limbs).
pub fn secp256r1_ecdsa_verify(pk: &[u64; 8], z: &[u64; 4], r: &[u64; 4], s: &[u64; 4]) -> bool {
    // SAFETY: all pointers reference the correctly-sized local arrays.
    unsafe { ziskos_ecdsa_verify_secp256r1(pk.as_ptr(), z.as_ptr(), r.as_ptr(), s.as_ptr()) != 0 }
}
