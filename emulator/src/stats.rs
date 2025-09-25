//! Emulator execution statistics
//!
//! Statistics include:
//! * Memory read/write counters (aligned and not aligned)
//! * Registers read/write counters (total and per register)
//! * Operations counters (total and per opcode)

use std::{
    fs::File,
    io::{BufWriter, Write},
};

use sm_arith::ArithFrops;
use sm_binary::{BinaryBasicFrops, BinaryExtensionFrops};
use zisk_core::{zisk_ops::ZiskOp, ZiskInst, ZiskOperationType, M3, REGS_IN_MAIN_TOTAL_NUMBER};

const AREA_PER_SEC: f64 = 1000000_f64;
const COST_MEM: f64 = 10_f64 / AREA_PER_SEC;
const COST_MEMA_R1: f64 = 20_f64 / AREA_PER_SEC;
const COST_MEMA_R2: f64 = 40_f64 / AREA_PER_SEC;
const COST_MEMA_W1: f64 = 40_f64 / AREA_PER_SEC;
const COST_MEMA_W2: f64 = 80_f64 / AREA_PER_SEC;
const COST_USUAL: f64 = 8_f64 / AREA_PER_SEC;
const COST_STEP: f64 = 50_f64 / AREA_PER_SEC;

const OP_DATA_BUFFER_DEFAULT_CAPACITY: usize = 128 * 1024 * 1024;

/// Keeps counters for every type of memory operation (including registers).
///
/// Since RISC-V registers are mapped to memory, memory operations include register access
/// operations.
#[derive(Default, Debug, Clone)]
pub struct MemoryOperations {
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
    /// Counter of byte writes where value was dirty (value & 0xFFFF_FFFF_FFFF_FF00 != 0)
    mwrite_dirty_byte: u64,
    /// Counter of byte writes where value was signextend (value & 0xFFFF_FFFF_FFFF_FF00 != 0xFFFF_FFFF_FFFF_FF00)
    mwrite_dirty_s64_byte: u64,
    mwrite_dirty_s32_byte: u64,
    mwrite_dirty_s16_byte: u64,
}

/// Keeps statistics of the emulator operations
#[derive(Debug, Clone)]
pub struct Stats {
    /// Counters of memory read/write operations, both aligned and non-aligned
    mops: MemoryOperations,
    /// Counter of FROPS (FRequentOPs)
    frops: u64,
    /// Counter of steps
    steps: u64,
    /// Counters of operations, one per possible u8 opcode (many remain unused)
    ops: [u64; 256],
    /// Counters of register accesses, one per register
    regs: [u64; REGS_IN_MAIN_TOTAL_NUMBER],
    /// Flag to indicate whether to store operation data in a buffer
    store_ops: bool,
    /// Buffer to store operation data before writing to file
    op_data_buffer: Vec<u8>,
}

impl Default for Stats {
    /// Default constructor for Stats structure.  Sets all counters to zero.
    fn default() -> Self {
        Self {
            mops: MemoryOperations::default(),
            frops: 0,
            steps: 0,
            ops: [0; 256],
            regs: [0; REGS_IN_MAIN_TOTAL_NUMBER],
            op_data_buffer: vec![],
            store_ops: false,
        }
    }
}

impl Stats {
    /// Called every time some data is read from memory, if statistics are enabled
    pub fn on_memory_read(&mut self, address: u64, width: u64) {
        // If the memory is alligned to 8 bytes, i.e. last 3 bits are zero, then increase the
        // aligned memory read counter
        if ((address & M3) == 0) && (width == 8) {
            self.mops.mread_a += 1;
        } else {
            // If the memory read operation requires reading 2 aligned chunks of 8 bytes to build
            // the requested width, i.e. the requested slice crosses an 8-bytes boundary, then
            // increase the non-aligned counter number 2
            if ((address + width - 1) >> 3) > (address >> 3) {
                self.mops.mread_na2 += 1;
            }
            // Otherwise increase the non-aligned counter number 1
            else {
                self.mops.mread_na1 += 1;
                if width == 1 {
                    self.mops.mread_byte += 1;
                }
            }
        }
    }

