//! Zisk standard library for guest programs.
//!
//! Provides three layers:
//! - [`lib`] — High-level arithmetic, hashing, and elliptic curve operations backed by syscalls.
//! - [`fcalls`] — Free-input call wrappers (hints) for operations that are not zk-friendly.
//! - [`fcalls_impl`] — Software implementations of fcalls, used on native targets and for trace
//!   generation.

mod fcalls;
#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
mod fcalls_impl;
pub mod lib;

pub use fcalls::*;
#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
pub use fcalls_impl::*;
pub use lib::*;

#[cfg(test)]
mod tests {
    use test_artifacts::{
        ELF_BLS12_381, ELF_BN254, ELF_MODEXP, ELF_SECP256K1, ELF_SECP256R1, ELF_UINT256,
    };
    use zisk_common::io::ZiskStdin;

    #[test]
    fn execute_bls12_381() {
        ELF_BLS12_381
            .run_emulation(ZiskStdin::new(), None)
            .expect("bls12_381 zisklib guest emulation failed");
    }

    #[test]
    fn execute_bn254() {
        ELF_BN254
            .run_emulation(ZiskStdin::new(), None)
            .expect("bn254 zisklib guest emulation failed");
    }

    #[test]
    fn execute_modexp() {
        ELF_MODEXP
            .run_emulation(ZiskStdin::new(), None)
            .expect("modexp zisklib guest emulation failed");
    }

    #[test]
    fn execute_secp256k1() {
        ELF_SECP256K1
            .run_emulation(ZiskStdin::new(), None)
            .expect("secp256k1 zisklib guest emulation failed");
    }

    #[test]
    fn execute_secp256r1() {
        ELF_SECP256R1
            .run_emulation(ZiskStdin::new(), None)
            .expect("secp256r1 zisklib guest emulation failed");
    }

    #[test]
    fn execute_uint256() {
        ELF_UINT256
            .run_emulation(ZiskStdin::new(), None)
            .expect("uint256 zisklib guest emulation failed");
    }
}
