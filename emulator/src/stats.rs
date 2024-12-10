//! Emulator execution statistics
//!
//! Statistics include:
//! * Memory read/write counters (aligned and not aligned)
//! * Registers read/write counters (total and per register)
//! * Operations counters (total and per opcode)

use zisk_core::{zisk_ops::ZiskOp, ZiskInst, M3, REG_FIRST, REG_LAST};

const AREA_PER_SEC: f64 = 1000000_f64;
const COST_MEM: f64 = 10_f64 / AREA_PER_SEC;
const COST_MEMA_R1: f64 = 20_f64 / AREA_PER_SEC;
const COST_MEMA_R2: f64 = 40_f64 / AREA_PER_SEC;
const COST_MEMA_W1: f64 = 40_f64 / AREA_PER_SEC;
const COST_MEMA_W2: f64 = 80_f64 / AREA_PER_SEC;
const COST_USUAL: f64 = 8_f64 / AREA_PER_SEC;
const COST_STEP: f64 = 50_f64 / AREA_PER_SEC;

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
}

/// Keeps counter for register read and write operations
#[derive(Default, Debug, Clone)]
pub struct RegistryOperations {
    /// Counter of reads from registers
    reads: u64,
    /// Counter of writes to registers
    writes: u64,
}

/// Keeps statistics of the emulator operations
#[derive(Debug, Clone)]
pub struct Stats {
    /// Counters of memory read/write operations, both aligned and non-aligned
    mops: MemoryOperations,
    /// Counters of register read/write operations
    rops: RegistryOperations,
    /// Counter of usual operations
    usual: u64,
    /// Counter of steps
    steps: u64,
    /// Counters of operations, one per possible u8 opcode (many remain unused)
    ops: [u64; 256],
    /// Counters or register writes, split per register (32)
    reg_writes: [u64; 32],
    /// Counters or register reads, split per register (32)
    reg_reads: [u64; 32],
}

impl Default for Stats {
    /// Default constructor for Stats structure.  Sets all counters to zero.
    fn default() -> Self {
        Self {
            mops: MemoryOperations::default(),
            rops: RegistryOperations::default(),
            usual: 0,
            steps: 0,
            ops: [0; 256],
            reg_writes: [0; 32],
            reg_reads: [0; 32],
        }
    }
}