    /// Called every time some data is writen to memory, if statistics are enabled
    pub fn on_memory_write(&mut self, address: u64, width: u64, value: u64) {
        // If the memory is alligned to 8 bytes, i.e. last 3 bits are zero, then increase the
        // aligned memory read counter
        if ((address & M3) == 0) && (width == 8) {
            self.mops.mwrite_a += 1;
        } else {
            // If the memory write operation requires writing 2 aligned chunks of 8 bytes to build
            // the requested width, i.e. the requested slice crosses an 8-bytes boundary, then
            // increase the non-aligned counter number 2
            if ((address + width - 1) >> 3) > (address >> 3) {
                self.mops.mwrite_na2 += 1;
            }
            // Otherwise increase the non-aligned counter number 1
            else {
                self.mops.mwrite_na1 += 1;
                if width == 1 {
                    self.mops.mwrite_byte += 1;
                    if (value & 0xFFFF_FFFF_FFFF_FF00) != 0 {
                        self.mops.mwrite_dirty_byte += 1;
                        if (value & 0xFFFF_FFFF_FFFF_FF00) != 0xFFFF_FFFF_FFFF_FF00 {
                            self.mops.mwrite_dirty_s64_byte += 1;
                        } else if (value & 0xFFFF_FFFF_FFFF_FF00) != 0xFFFF_FF00 {
                            self.mops.mwrite_dirty_s32_byte += 1;
                        } else if (value & 0xFFFF_FFFF_FFFF_FF00) != 0xFF00 {
                            self.mops.mwrite_dirty_s16_byte += 1;
                        }
                    }
                }
            }
            if ((address & M3) == 0) && (width == 8) {
                self.mops.mwrite_a += 1;
            }
        }
    }

    /// Called every time a register is read, if statistics are enabled
    pub fn on_register_read(&mut self, reg: usize) {
        assert!(reg < REGS_IN_MAIN_TOTAL_NUMBER);
        self.regs[reg] += 1;
    }

    /// Called every time a register is written, if statistics are enabled
    pub fn on_register_write(&mut self, reg: usize) {
        assert!(reg < REGS_IN_MAIN_TOTAL_NUMBER);
        self.regs[reg] += 1;
    }

    /// Called at every step with the current number of executed steps, if statistics are enabled
    pub fn on_steps(&mut self, steps: u64) {
        // Store the number of executed steps
        self.steps = steps;
    }

    /// Called every time an operation is executed, if statistics are enabled
    pub fn on_op(&mut self, instruction: &ZiskInst, a: u64, b: u64) {
        // If the operation is a usual operation, then increase the usual counter
        if self.store_ops
            && (instruction.op_type == ZiskOperationType::Arith
                || instruction.op_type == ZiskOperationType::Binary
                || instruction.op_type == ZiskOperationType::BinaryE)
        {
            // store op, a and b values in file
            self.store_op_data(instruction.op, a, b);
        }
        if self.is_frops(instruction, a, b) {
            self.frops += 1;
        }
        // Otherwise, increase the counter corresponding to this opcode
        else {
            self.ops[instruction.op as usize] += 1;
        }
    }
    pub fn set_store_ops(&mut self, store: bool) {
        self.store_ops = store;
        self.op_data_buffer = Vec::with_capacity(OP_DATA_BUFFER_DEFAULT_CAPACITY);
    }
    /// Store operation data in memory buffer
    fn store_op_data(&mut self, op: u8, a: u64, b: u64) {
        // Reserve space for: 1 byte (op) + 8 bytes (a) + 8 bytes (b) = 17 bytes
        self.op_data_buffer.reserve(17);

        // Store op as single byte
        self.op_data_buffer.push(op);

        // Store a and b as little-endian u64
        self.op_data_buffer.extend_from_slice(&a.to_le_bytes());
        self.op_data_buffer.extend_from_slice(&b.to_le_bytes());
    }

    /// Write all buffered operation data to file
    pub fn flush_op_data_to_file(&mut self, filename: &str) -> std::io::Result<()> {
        if self.op_data_buffer.is_empty() {
            return Ok(());
        }

        let file = File::create(filename)?;
        let mut writer = BufWriter::new(file);
        writer.write_all(&self.op_data_buffer)?;
        writer.flush()?;

        // Clear buffer after writing
        self.op_data_buffer.clear();
        Ok(())
    }

