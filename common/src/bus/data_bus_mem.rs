use crate::BusId;

use super::PayloadType;

pub const MEM_BUS_ID: BusId = BusId(2);

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

pub struct MemBusData;

impl MemBusData {
    #[inline(always)]
    pub fn get_addr(data: &[u64]) -> u32 {
        data[ADDR] as u32
    }

    #[inline(always)]
    pub fn get_op(data: &[u64]) -> u8 {
        data[OP] as u8
    }

    #[inline(always)]
    pub fn get_step(data: &[u64]) -> u64 {
        data[STEP]
    }

    #[inline(always)]
    pub fn get_bytes(data: &[u64]) -> u8 {
        data[BYTES] as u8
    }

    #[inline(always)]
    pub fn get_value(data: &[u64]) -> u64 {
        data[VALUE]
    }

    #[inline(always)]
    pub fn get_mem_values(data: &[u64]) -> [u64; 2] {
        [data[MEM_VALUE_0], data[MEM_VALUE_1]]
    }
}

pub struct MemCollectorInfo {
    pub from_addr: u32,
    pub to_addr: u32,
}

impl MemCollectorInfo {
    pub fn skip_addr(&self, addr: u32) -> bool {
        if addr > self.to_addr || addr < self.from_addr {
            return true;
        }
        false
    }
}