impl Stats {
    /// Called every time some data is read from memory, is statistics are enabled
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
            }
        }

        // If the address is within the range of register addresses, increase register counters
        if (REG_FIRST..=REG_LAST).contains(&address) {
            // Increase total register reads counter
            self.rops.reads += 1;

            // Increase the specific reads counter for this register
            self.reg_reads[((address - REG_FIRST) / 8) as usize] += 1;
        }
    }

    /// Called every time some data is writen to memory, is statistics are enabled
    pub fn on_memory_write(&mut self, address: u64, width: u64) {
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
            }
        }

        // If the address is within the range of register addresses, increase register counters
        if (REG_FIRST..=REG_LAST).contains(&address) {
            // Increase total register writes counter
            self.rops.writes += 1;

            // Increase the specific writes counter for this register
            self.reg_writes[((address - REG_FIRST) / 8) as usize] += 1;
        }
    }

    /// Called at every step with the current number of executed steps, if statistics are enabled
    pub fn on_steps(&mut self, steps: u64) {
        // Store the number of executed steps
        self.steps = steps;
    }

    /// Called every time an operation is executed, if statistics are enabled
    pub fn on_op(&mut self, instruction: &ZiskInst, a: u64, b: u64) {
        // If the operation is a usual operation, then increase the usual counter
        if self.is_usual(instruction, a, b) {
            self.usual += 1;
        }
        // Otherwise, increase the counter corresponding to this opcode
        else {
            self.ops[instruction.op as usize] += 1;
        }
    }

    /// Returns true if the provided operation is a usual operation
    fn is_usual(&self, instruction: &ZiskInst, a: u64, b: u64) -> bool {
        // ecall/system call functions are not candidates to be usual
        (instruction.op != 0xF1) &&
        // Internal functions are not candidates to be usual
        instruction.is_external_op &&
        // If both a and b parameters have low values (they fit into a byte) then the operation can
        // be efficiently proven using lookup tables
        (a < 256) && (b < 256)
    }

    /// Returns a string containing a human-readable text showing all caunters
    pub fn report(&self) -> String {
        const AREA_PER_SEC: f64 = 1000000_f64;

        // The result of his function is accumulated in this string
        let mut output = String::new();

        // First, log the cost constants
        output += "Cost definitions:\n";
        output += &format!("    AREA_PER_SEC: {} steps\n", AREA_PER_SEC);
        output += &format!("    COST_MEMA_R1: {:02} sec\n", COST_MEMA_R1);
        output += &format!("    COST_MEMA_R2: {:02} sec\n", COST_MEMA_R2);
        output += &format!("    COST_MEMA_W1: {:02} sec\n", COST_MEMA_W1);
        output += &format!("    COST_MEMA_W2: {:02} sec\n", COST_MEMA_W2);
        output += &format!("    COST_USUAL: {:02} sec\n", COST_USUAL);
        output += &format!("    COST_STEP: {:02} sec\n", COST_STEP);

        // Calculate some aggregated counters to be used in the logs
        let total_mem_ops = self.mops.mread_na1 +
            self.mops.mread_na2 +
            self.mops.mread_a +
            self.mops.mwrite_na1 +
            self.mops.mwrite_na2 +
            self.mops.mwrite_a;
        let total_mem_align_steps = self.mops.mread_na1 +
            self.mops.mread_na2 * 2 +
            self.mops.mwrite_na1 * 2 +
            self.mops.mwrite_na2 * 4;

        let cost_mem = total_mem_ops as f64 * COST_MEM;
        let cost_mem_align = self.mops.mread_na1 as f64 * COST_MEMA_R1 +
            self.mops.mread_na2 as f64 * COST_MEMA_R2 +
            self.mops.mwrite_na1 as f64 * COST_MEMA_W1 +
            self.mops.mwrite_na2 as f64 * COST_MEMA_W2;

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
        let cost_usual = self.usual as f64 * COST_USUAL;
        let cost_main = self.steps as f64 * COST_STEP;
        let total_cost = cost_main + cost_mem + cost_mem_align + total_opcode_cost + cost_usual;

        // Build the memory usage counters and cost values
        output += &format!("\nTotal Cost: {:.2} sec\n", total_cost);
        output += &format!("    Main Cost: {:.2} sec {} steps\n", cost_main, self.steps);
        output += &format!("    Mem Cost: {:.2} sec {} steps\n", cost_mem, total_mem_ops);
        output +=
            &format!("    Mem Align: {:.2} sec {} steps\n", cost_mem_align, total_mem_align_steps);
        output += &format!(
            "    Opcodes: {:.2} sec {} steps ({} ops)\n",
            total_opcode_cost, total_opcode_steps, total_opcodes
        );
        output += &format!("    Usual: {:.2} sec {} steps\n", cost_usual, self.usual);
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
        let reg_reads_percentage =
            if memory_reads != 0 { (self.rops.reads * 100) / memory_reads } else { 0 };
        let reg_writes_percentage =
            if memory_writes != 0 { (self.rops.writes * 100) / memory_writes } else { 0 };
        let reg_total = self.rops.reads + self.rops.writes;
        let reg_total_percentage =
            if memory_total != 0 { (reg_total * 100) / memory_total } else { 0 };

        output += &format!(
            "    Registy: reads={}={}% writes={}={}% total={}={}% r/w\n",
            self.rops.reads,
            reg_reads_percentage,
            self.rops.writes,
            reg_writes_percentage,
            reg_total,
            reg_total_percentage
        );

        // Build the registers usage counters and cost values
        for reg in 0..32 {
            let reads = self.reg_reads[reg];
            let writes = self.reg_writes[reg];
            let rw = reads + writes;
            let reads_percentage =
                if self.rops.reads != 0 { (reads * 100) / self.rops.reads } else { 0 };
            let writes_percentage =
                if self.rops.writes != 0 { (reads * 100) / self.rops.writes } else { 0 };
            let total_rw = self.rops.reads + self.rops.writes;
            let rw_percentage = if total_rw != 0 { (rw * 100) / total_rw } else { 0 };
            output += &format!(
                "        Reg {} reads={}={}% writes={}={}% r/w={}={}%\n",
                reg, reads, reads_percentage, writes, writes_percentage, rw, rw_percentage
            );
        }

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

        output
    }
}
