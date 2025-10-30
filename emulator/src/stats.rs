//! Emulator execution statistics
//!
//! Statistics include:
//! * Memory read/write counters (aligned and not aligned)
//! * Registers read/write counters (total and per register)
//! * Operations counters (total and per opcode)

use std::{
    collections::{BTreeMap, HashMap},
    fs::File,
    io::{BufWriter, Write},
    str::FromStr,
};

use sm_arith::ArithFrops;
use sm_binary::{BinaryBasicFrops, BinaryExtensionFrops};
use zisk_core::{
    zisk_ops::{OpStats, ZiskOp},
    ZiskInst, ZiskOperationType, ZiskRom, RAM_ADDR, REGS_IN_MAIN_TOTAL_NUMBER,
};

use crate::{
    get_ops_costs, get_ops_ranks, MemoryOperationsStats, RegionsOfInterest, StatsReport, BASE_COST,
    MAIN_COST,
};

const OP_DATA_BUFFER_DEFAULT_CAPACITY: usize = 128 * 1024 * 1024;

/// Keeps statistics of the emulator operations
#[derive(Debug, Clone)]
pub struct Stats {
    /// Counters of memory read/write operations, both aligned and non-aligned
    mops: MemoryOperationsStats,
    /// Counter of FROPS (FRequentOPs)
    frops: u64,
    /// Detail of FROPS
    frops_ops: [u64; 256],
    /// Cost of FROPS
    frops_cost: u64,
    /// Counter of steps
    steps: u64,
    /// Counters of operations, one per possible u8 opcode (many remain unused)
    ops: [u64; 256],
    /// Ops costs
    ops_cost: u64,
    /// Precompiled ops costs
    precompiled_cost: u64,
    /// Counters of register accesses, one per register
    regs: [u64; REGS_IN_MAIN_TOTAL_NUMBER],
    /// Flag to indicate whether to store operation data in a buffer
    store_ops: bool,
    /// Buffer to store operation data before writing to file
    op_data_buffer: Vec<u8>,
    rois_by_address: BTreeMap<u32, u32>,
    rois: Vec<RegionsOfInterest>,
    current_roi: Option<usize>,
    top_rois: usize,
    roi_callers: usize,
    top_rois_detail: bool,
    legacy_stats: bool,
    /// PC histogram, i.e. number of times each PC was executed
    pc_histogram: HashMap<u64, u64>,
}

impl Default for Stats {
    /// Default constructor for Stats structure.  Sets all counters to zero.
    fn default() -> Self {
        Self {
            mops: MemoryOperationsStats::default(),
            frops: 0,
            steps: 0,
            ops: [0; 256],
            frops_ops: [0; 256],
            regs: [0; REGS_IN_MAIN_TOTAL_NUMBER],
            op_data_buffer: vec![],
            store_ops: false,
            rois: Vec::new(),
            rois_by_address: BTreeMap::new(),
            current_roi: None,
            top_rois: 10,
            roi_callers: 10,
            ops_cost: 0,
            precompiled_cost: 0,
            frops_cost: 0,
            top_rois_detail: false,
            legacy_stats: false,
            pc_histogram: HashMap::new(),
        }
    }
}

impl Stats {
    /// Called every time some data is read from memory, if statistics are enabled
    pub fn on_memory_read(&mut self, address: u64, width: u64) {
        self.mops.memory_read(address, width);
        if let Some(roi_index) = self.current_roi {
            self.rois[roi_index].memory_read(address, width);
        }
    }

