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
};

use sm_arith::ArithFrops;
use sm_binary::{BinaryBasicFrops, BinaryExtensionFrops};
use zisk_core::{
    zisk_ops::{OpStats, ZiskOp},
    ZiskInst, ZiskOperationType, ZiskRom, RAM_ADDR, REGS_IN_MAIN_TOTAL_NUMBER, SRC_IMM, SRC_REG,
};

use crate::{
    get_ops_costs, get_ops_ranks, RegionsOfInterest, StatsCostMark, StatsCosts,
    StatsCoverageReport, StatsReport, BASE_COST, MAIN_COST,
};

#[derive(Debug, Clone, Default)]
pub struct CallStackEntry {
    pub pc: u64,
    pub ra: u64,
    pub caller_roi_index: Option<usize>,
    pub called_roi_index: Option<usize>,
    pub func_name: String,
    pub return_reg: u8,
    pub tail_calls: Vec<(usize, StatsCosts)>,
    costs: StatsCosts,
}

const OP_DATA_BUFFER_DEFAULT_CAPACITY: usize = 128 * 1024 * 1024;

const REG_RA_IDX: usize = 1;

/// Keeps statistics of the emulator operations
#[derive(Debug, Clone)]
pub struct Stats {
    /// Counter of FROPS (FRequentOPs)
    frops: u64,
    /// Cost of FROPS
    frops_cost: u64,
    /// Ops costs
    ops_cost: u64,
    /// Precompiled ops costs
    precompiled_cost: u64,
    /// Counters of register accesses, one per register
    regs: [u64; REGS_IN_MAIN_TOTAL_NUMBER],
    /// Flag to indicate whether to store operation data in a buffer
    store_ops: bool,
    costs: StatsCosts,
    /// Buffer to store operation data before writing to file
    op_data_buffer: Vec<u8>,
    rois_by_address: BTreeMap<u32, u32>,
    rois: Vec<RegionsOfInterest>,
    current_roi: Option<usize>,
    previous_roi: Option<usize>,
    top_rois: usize,
    roi_callers: usize,
    top_rois_detail: bool,
    coverage: bool,
    legacy_stats: bool,
    /// PC histogram, i.e. number of times each PC was executed
    pc_histogram: HashMap<u64, u64>,
    previous_pc: u64,
    call_stack: Vec<CallStackEntry>,
    previous_verbose: String,
    is_call: bool,
    is_return: bool,
    call_return_reg: u8,
    profile_marks: HashMap<u16, StatsCostMark>,
    individual_cost_marks: bool,
    main_name: String,
    profile_tags: HashMap<u16, String>,
    #[cfg(feature = "debug_stats_trace")]
    debug_step_stack: Vec<u64>,
    #[cfg(feature = "debug_stats_trace")]
    previous_stack_depth: usize,
}

impl Default for Stats {
    /// Default constructor for Stats structure.  Sets all counters to zero.
    fn default() -> Self {
        Self {
            frops: 0,
            costs: StatsCosts::default(),
            regs: [0; REGS_IN_MAIN_TOTAL_NUMBER],
            op_data_buffer: vec![],
            store_ops: false,
            rois: Vec::new(),
            rois_by_address: BTreeMap::new(),
            current_roi: None,
            previous_roi: None,
            top_rois: 25,
            roi_callers: 10,
            ops_cost: 0,
            precompiled_cost: 0,
            frops_cost: 0,
            top_rois_detail: false,
            coverage: false,
            legacy_stats: false,
            pc_histogram: HashMap::new(),
            previous_pc: 0,
            call_stack: Vec::new(),
            previous_verbose: String::default(),
            is_call: false,
            is_return: false,
            call_return_reg: 0,
            profile_marks: HashMap::new(),
            individual_cost_marks: false,
            main_name: "main".to_string(),
            profile_tags: HashMap::new(),
            #[cfg(feature = "debug_stats_trace")]
            debug_step_stack: Vec::new(),
            #[cfg(feature = "debug_stats_trace")]
            previous_stack_depth: 0,
        }
    }
}

