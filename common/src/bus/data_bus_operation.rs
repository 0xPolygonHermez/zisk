//! The `OperationBusData` module facilitates the handling and transformation of operation-related
//! data for communication over the operation bus. This includes data extraction from instructions
//! and managing the format of operation data.

use crate::{uninit_array, BusId, PayloadType};
use std::collections::VecDeque;
use zisk_core::{InstContext, ZiskInst, ZiskOperationType};

/// The unique bus ID for operation-related data communication.
pub const OPERATION_BUS_ID: BusId = BusId(0);
/// The size of the operation data payload.
pub const OPERATION_BUS_DATA_SIZE: usize = 4; // op,op_type,a,b
/// The size of the precompiled operation data payload, which includes an additional `step` parameter.
pub const OPERATION_PRECOMPILED_BUS_DATA_SIZE: usize = 5; // op,op_type,a,b, step

/// DMA operation data size for memory copy operations.
pub const DMA_ENCODED: usize = OPERATION_PRECOMPILED_BUS_DATA_SIZE;
/// DMA operation data size for memory comparison operations.
pub const DMA_MEMCMP_COUNT_BUS: usize = OPERATION_PRECOMPILED_BUS_DATA_SIZE + 1;
pub const MAX_OPERATION_DATA_SIZE: usize = OPERATION_PRECOMPILED_BUS_DATA_SIZE + 35;

/// Index of the operation value in the operation data payload.
pub const OP: usize = 0;

/// Index of the operation type in the operation data payload.
pub const OP_TYPE: usize = 1;

/// Index of the `a` value in the operation data payload.
pub const A: usize = 2;

/// Index of the `b` value in the operation data payload.
pub const B: usize = 3;

/// Index of the `STEP` value in the operation data payload (only for precompiled operations).
pub const STEP: usize = 4;

/// Type alias for operation data payload.
pub type OperationData<D> = [D; OPERATION_BUS_DATA_SIZE];

/// A precompiled operation payload: a 5-word header (`op, op_type, a, b, step`)
/// followed by the op's input data, stored in a fixed-capacity buffer. Only
/// `data[..len]` is meaningful. This single representation replaces the former
/// per-op array variants — the op is read from `data[OP]` and the length is
/// carried at runtime, so the bus is agnostic to which precompile produced it.
pub struct PrecompiledData<D> {
    /// Number of meaningful words in `data`.
    pub len: usize,
    /// Header + input data, backed by a buffer of the max precompiled payload size.
    pub data: [D; MAX_OPERATION_DATA_SIZE],
}

/// The `ExtOperationData` enum encapsulates the operation data transmitted over the operation bus.
pub enum ExtOperationData<D> {
    /// Generic (non-precompiled) operation: `[op, op_type, a, b]`.
    OperationData(OperationData<D>),
    /// Any precompiled operation: 5-word header + input data (see [`PrecompiledData`]).
    Precompiled(PrecompiledData<D>),
}

impl<D> ExtOperationData<D> {
    /// Returns the raw operation data payload as a slice, independent of the
    /// variant. Every `get_*` accessor reads a fixed index into this slice,
    /// so they share this single match instead of one per accessor.
    #[inline(always)]
    pub fn payload(&self) -> &[D] {
        match self {
            ExtOperationData::OperationData(d) => d,
            ExtOperationData::Precompiled(p) => &p.data[..p.len],
        }
    }
}

/// Decodes a raw operation-bus payload into a precompile's input record.
///
/// Implemented once per precompile input type. A mono-op input narrows the
/// `&[u64]` payload to its fixed width and builds itself; a multi-op aggregate
/// reads the op at `payload[OP]` and narrows to the matching sub-input's width.
/// The narrowing (`payload.try_into()` to `&[u64; N]`) restores the compile-time
/// width guarantee inside each decoder and fails fast with a clear message on a
/// mismatched payload. The generated precompile collector calls this uniformly.
pub trait FromBusPayload {
    fn from_bus_payload(payload: &[u64]) -> Self;
}

// impl<D: Copy + Into<u8>> TryFrom<&[D]> for ExtOperationData<D> {
impl<D: Copy + Into<u64>> TryFrom<&[D]> for ExtOperationData<D> {
    type Error = &'static str;

