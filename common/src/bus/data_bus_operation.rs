//! The `OperationBusData` module facilitates the handling and transformation of operation-related
//! data for communication over the operation bus. This includes data extraction from instructions
//! and managing the format of operation data.

use crate::{uninit_array, BusId, PayloadType};
use std::collections::VecDeque;
use zisk_core::zisk_ops::ZiskOp;
use zisk_core::{InstContext, ZiskInst, ZiskOperationType};

/// The unique bus ID for operation-related data communication.
pub const OPERATION_BUS_ID: BusId = BusId(0);

/// The size of the operation data payload.
pub const OPERATION_BUS_DATA_SIZE: usize = 4; // op,op_type,a,b

// worst case:
// arith_256:     3 x 256 + 2 addr = 3 * 4 + 2 = 14
// arith_256_mod: 4 x 256 + 2 addr = 4 * 4 + 2 = 18
// secp256k1_add: 4 x 256 + 2 addr = 4 * 4 + 2 = 18
// secp256k1_dbl: 2 x 256 + 1 addr = 2 * 4 + 1 = 9
// TODO: optimize and send only one value 64 upto 32-bits addr

const INDIRECTION_SIZE: usize = 1;
const PARAMS_SIZE: usize = 1;
const SINGLE_RESULT_SIZE: usize = 1;

const DATA_256_BITS_SIZE: usize = 4;
const POINT_256_BITS_SIZE: usize = 2 * DATA_256_BITS_SIZE;
const COMPLEX_OVER_256_BITS_SIZE: usize = 2 * DATA_256_BITS_SIZE;

const DATA_384_BITS_SIZE: usize = 6;
const POINT_384_BITS_SIZE: usize = 2 * DATA_384_BITS_SIZE;
const COMPLEX_OVER_384_BITS_SIZE: usize = 2 * DATA_384_BITS_SIZE;

// use OPERATION_BUS_DATA_SIZE because a = step, b = addr
pub const OPERATION_BUS_KECCAKF_DATA_SIZE: usize = OPERATION_BUS_DATA_SIZE + 25;
pub const OPERATION_BUS_SHA256F_DATA_SIZE: usize =
    OPERATION_BUS_DATA_SIZE + 2 * INDIRECTION_SIZE + 3 * DATA_256_BITS_SIZE;
pub const OPERATION_BUS_ARITH_256_DATA_SIZE: usize =
    OPERATION_BUS_DATA_SIZE + 5 * INDIRECTION_SIZE + 3 * DATA_256_BITS_SIZE;
pub const OPERATION_BUS_ARITH_256_MOD_DATA_SIZE: usize =
    OPERATION_BUS_DATA_SIZE + 5 * INDIRECTION_SIZE + 4 * DATA_256_BITS_SIZE;
pub const OPERATION_BUS_SECP256K1_ADD_DATA_SIZE: usize =
    OPERATION_BUS_DATA_SIZE + 2 * INDIRECTION_SIZE + 2 * POINT_256_BITS_SIZE;
pub const OPERATION_BUS_SECP256K1_DBL_DATA_SIZE: usize =
    OPERATION_BUS_DATA_SIZE + POINT_256_BITS_SIZE;
pub const OPERATION_BUS_BN254_CURVE_ADD_DATA_SIZE: usize =
    OPERATION_BUS_DATA_SIZE + 2 * INDIRECTION_SIZE + 2 * POINT_256_BITS_SIZE;
pub const OPERATION_BUS_BN254_CURVE_DBL_DATA_SIZE: usize =
    OPERATION_BUS_DATA_SIZE + POINT_256_BITS_SIZE;
pub const OPERATION_BUS_BN254_COMPLEX_ADD_DATA_SIZE: usize =
    OPERATION_BUS_DATA_SIZE + 2 * INDIRECTION_SIZE + 2 * COMPLEX_OVER_256_BITS_SIZE;
pub const OPERATION_BUS_BN254_COMPLEX_SUB_DATA_SIZE: usize =
    OPERATION_BUS_DATA_SIZE + 2 * INDIRECTION_SIZE + 2 * COMPLEX_OVER_256_BITS_SIZE;
pub const OPERATION_BUS_BN254_COMPLEX_MUL_DATA_SIZE: usize =
    OPERATION_BUS_DATA_SIZE + 2 * INDIRECTION_SIZE + 2 * COMPLEX_OVER_256_BITS_SIZE;
pub const OPERATION_BUS_ARITH_384_MOD_DATA_SIZE: usize =
    OPERATION_BUS_DATA_SIZE + 5 * INDIRECTION_SIZE + 4 * DATA_384_BITS_SIZE;
pub const OPERATION_BUS_BLS12_381_CURVE_ADD_DATA_SIZE: usize =
    OPERATION_BUS_DATA_SIZE + 2 * INDIRECTION_SIZE + 2 * POINT_384_BITS_SIZE;
pub const OPERATION_BUS_BLS12_381_CURVE_DBL_DATA_SIZE: usize =
    OPERATION_BUS_DATA_SIZE + POINT_384_BITS_SIZE;
pub const OPERATION_BUS_BLS12_381_COMPLEX_ADD_DATA_SIZE: usize =
    OPERATION_BUS_DATA_SIZE + 2 * INDIRECTION_SIZE + 2 * COMPLEX_OVER_384_BITS_SIZE;