    /// Called every time some data is writen to memory, if statistics are enabled
    pub fn on_memory_write(&mut self, address: u64, width: u64, value: u64) {
        self.mops.memory_write(address, width, value);
        if let Some(roi_index) = self.current_roi {
            self.rois[roi_index].memory_write(address, width, value);
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

    pub fn check_roi(&mut self, pc: u32) {
        if let Some(roi_index) = self.current_roi {
            let roi = &mut self.rois[roi_index];
            if pc >= roi.from_pc && pc <= roi.to_pc {
                roi.inc_step();
                return;
            }
        }
        self.current_roi = if let Some((_, index)) = self.rois_by_address.range(..=pc).next_back() {
            let roi = &mut self.rois[*index as usize];
            if pc >= roi.from_pc && pc <= roi.to_pc {
                if pc == roi.from_pc {
                    roi.call(self.current_roi);
                } else {
                    roi.inc_step();
                }
            }
            Some(*index as usize)
        } else {
            None
        }
    }
    /// Called every time an operation is executed, if statistics are enabled
    pub fn on_op(&mut self, instruction: &ZiskInst, a: u64, b: u64, pc: u64, _regs: &[u64; 3]) {
        self.check_roi(pc as u32);
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
            self.frops_ops[instruction.op as usize] += 1;
        }
        // Otherwise, increase the counter corresponding to this opcode
        else {
            if instruction.is_external_op {
                if let Some(roi_index) = self.current_roi {
                    let roi = &mut self.rois[roi_index];
                    roi.add_op(instruction.op);
                }
            }
            self.ops[instruction.op as usize] += 1;
        }
        // Increase the PC histogram entry for this PC
        self.pc_histogram.entry(pc).and_modify(|count| *count += 1).or_insert(1);
    }
    pub fn get_frops_cost(&self) -> u64 {
        get_ops_costs(&self.frops_ops).0
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

    pub fn get_top_rois(&self, by_step: bool) -> Vec<(usize, u64)> {
        let mut top_rois: Vec<(usize, u64)> = self
            .rois
            .iter()
            .enumerate()
            .map(|(index, roi)| (index, if by_step { roi.get_steps() } else { roi.get_cost() }))
            .collect();
        top_rois.sort_by(|a, b| b.1.cmp(&a.1));
        top_rois.truncate(self.top_rois);
        top_rois
    }

    pub fn update_costs(&mut self) {
        self.rois.iter_mut().for_each(|roi| roi.update_costs());
        let (ops_cost, precompiled_cost) = get_ops_costs(&self.ops);
        self.frops_cost = get_ops_costs(&self.frops_ops).0;
        self.ops_cost = ops_cost;
        self.precompiled_cost = precompiled_cost;
    }
    pub fn report_opcodes(&self, report: &mut StatsReport, ops: &[u64], title: &str) {
        let ranks = get_ops_ranks(ops);
        for (opcode, op_count) in ops.iter().enumerate() {
            if opcode > 1 && *op_count > 0 {
                if let Ok(inst) = ZiskOp::try_from_code(opcode as u8) {
                    let rank = if ranks[opcode] < 5 {
                        format!(" #{}", ranks[opcode])
                    } else {
                        String::new()
                    };
                    report.add_count_cost_perc(
                        &format!("{title} {:}", inst.name()),
                        *op_count,
                        *op_count * inst.steps(),
                        &rank,
                    );
                }
            }
        }
    }

    pub fn report_opcodes_hit(
        &self,
        report: &mut StatsReport,
        ops: &[u64],
        ops2: &[u64],
        title: &str,
    ) {
        let ranks = get_ops_ranks(ops);
        for (opcode, op_count) in ops.iter().enumerate() {
            if opcode > 1 && *op_count > 0 {
                if let Ok(inst) = ZiskOp::try_from_code(opcode as u8) {
                    let rank = if ranks[opcode] < 5 {
                        format!(" #{}", ranks[opcode])
                    } else {
                        String::new()
                    };
                    report.add_count_perc_cost_perc(
                        &format!("{title} {:}", inst.name()),
                        *op_count,
                        (*op_count as f64 * 100.0) / ((*op_count + ops2[opcode]) as f64),
                        *op_count * inst.steps(),
                        &rank,
                    );
                }
            }
        }
    }

    pub fn report_opcodes_coverage(
        &self,
        report: &mut StatsReport,
        no_frops: &[u64],
        frops: &[u64],
        title: &str,
        rom: &ZiskRom,
    ) {
        let mut ops: [u64; 256] = [0; 256];
        for i in 0..256_usize {
            ops[i] = no_frops[i] + frops[i];
        }
        let mut ops_total_counter: u64 = 0;
        let mut ops_used_counter: u64 = 0;
        let mut no_frops_used_counter: u64 = 0;
        let mut frops_used_counter: u64 = 0;

        let mut ops_used_text: String = String::new();
        let mut ops_unused_text: String = String::new();
        let mut no_frops_used_text: String = String::new();
        let mut no_frops_unused_text: String = String::new();
        let mut frops_used_text: String = String::new();

        for i in 0..256_usize {
            if let Ok(inst) = ZiskOp::try_from_code(i as u8) {
                ops_total_counter += 1;
                if ops[i] > 0 {
                    ops_used_counter += 1;
                    ops_used_text.push_str(&format!("{}, ", inst.name()));
                } else {
                    ops_unused_text.push_str(&format!("{}, ", inst.name()));
                }
                if no_frops[i] > 0 {
                    no_frops_used_counter += 1;
                    no_frops_used_text.push_str(&format!("{}, ", inst.name()));
                } else {
                    no_frops_unused_text.push_str(&format!("{}, ", inst.name()));
                }
                if frops[i] > 0 {
                    frops_used_counter += 1;
                    frops_used_text.push_str(&format!("{}, ", inst.name()));
                }
            }
        }
        let r = format!(
            "\n{}:\nAVAILABLE: {}\nUSED: {}\nUSED NO FROPS: {} ({:.2}%) [{}]\nUNUSED NO FROPS: {} ({:.2}%) [{}]\nUSED FROPS: {} ({:.2}%) [{}]\n",
            title,
            ops_total_counter,
            ops_used_counter,
            no_frops_used_counter,
            (no_frops_used_counter as f64 * 100.0) / (ops_total_counter as f64),
            no_frops_used_text,
            ops_total_counter - no_frops_used_counter,
            ((ops_total_counter - no_frops_used_counter) as f64 * 100.0) / (ops_total_counter as f64),
            no_frops_unused_text,
            frops_used_counter,
            (frops_used_counter as f64 * 100.0) / (ops_total_counter as f64),
            frops_used_text
        );

        report.add(&r);

        // Create a RISC-V instruction string histogram
        let mut riscv_histogram: HashMap<String, u64> = HashMap::new();

        // Fill it with the supported RISC-V instructions
        for riscv_inst in RISCV_IMACFD_ZICSR_INSTRUCTIONS.iter() {
            riscv_histogram.insert(String::from_str(riscv_inst).unwrap(), 0);
        }

        // For each executed Zisk instruction, get its RISC-V instruction and update the histogram
        let mut unsupported_riscv_instructions: Vec<String> = Vec::new();
        for pc in self.pc_histogram.keys() {
            // Get the RISC-V instruction at this pc from rom
            let zisk_inst = rom.get_instruction(*pc);

            // If the Zisk instruction did not come from a RISC-V instruction, skip it
            if zisk_inst.riscv_inst.is_none() {
                continue;
            }

            // Get the RISC-V instruction string
            let riscv_inst = zisk_inst.riscv_inst.as_ref().unwrap();

            // Get the count of times this Zisk instruction was executed
            let count = self.pc_histogram.get(pc).unwrap();

            // Update the RISC-V histogram
            let total_count = riscv_histogram.get_mut(riscv_inst);
            if total_count.is_none() {
                unsupported_riscv_instructions
                    .push(zisk_inst.riscv_inst.as_ref().unwrap().to_string());
                continue;
            }

            // Update the count
            let total_count = total_count.unwrap();
            *total_count += count;
        }

        // Calculate RISC-V instruction coverage
        let used_riscv_instructions_counter: u64 =
            riscv_histogram.values().filter(|&&count| count > 0).count() as u64;
        let riscv_coverage = used_riscv_instructions_counter as f64
            / RISCV_IMACFD_ZICSR_INSTRUCTIONS.len() as f64
            * 100.0;
        let r = format!(
            "\nRISC-V INSTRUCTION COVERAGE: {:.2}% ({} out of {})\n",
            riscv_coverage,
            used_riscv_instructions_counter,
            RISCV_IMACFD_ZICSR_INSTRUCTIONS.len()
        );
        report.add(&r);

        // Report unsupported RISC-V instructions, in alphabetical order
        if !unsupported_riscv_instructions.is_empty() {
            let mut r = String::from("UNSUPPORTED RISC-V INSTRUCTIONS EXECUTED: ");
            for inst in unsupported_riscv_instructions.iter() {
                r += &format!("{} ", inst);
            }
            r += "\n";
            report.add(&r);
        }
        let mut r = String::from("EXECUTED RISC-V INSTRUCTIONS: ");
        let mut executed_riscv_instructions: Vec<String> = Vec::new();
        for (inst, count) in riscv_histogram.iter() {
            if *count > 0 {
                executed_riscv_instructions.push(inst.to_string());
            }
        }
        executed_riscv_instructions.sort();
        for inst in executed_riscv_instructions.iter() {
            r += &format!("{} ", inst);
        }
        r += "\n";
        report.add(&r);

        // Report non-executed RISC-V instructions, in alphabetical order
        let mut non_executed_riscv_instructions: Vec<String> = Vec::new();
        let mut r = String::from("NON_EXECUTED RISC-V INSTRUCTIONS: ");
        for (inst, count) in riscv_histogram.iter() {
            if *count == 0 {
                non_executed_riscv_instructions.push(inst.to_string());
            }
        }
        non_executed_riscv_instructions.sort();
        for inst in non_executed_riscv_instructions.iter() {
            r += &format!("{} ", inst);
        }
        r += "\n";
        report.add(&r);
    }

    fn legacy_report(&self) -> String {
        let ops_cost = self.ops_cost;
        let precompiled_cost = self.precompiled_cost;
        let total_steps = self.steps;
        let mem_cost = self.mops.get_cost();
        let main_cost = total_steps * MAIN_COST;
        let base_cost = BASE_COST as u64;
        let total_cost = base_cost + mem_cost + main_cost + ops_cost + precompiled_cost;
        format!(
            "\nTOTAL COST: {total_cost}\n\
             STEPS: {total_steps}\n\
             BASE COST: {base_cost}\n\
             MAIN COST: {main_cost}\n\
             OPCODES COST: {ops_cost}\n\
             PRECOMPILED COST: {precompiled_cost}\n\
             MEMORY COST: {mem_cost}\n\n\
             NOTE: New stats flags:\
             \n  -X   Generate a detailed stats report.\
             \n  -S   Load symbols from the ELF file to collect additional stats (requires -X).\
             \n  -D   Show detailed caller statistics (requires -X and -S).\n",
        )
    }
    /// Returns a string containing a human-readable text showing all counters
    pub fn report(&self, rom: &ZiskRom) -> String {
        if self.legacy_stats {
            return self.legacy_report();
        }
        let ops_cost = self.ops_cost;
        let precompiled_cost = self.precompiled_cost;
        let total_steps = self.steps;
        let mem_cost = self.mops.get_cost();
        let main_cost = total_steps * MAIN_COST;
        let base_cost = BASE_COST as u64;
        let total_cost = base_cost + mem_cost + main_cost + ops_cost + precompiled_cost;
        let mut report = StatsReport::new();
        report.set_total_cost(total_cost);
        report.set_steps(self.steps);
        report.title_cost("REPORT", "");
        report.add_cost("STEPS", total_steps);

        report.title_cost_perc("COST DISTRIBUTION", "COST");
        report.add_cost_perc("BASE", base_cost);
        report.add_cost_perc("MAIN", main_cost);
        report.add_cost_perc("OPCODES", ops_cost);
        report.add_cost_perc("PRECOMPILES", precompiled_cost);
        report.add_cost_perc("MEMORY", mem_cost);
        report.ln();
        report.add_cost_perc("FROPS", self.frops_cost);
        report.add_perc("RAM USAGE", self.mops.get_max_ram_address() - RAM_ADDR + 1, 1 << 29);
        report.title_count_cost_perc("COST BY OPCODE", "COUNT", "COST", " RANK");
        self.report_opcodes(&mut report, &self.ops, "OP");

        report.title_count_perc_cost_perc("FROPS BY OPCODE", "COUNT", "HIT", "COST", " RANK");
        self.report_opcodes_hit(&mut report, &self.frops_ops, &self.ops, "FROP");
        self.report_opcodes_coverage(&mut report, &self.ops, &self.frops_ops, "OPS_COVERAGE", rom);

        if !self.rois.is_empty() {
            report.title_top_perc("TOP STEP FUNCTIONS");

            let top_step_rois = self.get_top_rois(true);
            for (index, _) in top_step_rois.iter() {
                let roi = &self.rois[*index];
                report.add_top_step_perc(&roi.name, roi.get_steps());
            }

            report.title_top_perc("TOP COST FUNCTIONS");

            // Create a vector with ROI indices and their steps for sorting
            let top_cost_rois = self.get_top_rois(false);

            for (index, _) in top_cost_rois.iter() {
                let roi = &self.rois[*index];
                report.add_top_cost_perc(&roi.name, roi.get_cost());
            }

            if self.top_rois_detail {
                for (index, _) in top_cost_rois.iter() {
                    let roi = &self.rois[*index];
                    let mut roi_report = StatsReport::new();
                    roi_report.set_total_cost(roi.get_cost());
                    roi_report.set_steps(roi.steps);
                    roi_report.title(&format!("DETAIL FUNCTION {}", roi.name));
                    roi_report.add_perc("STEPS", roi.get_steps(), total_steps);
                    roi_report.add_perc("COST", roi.get_cost(), total_cost);

                    roi_report.set_identation(1);
                    roi_report.title_count_cost_perc("COST BY OPCODE", "COUNT", "COST", " RANK");
                    self.report_opcodes(&mut roi_report, &roi.ops, "OP");

                    roi_report.title_top_count_perc("TOP STEP CALLERS (calls, steps)");
                    let mut callers: Vec<_> = roi.get_callers().collect();
                    callers.sort_by(|a, b| b.1.calls.cmp(&a.1.calls));

                    for (index, caller_info) in callers.iter().take(self.roi_callers) {
                        roi_report.add_top_count_step_perc(
                            &self.rois[**index].name,
                            caller_info.calls as u64,
                            caller_info.steps as u64,
                        );
                    }
                    report.add(&roi_report.output);
                }
            }
        }
        report.output
    }
    pub fn add_roi(&mut self, from_pc: u32, to_pc: u32, name: &str) {
        let roi = RegionsOfInterest::new(from_pc, to_pc, name);
        let index = self.rois.len() as u32;
        self.rois.push(roi);
        self.rois_by_address.insert(from_pc, index);
    }
    pub fn set_top_rois(&mut self, value: usize) {
        self.top_rois = value;
    }
    pub fn set_legacy_stats(&mut self, value: bool) {
        self.legacy_stats = value;
    }
    pub fn set_roi_callers(&mut self, value: usize) {
        self.roi_callers = value;
    }
    pub fn set_top_roi_detail(&mut self, value: bool) {
        self.top_rois_detail = value;
    }
}

impl OpStats for Stats {
    fn mem_align_read(&mut self, addr: u64, count: usize) {
        for index in 0..count {
            self.on_memory_read(addr + 8 * index as u64, 8);
        }
    }
    fn mem_align_write(&mut self, addr: u64, count: usize) {
        for index in 0..count {
            self.on_memory_write(addr + 8 * index as u64, 8, 0);
        }
    }
}

pub const RISCV_IMACFD_ZICSR_INSTRUCTIONS: [&str; 193] = [
    // ============================================
    // I Extension - Base Integer Instruction Set
    // ============================================
    // RV32I/RV64I Base Instructions
    "lui",
    "auipc",
    "jal",
    "jalr",
    "beq",
    "bne",
    "blt",
    "bge",
    "bltu",
    "bgeu",
    "lb",
    "lh",
    "lw",
    "lbu",
    "lhu",
    "lwu",
    "sb",
    "sh",
    "sw",
    "addi",
    "slti",
    "sltiu",
    "xori",
    "ori",
    "andi",
    "slli",
    "srli",
    "srai",
    "add",
    "sub",
    "sll",
    "slt",
    "sltu",
    "xor",
    "srl",
    "sra",
    "or",
    "and",
    "fence",
    "fence.i",
    "ecall",
    "ebreak",
    // RV64I-specific instructions
    "lw",
    "ld",
    "sd", // Note: lw/sw exist in RV32I, but have different behavior in RV64I
    "addiw",
    "slliw",
    "srliw",
    "sraiw",
    "addw",
    "subw",
    "sllw",
    "srlw",
    "sraw",
    // ============================================
    // M Extension - Integer Multiplication/Division
    // ============================================
    "mul",
    "mulh",
    "mulhsu",
    "mulhu",
    "div",
    "divu",
    "rem",
    "remu",
    // RV64M-specific
    "mulw",
    "divw",
    "divuw",
    "remw",
    "remuw",
    // ============================================
    // A Extension - Atomic Instructions
    // ============================================
    // Atomic Memory Operations
    "lr.w",
    "sc.w",
    "amoswap.w",
    "amoadd.w",
    "amoxor.w",
    "amoand.w",
    "amoor.w",
    "amomin.w",
    "amomax.w",
    "amominu.w",
    "amomaxu.w",
    // RV64A-specific
    "lr.d",
    "sc.d",
    "amoswap.d",
    "amoadd.d",
    "amoxor.d",
    "amoand.d",
    "amoor.d",
    "amomin.d",
    "amomax.d",
    "amominu.d",
    "amomaxu.d",
    // ============================================
    // C Extension - Compressed Instructions
    // ============================================
    // RV32C/RV64C Compressed Instructions
    "c.addi4spn",
    "c.fld",
    "c.lw",
    "c.flw",
    "c.ld",
    "c.fsd",
    "c.sw",
    "c.fsw",
    "c.sd",
    "c.addi",
    "c.jal",
    "c.li",
    "c.addi16sp",
    "c.lui",
    "c.srli",
    "c.srai",
    "c.andi",
    "c.sub",
    "c.xor",
    "c.or",
    "c.and",
    "c.j",
    "c.beqz",
    "c.bnez",
    "c.slli",
    "c.fldsp",
    "c.lwsp",
    "c.flwsp",
    "c.ldsp",
    "c.jr",
    "c.mv",
    "c.ebreak",
    "c.jalr",
    "c.add",
    "c.fsdsp",
    "c.swsp",
    "c.fswsp",
    "c.sdsp",
    // ============================================
    // F Extension - Single-Precision Floating-Point
    // ============================================
    "flw",
    "fsw",
    "fmadd.s",
    "fmsub.s",
    "fnmsub.s",
    "fnmadd.s",
    "fadd.s",
    "fsub.s",
    "fmul.s",
    "fdiv.s",
    "fsqrt.s",
    "fsgnj.s",
    "fsgnjn.s",
    "fsgnjx.s",
    "fmin.s",
    "fmax.s",
    "fcvt.w.s",
    "fcvt.wu.s",
    "fcvt.s.w",
    "fcvt.s.wu",
    "fmv.x.w",
    "fmv.w.x",
    "feq.s",
    "flt.s",
    "fle.s",
    "fclass.s",
    // RV64F-specific
    "fcvt.l.s",
    "fcvt.lu.s",
    "fcvt.s.l",
    "fcvt.s.lu",
    // ============================================
    // D Extension - Double-Precision Floating-Point
    // ============================================
    "fld",
    "fsd",
    "fmadd.d",
    "fmsub.d",
    "fnmsub.d",
    "fnmadd.d",
    "fadd.d",
    "fsub.d",
    "fmul.d",
    "fdiv.d",
    "fsqrt.d",
    "fsgnj.d",
    "fsgnjn.d",
    "fsgnjx.d",
    "fmin.d",
    "fmax.d",
    "fcvt.s.d",
    "fcvt.d.s",
    "fcvt.w.d",
    "fcvt.wu.d",
    "fcvt.d.w",
    "fcvt.d.wu",
    "feq.d",
    "flt.d",
    "fle.d",
    "fclass.d",
    "fcvt.l.d",
    "fcvt.lu.d",
    "fcvt.d.l",
    "fcvt.d.lu",
    // ============================================
    // Zicsr Extension - Control and Status Registers
    // ============================================
    // CSR Read/Write
    "csrrw",
    "csrrs",
    "csrrc",
    "csrrwi",
    "csrrsi",
    "csrrci",
    // CSR Read & Set/Clear (pseudo-instructions, but important for compilers)
    // "csrr",
    // "csrw",
    // "csrs",
    // "csrc",
    // "csrwi",
    // "csrsi",
    // "csrci",
    // Privileged Instructions (often grouped with Zicsr)
    // "mret",
    // "sret",
    // "wfi",
    // Timer and Counter Instructions
    // "rdcycle",
    // "rdtime",
    // "rdinstret",
    // "rdcycleh",
    // "rdtimeh",
    // "rdinstreth", // RV32 only
    // Machine Mode CSR Access
    // "mrs",
    // "msr", // Common aliases
    // Hypervisor CSRs (when H extension is present)
    // "hfv",
    // "hsv",
];