    fn try_from(data: &[D]) -> Result<Self, Self::Error> {
        let len = data.len();
        if len < OPERATION_BUS_DATA_SIZE {
            return Err("Invalid data length");
        }
        // A generic (non-precompiled) op is exactly [op, op_type, a, b]; anything
        // longer is a precompiled op: a 5-word header followed by input data.
        // The op-specific length is carried by the slice itself, so no per-op arm
        // is needed — the op code stays available at data[OP].
        if len == OPERATION_BUS_DATA_SIZE {
            let array: OperationData<D> =
                data.try_into().map_err(|_| "Invalid OperationData size")?;
            return Ok(ExtOperationData::OperationData(array));
        }
        if len > MAX_OPERATION_DATA_SIZE {
            return Err("Precompiled operation data exceeds maximum size");
        }
        let mut buf = [data[0]; MAX_OPERATION_DATA_SIZE];
        buf[..len].copy_from_slice(data);
        Ok(ExtOperationData::Precompiled(PrecompiledData { len, data: buf }))
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
        pending: &mut VecDeque<(BusId, Vec<u64>, Vec<u64>)>,
    ) {
        pending.push_back((OPERATION_BUS_ID, vec![op as u64, op_type, a, b], Vec::new()));
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
        let step = ctx.step;

        // Precompiles emit a 5-word header + input data; the op-specific length is
        // carried by `ctx.precompiled.input_data`, so a single branch serves them all
        // (the op stays available at data[OP]). The op_type set restricts this to the
        // precompile families and the `input_size` guard preserves the original behavior
        // where an input-less family op falls back to the generic OperationData payload.
        match inst.op_type {
            ZiskOperationType::Keccak
            | ZiskOperationType::Sha256
            | ZiskOperationType::Poseidon
            | ZiskOperationType::Blake2
            | ZiskOperationType::ArithEq
            | ZiskOperationType::ArithEq384
            | ZiskOperationType::BigInt
            | ZiskOperationType::Dma
                if inst.input_size > 0 =>
            {
                let len = OPERATION_PRECOMPILED_BUS_DATA_SIZE + ctx.precompiled.input_data.len();
                let mut data = unsafe { uninit_array::<MAX_OPERATION_DATA_SIZE>().assume_init() };
                data[0..OPERATION_PRECOMPILED_BUS_DATA_SIZE]
                    .copy_from_slice(&[op, op_type, a, b, step]);
                data[OPERATION_PRECOMPILED_BUS_DATA_SIZE..len]
                    .copy_from_slice(&ctx.precompiled.input_data);
                ExtOperationData::Precompiled(PrecompiledData { len, data })
            }

            _ => ExtOperationData::OperationData([op, op_type, a, b]),
        }
    }

    /// Writes the operation instruction payload into a provided buffer.
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
        let step = ctx.step;

        match inst.op_type {
            // All precompiles emit [5-word header + input_data]; the length is carried by
            // input_data at runtime, so a single branch serves them all. The guard preserves
            // the original behavior where a family op without input data falls through to the
            // generic OperationData payload.
            ZiskOperationType::Keccak
            | ZiskOperationType::Sha256
            | ZiskOperationType::Poseidon
            | ZiskOperationType::Blake2
            | ZiskOperationType::ArithEq
            | ZiskOperationType::ArithEq384
            | ZiskOperationType::BigInt
            | ZiskOperationType::Dma
                if inst.input_size > 0 =>
            {
                let len = OPERATION_PRECOMPILED_BUS_DATA_SIZE + ctx.precompiled.input_data.len();
                buffer[0..OPERATION_PRECOMPILED_BUS_DATA_SIZE]
                    .copy_from_slice(&[op, op_type, a, b, step]);
                buffer[OPERATION_PRECOMPILED_BUS_DATA_SIZE..len]
                    .copy_from_slice(&ctx.precompiled.input_data);
                &buffer[..len]
            }

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
        data.payload()[OP] as u8
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
        data.payload()[OP_TYPE]
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
        data.payload()[A]
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
        data.payload()[B]
    }
}
