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
    num_available_field = num_available_ops,
    ops = [
        (OperationArith384ModData         => Arith384Mod,         Arith384ModInput),
        (OperationBls12_381CurveAddData   => Bls12_381CurveAdd,   Bls12_381CurveAddInput),
        (OperationBls12_381CurveDblData   => Bls12_381CurveDbl,   Bls12_381CurveDblInput),
        (OperationBls12_381ComplexAddData => Bls12_381ComplexAdd, Bls12_381ComplexAddInput),
        (OperationBls12_381ComplexSubData => Bls12_381ComplexSub, Bls12_381ComplexSubInput),
        (OperationBls12_381ComplexMulData => Bls12_381ComplexMul, Bls12_381ComplexMulInput),
    ],
}
