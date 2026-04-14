//! Emulator execution statistics
//!
//! Statistics include:
//! * Memory read/write counters (aligned and not aligned)
//! * Registers read/write counters (total and per register)
//! * Operations counters (total and per opcode)

use sm_arith::ArithFrops;
use sm_binary::{BinaryBasicFrops, BinaryExtensionFrops};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs::File,
    io::{BufWriter, IsTerminal, Write},
};
use zisk_core::{
    zisk_ops::{OpStats, ZiskOp},
    InstContext, ZiskInst, ZiskOperationType, ZiskRom, REGS_IN_MAIN_TOTAL_NUMBER, ROM_ENTRY,
    ROM_ENTRY_SIZE, ROM_EXIT, SRC_REG,
};

use zisk_definitions::{
    PROFILE_END_COST_ID, PROFILE_END_STEPS_ID, PROFILE_REPORT_END_COST_ID,
    PROFILE_REPORT_END_STEPS_ID, PROFILE_REPORT_START_COST_ID, PROFILE_REPORT_START_STEPS_ID,
    PROFILE_START_COST_ID, PROFILE_START_STEPS_ID,
};

#[cfg(feature = "handle_stdout")]
use zisk_core::{STORE_IND, UART_ADDR};

use crate::{
    CallPathProfiler, OpsCosts, RamMonitor, RegionsOfInterest, StatsCosts, StatsCoverageReport,
    StatsReport, BASE_COST, MAIN_COST, NO_ROI_ID,
};

#[derive(Debug, Clone)]
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

#[derive(Debug)]
pub struct ProfileStats {
    pub tag: String,
    pub max_steps: u64,
    pub min_steps: u64,
    pub total_steps: u64,
    pub steps_calls: u64,
    pub min_cost: u64,
    pub max_cost: u64,
    pub total_cost: u64,
    pub cost_calls: u64,
    pub report_steps: bool,
    pub report_cost: bool,
}

impl Default for ProfileStats {
    fn default() -> Self {
        Self {
            tag: String::new(),
            max_steps: 0,
            min_steps: u64::MAX,
            total_steps: 0,
            steps_calls: 0,
            min_cost: u64::MAX,
            max_cost: 0,
            total_cost: 0,
            cost_calls: 0,
            report_steps: false,
            report_cost: false,
        }
    }
}

/// Keeps statistics of the emulator operations
#[derive(Debug)]
pub struct Stats {
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
    top_histogram: usize,
    legacy_stats: bool,
    sdk: bool,
    sdk_opcodes: bool,
    sdk_profile_tags: bool,
    sdk_top_functions: bool,
    /// PC histogram, i.e. number of times each PC was executed
    pc_histogram: HashMap<u64, u64>,
    previous_pc: u64,
    call_stack: Vec<CallStackEntry>,
    previous_verbose: String,
    is_call: bool,
    is_return: bool,
    call_return_reg: u8,
    profiler: Option<CallPathProfiler>,
    main_name: String,
    track_separator: String,
    use_thousands_sep: bool,
    top_rois_filter: bool,
    disable_call_stack: bool,
    use_colors: bool,
    compact_cost: bool,
    compact_names: Option<usize>,
    sdk_width: usize,
    ram_monitor: RamMonitor,
    profile_tags_map: HashMap<String, usize>,
    profile_tags: Vec<ProfileStats>,
    profile_stack: Vec<(usize, u64)>,
    current_variable_cost: u64,
    #[cfg(feature = "handle_stdout")]
    stdout_data: String,
    #[cfg(feature = "handle_stdout")]
    stdout_step: u64,
    #[cfg(feature = "debug_stats_trace")]
    debug_step_stack: Vec<u64>,
    #[cfg(feature = "debug_stats_trace")]
    previous_stack_depth: usize,
    profiler_output: String,
}

impl Default for Stats {
    /// Default constructor for Stats structure.  Sets all counters to zero.
    fn default() -> Self {
        let mut rois = Vec::with_capacity(4 * 1024);
        rois.push(RegionsOfInterest::new(
            0,
            ROM_ENTRY as u32,
            ROM_ENTRY as u32 + ROM_ENTRY_SIZE as u32,
            "ziskos::BIOS",
            true,
        ));
        let rois_by_address = BTreeMap::from([(ROM_ENTRY as u32, 0)]);
        Self {
            costs: StatsCosts::new_no_compact(),
            regs: [0; REGS_IN_MAIN_TOTAL_NUMBER],
            op_data_buffer: vec![],
            store_ops: false,
            rois,
            rois_by_address,
            current_roi: None,
            previous_roi: None,
            top_rois: 25,
            roi_callers: 10,
            top_rois_detail: false,
            coverage: false,
            legacy_stats: false,
            sdk: false,
            pc_histogram: HashMap::new(),
            previous_pc: 0,
            call_stack: Vec::new(),
            previous_verbose: String::default(),
            is_call: false,
            is_return: false,
            call_return_reg: 0,
            profiler: None,
            // profile_marks: HashMap::new(),
            // individual_cost_marks: false,
            main_name: "main".to_string(),
            top_histogram: 0,
            track_separator: ";".to_string(),
            use_thousands_sep: true,
            top_rois_filter: false,
            disable_call_stack: false,
            use_colors: std::io::stdout().is_terminal(),
            compact_cost: true,
            compact_names: None,
            sdk_width: 120,
            sdk_opcodes: false,
            sdk_profile_tags: false,
            sdk_top_functions: false,
            ram_monitor: RamMonitor::new(),
            profile_tags_map: HashMap::new(),
            profile_tags: Vec::new(),
            profile_stack: Vec::new(),
            current_variable_cost: 0,
            profiler_output: "profile.json.gz".to_string(),
            #[cfg(feature = "handle_stdout")]
            stdout_data: String::with_capacity(256),
            #[cfg(feature = "handle_stdout")]
            stdout_step: 0,
            #[cfg(feature = "debug_stats_trace")]
            debug_step_stack: Vec::new(),
            #[cfg(feature = "debug_stats_trace")]
            previous_stack_depth: 0,
        }
    }
}

