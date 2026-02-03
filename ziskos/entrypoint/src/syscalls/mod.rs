mod add256;
mod arith256;
mod arith256_mod;
mod arith384_mod;
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
mod syscall;

pub use add256::*;
pub use arith256::*;
pub use arith256_mod::*;
pub use arith384_mod::*;
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
pub use syscall::*;

#[macro_export]
macro_rules! ziskos_syscall {
    ($csr_addr:literal, $addr:expr) => {{
        unsafe {
            asm!(
                concat!("csrs ", stringify!($csr_addr), ", {value}"),
                value = in(reg) $addr
            );
        }
    }};
    ($csr_addr:literal, $arg0:expr, $arg1:expr, $arg2: expr) => {{
        unsafe {
            asm!(
                concat!("csrs ", stringify!($csr_addr), ", {0}"),
                "add x0, {1}, {2}",
                in(reg) $arg0,  // {0}
                in(reg) $arg1,  // {1}
                in(reg) $arg2,  // {2}
                options(nostack)
            );
        }
    }};
}

#[macro_export]
macro_rules! ziskos_syscall_ret_u64 {
    ($csr_addr:literal, $addr:expr) => {{
        let v: u64;
        unsafe {
            asm!(
                concat!("csrrs {0}, ", stringify!($csr_addr), ", {1}"),
                out(reg) v,
                in(reg) $addr,
                options(nostack)
            );
        }
        v
    }};
}
