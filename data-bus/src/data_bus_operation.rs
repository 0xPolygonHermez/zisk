//! The `OperationBusData` module facilitates the handling and transformation of operation-related
//! data for communication over the operation bus. This includes data extraction from instructions
//! and managing the format of operation data.

use crate::PayloadType;
use zisk_core::{InstContext, ZiskInst, ZiskOperationType};

/// The unique bus ID for operation-related data communication.
pub const OPERATION_BUS_ID: u16 = 5000;

/// The size of the operation data payload.
pub const OPERATION_BUS_DATA_SIZE: usize = 5;

/// Index of the step value in the operation data payload.
const STEP: usize = 0;

/// Index of the operation value in the operation data payload.
const OP: usize = 1;

/// Index of the operation type in the operation data payload.
const OP_TYPE: usize = 2;

/// Index of the `a` value in the operation data payload.
const A: usize = 3;

/// Index of the `b` value in the operation data payload.
const B: usize = 4;

/// Type alias for operation data payload.
pub type OperationData<D> = [D; OPERATION_BUS_DATA_SIZE];

/// Type alias for Keccak operation data payload.
pub type OperationKeccakData<D> = [D; OPERATION_BUS_DATA_SIZE + 25];

pub enum ExtOperationData<D> {
    OperationData(OperationData<D>),
    OperationKeccakData(OperationKeccakData<D>),
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
        step: u64,
        op: u8,
        op_type: PayloadType,
        a: u64,
        b: u64,
    ) -> OperationData<u64> {
        [step, op as u64, op_type, a, b]
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

        if inst.op_type == ZiskOperationType::Keccak {
            assert!(inst_ctx.precompiled.input_data.len() == 25);
            let mut data: OperationKeccakData<u64> = [0; OPERATION_BUS_DATA_SIZE + 25];
            data[0] = inst_ctx.step; // STEP
            data[1] = inst.op as u64; // OP
            data[2] = inst.op_type as u64; // OP_TYPE
            data[3] = a; // A
            data[4] = b; // B
            data[5..(5 + 25)].copy_from_slice(&inst_ctx.precompiled.input_data[..25]);
            ExtOperationData::OperationKeccakData(data)
        } else {
            ExtOperationData::OperationData([
                inst_ctx.step,       // STEP
                inst.op as u64,      // OP
                inst.op_type as u64, // OP_TYPE
                a,                   // A
                b,                   // B
            ])
        }
    }

    /// Retrieves the step value from operation data.
    ///
    /// # Arguments
    /// * `data` - A reference to the operation data payload.
    ///
    /// # Returns
    /// The step value as a `PayloadType`.
    #[inline(always)]
    pub fn get_step(data: &ExtOperationData<u64>) -> PayloadType {
        match data {
            ExtOperationData::OperationData(d) => d[STEP],
            ExtOperationData::OperationKeccakData(d) => d[STEP],
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
        }
    }
}
