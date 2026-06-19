use crate::BusId;

use super::PayloadType;

/// The `MEM_BUS_ID` constant defines the unique identifier for the memory bus.
pub const MEM_BUS_ID: BusId = BusId(2);

/// The `MEM_BUS_DATA_SIZE` constant specifies the size of the data payload for memory bus operations.
pub const MEM_BUS_DATA_SIZE: usize = 7;

const OP: usize = 0;
const ADDR: usize = 1;
const STEP: usize = 2;
const BYTES: usize = 3;
const MEM_VALUE_0: usize = 4;
const MEM_VALUE_1: usize = 5;
const VALUE: usize = 6;

/// Type representing a memory data payload consisting of four `PayloadType` values.
pub type MemData = [PayloadType; 4];

/// The `MemBusData` struct provides methods for accessing and manipulating memory bus data.
pub struct MemBusData;

impl MemBusData {
    /// Retrieves the address from the memory bus data.
    #[inline(always)]
    pub fn get_addr(data: &[u64]) -> u32 {
        data[ADDR] as u32
    }

    /// Retrieves the operation code from the memory bus data.
    #[inline(always)]
    pub fn get_op(data: &[u64]) -> u8 {
        data[OP] as u8
    }

    /// Retrieves the step value from the memory bus data.
    #[inline(always)]
    pub fn get_step(data: &[u64]) -> u64 {
        data[STEP]
    }

    /// Retrieves the byte count from the memory bus data.
    #[inline(always)]
    pub fn get_bytes(data: &[u64]) -> u8 {
        data[BYTES] as u8
    }

    /// Retrieves the memory value from the memory bus data.
    #[inline(always)]
    pub fn get_value(data: &[u64]) -> u64 {
        data[VALUE]
    }

    /// Retrieves the memory values from the memory bus data as an array of two `u64` values.
    #[inline(always)]
    pub fn get_mem_values(data: &[u64]) -> [u64; 2] {
        [data[MEM_VALUE_0], data[MEM_VALUE_1]]
    }
}
