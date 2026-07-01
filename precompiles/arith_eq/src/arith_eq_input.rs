use zisk_common::{FromBusPayload, OP, OPERATION_PRECOMPILED_BUS_DATA_SIZE};
use zisk_core::zisk_ops::ZiskOp;

// Bus-payload layout building blocks (in u64 words).
const INDIRECTION_SIZE: usize = 1;
const DATA_256_BITS_SIZE: usize = 4;
const POINT_256_BITS_SIZE: usize = 2 * DATA_256_BITS_SIZE;
const COMPLEX_OVER_256_BITS_SIZE: usize = 2 * DATA_256_BITS_SIZE;

/// 256-bit arithmetic operation data size.
pub const OPERATION_BUS_ARITH_256_DATA_SIZE: usize =
    OPERATION_PRECOMPILED_BUS_DATA_SIZE + 5 * INDIRECTION_SIZE + 3 * DATA_256_BITS_SIZE;
/// 256-bit modular arithmetic operation data size.
pub const OPERATION_BUS_ARITH_256_MOD_DATA_SIZE: usize =
    OPERATION_PRECOMPILED_BUS_DATA_SIZE + 5 * INDIRECTION_SIZE + 4 * DATA_256_BITS_SIZE;
/// Secp256k1 addition operation data size.
pub const OPERATION_BUS_SECP256K1_ADD_DATA_SIZE: usize =
    OPERATION_PRECOMPILED_BUS_DATA_SIZE + 2 * INDIRECTION_SIZE + 2 * POINT_256_BITS_SIZE;
/// Secp256k1 doubling operation data size.
pub const OPERATION_BUS_SECP256K1_DBL_DATA_SIZE: usize =
    OPERATION_PRECOMPILED_BUS_DATA_SIZE + POINT_256_BITS_SIZE;
/// BN254 curve addition operation data size.
pub const OPERATION_BUS_BN254_CURVE_ADD_DATA_SIZE: usize =
    OPERATION_PRECOMPILED_BUS_DATA_SIZE + 2 * INDIRECTION_SIZE + 2 * POINT_256_BITS_SIZE;
/// BN254 curve doubling operation data size.
pub const OPERATION_BUS_BN254_CURVE_DBL_DATA_SIZE: usize =
    OPERATION_PRECOMPILED_BUS_DATA_SIZE + POINT_256_BITS_SIZE;
/// BN254 complex addition operation data size.
pub const OPERATION_BUS_BN254_COMPLEX_ADD_DATA_SIZE: usize =
    OPERATION_PRECOMPILED_BUS_DATA_SIZE + 2 * INDIRECTION_SIZE + 2 * COMPLEX_OVER_256_BITS_SIZE;
/// BN254 complex subtraction operation data size.
pub const OPERATION_BUS_BN254_COMPLEX_SUB_DATA_SIZE: usize =
    OPERATION_PRECOMPILED_BUS_DATA_SIZE + 2 * INDIRECTION_SIZE + 2 * COMPLEX_OVER_256_BITS_SIZE;
/// BN254 complex multiplication operation data size.
pub const OPERATION_BUS_BN254_COMPLEX_MUL_DATA_SIZE: usize =
    OPERATION_PRECOMPILED_BUS_DATA_SIZE + 2 * INDIRECTION_SIZE + 2 * COMPLEX_OVER_256_BITS_SIZE;
/// Secp256r1 addition operation data size.
pub const OPERATION_BUS_SECP256R1_ADD_DATA_SIZE: usize =
    OPERATION_PRECOMPILED_BUS_DATA_SIZE + 2 * INDIRECTION_SIZE + 2 * POINT_256_BITS_SIZE;
/// Secp256r1 doubling operation data size.
pub const OPERATION_BUS_SECP256R1_DBL_DATA_SIZE: usize =
    OPERATION_PRECOMPILED_BUS_DATA_SIZE + POINT_256_BITS_SIZE;

