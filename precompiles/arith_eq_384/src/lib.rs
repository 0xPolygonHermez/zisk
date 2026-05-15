mod arith_eq_384;
mod arith_eq_384_bus_device;
mod arith_eq_384_constants;
mod arith_eq_384_input;
mod arith_eq_384_instance;
mod arith_eq_384_manager;
mod arith_eq_384_planner;
mod equations;
mod executors;
mod mem_inputs;
pub mod test_data;

pub use arith_eq_384::*;
pub use arith_eq_384_bus_device::*;
pub use arith_eq_384_constants::*;
pub use arith_eq_384_input::*;
pub use arith_eq_384_instance::*;
pub use arith_eq_384_manager::*;
pub use arith_eq_384_planner::*;

#[cfg(test)]
mod arith_eq_384_tests {
    use test_artifacts::{
        ELF_ARITH384_MOD, ELF_BLS12_381_ADD, ELF_BLS12_381_COMPLEX_ADD, ELF_BLS12_381_COMPLEX_MUL,
        ELF_BLS12_381_COMPLEX_SUB, ELF_BLS12_381_DBL,
    };
    use zisk_common::io::ZiskStdin;

    #[test]
    fn execute_arith384_mod_tests() {
        ELF_ARITH384_MOD
            .run_emulation(ZiskStdin::new(), None)
            .expect("arith384_mod guest emulation failed");
    }

    #[test]
    fn execute_bls12_381_add_tests() {
        ELF_BLS12_381_ADD
            .run_emulation(ZiskStdin::new(), None)
            .expect("bls12_381_add guest emulation failed");
    }

    #[test]
    fn execute_bls12_381_dbl_tests() {
        ELF_BLS12_381_DBL
            .run_emulation(ZiskStdin::new(), None)
            .expect("bls12_381_dbl guest emulation failed");
    }

    #[test]
    fn execute_bls12_381_complex_add_tests() {
        ELF_BLS12_381_COMPLEX_ADD
            .run_emulation(ZiskStdin::new(), None)
            .expect("bls12_381_complex_add guest emulation failed");
    }

    #[test]
    fn execute_bls12_381_complex_mul_tests() {
        ELF_BLS12_381_COMPLEX_MUL
            .run_emulation(ZiskStdin::new(), None)
            .expect("bls12_381_complex_mul guest emulation failed");
    }

    #[test]
    fn execute_bls12_381_complex_sub_tests() {
        ELF_BLS12_381_COMPLEX_SUB
            .run_emulation(ZiskStdin::new(), None)
            .expect("bls12_381_complex_sub guest emulation failed");
    }
}
