//! ruint_bench -- step cost of the 27 EF U256 operations implemented with the
//! alloy-rs `ruint` crate in plain RISC-V code (no ZisK precompiles), for comparison
//! with u256_bench_guest.c. Same operands, same loop shape: the operands are read
//! from globals with volatile loads and the result stored with volatile stores on
//! every iteration, so nothing is hoisted out of the loop.
//!
//! The EVM semantics that ruint doesn't provide directly (zero divisors, signed
//! division, SLT/SGT, BYTE, SAR, SIGNEXTEND) are written the way revm writes them.
//!
//! Input: [u64 length = 8][u32 op][u32 iterations]; op 255 is the empty loop. Output:
//! the result's 32 big-endian bytes, laid out like emit32() in the C benchmark, so
//! the outputs can be compared byte for byte.

#![no_main]
ziskos::entrypoint!(main);

use core::ptr::{read_volatile, write_volatile};
use ruint::aliases::U256;

const A: [u8; 32] = [
    0x9e, 0x37, 0x79, 0xb9, 0x7f, 0x4a, 0x7c, 0x15, 0xf3, 0x9c, 0xc0, 0x60, 0x5c, 0xed, 0xc8, 0x34,
    0x10, 0x82, 0x27, 0x6b, 0xf3, 0xa2, 0x72, 0x51, 0xf8, 0x6c, 0x6a, 0x11, 0xd0, 0xc1, 0x8e, 0x95,
];
const B: [u8; 32] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xd1, 0xb5, 0x4a, 0x32, 0xd1, 0x92, 0xed, 0x03,
    0x94, 0xd8, 0x8c, 0xa5, 0xe1, 0x97, 0x60, 0x2f,
];
const M: [u8; 32] = [
    0x73, 0xed, 0xa7, 0x53, 0x29, 0x9d, 0x7d, 0x48, 0x33, 0x39, 0xd8, 0x08, 0x09, 0xa1, 0xd8, 0x05,
    0x53, 0xbd, 0xa4, 0x02, 0xff, 0xfe, 0x5b, 0xfe, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x01,
];

static mut OA: U256 = U256::ZERO;
static mut OB: U256 = U256::ZERO;
static mut OM: U256 = U256::ZERO;
static mut OS: U256 = U256::ZERO;
static mut OR: U256 = U256::ZERO;
static mut OQ: U256 = U256::ZERO;

// ---- EVM semantics on ruint (as in revm) ------------------------------------

fn is_neg(x: U256) -> bool {
    x.bit(255)
}
fn abs(x: U256) -> U256 {
    if is_neg(x) { x.wrapping_neg() } else { x }
}
fn div(a: U256, b: U256) -> U256 {
    a.checked_div(b).unwrap_or_default()
}
fn rem(a: U256, b: U256) -> U256 {
    a.checked_rem(b).unwrap_or_default()
}
fn divmod(a: U256, b: U256) -> (U256, U256) {
    if b.is_zero() { (U256::ZERO, U256::ZERO) } else { a.div_rem(b) }
}
fn sdivmod(a: U256, b: U256) -> (U256, U256) {
    if b.is_zero() {
        return (U256::ZERO, U256::ZERO);
    }
    let (q, r) = abs(a).div_rem(abs(b));
    let q = if is_neg(a) != is_neg(b) { q.wrapping_neg() } else { q };
    let r = if is_neg(a) { r.wrapping_neg() } else { r };
    (q, r)
}
fn slt(a: U256, b: U256) -> bool {
    if is_neg(a) != is_neg(b) { is_neg(a) } else { a < b }
}
fn small(x: U256, limit: usize) -> usize {
    if x < U256::from(limit) { x.to::<usize>() } else { limit }
}
fn byte(i: U256, a: U256) -> U256 {
    let i = small(i, 32);
    if i < 32 { U256::from(a.byte(31 - i)) } else { U256::ZERO }
}
fn shl(s: U256, a: U256) -> U256 {
    let s = small(s, 256);
    if s < 256 { a << s } else { U256::ZERO }
}
fn shr(s: U256, a: U256) -> U256 {
    let s = small(s, 256);
    if s < 256 { a >> s } else { U256::ZERO }
}
fn sar(s: U256, a: U256) -> U256 {
    let s = small(s, 256);
    if !is_neg(a) {
        if s < 256 { a >> s } else { U256::ZERO }
    } else if s < 256 {
        !((!a) >> s)
    } else {
        U256::MAX
    }
}
fn signextend(b: U256, x: U256) -> U256 {
    let b = small(b, 31);
    if b < 31 {
        let bit = b * 8 + 7;
        let mask = (U256::from(1) << bit) - U256::from(1);
        if x.bit(bit) { x | !mask } else { x & mask }
    } else {
        x
    }
}
fn bool256(v: bool) -> U256 {
    U256::from(v as u64)
}