impl Stats {
    /// Helper method to clone costs according to compact_cost flag
    fn clone_costs(&self) -> StatsCosts {
        if self.compact_cost {
            self.costs.clone_compact()
        } else {
            self.costs.clone()
        }
    }

    /// Helper method to format ROI names according to compact_names flag
    fn format_roi_name(&self, name: &str) -> String {
        if let Some(max_len) = self.compact_names {
            crate::stats::compact_symbol(name, max_len)
        } else {
            name.to_string()
        }
    }

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

    pub fn is_roi_in_call_stack(&self, roi: usize) -> bool {
        self.call_stack
            .iter()
            .rev()
            .skip(1)
            .any(|entry| entry.called_roi_index.unwrap_or(NO_ROI_ID) == roi)
    }

    pub fn print_call_stack(&self) {
        println!("CALL STACK DUMP (top to bottom):");
        for (i, entry) in self.call_stack.iter().rev().enumerate() {
            if let Some(roi_index) = entry.called_roi_index {
                let formatted_name = self.format_roi_name(&self.rois[roi_index].name);
                println!(
                    "#{} PC:0x{:08X} RA:0x{:08X} ROI[{}]:{} STEPS:{}",
                    i, entry.pc, entry.ra, roi_index, formatted_name, entry.costs.steps
                );
            } else {
                println!(
                    "#{} PC:0x{:08X} RA:0x{:08X} ?????? STEPS:{}",
                    i, entry.pc, entry.ra, entry.costs.steps
                );
            };
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
    fn call_stack_error(&mut self, msg: &str) {
        if self.use_colors {
            println!("\x1B[1;31m{}\x1B[0m", msg);
        } else {
            println!("{}", msg);
        }
        self.disable_call_stack = true;
    }

    pub fn check_roi(&mut self, inst_ctx: &InstContext) {
        if self.disable_call_stack {
            return;
        }
        let pc = inst_ctx.pc as u32;
        #[cfg(feature = "debug_call_stack")]
        let _previous_roi = self.previous_roi;
        self.previous_roi = self.current_roi;

        // First, handle RETURN even if we're not changing ROI
        let return_call = if self.is_return && !self.call_stack.is_empty() {
            #[cfg(feature = "debug_call_stack")]
            {
                let _proi = if let Some(proi) = _previous_roi {
                    &format!("ROI[{proi}] RC:{}", self.rois[proi].call_stack_rc)
                } else {
                    "???"
                };
                let _croi = if let Some(croi) = self.current_roi {
                    &format!("ROI[{croi}] RC:{}", self.rois[croi].call_stack_rc)
                } else {
                    "???"
                };
                println!(
                    "CALL_STACK_DEBUG: RETURN P_PC:0x{:08x} {_proi} => PC:0x{pc:08x} {_croi}",
                    self.previous_pc
                );
            }

            if let Some(profiler) = &mut self.profiler {
                let ram_usage = self.ram_monitor.get_usage(inst_ctx);
                profiler.pop_call_path(self.costs.total_cost(), ram_usage);
            }
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
            if pc == ROM_ENTRY as u32 {
                // Simulate the call to bios and call to start
                self.rois[0].add_absolute_costs(&self.costs);

                if let Some(profiler) = &mut self.profiler {
                    let ram_usage = self.ram_monitor.get_usage(inst_ctx);
                    profiler.push_call_path(0, self.costs.total_cost(), ram_usage);
                    profiler.pop_call_path(self.costs.total_cost(), ram_usage);
                }
            }
            if pc == ROM_EXIT as u32 {
                // Simulate the call to bios and call to start
                if let Some(profiler) = &mut self.profiler {
                    let ram_usage = self.ram_monitor.get_usage(inst_ctx);
                    profiler.pop_call_path(self.costs.total_cost(), ram_usage);
                }
            }

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
                let cloned_costs = self.clone_costs();
                if let Some(top) = self.call_stack.last_mut() {
                    #[cfg(feature = "debug_call_stack")]
                    println!(
                        "CALL_STACK_DEBUG: TAIL CALL P_PC:0x{:08x} => PC:0x{pc:08x}",
                        self.previous_pc
                    );
                    top.tail_calls.push((roi_index, cloned_costs));
                    self.rois[roi_index].tail_jmp(previous_roi_index);
                    self.rois[roi_index].calls += 1;
                }
            }
        }

        // Now handle ROI updates and CALL/JMP
        if let Some(roi_index) = self.current_roi {
            let roi = &mut self.rois[roi_index];
            if pc >= roi.from_pc && pc <= roi.to_pc && !self.is_call && !self.is_return {
                if return_call.is_some() {
                    self.call_stack_error(
                        "ERROR: RETURN CALL unexpected, disabled call stack feature",
                    );
                }
                return;
            }
        }

        if let Some(roi_index) = self.current_roi {
            // At this point ROI change, search the new ROI
            // let roi = &mut self.rois[*index as usize];
            // If return after call, need to add delta costs
            if let Some(return_call) = return_call {
                if return_call.caller_roi_index != Some(roi_index) {
                    self.call_stack_error(
                        "ERROR: STACK MISMATCH DETECTED, disabled call stack feature",
                    );
                    #[cfg(feature = "debug_call_stack")]
                    {
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
                    }
                    return;
                }

                let last_caller_in_stack = self.rois[roi_index].return_call(self.call_stack.len());
                // TODO: check tail call re-entry calls
                // For all tail_call add costs from tail call to now
                let mut processed = HashSet::new();
                for (roi_index, tail_call) in return_call.tail_calls.iter() {
                    // insert returns false if element already exists
                    if !processed.insert(*roi_index) {
                        continue;
                    }
                    if Some(*roi_index) != self.current_roi {
                        if let Err(msg) =
                            self.rois[*roi_index].add_delta_costs(tail_call, &self.costs)
                        {
                            self.call_stack_error(&format!("ERROR: {msg} adding cost to self.rois[{roi_index}], disabled call stack feature"));
                            return;
                        }
                    }
                }

                // At this point we need to update costs of caller inside called ROI
                if let Some(called_roi_index) = return_call.called_roi_index {
                    // update with cost taking as reference the cost when is "called"
                    self.rois[called_roi_index].update_caller(
                        roi_index,
                        &return_call.costs,
                        &self.costs,
                    );
                    if last_caller_in_stack || roi_index != called_roi_index {
                        if let Err(msg) = self.rois[called_roi_index]
                            .add_delta_costs(&return_call.costs, &self.costs)
                        {
                            self.call_stack_error(&format!("ERROR: {msg} adding cost to self.rois[{called_roi_index}], disabled call stack feature"));
                            return;
                        }
                    }
                }
                let roi_steps = self.rois[roi_index].get_steps();
                if roi_steps > self.costs.steps {
                    self.call_stack_error("ERROR: COST OVERFLOW, disabled call stack feature");
                    return;
                }
            }

            if pc >= self.rois[roi_index].from_pc && pc <= self.rois[roi_index].to_pc {
                if self.is_call {
                    if self.is_return {
                        self.call_stack_error(
                            "ERROR: Unexpected RETURN, disabled call stack feature",
                        );
                        return;
                    }
                    let caller_name = if let Some(previous_roi_index) = previous_roi_index {
                        self.rois[previous_roi_index].caller_call();
                        &self.rois[previous_roi_index].name.clone()
                    } else {
                        ""
                    };
                    #[cfg(feature = "debug_call_stack")]
                    println!(
                        "CALL_STACK_DEBUG: CALL P_PC:0x{:08x} => PC:0x{pc:08x} CALLER_ROI:{} CALLED_ROI:{}",
                        self.previous_pc, previous_roi_index.unwrap_or(900_000_000), self.current_roi.unwrap_or(900_000_000)
                    );
                    let mut cloned_costs = self.clone_costs();
                    cloned_costs.steps -= 1; // Current step belongs to the called, we storing the starting point of the called
                    let func_name = self.rois[roi_index].name.clone();
                    self.call_stack.push(CallStackEntry {
                        pc: pc as u64,
                        ra: inst_ctx.regs[REG_RA_IDX],
                        caller_roi_index: previous_roi_index,
                        called_roi_index: self.current_roi,
                        costs: cloned_costs,
                        func_name,
                        return_reg: self.call_return_reg,
                        tail_calls: Vec::new(),
                    });
                    // Fast path: extend directly with the 3 chars, no temporary allocation
                    if let Some(profiler) = &mut self.profiler {
                        let ram_usage = self.ram_monitor.get_usage(inst_ctx);
                        profiler.push_call_path(roi_index, self.costs.total_cost(), ram_usage);
                    }
                    self.call_return_reg = 0;

                    self.rois[roi_index].call(previous_roi_index, self.call_stack.len());

                    // Track call parameters for selected ROIs
                    if self.rois[roi_index].is_selected_roi && self.rois[roi_index].track_calls > 0
                    {
                        self.rois[roi_index].track_call_parameters(
                            &inst_ctx.regs,
                            &self.track_separator,
                            caller_name,
                        );
                    }
                } else if !self.is_return {
                    // JMP: This is a tail call. Replace the top of the call stack if it exists
                    if let Some(top) = self.call_stack.last_mut() {
                        top.pc = pc as u64;
                        top.called_roi_index = Some(roi_index);
                        // Fast replace: truncate last 3 chars and extend with new 3
                        // Assumes call_path.len() >= 3 (always true for tail calls)
                        if let Some(profiler) = &mut self.profiler {
                            let ram_usage = self.ram_monitor.get_usage(inst_ctx);
                            profiler.update_call_path(
                                roi_index,
                                self.costs.total_cost(),
                                ram_usage,
                            );
                        }
                    }
                    self.rois[roi_index].calls += 1;
                    self.rois[roi_index].update_call_depth(self.call_stack.len());
                }
            }
        }
    }
    #[cfg(feature = "handle_stdout")]
    pub fn handle_stdout(&mut self) {}

    #[cfg(feature = "handle_stdout")]
    #[inline(always)]
    pub fn check_stdout(&mut self, instruction: &ZiskInst, inst_ctx: &InstContext) {
        if instruction.store == STORE_IND
            && (instruction.store_offset + inst_ctx.a as i64) as u64 == UART_ADDR
        {
            if (inst_ctx.step - self.stdout_step) > 16 {
                self.stdout_data.clear();
            }
            let _ch = inst_ctx.c as u8 as char;
            if _ch == '\n' {
                if !self.stdout_data.is_empty() {
                    self.handle_stdout();
                }
                self.stdout_data.clear();
            } else {
                if self.stdout_data.len() < 256 {
                    self.stdout_data.push(_ch);
                }
                self.stdout_step = inst_ctx.step;
            }
        }
    }
    pub fn before_op(&mut self) {
        self.current_variable_cost = 0;
    }
    /// Called every time an operation is executed, if statistics are enabled
    pub fn on_op(&mut self, instruction: &ZiskInst, inst_ctx: &InstContext) {
        // println!("##PC## 0x{:08X} STEPS: {}", inst_ctx.pc, self.costs.steps);
        let pc = inst_ctx.pc;
        self.costs.steps += 1;
        #[cfg(feature = "handle_stdout")]
        self.check_stdout(instruction, inst_ctx);
        self.check_roi(inst_ctx);
        #[cfg(feature = "debug_stats_trace")]
        self.debug_stats_trace(pc);

        if instruction.op == ZiskOp::PROFILE {
            let p_data = inst_ctx.mem.read(inst_ctx.a, 8);
            let count = inst_ctx.mem.read(inst_ctx.a + 8, 8);
            let bytes = inst_ctx.mem.read_slice(p_data, count);
            let tag = unsafe { std::str::from_utf8_unchecked(bytes) };
            match inst_ctx.b as u8 {
                PROFILE_START_COST_ID => {
                    self.start_profile_tag(tag, false);
                }
                PROFILE_START_STEPS_ID => {
                    self.start_profile_tag(tag, true);
                }
                PROFILE_REPORT_START_COST_ID => {
                    let id = self.start_profile_tag(tag, false);
                    self.profile_tags[id].report_cost = true;
                }
                PROFILE_REPORT_START_STEPS_ID => {
                    let id = self.start_profile_tag(tag, true);
                    self.profile_tags[id].report_steps = true;
                }
                PROFILE_END_COST_ID => {
                    println!("[{tag}] {}", self.end_profile_tag(tag, false));
                }
                PROFILE_END_STEPS_ID => {
                    println!("[{tag}] {}", self.end_profile_tag(tag, true));
                }
                PROFILE_REPORT_END_COST_ID => {
                    self.end_profile_tag(tag, false);
                }
                PROFILE_REPORT_END_STEPS_ID => {
                    self.end_profile_tag(tag, true);
                }
                _ => panic!("Unknown profile mark type: {}", inst_ctx.b),
            }
            // println!(
            //     "##PROFILE MARK## ID: {:08X} TYPE: {} [0x{p_data:08X},{count}] '{s}'",
            //     inst_ctx.a, inst_ctx.b
            // );
        }
        if self.store_ops
            && (instruction.op_type == ZiskOperationType::Arith
                || instruction.op_type == ZiskOperationType::Binary
                || instruction.op_type == ZiskOperationType::BinaryE)
        {
            // store op, a and b values in file
            self.store_op_data(instruction.op, inst_ctx.a, inst_ctx.b);
        }
        if self.is_frops(instruction, inst_ctx.a, inst_ctx.b) {
            self.costs.add_fixed_frops_cost_op(instruction.op);
        }
        // Otherwise, increase the counter corresponding to this opcode
        else if self.current_variable_cost == 0 {
            self.costs.add_fixed_cost_op(instruction.op);
        } else {
            self.costs.add_variable_cost_op(instruction.op, self.current_variable_cost);
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

    fn start_profile_tag(&mut self, tag: &str, use_steps: bool) -> usize {
        let id = self.profile_tags_map.entry(tag.to_string()).or_insert_with(|| {
            self.profile_tags.push(ProfileStats { tag: tag.to_string(), ..Default::default() });
            self.profile_tags.len() - 1
        });
        if use_steps {
            self.profile_stack.push((*id, self.costs.steps));
            self.profile_tags[*id].steps_calls += 1;
        } else {
            self.profile_stack.push((*id, self.costs.total_cost()));
            self.profile_tags[*id].cost_calls += 1;
        }
        *id
    }
    fn end_profile_tag(&mut self, tag: &str, use_steps: bool) -> u64 {
        let (rtag, reference) = self.profile_stack.pop().unwrap();
        if self.profile_tags.len() > rtag {
            if self.profile_tags[rtag].tag != tag {
                panic!(
                    "Profile tag mismatch: expected '{}', got '{}'",
                    self.profile_tags[rtag].tag, tag
                );
            }
            if use_steps {
                let delta = self.costs.steps - reference;
                self.profile_tags[rtag].total_steps += delta;
                self.profile_tags[rtag].max_steps = self.profile_tags[rtag].max_steps.max(delta);
                self.profile_tags[rtag].min_steps = self.profile_tags[rtag].min_steps.min(delta);
                delta
            } else {
                let delta = self.costs.total_cost() - reference;
                self.profile_tags[rtag].total_cost += delta;
                self.profile_tags[rtag].max_cost = self.profile_tags[rtag].max_cost.max(delta);
                self.profile_tags[rtag].min_cost = self.profile_tags[rtag].min_cost.min(delta);
                delta
            }
        } else {
            panic!("Profile tag index {} not found in profile_tags", rtag);
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

    pub fn get_top_rois(&self, by_step: bool) -> Vec<(usize, u64)> {
        let mut top_rois: Vec<(usize, u64)> = self
            .rois
            .iter()
            .enumerate()
            .filter(|(_, roi)| !self.top_rois_filter || roi.is_selected_roi)
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

    pub fn report_opcodes(&self, report: &mut StatsReport, title: &str, ops: &OpsCosts) {
        let top_opcodes = ops.top_cost_opcodes(5);
        for opcode in ZiskOp::MIN_OPCODE..=ZiskOp::MAX_OPCODE {
            if let Some((count, cost)) = ops.get_opcode_count_and_cost(opcode) {
                if count == 0 {
                    continue;
                }
                if let Ok(inst) = ZiskOp::try_from_code(opcode) {
                    let rank = if let Some(pos) = top_opcodes.iter().position(|&op| op == opcode) {
                        format!(" #{}", pos + 1)
                    } else {
                        String::new()
                    };
                    report.add_count_cost_perc2(
                        &format!("{title} {:}", inst.name()),
                        count as u64,
                        cost,
                        &rank,
                    );
                }
            }
        }
    }

    pub fn report_frops_hit(&self, report: &mut StatsReport, title: &str) {
        let top_opcodes = self.costs.top_cost_frops_opcodes(5);
        for opcode in ZiskOp::MIN_OPCODE..=ZiskOp::MAX_OPCODE {
            if let Some((frops_count, frops_cost)) =
                self.costs.get_opcode_frops_count_and_cost(opcode)
            {
                if frops_count == 0 {
                    continue;
                }
                if let Ok(inst) = ZiskOp::try_from_code(opcode) {
                    if inst.is_precompiled() {
                        // precompiled ops are not frops, skip them
                        continue;
                    }
                    let (no_frops_count, _) =
                        self.costs.get_opcode_count_and_cost(opcode).unwrap_or((0, 0));
                    let rank = if let Some(pos) = top_opcodes.iter().position(|&op| op == opcode) {
                        format!(" #{}", pos + 1)
                    } else {
                        String::new()
                    };
                    report.add_count_perc_cost_perc(
                        &format!("{title} {:}", inst.name()),
                        frops_count as u64,
                        (frops_count as f64 * 100.0) / ((frops_count + no_frops_count) as f64),
                        frops_cost,
                        &rank,
                    );
                }
            }
        }
    }

    fn sdk_report(&self) -> String {
        let ops_cost = self.costs.base_ops_cost();
        let precompiled_cost = self.costs.precompiled_ops_cost();
        let total_steps = self.costs.steps;
        let mem_cost = self.costs.mops.get_cost();
        let main_cost = total_steps * MAIN_COST;
        let base_cost = BASE_COST as u64;
        let total_cost = base_cost + mem_cost + main_cost + ops_cost + precompiled_cost;
        let frops_cost = self.costs.frops_cost();

        // Build SDK report using modular functions
        let mut report = StatsReport::new();
        report.set_total_cost(total_cost);
        report.set_steps(self.costs.steps);

        report.use_thousands_sep = self.use_thousands_sep;
        report.sdk_width = self.sdk_width;

        report.sdk_report_header("REPORT SUMMARY");
        report.sdk_report_summary_line("STEPS", self.costs.steps);
        report.sdk_report_summary_line("COST", total_cost);
        report.sdk_report_summary_data_line(
            "RAM",
            &format!(
                "{:>6.2} MB / {:>6.2} MB",
                self.ram_monitor.ram_used as f64 / (1024.0 * 1024.0),
                self.ram_monitor.ram_size as f64 / (1024.0 * 1024.0)
            ),
        );
        report.sdk_report_footer();

        report.sdk_report_header("COST DISTRIBUTION SUMMARY");
        report.sdk_cost_distribution_title();
        report.sdk_cost_distribution_separator();
        report.sdk_cost_distribution_line("Base", base_cost);
        report.sdk_cost_distribution_line("Main", main_cost);
        report.sdk_cost_distribution_line("Opcodes", ops_cost);
        report.sdk_cost_distribution_line("Precompiles", precompiled_cost);
        report.sdk_cost_distribution_line("Memory", mem_cost);
        report.sdk_cost_distribution_separator();
        report.sdk_cost_distribution_total_line("Total", total_cost);
        report.sdk_report_footer();

        if self.sdk_opcodes {
            report.sdk_report_dual_header("COST DISTRIBUTION BY OPCODE", "OPS vs FROPS");
            report.sdk_cost_frops_title();
            report.sdk_cost_frops_separator();

            let ops = &self.costs.ops_costs();
            let mut cost_frops_opcodes: Vec<(String, u64, Option<u64>)> =
                Vec::with_capacity(ZiskOp::OPCODES_COUNT);
            for opcode in ZiskOp::MIN_OPCODE..=ZiskOp::MAX_OPCODE {
                if let Some((count, cost)) = ops.get_opcode_count_and_cost(opcode) {
                    if count == 0 {
                        continue;
                    }
                    if let Ok(inst) = ZiskOp::try_from_code(opcode) {
                        if inst.is_precompiled() {
                            cost_frops_opcodes.push((inst.name().to_string(), cost, None));
                        } else {
                            let (_, frops_cost) = self
                                .costs
                                .get_opcode_frops_count_and_cost(opcode)
                                .unwrap_or((0, 0));
                            cost_frops_opcodes.push((
                                inst.name().to_string(),
                                cost,
                                Some(frops_cost),
                            ));
                        }
                    }
                }
            }
            cost_frops_opcodes.sort_unstable_by(|a, b| b.1.cmp(&a.1));
            for cost_frops in cost_frops_opcodes.iter().take(10) {
                report.sdk_cost_frops_line(&cost_frops.0, cost_frops.1, cost_frops.2);
            }
            report.sdk_cost_frops_separator();
            report.sdk_cost_frops_total_line("Total", ops_cost + precompiled_cost, frops_cost);
            report.sdk_report_footer();
        }

        if self.sdk_top_functions && !self.rois.is_empty() && !self.disable_call_stack {
            report.sdk_report_header("TOP COST FUNCTIONS");
            let top_cost_rois = self.get_top_rois(false);
            let label_width = report.sdk_top_cost_line_label_width() - 3;

            for (index, (roi_index, _)) in top_cost_rois.iter().enumerate() {
                let roi = &self.rois[*roi_index];
                let cost = roi.get_cost();
                if cost == 0 {
                    continue;
                }
                let formatted_name = crate::stats::compact_symbol(&roi.name, label_width);
                // report.add_top_cost_calls_perc(&formatted_name, cost, roi.calls);
                report.sdk_top_cost_line(&format!("{:>2} {}", index, formatted_name), cost);
            }

            report.sdk_report_footer();
        }

        if self.sdk_profile_tags {
            if self.profile_tags.iter().any(|t| t.report_steps) {
                report.sdk_report_header("STEPS PROFILE TAGS");
                // report.sdk_top_cost_line(label, cost);
                let mut tags_steps: Vec<_> =
                    self.profile_tags.iter().filter(|t| t.report_steps).collect();
                tags_steps.sort_by(|a, b| b.total_steps.cmp(&a.total_steps));
                let label_width = tags_steps.iter().map(|tag| tag.tag.len()).max().unwrap_or(0);
                for tag in tags_steps {
                    report.sdk_tag_step_line(&tag.tag, tag.total_steps, label_width);
                }
                report.sdk_report_footer();
            }
            if self.profile_tags.iter().any(|t| t.report_cost) {
                report.sdk_report_header("COST PROFILE TAGS");
                // report.sdk_top_cost_line(label, cost);
                let mut tags_cost: Vec<_> =
                    self.profile_tags.iter().filter(|t| t.report_cost).collect();
                tags_cost.sort_by(|a, b| b.total_cost.cmp(&a.total_cost));
                let label_width = tags_cost.iter().map(|tag| tag.tag.len()).max().unwrap_or(0);
                for tag in tags_cost {
                    report.sdk_tag_cost_line(&tag.tag, tag.total_cost, label_width);
                }
                report.sdk_report_footer();
            }
        }

        report.output
    }

    fn legacy_report(&self) -> String {
        let ops_cost = self.costs.base_ops_cost();
        let precompiled_cost = self.costs.precompiled_ops_cost();
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
        if self.sdk {
            return self.sdk_report();
        }
        if self.legacy_stats {
            return self.legacy_report();
        }

        // Determine what sections to show based on flags
        // If any new flag is set, respect them; otherwise show all (compatibility mode)
        let using_new_flags = self.sdk_opcodes || self.sdk_profile_tags || self.sdk_top_functions;
        let show_opcodes = self.sdk_opcodes || !using_new_flags;
        let show_top_functions = self.sdk_top_functions || !using_new_flags;
        let show_profile_tags = self.sdk_profile_tags || !using_new_flags;

        // Save profiler data to file if profiling is enabled
        if let Some(profiler) = &self.profiler {
            println!("Saving profiler data to {}...", self.profiler_output);
            profiler.save_to_file(&self.profiler_output, &self.rois).unwrap();
        }

        let ops_cost = self.costs.base_ops_cost();
        let precompiled_cost = self.costs.precompiled_ops_cost();
        let total_steps = self.costs.steps;
        let mem_cost = self.costs.mops.get_cost();
        let main_cost = total_steps * MAIN_COST;
        let base_cost = BASE_COST as u64;
        let total_cost = base_cost + mem_cost + main_cost + ops_cost + precompiled_cost;
        let mut report = StatsReport::new();
        report.use_thousands_sep = self.use_thousands_sep;

        report.set_total_cost(total_cost);
        report.set_steps(total_steps);
        report.title_cost("REPORT", "");
        report.add_cost("STEPS", total_steps);

        report.title_cost_perc("COST DISTRIBUTION", "COST");
        report.add_cost_perc("MAIN", main_cost);
        report.add_cost_perc("OPCODES", ops_cost);
        report.add_cost_perc("PRECOMPILES", precompiled_cost);
        report.add_cost_perc("MEMORY", mem_cost);
        report.add_separator_from(24);
        report.add_cost_perc("VARIABLE", total_cost - base_cost);
        report.add_cost_perc("BASE", base_cost);
        report.add_separator_from(24);
        report.add_cost_perc("TOTAL", total_cost);
        report.ln();
        report.set_total_cost(total_cost - base_cost);
        report.add_cost_perc("FROPS", self.costs.frops_cost());
        if self.ram_monitor.ram_size > 0 {
            report.add_perc("RAM USAGE", self.ram_monitor.ram_used, self.ram_monitor.ram_size);
        }

        if show_opcodes {
            report.title_count_cost_perc2("COST BY OPCODE", "COUNT", "COST", " RANK");
            self.report_opcodes(&mut report, "OP", self.costs.ops_costs());

            report.title_count_perc_cost_perc("FROPS BY OPCODE", "COUNT", "HIT", "COST", " RANK");
            self.report_frops_hit(&mut report, "FROP");
        }

        if self.coverage {
            StatsCoverageReport::report_opcodes_coverage(
                "OPS_COVERAGE",
                &self.pc_histogram,
                &mut report,
                &self.costs,
                rom,
            );
        }

        if show_top_functions && !self.rois.is_empty() && !self.disable_call_stack {
            report.title_auto_width(
                "TOP STEP FUNCTIONS (STEPS, % STEPS, CALLS, STEPS/CALL, FUNCTION)",
            );

            let top_step_rois = self.get_top_rois(true);
            for (index, _) in top_step_rois.iter() {
                let roi = &self.rois[*index];
                let steps = roi.get_steps();
                if steps == 0 {
                    continue;
                }
                let formatted_name = self.format_roi_name(&roi.name);
                report.add_top_step_calls_perc(&formatted_name, steps, roi.calls);
            }

            report.title_auto_width(
                "TOP COST FUNCTIONS (COST, % VARIABLE COST, CALLS, COST/CALL, FUNCTION)",
            );

            // Create a vector with ROI indices and their cost for sorting
            let top_cost_rois = self.get_top_rois(false);

            let mut final_top_cost_rois = Vec::new();
            for (index, _) in top_cost_rois.iter() {
                let roi = &self.rois[*index];
                let cost = roi.get_cost();
                if cost == 0 {
                    continue;
                }
                final_top_cost_rois.push(*index);
                let formatted_name = self.format_roi_name(&roi.name);
                report.add_top_cost_calls_perc(&formatted_name, cost, roi.calls);
            }

            if self.top_rois_detail {
                for index in final_top_cost_rois.iter() {
                    let roi = &self.rois[*index];
                    let mut roi_report = StatsReport::new();
                    roi_report.use_thousands_sep = self.use_thousands_sep;
                    roi_report.set_total_cost(roi.get_cost());
                    roi_report.set_steps(roi.get_steps());
                    let formatted_name = self.format_roi_name(&roi.name);
                    roi_report.title(&format!("DETAIL FUNCTION {}", formatted_name));
                    roi_report.add_perc("STEPS", roi.get_steps(), total_steps);
                    roi_report.add_perc("COST", roi.get_cost(), total_cost);

                    roi_report.set_identation(1);
                    roi_report.title_count_cost_perc("COST BY OPCODE", "COUNT", "COST", " RANK");
                    self.report_opcodes(&mut roi_report, "OP", roi.ops_costs());

                    roi_report.title_top_count_perc("TOP STEP CALLERS (calls, steps)");
                    let mut callers: Vec<_> = roi.get_callers().collect();
                    callers.sort_by(|a, b| b.1.calls.cmp(&a.1.calls));

                    for (index, caller_info) in callers.iter().take(self.roi_callers) {
                        let caller_name = self.format_roi_name(&self.rois[**index].name);
                        roi_report.add_top_count_step_perc(
                            &caller_name,
                            caller_info.calls as u64,
                            caller_info.steps as u64,
                        );
                    }
                    report.add(&roi_report.output);
                }
            }
        }

        if show_profile_tags {
            if self.profile_tags.iter().any(|t| t.report_steps) {
                report.ln();
                // TAG   TOTAL  % TOTAL  CALLS  AVG MIN MAX
                report.title_fixed_width(
                    "PROFILE TAGS STEPS (STEPS, % STEPS, CALLS, AVG, MIN, MAX)",
                    82,
                );
                let mut tags_steps: Vec<_> =
                    self.profile_tags.iter().filter(|t| t.report_steps).collect();
                tags_steps.sort_by(|a, b| b.total_steps.cmp(&a.total_steps));
                for tag in tags_steps {
                    report.add_profile_tag_steps(
                        &tag.tag,
                        tag.total_steps,
                        tag.steps_calls as usize,
                        tag.min_steps,
                        tag.max_steps,
                    );
                }
            }
            if self.profile_tags.iter().any(|t| t.report_cost) {
                report.ln();
                // TAG   TOTAL  % TOTAL  CALLS  AVG MIN MAX
                report.title_fixed_width(
                    "PROFILE TAGS COST (COST, % COST, CALLS, AVG, MIN, MAX)",
                    82,
                );
                let mut tags_cost: Vec<_> =
                    self.profile_tags.iter().filter(|t| t.report_cost).collect();
                tags_cost.sort_by(|a, b| b.total_cost.cmp(&a.total_cost));
                for tag in tags_cost {
                    report.add_profile_tag_cost(
                        &tag.tag,
                        tag.total_cost,
                        tag.cost_calls as usize,
                        tag.min_cost,
                        tag.max_cost,
                    );
                }
            }
        }

        if self.top_histogram > 0 {
            report.title_auto_width("TOP PC HISTOGRAM (EXECUTIONS, % EXECUTIONS, PC)");

            // Convert HashMap to Vec and sort by execution count (descending), then by PC (ascending)
            let mut pc_vec: Vec<_> = self.pc_histogram.iter().collect();
            pc_vec.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));

            // Show only top N entries
            let mut previous_count = 0;
            let mut initial_address = 0;
            let mut block_count = 0;
            let mut block_label = String::new();
            let last_index = std::cmp::min(self.top_histogram, pc_vec.len()) - 1;
            for (index, (pc, count)) in pc_vec.iter().take(self.top_histogram).enumerate() {
                let is_same_block = previous_count == **count
                    && **pc > initial_address
                    && (**pc - initial_address) < 512;

                if is_same_block {
                    block_count += **count;
                } else {
                    if block_count > 0 {
                        report.add_top_step_perc(
                            &format!(" -----------   {block_label}\n"),
                            block_count,
                        );
                    }
                    previous_count = **count;
                    initial_address = **pc;
                    block_count = **count;
                    block_label = if let Some((_, index)) =
                        self.rois_by_address.range(..=initial_address as u32).next_back()
                    {
                        self.format_roi_name(&self.rois[*index as usize].name)
                    } else {
                        String::new()
                    };
                }
                let instruction = rom.get_instruction(**pc);
                let pc_str = format!(" 0x{pc:08x}:   {}", instruction.verbose);
                report.add_top_step_perc(&pc_str, **count);
                if index == last_index {
                    report
                        .add_top_step_perc(&format!(" -----------   {block_label}\n"), block_count);
                }
            }
        }

        report.output
    }
    pub fn add_roi(&mut self, from_pc: u32, to_pc: u32, name: &str) {
        let roi = RegionsOfInterest::new(self.rois.len(), from_pc, to_pc, name, self.compact_cost);
        let index = self.rois.len() as u32;
        self.rois.push(roi);
        self.rois_by_address.insert(from_pc, index);
    }
    pub fn mark_roi_as_selected(&mut self, from_pc: u32, track_calls: usize) {
        if let Some(&index) = self.rois_by_address.get(&from_pc) {
            if let Some(roi) = self.rois.get_mut(index as usize) {
                roi.set_selected_roi(track_calls);
            }
        }
    }
    pub fn init_roi_tracking(&mut self, output_path: &str, separator: &str) -> std::io::Result<()> {
        self.track_separator = separator.to_string();

        // Track used filenames to detect collisions
        let mut used_filenames = std::collections::HashSet::new();

        for roi in &mut self.rois {
            if roi.is_selected_roi && roi.track_calls > 0 {
                // Clean function name: keep only alphanumeric and underscore
                let clean_name: String =
                    roi.name.chars().filter(|c| c.is_alphanumeric() || *c == '_').collect();

                // Check for collision
                let filename = if used_filenames.contains(&clean_name) {
                    // Collision detected, add ROI id
                    format!("{}_roi_{}", clean_name, roi.id)
                } else {
                    clean_name.clone()
                };

                used_filenames.insert(clean_name);
                roi.init_tracking(output_path, separator, &filename)?;
            }
        }
        Ok(())
    }
    pub fn set_track_separator(&mut self, separator: String) {
        self.track_separator = separator;
    }
    pub fn set_use_thousands_sep(&mut self, value: bool) {
        self.use_thousands_sep = value;
    }
    pub fn set_top_rois(&mut self, value: usize) {
        self.top_rois = value;
    }
    pub fn set_top_histogram(&mut self, value: usize) {
        self.top_histogram = value;
    }
    pub fn set_legacy_stats(&mut self, value: bool) {
        self.legacy_stats = value;
    }
    pub fn set_sdk(&mut self, value: bool) {
        self.sdk = value;
    }
    pub fn set_sdk_opcodes(&mut self, value: bool) {
        self.sdk_opcodes = value;
    }
    pub fn set_sdk_profile_tags(&mut self, value: bool) {
        self.sdk_profile_tags = value;
    }
    pub fn set_sdk_top_functions(&mut self, value: bool) {
        self.sdk_top_functions = value;
    }
    pub fn set_sdk_width(&mut self, value: usize) {
        self.sdk_width = value;
    }
    pub fn set_roi_callers(&mut self, value: usize) {
        self.roi_callers = value;
    }
    pub fn set_top_roi_detail(&mut self, value: bool) {
        self.top_rois_detail = value;
        self.compact_cost = !value;
    }
    pub fn set_heap_address(&mut self, heap_bottom: u64, heap_top: u64, heap_pos_address: u64) {
        self.ram_monitor.set_heap_address(heap_bottom, heap_top, heap_pos_address);
    }
    pub fn get_ram_usage(&self, inst_ctx: &InstContext) -> u64 {
        self.ram_monitor.get_usage(inst_ctx)
    }
    pub fn set_coverage(&mut self, value: bool) {
        self.coverage = value;
    }
    pub fn set_main_name(&mut self, value: String) {
        self.main_name = value;
    }
    pub fn set_top_rois_filter(&mut self, value: bool) {
        self.top_rois_filter = value;
    }
    pub fn set_compact_cost(&mut self, value: bool) {
        self.compact_cost = value;
    }
    pub fn set_compact_names(&mut self, max_len: Option<usize>) {
        self.compact_names = max_len;
    }
    pub fn set_profiler_output(&mut self, path: String) {
        self.profiler_output = path;
        if self.profiler.is_none() {
            self.profiler = Some(CallPathProfiler::new());
        }
    }
    pub fn on_finish(&mut self, inst_ctx: &InstContext) {
        self.ram_monitor.on_finish(inst_ctx);
        let ram_usage = self.ram_monitor.ram_used;
        if let Some(profiler) = &mut self.profiler {
            profiler.add_call_path_sample(self.costs.total_cost(), ram_usage);
        }
    }

    /// Write disassembly to file with execution counts
    pub fn write_disassembly(
        &self,
        rom: &ZiskRom,
        path: &str,
        symbols: Option<crate::ElfSymbolReader>,
    ) -> std::io::Result<()> {
        use crate::DisasmWriter;

        let mut disasm_writer = DisasmWriter::new(path)?;
        disasm_writer.set_pc_histogram(self.pc_histogram.clone());
        if let Some(syms) = symbols {
            disasm_writer.set_symbols(syms);
        }
        disasm_writer.write_header("ZisK Disassembly")?;
        disasm_writer.write_disassembly(rom)?;
        disasm_writer.flush()?;

        Ok(())
    }

    #[cfg(feature = "debug_stats_trace")]
    pub fn debug_stats_trace(&mut self, pc: u64) {
        if self.costs.steps == 1 || self.previous_roi != self.current_roi {
            let func_name = if let Some(roi_index) = self.current_roi {
                self.format_roi_name(&self.rois[roi_index].name)
            } else {
                String::new()
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
    fn set_variable_cost(&mut self, cost: u64) {
        self.current_variable_cost = cost;
    }
}
