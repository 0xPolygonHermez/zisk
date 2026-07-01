use zisk_common::{FromBusPayload, OP, OPERATION_PRECOMPILED_BUS_DATA_SIZE};
use zisk_core::zisk_ops::ZiskOp;

use crate::{ARITH_EQ_384_U64S, ARITH_EQ_384_U64S_DOUBLE};

const INDIRECTION_SIZE: usize = 1;

/// Arithmetic 384-bit modular operation data size.
pub const OPERATION_BUS_ARITH_384_MOD_DATA_SIZE: usize =
    OPERATION_PRECOMPILED_BUS_DATA_SIZE + 5 * INDIRECTION_SIZE + 4 * ARITH_EQ_384_U64S;
/// BLS12-381 curve addition operation data size.
pub const OPERATION_BUS_BLS12_381_CURVE_ADD_DATA_SIZE: usize =
    OPERATION_PRECOMPILED_BUS_DATA_SIZE + 2 * INDIRECTION_SIZE + 2 * ARITH_EQ_384_U64S_DOUBLE;
/// BLS12-381 curve doubling operation data size.
pub const OPERATION_BUS_BLS12_381_CURVE_DBL_DATA_SIZE: usize =
    OPERATION_PRECOMPILED_BUS_DATA_SIZE + ARITH_EQ_384_U64S_DOUBLE;
/// BLS12-381 complex addition operation data size.
pub const OPERATION_BUS_BLS12_381_COMPLEX_ADD_DATA_SIZE: usize =
    OPERATION_PRECOMPILED_BUS_DATA_SIZE + 2 * INDIRECTION_SIZE + 2 * ARITH_EQ_384_U64S_DOUBLE;
/// BLS12-381 complex subtraction operation data size.
pub const OPERATION_BUS_BLS12_381_COMPLEX_SUB_DATA_SIZE: usize =
    OPERATION_PRECOMPILED_BUS_DATA_SIZE + 2 * INDIRECTION_SIZE + 2 * ARITH_EQ_384_U64S_DOUBLE;
/// BLS12-381 complex multiplication operation data size.
pub const OPERATION_BUS_BLS12_381_COMPLEX_MUL_DATA_SIZE: usize =
    OPERATION_PRECOMPILED_BUS_DATA_SIZE + 2 * INDIRECTION_SIZE + 2 * ARITH_EQ_384_U64S_DOUBLE;

/// 384-bit modular arithmetic operation data type alias.
pub type OperationArith384ModData = [u64; OPERATION_BUS_ARITH_384_MOD_DATA_SIZE];
/// BLS12-381 curve addition operation data type alias.
pub type OperationBls12_381CurveAddData = [u64; OPERATION_BUS_BLS12_381_CURVE_ADD_DATA_SIZE];
/// BLS12-381 curve doubling operation data type alias.
pub type OperationBls12_381CurveDblData = [u64; OPERATION_BUS_BLS12_381_CURVE_DBL_DATA_SIZE];
/// BLS12-381 complex addition operation data type alias.
pub type OperationBls12_381ComplexAddData = [u64; OPERATION_BUS_BLS12_381_COMPLEX_ADD_DATA_SIZE];
/// BLS12-381 complex subtraction operation data type alias.
pub type OperationBls12_381ComplexSubData = [u64; OPERATION_BUS_BLS12_381_COMPLEX_SUB_DATA_SIZE];
/// BLS12-381 complex multiplication operation data type alias.
pub type OperationBls12_381ComplexMulData = [u64; OPERATION_BUS_BLS12_381_COMPLEX_MUL_DATA_SIZE];

#[derive(Debug)]
pub enum ArithEq384Input {
    Arith384Mod(Arith384ModInput),
    Bls12_381CurveAdd(Bls12_381CurveAddInput),
    Bls12_381CurveDbl(Bls12_381CurveDblInput),
    Bls12_381ComplexAdd(Bls12_381ComplexAddInput),
    Bls12_381ComplexSub(Bls12_381ComplexSubInput),
    Bls12_381ComplexMul(Bls12_381ComplexMulInput),
}

impl FromBusPayload for ArithEq384Input {
    /// Decodes a bus payload into the matching sub-input, selected by the op at
    /// `payload[OP]`. Each arm narrows the payload to its op's fixed-width type
    /// (inferred from the sub-input's `from`), failing fast on a wrong-width payload.
    fn from_bus_payload(payload: &[u64]) -> Self {
        match payload[OP] as u8 {
            ZiskOp::ARITH384_MOD => ArithEq384Input::Arith384Mod(Arith384ModInput::from(
                payload.try_into().expect("arith384_mod: wrong bus payload size"),
            )),
            ZiskOp::BLS12_381_CURVE_ADD => {
                ArithEq384Input::Bls12_381CurveAdd(Bls12_381CurveAddInput::from(
                    payload.try_into().expect("bls12_381_curve_add: wrong bus payload size"),
                ))
            }
            ZiskOp::BLS12_381_CURVE_DBL => {
                ArithEq384Input::Bls12_381CurveDbl(Bls12_381CurveDblInput::from(
                    payload.try_into().expect("bls12_381_curve_dbl: wrong bus payload size"),
                ))
            }
            ZiskOp::BLS12_381_COMPLEX_ADD => {
                ArithEq384Input::Bls12_381ComplexAdd(Bls12_381ComplexAddInput::from(
                    payload.try_into().expect("bls12_381_complex_add: wrong bus payload size"),
                ))
            }
            ZiskOp::BLS12_381_COMPLEX_SUB => {
                ArithEq384Input::Bls12_381ComplexSub(Bls12_381ComplexSubInput::from(
                    payload.try_into().expect("bls12_381_complex_sub: wrong bus payload size"),
                ))
            }
            ZiskOp::BLS12_381_COMPLEX_MUL => {
                ArithEq384Input::Bls12_381ComplexMul(Bls12_381ComplexMulInput::from(
                    payload.try_into().expect("bls12_381_complex_mul: wrong bus payload size"),
                ))
            }
            op => panic!("ArithEq384Input: unexpected op {op:#04x}"),
        }
    }
}