// ---- benchmark -----------------------------------------------------------------

macro_rules! bench {
    // Loads only the operands the operation uses, like the C benchmark.
    ($n:expr, |$($v:ident = $g:ident),*| $e:expr) => {
        for _ in 0..$n {
            $(let $v = unsafe { read_volatile(&raw const $g) };)*
            let r: U256 = $e;
            unsafe { write_volatile(&raw mut OR, r) };
        }
    };
}
macro_rules! bench2 {
    ($n:expr, |$a:ident, $b:ident| $e:expr) => {
        for _ in 0..$n {
            let ($a, $b) = unsafe { (read_volatile(&raw const OA), read_volatile(&raw const OB)) };
            let (q, r): (U256, U256) = $e;
            unsafe {
                write_volatile(&raw mut OQ, q);
                write_volatile(&raw mut OR, r);
            }
        }
    };
}

fn main() {
    let input = ziskos::io::read_slice();
    let op = u32::from_le_bytes(input[0..4].try_into().unwrap());
    let n = u32::from_le_bytes(input[4..8].try_into().unwrap());
    unsafe {
        OA = U256::from_be_bytes(A);
        OB = U256::from_be_bytes(B);
        OM = U256::from_be_bytes(M);
        let mut s = [0u8; 32];
        s[31] = 13;
        OS = U256::from_be_bytes(s);
    }
    // Ops in zkvm_u256.h order, with the operand shapes of u256_bench_guest.c.
    match op {
        0 => bench!(n, |a = OA, b = OB| a.wrapping_add(b)),
        1 => bench!(n, |a = OA, b = OB| a.wrapping_sub(b)),
        2 => bench!(n, |a = OA, b = OB| a.wrapping_mul(b)),
        3 => bench!(n, |a = OA, b = OB| div(a, b)),
        4 => bench!(n, |a = OA, b = OB| rem(a, b)),
        5 => bench2!(n, |a, b| divmod(a, b)),
        6 => bench!(n, |a = OA, b = OB, m = OM| a.add_mod(b, m)),
        7 => bench!(n, |a = OA, b = OB, m = OM| a.mul_mod(b, m)),
        8 => bench!(n, |a = OA, b = OB| a.pow(b)),
        9 => bench!(n, |a = OA, b = OB| sdivmod(a, b).0),
        10 => bench!(n, |a = OA, b = OB| sdivmod(a, b).1),
        11 => bench2!(n, |a, b| sdivmod(a, b)),
        12 => bench!(n, |a = OA, b = OB| bool256(a < b)),
        13 => bench!(n, |a = OA, b = OB| bool256(a > b)),
        14 => bench!(n, |a = OA, b = OB| bool256(slt(a, b))),
        15 => bench!(n, |a = OA, b = OB| bool256(slt(b, a))),
        16 => bench!(n, |a = OA, b = OB| bool256(a == b)),
        17 => bench!(n, |a = OA| bool256(a.is_zero())),
        18 => bench!(n, |a = OA, b = OB| a & b),
        19 => bench!(n, |a = OA, b = OB| a | b),
        20 => bench!(n, |a = OA, b = OB| a ^ b),
        21 => bench!(n, |a = OA| !a),
        22 => bench!(n, |a = OA, s = OS| byte(s, a)),
        23 => bench!(n, |a = OA, s = OS| shl(s, a)),
        24 => bench!(n, |a = OA, s = OS| shr(s, a)),
        25 => bench!(n, |a = OA, s = OS| sar(s, a)),
        26 => bench!(n, |a = OA, s = OS| signextend(s, a)),
        _ => {
            for i in 0..n {
                core::hint::black_box(i);
            }
        }
    }
    // Emit the result like emit32(): word i = bytes 4i..4i+3, little-endian.
    let r = unsafe { read_volatile(&raw const OR) }.to_be_bytes::<32>();
    let out = 0xA041_0000usize as *mut u32;
    for i in 0..8 {
        let w = u32::from_le_bytes(r[4 * i..4 * i + 4].try_into().unwrap());
        unsafe { write_volatile(out.add(i), w) };
    }
}
