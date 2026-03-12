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
            asm!(
                concat!("csrs {port}, {value}"),
                port = const $csr_addr,
                value = in(reg) $addr
            );
        }
    }};
    ($csr_addr:expr, $arg0:expr, $arg1:expr, $arg2: expr) => {{
        unsafe {
            asm!(
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
            asm!(
                concat!("csrrs {rd}, {port}, {rs1}"),
                port = const $csr_addr,
                rd = out(reg) v,
                rs1 = in(reg) $addr,
                options(nostack)
            );
        }
        v
    }};
}
