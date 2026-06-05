mod arith_eq_384;
mod arith_eq_384_constants;
mod arith_eq_384_input;
mod arith_eq_384_mem_inputs;
mod equations;
mod executors;
mod mem_inputs;
pub mod test_data;

pub use arith_eq_384::*;
pub use arith_eq_384_constants::*;
pub use arith_eq_384_input::*;

use zisk_common::zisk_precompile;

zisk_precompile! {
    name = ArithEq384,
    op_type = ArithEq384,
    trace = ArithEq384Trace,
    num_available = {
        ::zisk_pil::ArithEq384Trace::<()>::NUM_ROWS / ARITH_EQ_384_ROWS_BY_OP - 1
    },
    ops = [
        (OperationArith384ModData         => Arith384Mod,         Arith384ModInput),
        (OperationBls12_381CurveAddData   => Bls12_381CurveAdd,   Bls12_381CurveAddInput),
        (OperationBls12_381CurveDblData   => Bls12_381CurveDbl,   Bls12_381CurveDblInput),
        (OperationBls12_381ComplexAddData => Bls12_381ComplexAdd, Bls12_381ComplexAddInput),
        (OperationBls12_381ComplexSubData => Bls12_381ComplexSub, Bls12_381ComplexSubInput),
        (OperationBls12_381ComplexMulData => Bls12_381ComplexMul, Bls12_381ComplexMulInput),
    ],
}

#[cfg(test)]
mod arith_eq_384_tests {
    use serial_test::serial;
    use test_artifacts::{
        ELF_ARITH384_MOD, ELF_BLS12_381_ADD, ELF_BLS12_381_COMPLEX_ADD, ELF_BLS12_381_COMPLEX_MUL,
        ELF_BLS12_381_COMPLEX_SUB, ELF_BLS12_381_DBL,
    };
    use zisk_common::io::ZiskStdin;

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
