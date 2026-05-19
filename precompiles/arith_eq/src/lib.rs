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