/// 256-bit arithmetic operation data type alias.
pub type OperationArith256Data = [u64; OPERATION_BUS_ARITH_256_DATA_SIZE];
/// 256-bit modular arithmetic operation data type alias.
pub type OperationArith256ModData = [u64; OPERATION_BUS_ARITH_256_MOD_DATA_SIZE];
/// Secp256k1 addition operation data type alias.
pub type OperationSecp256k1AddData = [u64; OPERATION_BUS_SECP256K1_ADD_DATA_SIZE];
/// Secp256k1 doubling operation data type alias.
pub type OperationSecp256k1DblData = [u64; OPERATION_BUS_SECP256K1_DBL_DATA_SIZE];
/// BN254 curve addition operation data type alias.
pub type OperationBn254CurveAddData = [u64; OPERATION_BUS_BN254_CURVE_ADD_DATA_SIZE];
/// BN254 curve doubling operation data type alias.
pub type OperationBn254CurveDblData = [u64; OPERATION_BUS_BN254_CURVE_DBL_DATA_SIZE];
/// BN254 complex addition operation data type alias.
pub type OperationBn254ComplexAddData = [u64; OPERATION_BUS_BN254_COMPLEX_ADD_DATA_SIZE];
/// BN254 complex subtraction operation data type alias.
pub type OperationBn254ComplexSubData = [u64; OPERATION_BUS_BN254_COMPLEX_SUB_DATA_SIZE];
/// BN254 complex multiplication operation data type alias.
pub type OperationBn254ComplexMulData = [u64; OPERATION_BUS_BN254_COMPLEX_MUL_DATA_SIZE];
/// Secp256r1 addition operation data type alias.
pub type OperationSecp256r1AddData = [u64; OPERATION_BUS_SECP256R1_ADD_DATA_SIZE];
/// Secp256r1 doubling operation data type alias.
pub type OperationSecp256r1DblData = [u64; OPERATION_BUS_SECP256R1_DBL_DATA_SIZE];

#[derive(Debug)]
pub enum ArithEqInput {
    Arith256(Arith256Input),
    Arith256Mod(Arith256ModInput),
    Secp256k1Add(Secp256k1AddInput),
    Secp256k1Dbl(Secp256k1DblInput),
    Bn254CurveAdd(Bn254CurveAddInput),
    Bn254CurveDbl(Bn254CurveDblInput),
    Bn254ComplexAdd(Bn254ComplexAddInput),
    Bn254ComplexSub(Bn254ComplexSubInput),
    Bn254ComplexMul(Bn254ComplexMulInput),
    Secp256r1Add(Secp256r1AddInput),
    Secp256r1Dbl(Secp256r1DblInput),
}

