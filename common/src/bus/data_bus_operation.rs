//! The `OperationBusData` module facilitates the handling and transformation of operation-related
//! data for communication over the operation bus. This includes data extraction from instructions
//! and managing the format of operation data.

use crate::{BusId, PayloadType};
use zisk_core::zisk_ops::ZiskOp;
use zisk_core::{InstContext, ZiskInst, ZiskOperationType};

/// The unique bus ID for operation-related data communication.
pub const OPERATION_BUS_ID: BusId = BusId(0);

/// The size of the operation data payload.
pub const OPERATION_BUS_DATA_SIZE: usize = 4;

// worst case: 4 x 256 + 2 addr = 4 * 4 + 2 = 18 (secp256k1_add, arith_256_mod)
// arith_256: 3 x 256 + 2 addr = 3 * 4 + 2 = 14
// secp256k1_dbl: 2 x 256 + 1 addr = 2 * 4 + 1 = 9
// TODO: optimize and send only one value 64 upto 32-bits addr

const DATA_256_BITS_SIZE: usize = 4;
const POINT_256_BITS_SIZE: usize = 2 * DATA_256_BITS_SIZE;
const INDIRECTION_SIZE: usize = 1;

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

pub enum ExtOperationData<D> {
    OperationData(OperationData<D>),
    OperationKeccakData(OperationKeccakData<D>),
    OperationSha256Data(OperationSha256Data<D>),
    OperationArith256Data(OperationArith256Data<D>),
    OperationArith256ModData(OperationArith256ModData<D>),
    OperationSecp256k1AddData(OperationSecp256k1AddData<D>),
    OperationSecp256k1DblData(OperationSecp256k1DblData<D>),
}

const KECCAK_OP: u8 = ZiskOp::Keccak.code();
const SHA256_OP: u8 = ZiskOp::Sha256.code();
const ARITH256_OP: u8 = ZiskOp::Arith256.code();
const ARITH256_MOD_OP: u8 = ZiskOp::Arith256Mod.code();
const SECP256K1_ADD_OP: u8 = ZiskOp::Secp256k1Add.code();
const SECP256K1_DBL_OP: u8 = ZiskOp::Secp256k1Dbl.code();

// impl<D: Copy + Into<u8>> TryFrom<&[D]> for ExtOperationData<D> {
impl<D: Copy + Into<u64>> TryFrom<&[D]> for ExtOperationData<D> {
    type Error = &'static str;

    fn try_from(data: &[D]) -> Result<Self, Self::Error> {
        if data.len() < 4 {
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
    pub fn from_values(op: u8, op_type: PayloadType, a: u64, b: u64) -> OperationData<u64> {
        [op as u64, op_type, a, b]
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
    pub fn from_instruction(inst: &ZiskInst, inst_ctx: &InstContext) -> ExtOperationData<u64> {
        let a = if inst.m32 { inst_ctx.a & 0xffffffff } else { inst_ctx.a };
        let b = if inst.m32 { inst_ctx.b & 0xffffffff } else { inst_ctx.b };

        match inst.op_type {
            ZiskOperationType::Keccak => {
                let mut data: OperationKeccakData<u64> = [0; OPERATION_BUS_KECCAKF_DATA_SIZE];
                data[0] = inst.op as u64; // OP
                data[1] = inst.op_type as u64; // OP_TYPE
                data[2] = a; // A
                data[3] = b; // B
                data[4..].copy_from_slice(&inst_ctx.precompiled.input_data);
                ExtOperationData::OperationKeccakData(data)
            }
            ZiskOperationType::Sha256 => {
                let mut data: OperationSha256Data<u64> = [0; OPERATION_BUS_SHA256F_DATA_SIZE];
                data[0] = inst.op as u64; // OP
                data[1] = inst.op_type as u64; // OP_TYPE
                data[2] = a; // A
                data[3] = b; // B
                data[4..].copy_from_slice(&inst_ctx.precompiled.input_data);
                ExtOperationData::OperationSha256Data(data)
            }
            ZiskOperationType::ArithEq => {
                match inst.op {
                    ARITH256_OP => {
                        let mut data: OperationArith256Data<u64> =
                            [0; OPERATION_BUS_ARITH_256_DATA_SIZE];
                        data[0] = inst.op as u64; // OP
                        data[1] = inst.op_type as u64; // OP_TYPE
                        data[2] = a; // A step
                        data[3] = b; // B addr
                        data[4..].copy_from_slice(&inst_ctx.precompiled.input_data);
                        ExtOperationData::OperationArith256Data(data)
                    }
                    ARITH256_MOD_OP => {
                        let mut data: OperationArith256ModData<u64> =
                            [0; OPERATION_BUS_ARITH_256_MOD_DATA_SIZE];
                        data[0] = inst.op as u64; // OP
                        data[1] = inst.op_type as u64; // OP_TYPE
                        data[2] = a; // A step
                        data[3] = b; // B addr
                        data[4..].copy_from_slice(&inst_ctx.precompiled.input_data);
                        ExtOperationData::OperationArith256ModData(data)
                    }
                    SECP256K1_ADD_OP => {
                        let mut data: OperationSecp256k1AddData<u64> =
                            [0; OPERATION_BUS_SECP256K1_ADD_DATA_SIZE];
                        data[0] = inst.op as u64; // OP
                        data[1] = inst.op_type as u64; // OP_TYPE
                        data[2] = a; // A step
                        data[3] = b; // B addr
                        data[4..].copy_from_slice(&inst_ctx.precompiled.input_data);
                        ExtOperationData::OperationSecp256k1AddData(data)
                    }
                    SECP256K1_DBL_OP => {
                        let mut data: OperationSecp256k1DblData<u64> =
                            [0; OPERATION_BUS_SECP256K1_DBL_DATA_SIZE];
                        data[0] = inst.op as u64; // OP
                        data[1] = inst.op_type as u64; // OP_TYPE
                        data[2] = a; // A step
                        data[3] = b; // B addr
                        data[4..].copy_from_slice(&inst_ctx.precompiled.input_data);
                        ExtOperationData::OperationSecp256k1DblData(data)
                    }
                    _ => {
                        ExtOperationData::OperationData([
                            inst.op as u64,      // OP
                            inst.op_type as u64, // OP_TYPE
                            a,                   // A
                            b,                   // B
                        ])
                    }
                }
            }
            _ => {
                ExtOperationData::OperationData([
                    inst.op as u64,      // OP
                    inst.op_type as u64, // OP_TYPE
                    a,                   // A
                    b,                   // B
                ])
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
        }
    }

    /// Retrieves the extra data from operation data.
    ///
    /// # Arguments
    /// * `data` - A reference to the operation data payload.
    ///
    /// # Returns
    /// The extra data as a `Vec<PayloadType>`.
    #[inline(always)]
    pub fn get_extra_data(data: &ExtOperationData<u64>) -> Vec<PayloadType> {
        match data {
            ExtOperationData::OperationKeccakData(d) => d[4..].to_vec(),
            ExtOperationData::OperationSha256Data(d) => d[4..].to_vec(),
            ExtOperationData::OperationArith256Data(d) => d[4..].to_vec(),
            ExtOperationData::OperationArith256ModData(d) => d[4..].to_vec(),
            ExtOperationData::OperationSecp256k1AddData(d) => d[4..].to_vec(),
            ExtOperationData::OperationSecp256k1DblData(d) => d[4..].to_vec(),
            _ => vec![],
        }
    }
}