pub const OPERATION_BUS_BLS12_381_COMPLEX_SUB_DATA_SIZE: usize =
    OPERATION_BUS_DATA_SIZE + 2 * INDIRECTION_SIZE + 2 * COMPLEX_OVER_384_BITS_SIZE;
pub const OPERATION_BUS_BLS12_381_COMPLEX_MUL_DATA_SIZE: usize =
    OPERATION_BUS_DATA_SIZE + 2 * INDIRECTION_SIZE + 2 * COMPLEX_OVER_384_BITS_SIZE;

// bus_data_size + 4 params (&a, &b, cin, &c, a, b)
pub const OPERATION_BUS_ADD_256_DATA_SIZE: usize =
    OPERATION_BUS_DATA_SIZE + 4 * PARAMS_SIZE + 2 * DATA_256_BITS_SIZE + SINGLE_RESULT_SIZE;

// 4 bus_data + 5 addr + 4 x 384 = 4 + 5 + 4 * 6 = 33
pub const MAX_OPERATION_DATA_SIZE: usize = OPERATION_BUS_ARITH_384_MOD_DATA_SIZE;

/// Index of the operation value in the operation data payload.
pub const OP: usize = 0;

/// Index of the operation type in the operation data payload.
pub const OP_TYPE: usize = 1;

/// Index of the `a` value in the operation data payload.
pub const A: usize = 2;

/// Index of the `b` value in the operation data payload.
pub const B: usize = 3;

/// Type alias for operation data payload.
pub type OperationData<D> = [D; OPERATION_BUS_DATA_SIZE];

/// Type alias for precompiles operation data payload.
pub type OperationKeccakData<D> = [D; OPERATION_BUS_KECCAKF_DATA_SIZE];
pub type OperationSha256Data<D> = [D; OPERATION_BUS_SHA256F_DATA_SIZE];
pub type OperationArith256Data<D> = [D; OPERATION_BUS_ARITH_256_DATA_SIZE];
pub type OperationArith256ModData<D> = [D; OPERATION_BUS_ARITH_256_MOD_DATA_SIZE];
pub type OperationSecp256k1AddData<D> = [D; OPERATION_BUS_SECP256K1_ADD_DATA_SIZE];
pub type OperationSecp256k1DblData<D> = [D; OPERATION_BUS_SECP256K1_DBL_DATA_SIZE];
pub type OperationBn254CurveAddData<D> = [D; OPERATION_BUS_BN254_CURVE_ADD_DATA_SIZE];
pub type OperationBn254CurveDblData<D> = [D; OPERATION_BUS_BN254_CURVE_DBL_DATA_SIZE];
pub type OperationBn254ComplexAddData<D> = [D; OPERATION_BUS_BN254_COMPLEX_ADD_DATA_SIZE];
pub type OperationBn254ComplexSubData<D> = [D; OPERATION_BUS_BN254_COMPLEX_SUB_DATA_SIZE];
pub type OperationBn254ComplexMulData<D> = [D; OPERATION_BUS_BN254_COMPLEX_MUL_DATA_SIZE];
pub type OperationArith384ModData<D> = [D; OPERATION_BUS_ARITH_384_MOD_DATA_SIZE];
pub type OperationBls12_381CurveAddData<D> = [D; OPERATION_BUS_BLS12_381_CURVE_ADD_DATA_SIZE];
pub type OperationBls12_381CurveDblData<D> = [D; OPERATION_BUS_BLS12_381_CURVE_DBL_DATA_SIZE];
pub type OperationBls12_381ComplexAddData<D> = [D; OPERATION_BUS_BLS12_381_COMPLEX_ADD_DATA_SIZE];
pub type OperationBls12_381ComplexSubData<D> = [D; OPERATION_BUS_BLS12_381_COMPLEX_SUB_DATA_SIZE];
pub type OperationBls12_381ComplexMulData<D> = [D; OPERATION_BUS_BLS12_381_COMPLEX_MUL_DATA_SIZE];
pub type OperationAdd256Data<D> = [D; OPERATION_BUS_ADD_256_DATA_SIZE];

pub enum ExtOperationData<D> {
    OperationData(OperationData<D>),
    OperationKeccakData(OperationKeccakData<D>),
    OperationSha256Data(OperationSha256Data<D>),
    OperationArith256Data(OperationArith256Data<D>),
    OperationArith256ModData(OperationArith256ModData<D>),
    OperationSecp256k1AddData(OperationSecp256k1AddData<D>),
    OperationSecp256k1DblData(OperationSecp256k1DblData<D>),
    OperationBn254CurveAddData(OperationBn254CurveAddData<D>),
    OperationBn254CurveDblData(OperationBn254CurveDblData<D>),
    OperationBn254ComplexAddData(OperationBn254ComplexAddData<D>),
    OperationBn254ComplexSubData(OperationBn254ComplexSubData<D>),
    OperationBn254ComplexMulData(OperationBn254ComplexMulData<D>),
    OperationArith384ModData(OperationArith384ModData<D>),
    OperationBls12_381CurveAddData(OperationBls12_381CurveAddData<D>),
    OperationBls12_381CurveDblData(OperationBls12_381CurveDblData<D>),
    OperationBls12_381ComplexAddData(OperationBls12_381ComplexAddData<D>),
    OperationBls12_381ComplexSubData(OperationBls12_381ComplexSubData<D>),
    OperationBls12_381ComplexMulData(OperationBls12_381ComplexMulData<D>),
    OperationAdd256Data(OperationAdd256Data<D>),
}