impl FromBusPayload for ArithEqInput {
    /// Decodes a bus payload into the matching sub-input, selected by the op at
    /// `payload[OP]`. Each arm narrows the payload to its op's fixed-width type
    /// (inferred from the sub-input's `from`), failing fast on a wrong-width payload.
    fn from_bus_payload(payload: &[u64]) -> Self {
        match payload[OP] as u8 {
            ZiskOp::ARITH256 => ArithEqInput::Arith256(Arith256Input::from(
                payload.try_into().expect("arith256: wrong bus payload size"),
            )),
            ZiskOp::ARITH256_MOD => ArithEqInput::Arith256Mod(Arith256ModInput::from(
                payload.try_into().expect("arith256_mod: wrong bus payload size"),
            )),
            ZiskOp::SECP256K1_ADD => ArithEqInput::Secp256k1Add(Secp256k1AddInput::from(
                payload.try_into().expect("secp256k1_add: wrong bus payload size"),
            )),
            ZiskOp::SECP256K1_DBL => ArithEqInput::Secp256k1Dbl(Secp256k1DblInput::from(
                payload.try_into().expect("secp256k1_dbl: wrong bus payload size"),
            )),
            ZiskOp::BN254_CURVE_ADD => ArithEqInput::Bn254CurveAdd(Bn254CurveAddInput::from(
                payload.try_into().expect("bn254_curve_add: wrong bus payload size"),
            )),
            ZiskOp::BN254_CURVE_DBL => ArithEqInput::Bn254CurveDbl(Bn254CurveDblInput::from(
                payload.try_into().expect("bn254_curve_dbl: wrong bus payload size"),
            )),
            ZiskOp::BN254_COMPLEX_ADD => ArithEqInput::Bn254ComplexAdd(Bn254ComplexAddInput::from(
                payload.try_into().expect("bn254_complex_add: wrong bus payload size"),
            )),
            ZiskOp::BN254_COMPLEX_SUB => ArithEqInput::Bn254ComplexSub(Bn254ComplexSubInput::from(
                payload.try_into().expect("bn254_complex_sub: wrong bus payload size"),
            )),
            ZiskOp::BN254_COMPLEX_MUL => ArithEqInput::Bn254ComplexMul(Bn254ComplexMulInput::from(
                payload.try_into().expect("bn254_complex_mul: wrong bus payload size"),
            )),
            ZiskOp::SECP256R1_ADD => ArithEqInput::Secp256r1Add(Secp256r1AddInput::from(
                payload.try_into().expect("secp256r1_add: wrong bus payload size"),
            )),
            ZiskOp::SECP256R1_DBL => ArithEqInput::Secp256r1Dbl(Secp256r1DblInput::from(
                payload.try_into().expect("secp256r1_dbl: wrong bus payload size"),
            )),
            op => panic!("ArithEqInput: unexpected op {op:#04x}"),
        }
    }
}

#[derive(Debug)]
pub struct Arith256Input {
    pub addr: u32,
    pub a_addr: u32,
    pub b_addr: u32,
    pub c_addr: u32,
    pub dh_addr: u32,
    pub dl_addr: u32,
    pub step: u64,
    pub a: [u64; 4],
    pub b: [u64; 4],
    pub c: [u64; 4],
}

