use std::collections::BTreeMap;

use zisk_core::{INPUT_ADDR, MAX_INPUT_SIZE, RAM_ADDR, RAM_SIZE, ROM_ADDR, ROM_ADDR_MAX};

/// Keeps counters for every type of memory operation (including registers).
///
/// Since RISC-V registers are mapped to memory, memory operations include register access
/// operations.
use crate::emu_costs::{
    MEM_READ_BYTE_COST, MEM_READ_COST, MEM_READ_UNALIGNED_1_COST, MEM_READ_UNALIGNED_2_COST,
    MEM_WRITE_BYTE_COST, MEM_WRITE_COST,
};

#[derive(Default, Debug, Clone)]
pub struct MemoryZoneStatsData {
    /// Counter of reads from aligned memory addresses
    mread_a: u64,
    /// Counter of writes to aligned memory addresses
    mwrite_a: u64,
    /// Counter of reads from non-aligned memory addresses (1)
    mread_na1: u64,
    /// Counter of writes to non-aligned memory addresses (1)
    mwrite_na1: u64,
    /// Counter of reads from non-aligned memory addresses (2)
    mread_na2: u64,
    /// Counter of writes to non-aligned memory addresses (2)
    mwrite_na2: u64,
    /// Counter of byte reads
    mread_byte: u64,
    /// Counter of byte writes where value was a byte (value & 0xFFFF_FFFF_FFFF_FF00 == 0)
    mwrite_byte: u64,
    max_addr_read: u64,
    max_addr_write: u64,
}

#[derive(Default, Debug, Clone)]
pub struct MemoryOperationsStats {
    rom: MemoryZoneStatsData,
    ram: MemoryZoneStatsData,
    input: MemoryZoneStatsData,

    mwrite_dirty_byte: u64,
    mwrite_dirty_s64_byte: u64,
    mwrite_dirty_s32_byte: u64,
    mwrite_dirty_s16_byte: u64,

    full: bool,
    pages: BTreeMap<u64, MemoryZoneStatsData>,
}

impl MemoryOperationsStats {
    /// Creates a new MemoryOperations structure with all counters set to zero.
    pub fn new() -> Self {
        Self::default()
    }
    pub fn enable_full_stats(&mut self) {
        self.full = true;
    }
    pub fn memory_write(&mut self, address: u64, width: u64, value: u64) {
        if (RAM_ADDR..(RAM_ADDR + RAM_SIZE)).contains(&address) {
            self.ram.memory_write(address, width, value);
            self.pages.entry(address >> 24).or_default().memory_write(address, width, value);
        } else if (ROM_ADDR..=ROM_ADDR_MAX).contains(&address) {
            self.rom.memory_write(address, width, value);
        } else if (INPUT_ADDR..(INPUT_ADDR + MAX_INPUT_SIZE)).contains(&address) {
            self.input.memory_write(address, width, value);
        }
        if width == 1 && (value & 0xFFFF_FFFF_FFFF_FF00) != 0 {
            self.mwrite_dirty_byte += 1;
            if (value & 0xFFFF_FFFF_FFFF_FF00) != 0xFFFF_FFFF_FFFF_FF00 {
                self.mwrite_dirty_s64_byte += 1;
            } else if (value & 0xFFFF_FFFF_FFFF_FF00) != 0xFFFF_FF00 {
                self.mwrite_dirty_s32_byte += 1;
            } else if (value & 0xFFFF_FFFF_FFFF_FF00) != 0xFF00 {
                self.mwrite_dirty_s16_byte += 1;
            }
        }
    }
    pub fn memory_read(&mut self, address: u64, width: u64) {
        if (RAM_ADDR..(RAM_ADDR + RAM_SIZE)).contains(&address) {
            self.ram.memory_read(address, width);
            self.pages.entry(address >> 24).or_default().memory_read(address, width);
        } else if (ROM_ADDR..=ROM_ADDR_MAX).contains(&address) {
            self.rom.memory_read(address, width);
        } else if (INPUT_ADDR..=(INPUT_ADDR + MAX_INPUT_SIZE)).contains(&address) {
            self.input.memory_read(address, width);
        }
    }
    pub fn get_cost(&self) -> u64 {
        self.rom.get_cost() + self.ram.get_cost() + self.input.get_cost()
    }
    pub fn get_max_ram_address(&self) -> u64 {
        self.ram.max_addr_read.max(self.ram.max_addr_write)
    }
}
impl MemoryZoneStatsData {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn memory_write(&mut self, address: u64, width: u64, value: u64) {
        // If the memory is alligned to 8 bytes, i.e. last 3 bits are zero, then increase the
        // aligned memory read counter
        self.max_addr_write = self.max_addr_write.max(address + width - 1);
        if ((address & 0x07) == 0) && (width == 8) {
            self.mwrite_a += 1;
        } else {
            // If the memory write operation requires writing 2 aligned chunks of 8 bytes to build
            // the requested width, i.e. the requested slice crosses an 8-bytes boundary, then
            // increase the non-aligned counter number 2
            if ((address + width - 1) >> 3) > (address >> 3) {
                self.mwrite_na2 += 1;
            }
            // Otherwise increase the non-aligned counter number 1
            else {
                self.mwrite_na1 += 1;
                if width == 1 && value & 0xFFFF_FFFF_FFFF_FF00 == 0 {
                    self.mwrite_byte += 1;
                }
            }
            if ((address & 0x07) == 0) && (width == 8) {
                self.mwrite_a += 1;
            }
        }
    }
    pub fn memory_read(&mut self, address: u64, width: u64) {
        // If the memory is alligned to 8 bytes, i.e. last 3 bits are zero, then increase the
        // aligned memory read counter
        self.max_addr_read = self.max_addr_read.max(address + width - 1);
        if ((address & 0x07) == 0) && (width == 8) {
            self.mread_a += 1;
        } else {
            // If the memory read operation requires reading 2 aligned chunks of 8 bytes to build
            // the requested width, i.e. the requested slice crosses an 8-bytes boundary, then
            // increase the non-aligned counter number 2
            if ((address + width - 1) >> 3) > (address >> 3) {
                self.mread_na2 += 1;
            }
            // Otherwise increase the non-aligned counter number 1
            else {
                self.mread_na1 += 1;
                if width == 1 {
                    self.mread_byte += 1;
                }
            }
        }
    }
    pub fn get_cost(&self) -> u64 {
        self.mwrite_a * MEM_WRITE_COST
            + self.mread_a * MEM_READ_COST
            + self.mread_byte * MEM_READ_BYTE_COST
            + self.mwrite_byte * MEM_WRITE_BYTE_COST
            + self.mread_na2 * MEM_READ_UNALIGNED_2_COST
            + (self.mread_na1 - self.mread_byte) * MEM_READ_UNALIGNED_1_COST
            + self.mwrite_na2 * MEM_READ_UNALIGNED_2_COST
            + (self.mwrite_na1 - self.mwrite_byte) * MEM_READ_UNALIGNED_1_COST
    }
}
