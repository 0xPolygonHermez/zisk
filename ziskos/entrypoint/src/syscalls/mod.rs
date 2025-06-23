pub mod arith256;
pub mod arith256_mod;
pub mod bn254_complex_add;
pub mod bn254_complex_mul;
pub mod bn254_complex_sub;
pub mod bn254_curve_add;
pub mod bn254_curve_dbl;
pub mod complex256;
pub mod keccakf;
pub mod point256;
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