const KECCAK_OP: u8 = ZiskOp::Keccak.code();
const SHA256_OP: u8 = ZiskOp::Sha256.code();
const ARITH256_OP: u8 = ZiskOp::Arith256.code();
const ARITH256_MOD_OP: u8 = ZiskOp::Arith256Mod.code();
const SECP256K1_ADD_OP: u8 = ZiskOp::Secp256k1Add.code();
const SECP256K1_DBL_OP: u8 = ZiskOp::Secp256k1Dbl.code();
const BN254_CURVE_ADD_OP: u8 = ZiskOp::Bn254CurveAdd.code();
const BN254_CURVE_DBL_OP: u8 = ZiskOp::Bn254CurveDbl.code();
const BN254_COMPLEX_ADD_OP: u8 = ZiskOp::Bn254ComplexAdd.code();
const BN254_COMPLEX_SUB_OP: u8 = ZiskOp::Bn254ComplexSub.code();
const BN254_COMPLEX_MUL_OP: u8 = ZiskOp::Bn254ComplexMul.code();
const ARITH384_MOD_OP: u8 = ZiskOp::Arith384Mod.code();
const BLS12_381_CURVE_ADD_OP: u8 = ZiskOp::Bls12_381CurveAdd.code();
const BLS12_381_CURVE_DBL_OP: u8 = ZiskOp::Bls12_381CurveDbl.code();
const BLS12_381_COMPLEX_ADD_OP: u8 = ZiskOp::Bls12_381ComplexAdd.code();
const BLS12_381_COMPLEX_SUB_OP: u8 = ZiskOp::Bls12_381ComplexSub.code();
const BLS12_381_COMPLEX_MUL_OP: u8 = ZiskOp::Bls12_381ComplexMul.code();
const ADD256_OP: u8 = ZiskOp::Add256.code();

// impl<D: Copy + Into<u8>> TryFrom<&[D]> for ExtOperationData<D> {
impl<D: Copy + Into<u64>> TryFrom<&[D]> for ExtOperationData<D> {
    type Error = &'static str;

    fn try_from(data: &[D]) -> Result<Self, Self::Error> {
        if data.len() < OPERATION_BUS_DATA_SIZE {
            return Err("Invalid data length");
        }
        let op = data[OP].into();
        match op as u8 {
            KECCAK_OP => {
                let array: OperationKeccakData<D> =
                    data.try_into().map_err(|_| "Invalid OperationKeccakData size")?;
                Ok(ExtOperationData::OperationKeccakData(array))
            }
            SHA256_OP => {
                let array: OperationSha256Data<D> =
                    data.try_into().map_err(|_| "Invalid OperationSha256Data size")?;
                Ok(ExtOperationData::OperationSha256Data(array))
            }
            ARITH256_OP => {
                let array: OperationArith256Data<D> =
                    data.try_into().map_err(|_| "Invalid OperationArith256Data size")?;
                Ok(ExtOperationData::OperationArith256Data(array))
            }
            ARITH256_MOD_OP => {
                let array: OperationArith256ModData<D> =
                    data.try_into().map_err(|_| "Invalid OperationArith256ModData size")?;
                Ok(ExtOperationData::OperationArith256ModData(array))
            }
            SECP256K1_ADD_OP => {
                let array: OperationSecp256k1AddData<D> =
                    data.try_into().map_err(|_| "Invalid OperationSecp256k1AddData size")?;
                Ok(ExtOperationData::OperationSecp256k1AddData(array))
            }
            SECP256K1_DBL_OP => {
                let array: OperationSecp256k1DblData<D> =
                    data.try_into().map_err(|_| "Invalid OperationSecp256k1DblData size")?;
                Ok(ExtOperationData::OperationSecp256k1DblData(array))
            }
            BN254_CURVE_ADD_OP => {
                let array: OperationBn254CurveAddData<D> =
                    data.try_into().map_err(|_| "Invalid OperationBn254CurveAddData size")?;
                Ok(ExtOperationData::OperationBn254CurveAddData(array))
            }
            BN254_CURVE_DBL_OP => {
                let array: OperationBn254CurveDblData<D> =
                    data.try_into().map_err(|_| "Invalid OperationBn254CurveDblData size")?;
                Ok(ExtOperationData::OperationBn254CurveDblData(array))
            }
            BN254_COMPLEX_ADD_OP => {
                let array: OperationBn254ComplexAddData<D> =
                    data.try_into().map_err(|_| "Invalid OperationBn254ComplexAddData size")?;
                Ok(ExtOperationData::OperationBn254ComplexAddData(array))
            }
            BN254_COMPLEX_SUB_OP => {
                let array: OperationBn254ComplexSubData<D> =
                    data.try_into().map_err(|_| "Invalid OperationBn254ComplexSubData size")?;
                Ok(ExtOperationData::OperationBn254ComplexSubData(array))
            }
            BN254_COMPLEX_MUL_OP => {
                let array: OperationBn254ComplexMulData<D> =
                    data.try_into().map_err(|_| "Invalid OperationBn254ComplexMulData size")?;
                Ok(ExtOperationData::OperationBn254ComplexMulData(array))
            }
            ARITH384_MOD_OP => {
                let array: OperationArith384ModData<D> =
                    data.try_into().map_err(|_| "Invalid OperationArith384ModData size")?;
                Ok(ExtOperationData::OperationArith384ModData(array))
            }
            BLS12_381_CURVE_ADD_OP => {
                let array: OperationBls12_381CurveAddData<D> =
                    data.try_into().map_err(|_| "Invalid OperationBls12_381CurveAddData size")?;
                Ok(ExtOperationData::OperationBls12_381CurveAddData(array))
            }
            BLS12_381_CURVE_DBL_OP => {
                let array: OperationBls12_381CurveDblData<D> =
                    data.try_into().map_err(|_| "Invalid OperationBls12_381CurveDblData size")?;
                Ok(ExtOperationData::OperationBls12_381CurveDblData(array))
            }
            BLS12_381_COMPLEX_ADD_OP => {
                let array: OperationBls12_381ComplexAddData<D> =
                    data.try_into().map_err(|_| "Invalid OperationBls12_381ComplexAddData size")?;
                Ok(ExtOperationData::OperationBls12_381ComplexAddData(array))
            }
            BLS12_381_COMPLEX_SUB_OP => {
                let array: OperationBls12_381ComplexSubData<D> =
                    data.try_into().map_err(|_| "Invalid OperationBls12_381ComplexSubData size")?;
                Ok(ExtOperationData::OperationBls12_381ComplexSubData(array))
            }
            BLS12_381_COMPLEX_MUL_OP => {
                let array: OperationBls12_381ComplexMulData<D> =
                    data.try_into().map_err(|_| "Invalid OperationBls12_381ComplexMulData size")?;
                Ok(ExtOperationData::OperationBls12_381ComplexMulData(array))
            }
            ADD256_OP => {
                let array: OperationAdd256Data<D> =
                    data.try_into().map_err(|_| "Invalid OperationAdd256Data size")?;
                Ok(ExtOperationData::OperationAdd256Data(array))
            }
            _ => {
                let array: OperationData<D> =
                    data.try_into().map_err(|_| "Invalid OperationData size")?;
                Ok(ExtOperationData::OperationData(array))
            }
        }
    }
}

