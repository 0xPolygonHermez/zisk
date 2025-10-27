pub mod add256;
pub mod arith256;
pub mod arith256_mod;
pub mod arith384_mod;
pub mod bls12_381_complex_add;
pub mod bls12_381_complex_mul;
pub mod bls12_381_complex_sub;
pub mod bls12_381_curve_add;
pub mod bls12_381_curve_dbl;
pub mod bn254_complex_add;
pub mod bn254_complex_mul;
pub mod bn254_complex_sub;
pub mod bn254_curve_add;
pub mod bn254_curve_dbl;
pub mod complex;
pub mod keccakf;
pub mod point;
pub mod secp256k1_add;
pub mod secp256k1_dbl;
pub mod sha256f;
mod syscall;

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
                options(nostack, nomem)
            );
        }
        v
    }};
}