impl Arith256Input {
    pub fn from(values: &OperationArith256Data) -> Self {
        Self {
            addr: values[3] as u32,
            a_addr: values[5] as u32,
            b_addr: values[6] as u32,
            c_addr: values[7] as u32,
            dl_addr: values[8] as u32,
            dh_addr: values[9] as u32,
            step: values[4],
            a: values[10..14].try_into().unwrap(),
            b: values[14..18].try_into().unwrap(),
            c: values[18..22].try_into().unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct Arith256ModInput {
    pub addr: u32,
    pub a_addr: u32,
    pub b_addr: u32,
    pub c_addr: u32,
    pub module_addr: u32,
    pub d_addr: u32,
    pub step: u64,
    pub a: [u64; 4],
    pub b: [u64; 4],
    pub c: [u64; 4],
    pub module: [u64; 4],
}

impl Arith256ModInput {
    pub fn from(values: &OperationArith256ModData) -> Self {
        Self {
            addr: values[3] as u32,
            a_addr: values[5] as u32,
            b_addr: values[6] as u32,
            c_addr: values[7] as u32,
            module_addr: values[8] as u32,
            d_addr: values[9] as u32,
            step: values[4],
            a: values[10..14].try_into().unwrap(),
            b: values[14..18].try_into().unwrap(),
            c: values[18..22].try_into().unwrap(),
            module: values[22..26].try_into().unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct Secp256k1AddInput {
    pub addr: u32,
    pub p1_addr: u32,
    pub p2_addr: u32,
    pub step: u64,
    pub p1: [u64; 8],
    pub p2: [u64; 8],
}

impl Secp256k1AddInput {
    pub fn from(values: &OperationSecp256k1AddData) -> Self {
        Self {
            addr: values[3] as u32,
            p1_addr: values[5] as u32,
            p2_addr: values[6] as u32,
            step: values[4],
            p1: values[7..15].try_into().unwrap(),
            p2: values[15..23].try_into().unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct Secp256k1DblInput {
    pub addr: u32,
    pub step: u64,
    pub p1: [u64; 8],
}

impl Secp256k1DblInput {
    pub fn from(values: &OperationSecp256k1DblData) -> Self {
        Self { addr: values[3] as u32, step: values[4], p1: values[5..13].try_into().unwrap() }
    }
}

#[derive(Debug)]
pub struct Bn254CurveAddInput {
    pub addr: u32,
    pub p1_addr: u32,
    pub p2_addr: u32,
    pub step: u64,
    pub p1: [u64; 8],
    pub p2: [u64; 8],
}

impl Bn254CurveAddInput {
    pub fn from(values: &OperationBn254CurveAddData) -> Self {
        Self {
            addr: values[3] as u32,
            p1_addr: values[5] as u32,
            p2_addr: values[6] as u32,
            step: values[4],
            p1: values[7..15].try_into().unwrap(),
            p2: values[15..23].try_into().unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct Bn254CurveDblInput {
    pub addr: u32,
    pub step: u64,
    pub p1: [u64; 8],
}

impl Bn254CurveDblInput {
    pub fn from(values: &OperationBn254CurveDblData) -> Self {
        Self { addr: values[3] as u32, step: values[4], p1: values[5..13].try_into().unwrap() }
    }
}

#[derive(Debug)]
pub struct Bn254ComplexAddInput {
    pub addr: u32,
    pub f1_addr: u32,
    pub f2_addr: u32,
    pub step: u64,
    pub f1: [u64; 8],
    pub f2: [u64; 8],
}

impl Bn254ComplexAddInput {
    pub fn from(values: &OperationBn254ComplexAddData) -> Self {
        Self {
            addr: values[3] as u32,
            f1_addr: values[5] as u32,
            f2_addr: values[6] as u32,
            step: values[4],
            f1: values[7..15].try_into().unwrap(),
            f2: values[15..23].try_into().unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct Bn254ComplexSubInput {
    pub addr: u32,
    pub f1_addr: u32,
    pub f2_addr: u32,
    pub step: u64,
    pub f1: [u64; 8],
    pub f2: [u64; 8],
}

impl Bn254ComplexSubInput {
    pub fn from(values: &OperationBn254ComplexSubData) -> Self {
        Self {
            addr: values[3] as u32,
            f1_addr: values[5] as u32,
            f2_addr: values[6] as u32,
            step: values[4],
            f1: values[7..15].try_into().unwrap(),
            f2: values[15..23].try_into().unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct Bn254ComplexMulInput {
    pub addr: u32,
    pub f1_addr: u32,
    pub f2_addr: u32,
    pub step: u64,
    pub f1: [u64; 8],
    pub f2: [u64; 8],
}

impl Bn254ComplexMulInput {
    pub fn from(values: &OperationBn254ComplexMulData) -> Self {
        Self {
            addr: values[3] as u32,
            f1_addr: values[5] as u32,
            f2_addr: values[6] as u32,
            step: values[4],
            f1: values[7..15].try_into().unwrap(),
            f2: values[15..23].try_into().unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct Secp256r1AddInput {
    pub addr: u32,
    pub p1_addr: u32,
    pub p2_addr: u32,
    pub step: u64,
    pub p1: [u64; 8],
    pub p2: [u64; 8],
}

impl Secp256r1AddInput {
    pub fn from(values: &OperationSecp256r1AddData) -> Self {
        Self {
            addr: values[3] as u32,
            p1_addr: values[5] as u32,
            p2_addr: values[6] as u32,
            step: values[4],
            p1: values[7..15].try_into().unwrap(),
            p2: values[15..23].try_into().unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct Secp256r1DblInput {
    pub addr: u32,
    pub step: u64,
    pub p1: [u64; 8],
}

impl Secp256r1DblInput {
    pub fn from(values: &OperationSecp256r1DblData) -> Self {
        Self { addr: values[3] as u32, step: values[4], p1: values[5..13].try_into().unwrap() }
    }
}