impl Stats {
    /// Called every time some data is read from memory, if statistics are enabled
    pub fn on_memory_read(&mut self, address: u64, width: u64) {
        self.costs.memory_read(address, width);
    }

    /// Called every time some data is writen to memory, if statistics are enabled
    pub fn on_memory_write(&mut self, address: u64, width: u64, value: u64) {
        self.costs.memory_write(address, width, value);
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
        assert_eq!(self.costs.steps, steps + 1);
    }

    pub fn print_call_stack(&self) {
        println!("CALL STACK DUMP (top to bottom):");
        for (i, entry) in self.call_stack.iter().rev().enumerate() {
            let roi_name = if let Some(roi_index) = entry.called_roi_index {
                &self.rois[roi_index].name
            } else {
                "????"
            };
            println!(
                "#{} PC:0x{:08X} RA:0x{:08X} ROI:{} STEPS:{}",
                i, entry.pc, entry.ra, roi_name, entry.costs.steps
            );
        }
    }
    pub fn static_print_call_stack(call_stack: &[CallStackEntry], prefix: &str) {
        for (i, entry) in call_stack.iter().rev().enumerate() {
            println!(
                "{prefix}#{} PC:0x{:08X} RA:0x{:08X} ROI[{}]:{} STEPS:{}",
                i,
                entry.pc,
                entry.ra,
                entry.called_roi_index.unwrap_or(usize::MAX),
                entry.func_name,
                entry.costs.steps
            );
        }
    }
    pub fn check_roi(&mut self, pc: u32, regs: &[u64]) {
        self.previous_roi = self.current_roi;

        // First, handle RETURN even if we're not changing ROI
        let return_call = if self.is_return && !self.call_stack.is_empty() {
            #[cfg(feature = "debug_call_stack")]
            println!("CALL_STACK_DEBUG: RETURN P_PC:0x{:08x} => PC:0x{pc:08x}", self.previous_pc);
            self.call_stack.pop()
        } else {
            None
        };

        let previous_roi_index = self.current_roi;

        let update_roi = if let Some(previous_index) = self.current_roi {
            let roi = &self.rois[previous_index];
            pc < roi.from_pc || pc > roi.to_pc
        } else {
            true
        };

        if update_roi {
            self.current_roi =
                if let Some((_, index)) = self.rois_by_address.range(..=pc).next_back() {
                    Some(*index as usize)
                } else {
                    None
                };
        }

        if previous_roi_index != self.current_roi && !self.is_return && !self.is_call {
            if let Some(roi_index) = self.current_roi {
                // Tail call
                if let Some(top) = self.call_stack.last_mut() {
                    #[cfg(feature = "debug_call_stack")]
                    println!(
                        "CALL_STACK_DEBUG: TAIL CALL P_PC:0x{:08x} => PC:0x{pc:08x}",
                        self.previous_pc
                    );
                    top.tail_calls.push((roi_index, self.costs.clone()));
                    self.rois[roi_index].tail_jmp(previous_roi_index);
                    if top.tail_calls.len() < 2 {
                        self.rois[roi_index].calls += 1;
                    }
                }
            }
        }

        // Now handle ROI updates and CALL/JMP
        if let Some(roi_index) = self.current_roi {
            let roi = &mut self.rois[roi_index];
            if pc >= roi.from_pc && pc <= roi.to_pc && !self.is_call && !self.is_return {
                assert!(return_call.is_none());
                return;
            }
        }

        if let Some(roi_index) = self.current_roi {
            // At this point ROI change, search the new ROI
            // let roi = &mut self.rois[*index as usize];
            // If return after call, need to add delta costs
            if let Some(return_call) = return_call {
                if return_call.caller_roi_index != Some(roi_index) {
                    println!("**** STACK MISMATCH DETECTED ****\n");
                    println!(
                        "PC:[0x{pc:08x}] RA:[0x{:08x}] P_PC:[0x{:08x}]",
                        regs[1], self.previous_pc
                    );
                    if let Some(caller_roi_index) = return_call.caller_roi_index {
                        let _roi = &self.rois[caller_roi_index];
                        println!("caller_roi_index (expected): {caller_roi_index} [0x{:08x}, 0x{:08x}] {}", _roi.from_pc, _roi.to_pc, _roi.name);
                    } else {
                        println!("caller_roi_index (expected): None !!");
                    }
                    let _roi = &self.rois[roi_index];
                    println!(
                        "caller_roi_index (current): {roi_index} [0x{:08x}, 0x{:08x}] {}",
                        _roi.from_pc, _roi.to_pc, _roi.name
                    );
                    if let Some(called_roi_index) = return_call.called_roi_index {
                        let _roi = &self.rois[called_roi_index];
                        println!(
                            "called_roi_index: {called_roi_index} [0x{:08x}, 0x{:08x}] {}",
                            _roi.from_pc, _roi.to_pc, _roi.name
                        );
                    } else {
                        println!("called_roi_index (expected): None !!");
                    }
                    println!("\n");
                    Self::static_print_call_stack(&self.call_stack, "");
                    panic!("CALL STACK EMU: STACK MISMATCH DETECTED on 0x{pc:08x}");
                }

                self.rois[roi_index].return_call(self.call_stack.len());

                // TODO: check tail call re-entry calls
                // For all tail_call add costs from tail call to now
                let mut processed = Vec::new();
                for (roi_index, tail_call) in return_call.tail_calls.iter() {
                    if processed.contains(roi_index) {
                        continue;
                    }
                    if Some(*roi_index) != self.current_roi {
                        self.rois[*roi_index].add_delta_costs(tail_call, &self.costs);
                    }
                    processed.push(*roi_index);
                }

                // At this point we need to update costs of caller inside called ROI
                if let Some(called_roi_index) = return_call.called_roi_index {
                    // update with cost taking as reference the cost when is "called"
                    self.rois[called_roi_index].update_caller(
                        roi_index,
                        &return_call.costs,
                        &self.costs,
                    );
                    self.rois[called_roi_index].add_delta_costs(&return_call.costs, &self.costs);
                }
                let _steps = self.rois[roi_index].get_steps();
                assert!(_steps <= self.costs.steps);

                let roi_steps = self.rois[roi_index].get_steps();
                if roi_steps > self.costs.steps {
                    self.print_call_stack();
                    panic!(
                        "roi.costs.steps({}) > self.costs.steps({}) ref.steps: {} on #{}",
                        roi_steps,
                        self.costs.steps,
                        return_call.costs.steps,
                        self.call_stack.len()
                    );
                }
            }

            if pc >= self.rois[roi_index].from_pc && pc <= self.rois[roi_index].to_pc {
                if self.is_call {
                    assert!(!self.is_return);
                    if let Some(previous_roi_index) = previous_roi_index {
                        self.rois[previous_roi_index].caller_call();
                    }
                    #[cfg(feature = "debug_call_stack")]
                    println!(
                        "CALL_STACK_DEBUG: CALL P_PC:0x{:08x} => PC:0x{pc:08x} CALLER_ROI:{} CALLED_ROI:{}",
                        self.previous_pc, previous_roi_index.unwrap_or(900_000_000), self.current_roi.unwrap_or(900_000_000)
                    );
                    self.call_stack.push(CallStackEntry {
                        pc: pc as u64,
                        ra: regs[REG_RA_IDX],
                        caller_roi_index: previous_roi_index,
                        called_roi_index: self.current_roi,
                        costs: self.costs.clone(),
                        func_name: self.rois[roi_index].name.clone(),
                        return_reg: self.call_return_reg,
                        ..Default::default()
                    });
                    self.call_return_reg = 0;

                    self.rois[roi_index].call(previous_roi_index, self.call_stack.len());
                } else if !self.is_return {
                    // JMP: This is a tail call. Replace the top of the call stack if it exists
                    if let Some(top) = self.call_stack.last_mut() {
                        top.pc = pc as u64;
                        top.called_roi_index = Some(roi_index);
                    }
                    self.rois[roi_index].calls += 1;
                    self.rois[roi_index].update_call_depth(self.call_stack.len());
                }
            }
        }
    }

