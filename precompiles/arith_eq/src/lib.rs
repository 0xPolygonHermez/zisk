mod arith_eq;
mod arith_eq_constants;
mod arith_eq_input;
mod arith_eq_lt_table;
mod arith_eq_mem_inputs;
mod equations;
mod executors;
pub mod generator;
mod mem_inputs;
pub mod test_data;

pub use arith_eq::*;
pub use arith_eq_constants::*;
pub use arith_eq_input::*;
pub use arith_eq_lt_table::*;

use zisk_common::zisk_precompile;

zisk_precompile! {
    name = ArithEq,
    op_type = ArithEq,
    trace = ArithEqTrace,
    num_available_field = num_available_ops,
    ops = [
        (OperationArith256Data        => Arith256,        Arith256Input),
        (OperationArith256ModData     => Arith256Mod,     Arith256ModInput),
        (OperationSecp256k1AddData    => Secp256k1Add,    Secp256k1AddInput),
        (OperationSecp256k1DblData    => Secp256k1Dbl,    Secp256k1DblInput),
        (OperationBn254CurveAddData   => Bn254CurveAdd,   Bn254CurveAddInput),
        (OperationBn254CurveDblData   => Bn254CurveDbl,   Bn254CurveDblInput),
        (OperationBn254ComplexAddData => Bn254ComplexAdd, Bn254ComplexAddInput),
        (OperationBn254ComplexSubData => Bn254ComplexSub, Bn254ComplexSubInput),
        (OperationBn254ComplexMulData => Bn254ComplexMul, Bn254ComplexMulInput),
        (OperationSecp256r1AddData    => Secp256r1Add,    Secp256r1AddInput),
        (OperationSecp256r1DblData    => Secp256r1Dbl,    Secp256r1DblInput),
    ],
}

#[cfg(test)]
mod arith_eq_tests {
    use serial_test::serial;
    use test_artifacts::{
        ELF_ARITH256, ELF_ARITH256_MOD, ELF_BN254_ADD, ELF_BN254_COMPLEX_ADD,
        ELF_BN254_COMPLEX_MUL, ELF_BN254_COMPLEX_SUB, ELF_BN254_DBL, ELF_SECP256K1_ADD,
        ELF_SECP256K1_DBL, ELF_SECP256R1_ADD, ELF_SECP256R1_DBL,
    };
    use zisk_common::io::ZiskStdin;

    // Tests share a global lock (#[serial]) because each `run_emulation`
    // allocates several GB; running them in parallel exceeds RAM.

    #[test]
    #[serial]
    fn arith256_tests() {
        ELF_ARITH256
            .run_emulation(ZiskStdin::new(), None)
            .expect("arith256 guest emulation failed");
    }

    #[test]
    #[serial]
    fn arith256_mod_tests() {
        ELF_ARITH256_MOD
            .run_emulation(ZiskStdin::new(), None)
            .expect("arith256_mod guest emulation failed");
    }

    #[test]
    #[serial]
    fn secp256k1_add_tests() {
        ELF_SECP256K1_ADD
            .run_emulation(ZiskStdin::new(), None)
            .expect("secp256k1_add guest emulation failed");
    }

    #[test]
    #[serial]
    fn secp256k1_dbl_tests() {
        ELF_SECP256K1_DBL
            .run_emulation(ZiskStdin::new(), None)
            .expect("secp256k1_dbl guest emulation failed");
    }

    #[test]
    #[serial]
    fn secp256r1_add_tests() {
        ELF_SECP256R1_ADD
            .run_emulation(ZiskStdin::new(), None)
            .expect("secp256r1_add guest emulation failed");
    }

    #[test]
    #[serial]
    fn secp256r1_dbl_tests() {
        ELF_SECP256R1_DBL
            .run_emulation(ZiskStdin::new(), None)
            .expect("secp256r1_dbl guest emulation failed");
    }

    #[test]
    #[serial]
    fn bn254_add_tests() {
        ELF_BN254_ADD
            .run_emulation(ZiskStdin::new(), None)
            .expect("bn254_add guest emulation failed");
    }

    #[test]
    #[serial]
    fn bn254_dbl_tests() {
        ELF_BN254_DBL
            .run_emulation(ZiskStdin::new(), None)
            .expect("bn254_dbl guest emulation failed");
    }

    #[test]
    #[serial]
    fn bn254_complex_add_tests() {
        ELF_BN254_COMPLEX_ADD
            .run_emulation(ZiskStdin::new(), None)
            .expect("bn254_complex_add guest emulation failed");
    }

    #[test]
    #[serial]
    fn bn254_complex_mul_tests() {
        ELF_BN254_COMPLEX_MUL
            .run_emulation(ZiskStdin::new(), None)
            .expect("bn254_complex_mul guest emulation failed");
    }

    #[test]
    #[serial]
    fn bn254_complex_sub_tests() {
        ELF_BN254_COMPLEX_SUB
            .run_emulation(ZiskStdin::new(), None)
            .expect("bn254_complex_sub guest emulation failed");
    }
}
