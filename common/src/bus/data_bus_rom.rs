//! The `RomBusData` module provides functionality for creating and managing ROM-related data
//! for communication over the ROM bus. This includes extracting relevant details from instructions
//! and formatting them for use with the ROM bus.

use crate::{BusId, PayloadType};
use zisk_core::{InstContext, ZiskInst};

/// The unique bus ID for ROM-related data communication.
pub const ROM_BUS_ID: BusId = BusId(1);

/// The size of the ROM data payload.
pub const ROM_BUS_DATA_SIZE: usize = 3;

/// Index of the step value in the ROM data payload.
const STEP: usize = 0;

/// Index of the program counter (PC) value in the ROM data payload.
const PC: usize = 1;

/// Index of the end flag in the ROM data payload.
const END: usize = 2;

/// Type alias for ROM data payload.
pub type RomData<D> = [D; ROM_BUS_DATA_SIZE];

/// Provides utility functions for creating and interacting with ROM bus data.
///
/// This struct is implemented as a zero-sized type with a `PhantomData` marker to enable
/// type-specific functionality for `u64` ROM data.
pub struct RomBusData<D>(std::marker::PhantomData<D>);

impl RomBusData<u64> {
    /// Creates ROM data from a `ZiskInst` instruction and its context.
    ///
    /// # Arguments
    /// * `instruction` - A reference to the `ZiskInst` representing the instruction.
    /// * `inst_ctx` - A reference to the instruction context containing metadata for the
    ///   instruction.
    ///
    /// # Returns
    /// An array representing the ROM data payload.
    #[inline(always)]
    pub fn from_instruction(instruction: &ZiskInst, inst_ctx: &InstContext) -> RomData<u64> {
        [
            inst_ctx.step,          // STEP
            inst_ctx.pc,            // PC
            instruction.end as u64, // END
        ]
    }

    /// Retrieves the step value from ROM data.
    ///
    /// # Arguments
    /// * `data` - A reference to the ROM data payload.
    ///
    /// # Returns
    /// The step value as a `PayloadType`.
    #[inline(always)]
    pub fn get_step(data: &RomData<u64>) -> PayloadType {
        data[STEP]
    }

    /// Retrieves the program counter (PC) value from ROM data.
    ///
    /// # Arguments
    /// * `data` - A reference to the ROM data payload.
    ///
    /// # Returns
    /// The PC value as a `PayloadType`.
    #[inline(always)]
    pub fn get_pc(data: &RomData<u64>) -> PayloadType {
        data[PC]
    }

    /// Retrieves the end flag from ROM data.
    ///
    /// # Arguments
    /// * `data` - A reference to the ROM data payload.
    ///
    /// # Returns
    /// The end flag as a `PayloadType`.
    #[inline(always)]
    pub fn get_end(data: &RomData<u64>) -> PayloadType {
        data[END]
    }
}