    fn on_start_mark(&mut self, id: u64) {
        assert!(id < u16::MAX as u64);
        let mark = self.profile_marks.entry(id as u16).or_default();
        mark.start = Some(self.costs.clone());
    }

    fn on_end_mark(&mut self, id: u64) {
        assert!(id < u16::MAX as u64);
        let mark = self.profile_marks.get_mut(&(id as u16)).unwrap_or_else(|| {
            panic!("Cost mark with id {} does not exist. Must call on_start_mark first.", id)
        });

        let start_costs = mark.start.as_ref().unwrap_or_else(|| {
            panic!("Cost mark with id {} has no start point. Must call on_start_mark before on_end_mark.", id)
        });

        // Create a new StatsCosts with the delta
        let mut delta_costs = StatsCosts::new();
        if self.individual_cost_marks {
            delta_costs.add_delta(start_costs, &self.costs);
            mark.costs.push(delta_costs);
        } else {
            if mark.costs.is_empty() {
                mark.costs.push(StatsCosts::new());
            }
            mark.costs[0].add_delta(start_costs, &self.costs);
        }
        mark.count += 1;
        mark.start = None;
    }
    fn on_absolute_mark(&mut self, id: u64) {
        assert!(id < u16::MAX as u64);
        let mark = self.profile_marks.entry(id as u16).or_default();
        mark.costs.push(self.costs.clone());
    }
    fn on_relative_mark(&mut self, id: u64) {
        assert!(id < u16::MAX as u64);
        let mark = self.profile_marks.entry(id as u16).or_default();

        if let Some(start) = &mark.start {
            let mut delta_costs = StatsCosts::new();
            delta_costs.add_delta(start, &self.costs);
            mark.costs.push(delta_costs);
        } else {
            mark.costs.push(self.costs.clone());
        }

        mark.start = Some(self.costs.clone());
    }
    fn on_reset_relative_mark(&mut self, id: u64) {
        assert!(id < u16::MAX as u64);
        let mark = self.profile_marks.entry(id as u16).or_default();
        mark.start = Some(self.costs.clone());
    }
    fn on_counter_mark(&mut self, id: u64) {
        assert!(id < u16::MAX as u64);
        let mark = self.profile_marks.entry(id as u16).or_default();
        mark.count += 1;
    }
    fn on_value_mark(&mut self, id: u64, value: u64) {
        assert!(id < u16::MAX as u64);
        let mark = self.profile_marks.entry(id as u16).or_default();
        if mark.min_value.unwrap_or(u64::MAX) >= value {
            mark.min_value = Some(value);
        }
        if mark.max_value.unwrap_or(u64::MIN) <= value {
            mark.max_value = Some(value);
        }
        mark.total_value += value as u128;
        mark.count += 1;
    }
    fn on_argument_mark(&mut self, id: u64, index: u8, value: u64) {
        assert!(id < u16::MAX as u64);
        let mark = self.profile_marks.entry(id as u16).or_default();
        if mark.arguments.len() <= index as usize {
            mark.arguments.resize(index as usize + 1, 0);
        }
        mark.arguments[index as usize] = value;
    }
    /// Called every time an operation is executed, if statistics are enabled
    pub fn on_op(&mut self, instruction: &ZiskInst, a: u64, b: u64, pc: u64, regs: &[u64]) {
        // println!("##PC## 0x{pc:08X}");
        self.costs.steps += 1;
        self.check_roi(pc as u32, regs);
        #[cfg(feature = "debug_stats_trace")]
        self.debug_stats_trace(pc);
        // If the operation is a usual operation, then increase the usual counter

        if instruction.op == 0 && instruction.a_src == SRC_REG && instruction.b_src == SRC_IMM {
            if instruction.b_offset_imm0 < 256 {
                match instruction.a_offset_imm0 {
                    1 => self.on_start_mark(instruction.b_offset_imm0),
                    2 => self.on_end_mark(instruction.b_offset_imm0),
                    3 => self.on_absolute_mark(instruction.b_offset_imm0),
                    4 => self.on_relative_mark(instruction.b_offset_imm0),
                    5 => self.on_reset_relative_mark(instruction.b_offset_imm0),
                    6 => self.on_counter_mark(instruction.b_offset_imm0),
                    _ => (),
                }
            } else {
                let flags = (instruction.b_offset_imm0 >> 8) as u8;
                match flags {
                    1..2 => self.on_argument_mark(
                        instruction.b_offset_imm0 & 0xFF,
                        flags - 1,
                        regs[instruction.a_offset_imm0 as usize],
                    ),
                    4 => self.on_value_mark(
                        instruction.b_offset_imm0 & 0xFF,
                        regs[instruction.a_offset_imm0 as usize],
                    ),
                    _ => (),
                }
            }
        }

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
            self.costs.frops_ops[instruction.op as usize] += 1;
        }
        // Otherwise, increase the counter corresponding to this opcode
        else {
            self.costs.ops[instruction.op as usize] += 1;
        }
        // Increase the PC histogram entry for this PC
        self.pc_histogram.entry(pc).and_modify(|count| *count += 1).or_insert(1);
        self.previous_pc = pc;
        self.previous_verbose = instruction.verbose.clone();
        let is_jmp = instruction.set_pc
            || (instruction.op == 0
                && (instruction.jmp_offset1 > 4 || instruction.jmp_offset1 < 0));
        if is_jmp {
            // CALL: set_pc=true, store_ra=true, store_offset=1 (stores PC+4 or PC+2 in ra)
            // self.is_call = instruction.store_ra && instruction.store_offset == 1;
            self.is_call = instruction.store_pc;
            self.call_return_reg = if self.is_call { instruction.store_offset as u8 } else { 0 };

            // RETURN: set_pc=true, store_pc=false (no stores RA), b_src=SRC_REG, b_offset_imm0=1 (jumps to ra/x1)
            // Additionally, verify that the target PC matches the expected return address from the call stack
            let is_jalr_ra = !instruction.store_pc
                && instruction.set_pc
                && instruction.b_src == SRC_REG
                && instruction.b_offset_imm0 == 1;

            if is_jalr_ra && !self.call_stack.is_empty() {
                // Check if we're jumping to the expected return address
                if let Some(_top) = self.call_stack.last() {
                    // The new PC should match the RA from the call stack
                    // Note: we can't check the future PC here, so we rely on the pattern
                    self.is_return = true;
                } else {
                    self.is_return = false;
                }
            } else if let Some(top) = self.call_stack.last() {
                self.is_return = !instruction.store_pc
                    && instruction.b_src == SRC_REG
                    && instruction.b_offset_imm0 == top.return_reg as u64;
            } else {
                self.is_return = false;
            }
        } else {
            self.is_call = false;
            self.is_return = false;
        }
    }
    pub fn get_frops_cost(&self) -> u64 {
        get_ops_costs(&self.costs.frops_ops).0
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

        // If there is an ROI whose name contains main_func_name, remove all entries from the
        // beginning up to and including it.
        if !self.main_name.is_empty() {
            if let Some(pos) = top_rois.iter().position(|(index, _)| {
                self.rois[*index].name == self.main_name && self.rois[*index].get_steps() > 0
            }) {
                top_rois.drain(0..=pos);
            }
        }
        top_rois.truncate(self.top_rois);
        top_rois
    }

    pub fn update_costs(&mut self) {
        self.rois.iter_mut().for_each(|roi| roi.update_costs());
        let (ops_cost, precompiled_cost) = get_ops_costs(&self.costs.ops);
        self.frops_cost = get_ops_costs(&self.costs.frops_ops).0;
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

    fn legacy_report(&self) -> String {
        let ops_cost = self.ops_cost;
        let precompiled_cost = self.precompiled_cost;
        let total_steps = self.costs.steps;
        let mem_cost = self.costs.mops.get_cost();
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
        let total_steps = self.costs.steps;
        let mem_cost = self.costs.mops.get_cost();
        let main_cost = total_steps * MAIN_COST;
        let base_cost = BASE_COST as u64;
        let total_cost = base_cost + mem_cost + main_cost + ops_cost + precompiled_cost;
        let mut report = StatsReport::new();
        report.set_total_cost(total_cost);
        report.set_steps(self.costs.steps);
        report.title_cost("REPORT", "");
        report.add_cost("STEPS", total_steps);

        report.title_cost_perc("COST DISTRIBUTION", "COST");
        report.add_cost_perc("BASE", base_cost);
        report.add_cost_perc("MAIN", main_cost);
        report.add_cost_perc("OPCODES", ops_cost);
        report.add_cost_perc("PRECOMPILES", precompiled_cost);
        report.add_cost_perc("MEMORY", mem_cost);
        report.ln();
        report.add_cost_perc("TOTAL", total_cost);
        report.ln();
        report.add_cost_perc("FROPS", self.frops_cost);
        report.add_perc("RAM USAGE", self.costs.mops.get_max_ram_address() - RAM_ADDR + 1, 1 << 29);
        report.title_count_cost_perc("COST BY OPCODE", "COUNT", "COST", " RANK");
        self.report_opcodes(&mut report, &self.costs.ops, "OP");

        report.title_count_perc_cost_perc("FROPS BY OPCODE", "COUNT", "HIT", "COST", " RANK");
        self.report_opcodes_hit(&mut report, &self.costs.frops_ops, &self.costs.ops, "FROP");
        if self.coverage {
            StatsCoverageReport::report_opcodes_coverage(
                &self.pc_histogram,
                &mut report,
                &self.costs.ops,
                &self.costs.frops_ops,
                "OPS_COVERAGE",
                rom,
            );
        }

        if !self.rois.is_empty() {
            report.title_autowidth("TOP STEP FUNCTIONS (STEPS, % STEPS, CALLS, FUNCTION)");

            let top_step_rois = self.get_top_rois(true);
            for (index, _) in top_step_rois.iter() {
                let roi = &self.rois[*index];
                let steps = roi.get_steps();
                if steps == 0 {
                    continue;
                }
                report.add_top_step_calls_perc(&roi.name, steps, roi.calls);
            }

            report.title_autowidth("TOP COST FUNCTIONS (COST, % COST, CALLS, FUNCTION)");

            // Create a vector with ROI indices and their steps for sorting
            let top_cost_rois = self.get_top_rois(false);

            let mut final_top_cost_rois = Vec::new();
            for (index, _) in top_cost_rois.iter() {
                let roi = &self.rois[*index];
                let cost = roi.get_cost();
                if cost == 0 {
                    continue;
                }
                final_top_cost_rois.push(*index);
                report.add_top_cost_calls_perc(&roi.name, cost, roi.calls);
            }

            if self.top_rois_detail {
                for index in final_top_cost_rois.iter() {
                    let roi = &self.rois[*index];
                    let mut roi_report = StatsReport::new();
                    roi_report.set_total_cost(roi.get_cost());
                    roi_report.set_steps(roi.get_steps());
                    roi_report.title(&format!("DETAIL FUNCTION {}", roi.name));
                    roi_report.add_perc("STEPS", roi.get_steps(), total_steps);
                    roi_report.add_perc("COST", roi.get_cost(), total_cost);

                    roi_report.set_identation(1);
                    roi_report.title_count_cost_perc("COST BY OPCODE", "COUNT", "COST", " RANK");
                    self.report_opcodes(&mut roi_report, roi.get_ops_costs(), "OP");

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
        if !self.profile_marks.is_empty() {
            let mut keys = self.profile_marks.keys().cloned().collect::<Vec<u16>>();
            keys.sort_by_key(|k| *k);
            report.ln();
            report.title_step_cost_detail_cost(
                "MARK_ID",
                "INDEX",
                "COUNT",
                "STEPS",
                "TOTAL COST",
                "MAIN COST",
                "OPCODE COST",
                "PRECOMPILE COST",
                "MEMORY COST",
            );
            report.add_separator_width(158);
            for id in keys {
                let mark = &self.profile_marks[&id];
                let tag = if let Some(name) = self.profile_tags.get(&id) {
                    name.as_str()
                } else {
                    &format!("{id}")
                };
                if mark.costs.is_empty() && mark.count > 0 {
                    if mark.min_value.is_none() {
                        report.add_step_cost_detail_cost(tag, 0, mark.count, 0, 0, 0, 0, 0, 0, "");
                    } else {
                        report.add_step_cost_detail_cost(
                            tag,
                            0,
                            mark.count,
                            mark.min_value.unwrap_or(0),
                            mark.max_value.unwrap_or(0),
                            0,
                            0,
                            0,
                            0,
                            "",
                        );
                    }
                }
                for (i, costs) in mark.costs.iter().enumerate() {
                    let costs = costs.summary();
                    let main_cost = costs.0 * MAIN_COST;
                    report.add_step_cost_detail_cost(
                        tag,
                        i,
                        mark.count,
                        costs.0,
                        main_cost + costs.1 + costs.2 + costs.3,
                        main_cost,
                        costs.1,
                        costs.2,
                        costs.3,
                        "",
                    );
                }
            }
        }
        report.output
    }
    pub fn add_profile_tag(&mut self, id: u16, name: &str) {
        self.profile_tags.insert(id, name.to_string());
    }
    pub fn add_roi(&mut self, from_pc: u32, to_pc: u32, name: &str) {
        let roi = RegionsOfInterest::new(self.rois.len(), from_pc, to_pc, name);
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
    pub fn set_coverage(&mut self, value: bool) {
        self.coverage = value;
    }
    pub fn set_main_name(&mut self, value: String) {
        self.main_name = value;
    }
    #[cfg(feature = "debug_stats_trace")]
    pub fn debug_stats_trace(&mut self, pc: u64) {
        if self.costs.steps == 1 || self.previous_roi != self.current_roi {
            let func_name = if let Some(roi_index) = self.current_roi {
                &self.rois[roi_index].name
            } else {
                &"".to_string()
            };

            let stack_depth = self.call_stack.len();
            let mut down = false;
            let mut jmp_type = 'J';
            if stack_depth != self.previous_stack_depth {
                for index in self.previous_stack_depth..stack_depth {
                    if index >= self.debug_step_stack.len() {
                        self.debug_step_stack.push(self.costs.steps);
                    } else {
                        self.debug_step_stack[index] = self.costs.steps;
                    }
                }
                down = stack_depth < self.previous_stack_depth;
                jmp_type = if down { 'R' } else { 'C' };
            }

            println!(
                "#T: {:>10} {:>7} {jmp_type} {:>10} 0x{pc:08x} {func_name}",
                self.costs.steps,
                self.call_stack.len(),
                if down { self.costs.steps - self.debug_step_stack[stack_depth - 1] } else { 0 }
            );
            self.previous_stack_depth = stack_depth;
        }
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
    fn add_extras(&mut self, extras: &[(u8, usize)]) {
        for (opcode, count) in extras {
            self.costs.ops[*opcode as usize] += *count as u64;
        }
    }
}
