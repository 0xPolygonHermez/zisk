//! Syscall interception layer for ZisK precompiled operations.
//!
//! Each syscall wraps a hardware-accelerated operation that the ZisK VM executes as a
//! single precompiled instruction. On zkVM targets, the syscall emits a CSR instruction;
//! on native targets, it falls back to a software implementation.
//!
//! ## Available syscalls
//!
//! ### Arithmetic
//! - [`syscall_add256`] — 256-bit addition with carry: `a + b + cin = cout | c`
//! - [`syscall_arith256`] — 256-bit multiply-add: `a * b + c = dh | dl`
//! - [`syscall_arith256_mod`] — 256-bit modular multiply-add: `d = (a * b + c) mod m`
//! - [`syscall_arith384_mod`] — 384-bit modular multiply-add: `d = (a * b + c) mod m`
//!
//! ### Hashing
//! - [`syscall_blake2br`] — BLAKE2b round function
//! - [`syscall_keccakf`] — Keccak-f\[1600\] permutation
//! - [`syscall_poseidon2`] — Poseidon2 hash function
//! - [`syscall_sha256f`] — SHA-256 compression function
//!
//! ### Elliptic curve (secp256k1)
//! - [`syscall_secp256k1_add`] — Point addition on secp256k1
//! - [`syscall_secp256k1_dbl`] — Point doubling on secp256k1
//!
//! ### Elliptic curve (secp256r1)
//! - [`syscall_secp256r1_add`] — Point addition on secp256r1
//! - [`syscall_secp256r1_dbl`] — Point doubling on secp256r1
//!
//! ### Elliptic curve (BN254)
//! - [`syscall_bn254_curve_add`] / [`syscall_bn254_curve_dbl`] — Curve operations
//! - [`syscall_bn254_complex_add`] / [`syscall_bn254_complex_mul`] / [`syscall_bn254_complex_sub`] — Fp2 arithmetic
//!
//! ### Elliptic curve (BLS12-381)
//! - [`syscall_bls12_381_curve_add`] / [`syscall_bls12_381_curve_dbl`] — Curve operations
//! - [`syscall_bls12_381_complex_add`] / [`syscall_bls12_381_complex_mul`] / [`syscall_bls12_381_complex_sub`] — Fp2 arithmetic

mod add256;
mod arith256;
mod arith256_mod;
mod arith384_mod;
mod blake2br;
mod bls12_381_complex_add;
mod bls12_381_complex_mul;
mod bls12_381_complex_sub;
mod bls12_381_curve_add;
mod bls12_381_curve_dbl;
mod bn254_complex_add;
mod bn254_complex_mul;
mod bn254_complex_sub;
mod bn254_curve_add;
mod bn254_curve_dbl;
mod complex;
mod keccakf;
mod point;
mod poseidon2;
mod secp256k1_add;
mod secp256k1_dbl;
mod secp256r1_add;
mod secp256r1_dbl;
mod sha256f;

pub use add256::*;
pub use arith256::*;
pub use arith256_mod::*;
pub use arith384_mod::*;
pub use blake2br::*;
pub use bls12_381_complex_add::*;
pub use bls12_381_complex_mul::*;
pub use bls12_381_complex_sub::*;
pub use bls12_381_curve_add::*;
pub use bls12_381_curve_dbl::*;
pub use bn254_complex_add::*;
pub use bn254_complex_mul::*;
pub use bn254_complex_sub::*;
pub use bn254_curve_add::*;
pub use bn254_curve_dbl::*;
pub use complex::*;
pub use keccakf::*;
pub use point::*;
pub use poseidon2::*;
pub use secp256k1_add::*;
pub use secp256k1_dbl::*;
pub use secp256r1_add::*;
pub use secp256r1_dbl::*;
pub use sha256f::*;

#[macro_export]
macro_rules! ziskos_syscall {
    ($csr_addr:expr, $addr:expr) => {{
        unsafe {
            core::arch::asm!(
                concat!("csrs {port}, {value}"),
                port = const $csr_addr,
                value = in(reg) $addr
            );
        }
    }};
    ($csr_addr:expr, $cmd: expr, $arg0:expr) => {{
        unsafe {
            core::arch::asm!(
                concat!("csrs {port}, {p0}"),
                "addi x0, x0, {imm}",
                port = const $csr_addr,
                p0 = in(reg) $arg0,  // {0}
                imm = const $cmd,
                options(nostack)
            );
        }
    }};
    ($csr_addr:expr, $cmd: expr, $arg0:expr, $arg1:expr) => {{
        unsafe {
            core::arch::asm!(
                concat!("csrs {port}, {p0}"),
                "addi x0, {p1}, {imm}",
                port = const $csr_addr,
                p0 = in(reg) $arg0,  // {0}
                p1 = in(reg) $arg1,  // {1}
                imm = const $cmd,
                options(nostack)
            );
        }
    }};
    ($csr_addr:expr, $arg0:expr, $arg1:expr, $arg2: expr) => {{
        unsafe {
            core::arch::asm!(
                concat!("csrs {port}, {p0}"),
                "add x0, {p1}, {p2}",
                port = const $csr_addr,
                p0 = in(reg) $arg0,  // {0}
                p1 = in(reg) $arg1,  // {1}
                p2 = in(reg) $arg2,  // {2}
                options(nostack)
            );
        }
    }};
}

#[macro_export]
macro_rules! ziskos_syscall_ret_u64 {
    ($csr_addr:expr, $addr:expr) => {{
        let v: u64;
        unsafe {
            core::arch::asm!(
                concat!("csrrs {rd}, {port}, {rs1}"),
                port = const $csr_addr,
                rd = out(reg) v,
                rs1 = in(reg) $addr,
                options(nostack)
            );
        }
        v
    }};
    ($csr_addr:expr, $arg0:expr, $arg1:expr, $arg2: expr) => {{
        let v: u64;
        unsafe {
            core::arch::asm!(
                concat!("csrrs {rd}, {port}, {p0}"),
                "add x0, {p1}, {p2}",
                port = const $csr_addr,
                p0 = in(reg) $arg0,  // {0}
                p1 = in(reg) $arg1,  // {1}
                p2 = in(reg) $arg2,  // {2}
                rd = out(reg) v,
                options(nostack)
            );
        }
        v
    }};
}
