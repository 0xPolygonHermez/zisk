mod arith_eq;
mod arith_eq_bus_device;
mod arith_eq_constants;
mod arith_eq_input;
mod arith_eq_instance;
mod arith_eq_lt_table;
mod arith_eq_manager;
mod arith_eq_planner;
mod equations;
mod executors;
pub mod generator;
mod mem_inputs;
pub mod test_data;

pub use arith_eq::*;
pub use arith_eq_bus_device::*;
pub use arith_eq_constants::*;
pub use arith_eq_input::*;
pub use arith_eq_instance::*;
pub use arith_eq_lt_table::*;
pub use arith_eq_manager::*;
pub use arith_eq_planner::*;

#[cfg(test)]
mod arith_eq_tests {
    use test_artifacts::{
        ELF_ARITH256, ELF_ARITH256_MOD, ELF_BN254_ADD, ELF_BN254_COMPLEX_ADD,
        ELF_BN254_COMPLEX_MUL, ELF_BN254_COMPLEX_SUB, ELF_BN254_DBL, ELF_SECP256K1_ADD,
        ELF_SECP256K1_DBL, ELF_SECP256R1_ADD, ELF_SECP256R1_DBL,
    };
    use zisk_common::io::ZiskStdin;

    #[test]
    fn execute_arith256_tests() {
        let stdin = ZiskStdin::new();

        ELF_ARITH256.run_emulation(stdin, None).expect("arith256 guest emulation failed");
    }

    #[test]
    fn execute_arith256_mod_tests() {
        let stdin = ZiskStdin::new();

        ELF_ARITH256_MOD.run_emulation(stdin, None).expect("arith256_mod guest emulation failed");
    }

    #[test]
    fn execute_secp256k1_add_tests() {
        let stdin = ZiskStdin::new();

        ELF_SECP256K1_ADD.run_emulation(stdin, None).expect("secp256k1_add guest emulation failed");
    }

    #[test]
    fn execute_secp256k1_dbl_tests() {
        let stdin = ZiskStdin::new();

        ELF_SECP256K1_DBL.run_emulation(stdin, None).expect("secp256k1_dbl guest emulation failed");
    }

    #[test]
    fn execute_secp256r1_add_tests() {
        let stdin = ZiskStdin::new();

        ELF_SECP256R1_ADD.run_emulation(stdin, None).expect("secp256r1_add guest emulation failed");
    }

    #[test]
    fn execute_secp256r1_dbl_tests() {
        let stdin = ZiskStdin::new();

        ELF_SECP256R1_DBL.run_emulation(stdin, None).expect("secp256r1_dbl guest emulation failed");
    }

    #[test]
    fn execute_bn254_add_tests() {
        let stdin = ZiskStdin::new();

        ELF_BN254_ADD.run_emulation(stdin, None).expect("bn254_add guest emulation failed");
    }

    #[test]
    fn execute_bn254_dbl_tests() {
        let stdin = ZiskStdin::new();

        ELF_BN254_DBL.run_emulation(stdin, None).expect("bn254_dbl guest emulation failed");
    }

    #[test]
    fn execute_bn254_complex_add_tests() {
        let stdin = ZiskStdin::new();

        ELF_BN254_COMPLEX_ADD
            .run_emulation(stdin, None)
            .expect("bn254_complex_add guest emulation failed");
    }

    #[test]
    fn execute_bn254_complex_mul_tests() {
        let stdin = ZiskStdin::new();

        ELF_BN254_COMPLEX_MUL
            .run_emulation(stdin, None)
            .expect("bn254_complex_mul guest emulation failed");
    }

    #[test]
    fn execute_bn254_complex_sub_tests() {
        let stdin = ZiskStdin::new();

        ELF_BN254_COMPLEX_SUB
            .run_emulation(stdin, None)
            .expect("bn254_complex_sub guest emulation failed");
    }
}