/// Provides utility functions for creating and interacting with operation bus data.
///
/// This struct is implemented as a zero-sized type with a `PhantomData` marker to enable
/// type-specific functionality for `u64` operation data.
pub struct OperationBusData<D>(std::marker::PhantomData<D>);

impl OperationBusData<u64> {
    /// Creates operation data from raw values.
    ///
    /// # Arguments
    /// * `step` - The current step of the operation.
    /// * `op` - The operation code.
    /// * `op_type` - The type of operation payload.
    /// * `a` - The value of the `a` parameter.
    /// * `b` - The value of the `b` parameter.
    ///
    /// # Returns
    /// An array representing the operation data payload.
    #[inline(always)]
    pub fn from_values(
        op: u8,
        op_type: PayloadType,
        a: u64,
        b: u64,
        pending: &mut VecDeque<(BusId, Vec<u64>)>,
    ) {
        pending.push_back((OPERATION_BUS_ID, vec![op as u64, op_type, a, b]));
    }

    /// Creates operation data from a `ZiskInst` instruction and its context.
    ///
    /// # Arguments
    /// * `inst` - A reference to the `ZiskInst` representing the operation.
    /// * `inst_ctx` - A reference to the instruction context containing metadata for the operation.
    ///
    /// # Returns
    /// An array representing the operation data payload.
    #[inline(always)]
    pub fn from_instruction(inst: &ZiskInst, ctx: &InstContext) -> ExtOperationData<u64> {
        let a = if inst.m32 { ctx.a & 0xffff_ffff } else { ctx.a };
        let b = if inst.m32 { ctx.b & 0xffff_ffff } else { ctx.b };
        let op = inst.op as u64;
        let op_type = inst.op_type as u64;

        match inst.op_type {
            ZiskOperationType::Keccak => {
                let mut data =
                    unsafe { uninit_array::<OPERATION_BUS_KECCAKF_DATA_SIZE>().assume_init() };
                data[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                data[OPERATION_BUS_DATA_SIZE..].copy_from_slice(&ctx.precompiled.input_data);
                ExtOperationData::OperationKeccakData(data)
            }

            ZiskOperationType::Sha256 => {
                let mut data =
                    unsafe { uninit_array::<OPERATION_BUS_SHA256F_DATA_SIZE>().assume_init() };
                data[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                data[OPERATION_BUS_DATA_SIZE..].copy_from_slice(&ctx.precompiled.input_data);
                ExtOperationData::OperationSha256Data(data)
            }

            ZiskOperationType::ArithEq => match inst.op {
                ARITH256_OP => {
                    let mut data = unsafe {
                        uninit_array::<OPERATION_BUS_ARITH_256_DATA_SIZE>().assume_init()
                    };
                    data[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    data[OPERATION_BUS_DATA_SIZE..].copy_from_slice(&ctx.precompiled.input_data);
                    ExtOperationData::OperationArith256Data(data)
                }
                ARITH256_MOD_OP => {
                    let mut data = unsafe {
                        uninit_array::<OPERATION_BUS_ARITH_256_MOD_DATA_SIZE>().assume_init()
                    };
                    data[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    data[OPERATION_BUS_DATA_SIZE..].copy_from_slice(&ctx.precompiled.input_data);
                    ExtOperationData::OperationArith256ModData(data)
                }
                SECP256K1_ADD_OP => {
                    let mut data = unsafe {
                        uninit_array::<OPERATION_BUS_SECP256K1_ADD_DATA_SIZE>().assume_init()
                    };
                    data[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    data[OPERATION_BUS_DATA_SIZE..].copy_from_slice(&ctx.precompiled.input_data);
                    ExtOperationData::OperationSecp256k1AddData(data)
                }
                SECP256K1_DBL_OP => {
                    let mut data = unsafe {
                        uninit_array::<OPERATION_BUS_SECP256K1_DBL_DATA_SIZE>().assume_init()
                    };
                    data[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    data[OPERATION_BUS_DATA_SIZE..].copy_from_slice(&ctx.precompiled.input_data);
                    ExtOperationData::OperationSecp256k1DblData(data)
                }
                BN254_CURVE_ADD_OP => {
                    let mut data = unsafe {
                        uninit_array::<OPERATION_BUS_BN254_CURVE_ADD_DATA_SIZE>().assume_init()
                    };
                    data[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    data[OPERATION_BUS_DATA_SIZE..].copy_from_slice(&ctx.precompiled.input_data);
                    ExtOperationData::OperationBn254CurveAddData(data)
                }
                BN254_CURVE_DBL_OP => {
                    let mut data = unsafe {
                        uninit_array::<OPERATION_BUS_BN254_CURVE_DBL_DATA_SIZE>().assume_init()
                    };
                    data[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    data[OPERATION_BUS_DATA_SIZE..].copy_from_slice(&ctx.precompiled.input_data);
                    ExtOperationData::OperationBn254CurveDblData(data)
                }
                BN254_COMPLEX_ADD_OP => {
                    let mut data = unsafe {
                        uninit_array::<OPERATION_BUS_BN254_COMPLEX_ADD_DATA_SIZE>().assume_init()
                    };
                    data[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    data[OPERATION_BUS_DATA_SIZE..].copy_from_slice(&ctx.precompiled.input_data);
                    ExtOperationData::OperationBn254ComplexAddData(data)
                }
                BN254_COMPLEX_SUB_OP => {
                    let mut data = unsafe {
                        uninit_array::<OPERATION_BUS_BN254_COMPLEX_SUB_DATA_SIZE>().assume_init()
                    };
                    data[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    data[OPERATION_BUS_DATA_SIZE..].copy_from_slice(&ctx.precompiled.input_data);
                    ExtOperationData::OperationBn254ComplexSubData(data)
                }
                BN254_COMPLEX_MUL_OP => {
                    let mut data = unsafe {
                        uninit_array::<OPERATION_BUS_BN254_COMPLEX_MUL_DATA_SIZE>().assume_init()
                    };
                    data[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    data[OPERATION_BUS_DATA_SIZE..].copy_from_slice(&ctx.precompiled.input_data);
                    ExtOperationData::OperationBn254ComplexMulData(data)
                }
                _ => ExtOperationData::OperationData([op, op_type, a, b]),
            },

            ZiskOperationType::ArithEq384 => match inst.op {
                ARITH384_MOD_OP => {
                    let mut data = unsafe {
                        uninit_array::<OPERATION_BUS_ARITH_384_MOD_DATA_SIZE>().assume_init()
                    };
                    data[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    data[OPERATION_BUS_DATA_SIZE..].copy_from_slice(&ctx.precompiled.input_data);
                    ExtOperationData::OperationArith384ModData(data)
                }
                BLS12_381_CURVE_ADD_OP => {
                    let mut data = unsafe {
                        uninit_array::<OPERATION_BUS_BLS12_381_CURVE_ADD_DATA_SIZE>().assume_init()
                    };
                    data[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    data[OPERATION_BUS_DATA_SIZE..].copy_from_slice(&ctx.precompiled.input_data);
                    ExtOperationData::OperationBls12_381CurveAddData(data)
                }
                BLS12_381_CURVE_DBL_OP => {
                    let mut data = unsafe {
                        uninit_array::<OPERATION_BUS_BLS12_381_CURVE_DBL_DATA_SIZE>().assume_init()
                    };
                    data[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    data[OPERATION_BUS_DATA_SIZE..].copy_from_slice(&ctx.precompiled.input_data);
                    ExtOperationData::OperationBls12_381CurveDblData(data)
                }
                BLS12_381_COMPLEX_ADD_OP => {
                    let mut data = unsafe {
                        uninit_array::<OPERATION_BUS_BLS12_381_COMPLEX_ADD_DATA_SIZE>()
                            .assume_init()
                    };
                    data[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    data[OPERATION_BUS_DATA_SIZE..].copy_from_slice(&ctx.precompiled.input_data);
                    ExtOperationData::OperationBls12_381ComplexAddData(data)
                }
                BLS12_381_COMPLEX_SUB_OP => {
                    let mut data = unsafe {
                        uninit_array::<OPERATION_BUS_BLS12_381_COMPLEX_SUB_DATA_SIZE>()
                            .assume_init()
                    };
                    data[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    data[OPERATION_BUS_DATA_SIZE..].copy_from_slice(&ctx.precompiled.input_data);
                    ExtOperationData::OperationBls12_381ComplexSubData(data)
                }
                BLS12_381_COMPLEX_MUL_OP => {
                    let mut data = unsafe {
                        uninit_array::<OPERATION_BUS_BLS12_381_COMPLEX_MUL_DATA_SIZE>()
                            .assume_init()
                    };
                    data[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    data[OPERATION_BUS_DATA_SIZE..].copy_from_slice(&ctx.precompiled.input_data);
                    ExtOperationData::OperationBls12_381ComplexMulData(data)
                }
                _ => ExtOperationData::OperationData([op, op_type, a, b]),
            },
            ZiskOperationType::BigInt => match inst.op {
                ADD256_OP => {
                    let mut data =
                        unsafe { uninit_array::<OPERATION_BUS_ADD_256_DATA_SIZE>().assume_init() };
                    data[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    data[OPERATION_BUS_DATA_SIZE..].copy_from_slice(&ctx.precompiled.input_data);
                    ExtOperationData::OperationAdd256Data(data)
                }
                _ => ExtOperationData::OperationData([op, op_type, a, b]),
            },

            _ => ExtOperationData::OperationData([op, op_type, a, b]),
        }
    }

    #[inline(always)]
    pub fn write_instruction_payload<'a>(
        inst: &ZiskInst,
        ctx: &InstContext,
        buffer: &'a mut [u64; MAX_OPERATION_DATA_SIZE],
    ) -> &'a [u64] {
        let a = if inst.m32 { ctx.a & 0xffff_ffff } else { ctx.a };
        let b = if inst.m32 { ctx.b & 0xffff_ffff } else { ctx.b };
        let op = inst.op as u64;
        let op_type = inst.op_type as u64;

        match inst.op_type {
            ZiskOperationType::Keccak => {
                debug_assert_eq!(ctx.precompiled.input_data.len(), 25);
                buffer[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                buffer[OPERATION_BUS_DATA_SIZE..OPERATION_BUS_KECCAKF_DATA_SIZE]
                    .copy_from_slice(&ctx.precompiled.input_data);
                &buffer[..OPERATION_BUS_KECCAKF_DATA_SIZE]
            }

            ZiskOperationType::Sha256 => {
                debug_assert_eq!(ctx.precompiled.input_data.len(), 14);
                buffer[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                buffer[OPERATION_BUS_DATA_SIZE..OPERATION_BUS_SHA256F_DATA_SIZE]
                    .copy_from_slice(&ctx.precompiled.input_data);
                &buffer[..OPERATION_BUS_SHA256F_DATA_SIZE]
            }

            ZiskOperationType::ArithEq => match inst.op {
                ARITH256_OP => {
                    let len = OPERATION_BUS_DATA_SIZE + ctx.precompiled.input_data.len();
                    buffer[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    buffer[OPERATION_BUS_DATA_SIZE..len]
                        .copy_from_slice(&ctx.precompiled.input_data);
                    &buffer[..len]
                }
                ARITH256_MOD_OP => {
                    let len = OPERATION_BUS_DATA_SIZE + ctx.precompiled.input_data.len();
                    buffer[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    buffer[OPERATION_BUS_DATA_SIZE..len]
                        .copy_from_slice(&ctx.precompiled.input_data);
                    &buffer[..len]
                }
                SECP256K1_ADD_OP => {
                    let len = OPERATION_BUS_DATA_SIZE + ctx.precompiled.input_data.len();
                    buffer[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    buffer[OPERATION_BUS_DATA_SIZE..len]
                        .copy_from_slice(&ctx.precompiled.input_data);
                    &buffer[..len]
                }
                SECP256K1_DBL_OP => {
                    let len = OPERATION_BUS_DATA_SIZE + ctx.precompiled.input_data.len();
                    buffer[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    buffer[OPERATION_BUS_DATA_SIZE..len]
                        .copy_from_slice(&ctx.precompiled.input_data);
                    &buffer[..len]
                }
                BN254_CURVE_ADD_OP => {
                    let len = OPERATION_BUS_DATA_SIZE + ctx.precompiled.input_data.len();
                    buffer[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    buffer[OPERATION_BUS_DATA_SIZE..len]
                        .copy_from_slice(&ctx.precompiled.input_data);
                    &buffer[..len]
                }
                BN254_CURVE_DBL_OP => {
                    let len = OPERATION_BUS_DATA_SIZE + ctx.precompiled.input_data.len();
                    buffer[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    buffer[OPERATION_BUS_DATA_SIZE..len]
                        .copy_from_slice(&ctx.precompiled.input_data);
                    &buffer[..len]
                }
                BN254_COMPLEX_ADD_OP => {
                    let len = OPERATION_BUS_DATA_SIZE + ctx.precompiled.input_data.len();
                    buffer[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    buffer[OPERATION_BUS_DATA_SIZE..len]
                        .copy_from_slice(&ctx.precompiled.input_data);
                    &buffer[..len]
                }
                BN254_COMPLEX_SUB_OP => {
                    let len = OPERATION_BUS_DATA_SIZE + ctx.precompiled.input_data.len();
                    buffer[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    buffer[OPERATION_BUS_DATA_SIZE..len]
                        .copy_from_slice(&ctx.precompiled.input_data);
                    &buffer[..len]
                }
                BN254_COMPLEX_MUL_OP => {
                    let len = OPERATION_BUS_DATA_SIZE + ctx.precompiled.input_data.len();
                    buffer[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    buffer[OPERATION_BUS_DATA_SIZE..len]
                        .copy_from_slice(&ctx.precompiled.input_data);
                    &buffer[..len]
                }
                _ => {
                    buffer[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    &buffer[..OPERATION_BUS_DATA_SIZE]
                }
            },

            ZiskOperationType::ArithEq384 => match inst.op {
                ARITH384_MOD_OP => {
                    let len = OPERATION_BUS_DATA_SIZE + ctx.precompiled.input_data.len();
                    buffer[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    buffer[OPERATION_BUS_DATA_SIZE..len]
                        .copy_from_slice(&ctx.precompiled.input_data);
                    &buffer[..len]
                }
                BLS12_381_CURVE_ADD_OP => {
                    let len = OPERATION_BUS_DATA_SIZE + ctx.precompiled.input_data.len();
                    buffer[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    buffer[OPERATION_BUS_DATA_SIZE..len]
                        .copy_from_slice(&ctx.precompiled.input_data);
                    &buffer[..len]
                }
                BLS12_381_CURVE_DBL_OP => {
                    let len = OPERATION_BUS_DATA_SIZE + ctx.precompiled.input_data.len();
                    buffer[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    buffer[OPERATION_BUS_DATA_SIZE..len]
                        .copy_from_slice(&ctx.precompiled.input_data);
                    &buffer[..len]
                }
                BLS12_381_COMPLEX_ADD_OP => {
                    let len = OPERATION_BUS_DATA_SIZE + ctx.precompiled.input_data.len();
                    buffer[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    buffer[OPERATION_BUS_DATA_SIZE..len]
                        .copy_from_slice(&ctx.precompiled.input_data);
                    &buffer[..len]
                }
                BLS12_381_COMPLEX_SUB_OP => {
                    let len = OPERATION_BUS_DATA_SIZE + ctx.precompiled.input_data.len();
                    buffer[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    buffer[OPERATION_BUS_DATA_SIZE..len]
                        .copy_from_slice(&ctx.precompiled.input_data);
                    &buffer[..len]
                }
                BLS12_381_COMPLEX_MUL_OP => {
                    let len = OPERATION_BUS_DATA_SIZE + ctx.precompiled.input_data.len();
                    buffer[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    buffer[OPERATION_BUS_DATA_SIZE..len]
                        .copy_from_slice(&ctx.precompiled.input_data);
                    &buffer[..len]
                }
                _ => {
                    buffer[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    &buffer[..OPERATION_BUS_DATA_SIZE]
                }
            },
            ZiskOperationType::BigInt => match inst.op {
                ADD256_OP => {
                    let len = OPERATION_BUS_DATA_SIZE + ctx.precompiled.input_data.len();
                    buffer[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    buffer[OPERATION_BUS_DATA_SIZE..len]
                        .copy_from_slice(&ctx.precompiled.input_data);
                    &buffer[..len]
                }
                _ => {
                    buffer[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                    &buffer[..OPERATION_BUS_DATA_SIZE]
                }
            },

            _ => {
                buffer[0..OPERATION_BUS_DATA_SIZE].copy_from_slice(&[op, op_type, a, b]);
                &buffer[..OPERATION_BUS_DATA_SIZE]
            }
        }
    }

    /// Retrieves the operation code from operation data.
    ///
    /// # Arguments
    /// * `data` - A reference to the operation data payload.
    ///
    /// # Returns
    /// The operation code as a `u8`.
    #[inline(always)]
    pub fn get_op(data: &ExtOperationData<u64>) -> u8 {
        match data {
            ExtOperationData::OperationData(d) => d[OP] as u8,
            ExtOperationData::OperationKeccakData(d) => d[OP] as u8,
            ExtOperationData::OperationSha256Data(d) => d[OP] as u8,
            ExtOperationData::OperationArith256Data(d) => d[OP] as u8,
            ExtOperationData::OperationArith256ModData(d) => d[OP] as u8,
            ExtOperationData::OperationSecp256k1AddData(d) => d[OP] as u8,
            ExtOperationData::OperationSecp256k1DblData(d) => d[OP] as u8,
            ExtOperationData::OperationBn254CurveAddData(d) => d[OP] as u8,
            ExtOperationData::OperationBn254CurveDblData(d) => d[OP] as u8,
            ExtOperationData::OperationBn254ComplexAddData(d) => d[OP] as u8,
            ExtOperationData::OperationBn254ComplexSubData(d) => d[OP] as u8,
            ExtOperationData::OperationBn254ComplexMulData(d) => d[OP] as u8,
            ExtOperationData::OperationArith384ModData(d) => d[OP] as u8,
            ExtOperationData::OperationBls12_381CurveAddData(d) => d[OP] as u8,
            ExtOperationData::OperationBls12_381CurveDblData(d) => d[OP] as u8,
            ExtOperationData::OperationBls12_381ComplexAddData(d) => d[OP] as u8,
            ExtOperationData::OperationBls12_381ComplexSubData(d) => d[OP] as u8,
            ExtOperationData::OperationBls12_381ComplexMulData(d) => d[OP] as u8,
            ExtOperationData::OperationAdd256Data(d) => d[OP] as u8,
        }
    }

    /// Retrieves the operation type from operation data.
    ///
    /// # Arguments
    /// * `data` - A reference to the operation data payload.
    ///
    /// # Returns
    /// The operation type as a `PayloadType`.
    #[inline(always)]
    pub fn get_op_type(data: &ExtOperationData<u64>) -> PayloadType {
        match data {
            ExtOperationData::OperationData(d) => d[OP_TYPE],
            ExtOperationData::OperationKeccakData(d) => d[OP_TYPE],
            ExtOperationData::OperationSha256Data(d) => d[OP_TYPE],
            ExtOperationData::OperationArith256Data(d) => d[OP_TYPE],
            ExtOperationData::OperationArith256ModData(d) => d[OP_TYPE],
            ExtOperationData::OperationSecp256k1AddData(d) => d[OP_TYPE],
            ExtOperationData::OperationSecp256k1DblData(d) => d[OP_TYPE],
            ExtOperationData::OperationBn254CurveAddData(d) => d[OP_TYPE],
            ExtOperationData::OperationBn254CurveDblData(d) => d[OP_TYPE],
            ExtOperationData::OperationBn254ComplexAddData(d) => d[OP_TYPE],
            ExtOperationData::OperationBn254ComplexSubData(d) => d[OP_TYPE],
            ExtOperationData::OperationBn254ComplexMulData(d) => d[OP_TYPE],
            ExtOperationData::OperationArith384ModData(d) => d[OP_TYPE],
            ExtOperationData::OperationBls12_381CurveAddData(d) => d[OP_TYPE],
            ExtOperationData::OperationBls12_381CurveDblData(d) => d[OP_TYPE],
            ExtOperationData::OperationBls12_381ComplexAddData(d) => d[OP_TYPE],
            ExtOperationData::OperationBls12_381ComplexSubData(d) => d[OP_TYPE],
            ExtOperationData::OperationBls12_381ComplexMulData(d) => d[OP_TYPE],
            ExtOperationData::OperationAdd256Data(d) => d[OP_TYPE],
        }
    }

    /// Retrieves the `a` parameter from operation data.
    ///
    /// # Arguments
    /// * `data` - A reference to the operation data payload.
    ///
    /// # Returns
    /// The `a` parameter as a `PayloadType`.
    #[inline(always)]
    pub fn get_a(data: &ExtOperationData<u64>) -> PayloadType {
        match data {
            ExtOperationData::OperationData(d) => d[A],
            ExtOperationData::OperationKeccakData(d) => d[A],
            ExtOperationData::OperationSha256Data(d) => d[A],
            ExtOperationData::OperationArith256Data(d) => d[A],
            ExtOperationData::OperationArith256ModData(d) => d[A],
            ExtOperationData::OperationSecp256k1AddData(d) => d[A],
            ExtOperationData::OperationSecp256k1DblData(d) => d[A],
            ExtOperationData::OperationBn254CurveAddData(d) => d[A],
            ExtOperationData::OperationBn254CurveDblData(d) => d[A],
            ExtOperationData::OperationBn254ComplexAddData(d) => d[A],
            ExtOperationData::OperationBn254ComplexSubData(d) => d[A],
            ExtOperationData::OperationBn254ComplexMulData(d) => d[A],
            ExtOperationData::OperationArith384ModData(d) => d[A],
            ExtOperationData::OperationBls12_381CurveAddData(d) => d[A],
            ExtOperationData::OperationBls12_381CurveDblData(d) => d[A],
            ExtOperationData::OperationBls12_381ComplexAddData(d) => d[A],
            ExtOperationData::OperationBls12_381ComplexSubData(d) => d[A],
            ExtOperationData::OperationBls12_381ComplexMulData(d) => d[A],
            ExtOperationData::OperationAdd256Data(d) => d[A],
        }
    }

    /// Retrieves the `b` parameter from operation data.
    ///
    /// # Arguments
    /// * `data` - A reference to the operation data payload.
    ///
    /// # Returns
    /// The `b` parameter as a `PayloadType`.
    #[inline(always)]
    pub fn get_b(data: &ExtOperationData<u64>) -> PayloadType {
        match data {
            ExtOperationData::OperationData(d) => d[B],
            ExtOperationData::OperationKeccakData(d) => d[B],
            ExtOperationData::OperationSha256Data(d) => d[B],
            ExtOperationData::OperationArith256Data(d) => d[B],
            ExtOperationData::OperationArith256ModData(d) => d[B],
            ExtOperationData::OperationSecp256k1AddData(d) => d[B],
            ExtOperationData::OperationSecp256k1DblData(d) => d[B],
            ExtOperationData::OperationBn254CurveAddData(d) => d[B],
            ExtOperationData::OperationBn254CurveDblData(d) => d[B],
            ExtOperationData::OperationBn254ComplexAddData(d) => d[B],
            ExtOperationData::OperationBn254ComplexSubData(d) => d[B],
            ExtOperationData::OperationBn254ComplexMulData(d) => d[B],
            ExtOperationData::OperationArith384ModData(d) => d[B],
            ExtOperationData::OperationBls12_381CurveAddData(d) => d[B],
            ExtOperationData::OperationBls12_381CurveDblData(d) => d[B],
            ExtOperationData::OperationBls12_381ComplexAddData(d) => d[B],
            ExtOperationData::OperationBls12_381ComplexSubData(d) => d[B],
            ExtOperationData::OperationBls12_381ComplexMulData(d) => d[B],
            ExtOperationData::OperationAdd256Data(d) => d[B],
        }
    }
}