    /// Get the number of operations stored in buffer
    pub fn get_buffered_ops_count(&self) -> usize {
        self.op_data_buffer.len() / 17 // Each operation is 17 bytes
    }

    /// Clear the operation data buffer without writing to file
    pub fn clear_op_buffer(&mut self) {
        self.op_data_buffer.clear();
    }

    /// Returns true if the provided operation is a usual operation
    fn is_frops(&self, instruction: &ZiskInst, a: u64, b: u64) -> bool {
        match instruction.op_type {
            ZiskOperationType::Arith => ArithFrops::is_frequent_op(instruction.op, a, b),
            ZiskOperationType::Binary => BinaryBasicFrops::is_frequent_op(instruction.op, a, b),
            ZiskOperationType::BinaryE => {
                BinaryExtensionFrops::is_frequent_op(instruction.op, a, b)
            }
            _ => false,
        }
    }

    /// Returns a string containing a human-readable text showing all caunters
    pub fn report(&self) -> String {
        const AREA_PER_SEC: f64 = 1000000_f64;

        // The result of his function is accumulated in this string
        let mut output = String::new();

        // First, log the cost constants
        output += "Cost definitions:\n";
        output += &format!("    AREA_PER_SEC: {AREA_PER_SEC} steps\n");
        output += &format!("    COST_MEMA_R1: {COST_MEMA_R1:02} sec\n");
        output += &format!("    COST_MEMA_R2: {COST_MEMA_R2:02} sec\n");
        output += &format!("    COST_MEMA_W1: {COST_MEMA_W1:02} sec\n");
        output += &format!("    COST_MEMA_W2: {COST_MEMA_W2:02} sec\n");
        output += &format!("    COST_USUAL: {COST_USUAL:02} sec\n");
        output += &format!("    COST_STEP: {COST_STEP:02} sec\n");

        // Calculate some aggregated counters to be used in the logs
        let total_mem_ops = self.mops.mread_na1
            + self.mops.mread_na2
            + self.mops.mread_a
            + self.mops.mwrite_na1
            + self.mops.mwrite_na2
            + self.mops.mwrite_a;
        let total_mem_align_steps = self.mops.mread_na1
            + self.mops.mread_na2 * 2
            + self.mops.mwrite_na1 * 2
            + self.mops.mwrite_na2 * 4;

        let cost_mem = total_mem_ops as f64 * COST_MEM;
        let cost_mem_align = self.mops.mread_na1 as f64 * COST_MEMA_R1
            + self.mops.mread_na2 as f64 * COST_MEMA_R2
            + self.mops.mwrite_na1 as f64 * COST_MEMA_W1
            + self.mops.mwrite_na2 as f64 * COST_MEMA_W2;

        // Declare some total counters for the opcodes
        let mut total_opcodes: u64 = 0;
        let mut opcode_steps: [u64; 256] = [0; 256];
        let mut total_opcode_steps: u64 = 0;
        let mut opcode_cost: [f64; 256] = [0_f64; 256];
        let mut total_opcode_cost: f64 = 0_f64;

        // For every possible opcode value
        for opcode in 0..256 {
            // Skip opcode counters that are zero
            if self.ops[opcode] == 0 {
                continue;
            }

            // Increase total opcodes counter
            total_opcodes += self.ops[opcode];

            // Get the Zisk instruction corresponding to this opcode; if the counter has been
            // increased, then the opcode must be a valid one
            let inst = ZiskOp::try_from_code(opcode as u8).unwrap();

            // Increase steps, both per opcode and total
            opcode_steps[opcode] += inst.steps();
            total_opcode_steps += inst.steps();

            // Increse cost, both per opcode and total
            let value = self.ops[opcode] as f64;
            opcode_cost[opcode] += value * inst.steps() as f64 / AREA_PER_SEC;
            total_opcode_cost += value * inst.steps() as f64 / AREA_PER_SEC;
        }

        // Calculate some costs
        let cost_frops = self.frops as f64 * COST_USUAL;
        let cost_main = self.steps as f64 * COST_STEP;
        let total_cost = cost_main + cost_mem + cost_mem_align + total_opcode_cost + cost_frops;

        // Build the memory usage counters and cost values
        output += &format!("\nTotal Cost: {total_cost:.2} sec\n");
        output += &format!("    Main Cost: {:.2} sec {} steps\n", cost_main, self.steps);
        output += &format!("    Mem Cost: {cost_mem:.2} sec {total_mem_ops} steps\n");
        output +=
            &format!("    Mem Align: {cost_mem_align:.2} sec {total_mem_align_steps} steps\n");
        output += &format!(
            "    Opcodes: {total_opcode_cost:.2} sec {total_opcode_steps} steps ({total_opcodes} ops)\n"
        );
        output += &format!("    Frops: {:.2} sec {} steps\n", cost_frops, self.frops);
        let memory_reads = self.mops.mread_a + self.mops.mread_na1 + self.mops.mread_na2;
        let memory_writes = self.mops.mwrite_a + self.mops.mwrite_na1 + self.mops.mwrite_na2;
        let memory_total = memory_reads + memory_writes;
        output += &format!(
            "    Memory: {} a reads + {} na1 reads + {} na2 reads + {} a writes + {} na1 writes + {} na2 writes = {} reads + {} writes = {} r/w\n",
            self.mops.mread_a,
            self.mops.mread_na1,
            self.mops.mread_na2,
            self.mops.mwrite_a,
            self.mops.mwrite_na1,
            self.mops.mwrite_na2,
            memory_reads,
            memory_writes,
            memory_total
        );
        let mwrite_dirty_sext_byte = self.mops.mwrite_dirty_s64_byte
            + self.mops.mwrite_dirty_s32_byte
            + self.mops.mwrite_dirty_s16_byte;
        output += &format!(
            "    MemoryAlignByte: {} reads + {} writes / {} dirt_nosext_writes ({:.2}%) / {} dirt_sext_writes ({:.2}%) (64:{}, 32:{}, 16:{})\n",
            self.mops.mread_byte,
            self.mops.mwrite_byte,
            self.mops.mwrite_dirty_byte - mwrite_dirty_sext_byte,
            if self.mops.mwrite_byte == 0 {
                0.0
            } else {
                (self.mops.mwrite_dirty_byte - mwrite_dirty_sext_byte) as f64 * 100.0 / self.mops.mwrite_byte as f64
            },
            mwrite_dirty_sext_byte,
            if mwrite_dirty_sext_byte == 0 {
                0.0
            } else {
                mwrite_dirty_sext_byte as f64 * 100.0 / self.mops.mwrite_byte as f64
            },
            self.mops.mwrite_dirty_s64_byte,
            self.mops.mwrite_dirty_s32_byte,
            self.mops.mwrite_dirty_s16_byte
        );

        // Build the operations usage counters and cost values
        output += "\nOpcodes:\n";
        for opcode in 0..256 {
            // Skip zero counters
            if self.ops[opcode] == 0 {
                continue;
            }

            // Get the Zisk instruction corresponding to this opcode
            let inst = ZiskOp::try_from_code(opcode as u8).unwrap();

            // Log opcode cost
            output += &format!(
                "    {}: {:.2} sec ({} steps/op) ({} ops)\n",
                inst.name(),
                opcode_cost[opcode],
                opcode_steps[opcode],
                self.ops[opcode]
            );
        }

        // Build the register counters
        output += "\nRegisters:\n";
        let mut total_regs = 0u64;
        for reg in self.regs.iter() {
            total_regs += reg;
        }
        if total_regs == 0 {
            total_regs = 1;
        }
        output += &format!("total regs = {total_regs}\n");
        output += &format!("total steps = {}\n", self.steps);
        let regs_per_step = total_regs * 1000 / if self.steps == 0 { 1 } else { self.steps };
        output += &format!("total regs / steps = {regs_per_step} %o\n");

        for (i, reg) in self.regs.iter().enumerate() {
            let per_thousand = reg * 1000 / total_regs;
            output += &format!("reg[{i}] = {reg} ({per_thousand}%o)\n");
        }

        output
    }
}