#[derive(Debug)]
pub struct Arith384ModInput {
    pub addr: u32,
    pub a_addr: u32,
    pub b_addr: u32,
    pub c_addr: u32,
    pub module_addr: u32,
    pub d_addr: u32,
    pub step: u64,
    pub a: [u64; ARITH_EQ_384_U64S],
    pub b: [u64; ARITH_EQ_384_U64S],
    pub c: [u64; ARITH_EQ_384_U64S],
    pub module: [u64; ARITH_EQ_384_U64S],
}

impl Arith384ModInput {
    pub fn from(values: &OperationArith384ModData) -> Self {
        Self {
            addr: values[3] as u32,
            a_addr: values[5] as u32,
            b_addr: values[6] as u32,
            c_addr: values[7] as u32,
            module_addr: values[8] as u32,
            d_addr: values[9] as u32,
            step: values[4],
            a: values[10..16].try_into().unwrap(),
            b: values[16..22].try_into().unwrap(),
            c: values[22..28].try_into().unwrap(),
            module: values[28..34].try_into().unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct Bls12_381CurveAddInput {
    pub addr: u32,
    pub p1_addr: u32,
    pub p2_addr: u32,
    pub step: u64,
    pub p1: [u64; ARITH_EQ_384_U64S_DOUBLE],
    pub p2: [u64; ARITH_EQ_384_U64S_DOUBLE],
}

impl Bls12_381CurveAddInput {
    pub fn from(values: &OperationBls12_381CurveAddData) -> Self {
        Self {
            addr: values[3] as u32,
            p1_addr: values[5] as u32,
            p2_addr: values[6] as u32,
            step: values[4],
            p1: values[7..19].try_into().unwrap(),
            p2: values[19..31].try_into().unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct Bls12_381CurveDblInput {
    pub addr: u32,
    pub step: u64,
    pub p1: [u64; ARITH_EQ_384_U64S_DOUBLE],
}

impl Bls12_381CurveDblInput {
    pub fn from(values: &OperationBls12_381CurveDblData) -> Self {
        Self { addr: values[3] as u32, step: values[4], p1: values[5..17].try_into().unwrap() }
    }
}

#[derive(Debug)]
pub struct Bls12_381ComplexAddInput {
    pub addr: u32,
    pub f1_addr: u32,
    pub f2_addr: u32,
    pub step: u64,
    pub f1: [u64; ARITH_EQ_384_U64S_DOUBLE],
    pub f2: [u64; ARITH_EQ_384_U64S_DOUBLE],
}

impl Bls12_381ComplexAddInput {
    pub fn from(values: &OperationBls12_381ComplexAddData) -> Self {
        Self {
            addr: values[3] as u32,
            f1_addr: values[5] as u32,
            f2_addr: values[6] as u32,
            step: values[4],
            f1: values[7..19].try_into().unwrap(),
            f2: values[19..31].try_into().unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct Bls12_381ComplexSubInput {
    pub addr: u32,
    pub f1_addr: u32,
    pub f2_addr: u32,
    pub step: u64,
    pub f1: [u64; ARITH_EQ_384_U64S_DOUBLE],
    pub f2: [u64; ARITH_EQ_384_U64S_DOUBLE],
}

impl Bls12_381ComplexSubInput {
    pub fn from(values: &OperationBls12_381ComplexSubData) -> Self {
        Self {
            addr: values[3] as u32,
            f1_addr: values[5] as u32,
            f2_addr: values[6] as u32,
            step: values[4],
            f1: values[7..19].try_into().unwrap(),
            f2: values[19..31].try_into().unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct Bls12_381ComplexMulInput {
    pub addr: u32,
    pub f1_addr: u32,
    pub f2_addr: u32,
    pub step: u64,
    pub f1: [u64; ARITH_EQ_384_U64S_DOUBLE],
    pub f2: [u64; ARITH_EQ_384_U64S_DOUBLE],
}

impl Bls12_381ComplexMulInput {
    pub fn from(values: &OperationBls12_381ComplexMulData) -> Self {
        Self {
            addr: values[3] as u32,
            f1_addr: values[5] as u32,
            f2_addr: values[6] as u32,
            step: values[4],
            f1: values[7..19].try_into().unwrap(),
            f2: values[19..31].try_into().unwrap(),
        }
    }
}
