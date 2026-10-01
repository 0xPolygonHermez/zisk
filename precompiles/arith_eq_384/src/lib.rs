mod arith_eq_384;
mod arith_eq_384_constants;
mod arith_eq_384_family;
mod arith_eq_384_input;
mod arith_eq_384_mem_inputs;
mod arith_eq_384_row;
mod equations;
mod executors;
mod mem_inputs;
pub mod test_data;

pub use arith_eq_384::*;
pub use arith_eq_384_constants::*;
pub use arith_eq_384_family::*;
pub use arith_eq_384_input::*;
pub use arith_eq_384_row::*;

#[cfg(test)]
mod arith_eq_384_big_endian_tests;

#[cfg(test)]
mod arith_eq_384_tests {
    use serial_test::serial;
    use zisk_common::io::ZiskStdin;
    use zisk_test_artifacts::{
        ELF_ARITH384_MOD, ELF_BLS12_381_ADD, ELF_BLS12_381_COMPLEX_ADD, ELF_BLS12_381_COMPLEX_MUL,
        ELF_BLS12_381_COMPLEX_SUB, ELF_BLS12_381_DBL,
    };

    // Tests share a global lock (#[serial]) because each `run_emulation`
    // allocates several GB; running them in parallel exceeds RAM.

    #[test]
    #[serial]
    fn arith384_mod_tests() {
        ELF_ARITH384_MOD
            .run_emulation(ZiskStdin::new(), None)
            .expect("arith384_mod guest emulation failed");
    }

    #[test]
    #[serial]
    fn bls12_381_add_tests() {
        ELF_BLS12_381_ADD
            .run_emulation(ZiskStdin::new(), None)
            .expect("bls12_381_add guest emulation failed");
    }

    #[test]
    #[serial]
    fn bls12_381_dbl_tests() {
        ELF_BLS12_381_DBL
            .run_emulation(ZiskStdin::new(), None)
            .expect("bls12_381_dbl guest emulation failed");
    }

    #[test]
    #[serial]
    fn bls12_381_complex_add_tests() {
        ELF_BLS12_381_COMPLEX_ADD
            .run_emulation(ZiskStdin::new(), None)
            .expect("bls12_381_complex_add guest emulation failed");
    }

    #[test]
    #[serial]
    fn bls12_381_complex_mul_tests() {
        ELF_BLS12_381_COMPLEX_MUL
            .run_emulation(ZiskStdin::new(), None)
            .expect("bls12_381_complex_mul guest emulation failed");
    }

    #[test]
    #[serial]
    fn bls12_381_complex_sub_tests() {
        ELF_BLS12_381_COMPLEX_SUB
            .run_emulation(ZiskStdin::new(), None)
            .expect("bls12_381_complex_sub guest emulation failed");
    }
}
