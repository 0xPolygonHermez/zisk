//! Emulator execution statistics
//!
//! Statistics include:
//! * Memory read/write counters (aligned and not aligned)
//! * Registers read/write counters (total and per register)
//! * Operations counters (total and per opcode)

use fields::Goldilocks;
use riscv::RiscVRegisters;
use sm_arith::ArithFrops;
use sm_binary::{BinaryBasicFrops, BinaryExtensionFrops};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs::File,
    io::{BufWriter, IsTerminal, Write},
};
use zisk_core::{
    zisk_ops::{OpStats, ZiskOp},
    InstContext, ZiskInst, ZiskOperationType, ZiskRom, RAM_ADDR, REGS_IN_MAIN_TOTAL_NUMBER,
    ROM_ADDR, ROM_ENTRY, ROM_ENTRY_SIZE, ROM_EXIT, ROM_SIZE, STORE_NONE, SYS_ADDR,
};
use zisk_pil::RomRomTrace;

use zisk_definitions::{
    PROFILE_END_COST_ID, PROFILE_END_STEPS_ID, PROFILE_REPORT_END_COST_ID,
    PROFILE_REPORT_END_STEPS_ID, PROFILE_REPORT_START_COST_ID, PROFILE_REPORT_START_STEPS_ID,
    PROFILE_START_COST_ID, PROFILE_START_STEPS_ID,
};

#[cfg(feature = "handle_stdout")]
use zisk_core::{STORE_IND, UART_ADDR};

use crate::{
    CallPathProfiler, MemoryOperationsStats, OpsCosts, RamMonitor, RegionsOfInterest, StatsCosts,
    StatsCoverageReport, StatsReport, BASE_COST, MAIN_COST, MEM_WRITE_COST, NO_ROI_ID,
    ROM_READ_COST,
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

const REG_RA_IDX: u8 = 1;
const REG_T0_IDX: u8 = 5;
const RETURN_REGS: [u8; 2] = [REG_RA_IDX, REG_T0_IDX];

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
    rois_by_pc: BTreeMap<u32, u32>,
    rois: Vec<RegionsOfInterest>,
    cached_roi: Option<usize>,
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
    mem_stats: bool,
    mem_full_stats: bool,
    /// PC histogram, i.e. number of times each PC was executed
    pc_histogram: HashMap<u64, u64>,
    previous_pc: u64,
    /// pc of the instruction currently being executed, set once per step; used to give execution
    /// context when reporting an invalid memory access.
    current_pc: u64,
    call_stack: Vec<CallStackEntry>,
    is_call: bool,
    is_return: bool,
    is_tail_call: bool,
    call_return_reg: u8,
    call_return_value: u64,
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
    /// When set, `on_op` prints a per-instruction execution trace (ziskemu's
    /// `--trace-steps`).
    trace_steps: bool,
    /// Change-trace window [trace_from, trace_to]: within it, register and stack
    /// writes are printed right after each instruction trace line (ziskemu's
    /// `--trace-from` / `--trace-to`). `trace_from` defaults to 0, `trace_to` to
    /// unbounded.
    trace_from: Option<u64>,
    trace_to: Option<u64>,
    #[cfg(feature = "handle_stdout")]
    stdout_data: String,
    #[cfg(feature = "handle_stdout")]
    stdout_step: u64,
    #[cfg(feature = "debug_stats_trace")]
    debug_step_stack: Vec<u64>,
    #[cfg(feature = "debug_stats_trace")]
    previous_stack_depth: usize,
    profiler_output: String,
    inst_count: usize,
    rom_init_count: usize,
    ram_init_count: usize,
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
        let rois_by_pc = BTreeMap::from([(ROM_ENTRY as u32, 0)]);
        Self {
            costs: StatsCosts::new_no_compact(),
            regs: [0; REGS_IN_MAIN_TOTAL_NUMBER],
            op_data_buffer: vec![],
            store_ops: false,
            rois,
            rois_by_pc,
            current_roi: None,
            previous_roi: None,
            cached_roi: None,
            top_rois: 25,
            roi_callers: 10,
            top_rois_detail: false,
            coverage: false,
            legacy_stats: false,
            sdk: false,
            pc_histogram: HashMap::new(),
            previous_pc: 0,
            current_pc: 0,
            call_stack: Vec::new(),
            is_call: false,
            is_tail_call: false,
            is_return: false,
            call_return_reg: 0,
            call_return_value: 0,
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
            mem_stats: false,
            mem_full_stats: false,
            ram_monitor: RamMonitor::new(),
            profile_tags_map: HashMap::new(),
            profile_tags: Vec::new(),
            profile_stack: Vec::new(),
            current_variable_cost: 0,
            trace_steps: false,
            trace_from: None,
            trace_to: None,
            profiler_output: "profile.json.gz".to_string(),
            #[cfg(feature = "handle_stdout")]
            stdout_data: String::with_capacity(256),
            #[cfg(feature = "handle_stdout")]
            stdout_step: 0,
            #[cfg(feature = "debug_stats_trace")]
            debug_step_stack: Vec::new(),
            #[cfg(feature = "debug_stats_trace")]
            previous_stack_depth: 0,
            inst_count: 0,
            rom_init_count: 0,
            ram_init_count: 0,
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

    /// Records the pc of the instruction currently being executed (set once per step), so an invalid
    /// memory access can be reported with the pc/function where it happened.
    pub fn set_current_pc(&mut self, pc: u64) {
        self.current_pc = pc;
    }

    /// Called every time some data is read from memory, if statistics are enabled
    pub fn on_memory_read(&mut self, address: u64, width: u64) {
        if !self.costs.memory_read(address, width) {
            self.report_invalid_mem_access("read", address, width);
        }
    }

    /// Called every time some data is writen to memory, if statistics are enabled
    pub fn on_memory_write(&mut self, address: u64, width: u64, value: u64) {
        if !self.costs.memory_write(address, width, value) {
            self.report_invalid_mem_access("write", address, width);
        }
    }

    /// Reports a memory access to an address outside every known region (an unauthorized access, which
    /// would fault on real hardware) with the execution context needed to locate it: the current pc,
    /// step, decoded instruction and enclosing function.
    fn report_invalid_mem_access(&self, kind: &str, address: u64, width: u64) {
        let pc = self.current_pc;
        let func = self
            .rois_by_pc
            .range(..=(pc as u32))
            .next_back()
            .map(|(_, &i)| self.rois[i as usize].name.as_str())
            .unwrap_or("<unknown>");
        panic!(
            "Invalid memory {kind} to 0x{address:08x} (width {width}) at pc=0x{pc:08x} \
             step={} fn='{func}'",
            self.costs.steps
        );
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

    /// Print the full ROI list (index and hex PC range). Used at startup with the
    /// `debug_call_stack` feature so the ROI indices in the call-stack trace can
    /// be mapped back to functions and address ranges.
    #[cfg(feature = "debug_call_stack")]
    pub fn print_rois(&self) {
        println!("CALL_STACK_DEBUG: ROI list ({} entries):", self.rois.len());
        for roi in &self.rois {
            println!(
                "CALL_STACK_DEBUG: ROI[{}] 0x{:08X}-0x{:08X} {}",
                roi.id, roi.from_pc, roi.to_pc, roi.name
            );
        }
    }
    pub fn summary_call_stack(&self) -> String {
        // Compact one-line summary showing only the ROI of each stack entry, in
        // top-of-stack-first order (matching `static_print_call_stack`). With more
        // than 8 entries, show the top 4 and the last 4 elided with "...";
        // otherwise show them all. The trailing `:N` is the total entry count.
        let n = self.call_stack.len();
        let rois: Vec<String> = self
            .call_stack
            .iter()
            .rev()
            .map(|entry| {
                entry.called_roi_index.map(|roi| roi.to_string()).unwrap_or_else(|| "-".to_string())
            })
            .collect();

        let body = if n <= 8 {
            rois.join(", ")
        } else {
            format!("{}, ..., {}", rois[..4].join(", "), rois[n - 4..].join(", "))
        };

        format!("[{body}]:{n}")
    }
    fn call_stack_error(&mut self, msg: &str) {
        if self.use_colors {
            println!("\x1B[1;31mCALL_STACK_ERROR: {}\x1B[0m", msg);
        } else {
            println!("CALL_STACK_ERROR: {}", msg);
        }
        self.disable_call_stack = true;
    }

    pub fn check_roi(&mut self, instruction: &ZiskInst, inst_ctx: &InstContext) {
        if self.disable_call_stack {
            return;
        }
        let pc = inst_ctx.pc as u32;
        let regular_pc = pc & 0x01 == 0;
        self.previous_roi = self.current_roi;

        // First, handle RETURN even if we're not changing ROI
        let return_call = if self.is_return && !self.call_stack.is_empty() {
            #[cfg(feature = "debug_call_stack")]
            self.debug_call_stack_operation("RETURN", instruction, pc);

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
            regular_pc && (pc < roi.from_pc || pc > roi.to_pc)
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
            let roi_pc = instruction.external_ref_addr.unwrap_or(pc as u64) as u32;
            self.current_roi = self.get_cached_roi_index_from_pc(roi_pc);
        }

        if previous_roi_index != self.current_roi {
            if !self.is_return && !self.is_call && !self.is_tail_call && !self.call_stack.is_empty()
            {
                #[cfg(feature = "debug_call_stack")]
                {
                    self.debug_call_stack_operation("NONE", instruction, pc);
                }
                self.call_stack_error(&format!("ROI change without CALL/RETURN, disabled call stack feature (P_PC:0x{:08x} => PC:0x{pc:08x})",
                    self.previous_pc
                ));
                return;
            } else if self.is_tail_call {
                if let Some(roi_index) = self.current_roi {
                    // Tail call
                    let cloned_costs = self.clone_costs();
                    #[cfg(feature = "debug_call_stack")]
                    let summary_call_stack = if self.call_stack.is_empty() {
                        String::new()
                    } else {
                        self.summary_call_stack()
                    };
                    if let Some(top) = self.call_stack.last_mut() {
                        #[cfg(feature = "debug_call_stack")]
                        println!(
                            "CALL_STACK_DEBUG: TAIL CALL P_PC:0x{:08x}[{roi_index}] => PC:0x{pc:08x}[{roi_index}] STACK:{summary_call_stack}",
                            self.previous_pc
                        );
                        top.tail_calls.push((roi_index, cloned_costs));
                        self.rois[roi_index].tail_jmp(previous_roi_index);
                        self.rois[roi_index].calls += 1;
                    }
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
                            self.regs[1], self.previous_pc
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

                let (last_caller_in_stack, ok_return_call) =
                    self.rois[roi_index].return_call(self.call_stack.len());
                if !ok_return_call {
                    self.call_stack_error("RETURN CALL unexpected, disabled call stack feature");
                    return;
                }
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
                        "CALL_STACK_DEBUG: CALL P_PC:0x{:08x}[{}] => PC:0x{pc:08x}[{}] RETURN:0x{:08x} STACK:{}",
                        self.previous_pc,
                        previous_roi_index.unwrap_or(900_000_000),
                        self.current_roi.unwrap_or(900_000_000),
                        self.call_return_value,
                        self.summary_call_stack()
                    );
                    let mut cloned_costs = self.clone_costs();
                    cloned_costs.steps -= 1; // Current step belongs to the called, we storing the starting point of the called
                    let func_name = self.rois[roi_index].name.clone();
                    self.call_stack.push(CallStackEntry {
                        pc: pc as u64,
                        ra: inst_ctx.regs[REG_RA_IDX as usize],
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
    fn valid_code_addr(&self, code_addr: u64) -> bool {
        (ROM_ADDR..(ROM_ADDR + ROM_SIZE)).contains(&code_addr)
            || (ROM_ENTRY..(ROM_ENTRY + ROM_ENTRY_SIZE)).contains(&code_addr)
    }
    #[inline(always)]
    fn change_roi(&mut self, code_addr: u32) -> bool {
        !self.same_roi(code_addr)
    }
    #[inline(always)]
    fn same_roi(&self, code_addr: u32) -> bool {
        self.same_roi_recursive(code_addr).0
    }
    fn same_roi_recursive(&self, code_addr: u32) -> (bool, bool) {
        if let Some(roi_index) = self.current_roi {
            if code_addr & 1 == 0
                && self.rois[roi_index].from_pc <= code_addr
                && code_addr <= self.rois[roi_index].to_pc
            {
                (true, self.rois[roi_index].from_pc == code_addr)
            } else if code_addr & 1 == 1 {
                if let Some((from_pc, to_pc)) = self.rois[roi_index].internal_from_to_pc {
                    (from_pc <= code_addr && code_addr <= to_pc, false)
                } else {
                    (false, false)
                }
            } else {
                (false, false)
            }
        } else {
            (false, false)
        }
    }
    #[cfg(feature = "debug_call_stack")]
    fn debug_call_stack_operation(&mut self, operation: &str, instruction: &ZiskInst, pc: u32) {
        let external_pc = instruction.external_ref_addr.unwrap_or(pc as u64) as u32;
        let pc_roi = self.get_cached_roi_index_from_pc(external_pc);
        let _proi = if let Some(proi) = self.previous_roi {
            &format!("[{proi}] RC:{}", self.rois[proi].call_stack_rc)
        } else {
            "???"
        };
        let _croi = if let Some(croi) = pc_roi {
            &format!("[{croi}] RC:{}", self.rois[croi].call_stack_rc)
        } else {
            "???"
        };
        println!(
            "CALL_STACK_DEBUG: {operation} P_PC:0x{:08x}{_proi} => PC:0x{pc:08x}{_croi} STACK:{}",
            self.previous_pc,
            self.summary_call_stack()
        );
    }
    fn check_call_return(
        &mut self,
        call_addr: i64,
        return_addr: i64,
        allow_recursive: bool,
    ) -> bool {
        let call_addr = call_addr as u64;
        let return_addr = return_addr as u64;
        if !self.valid_code_addr(call_addr) || !self.valid_code_addr(return_addr) {
            return false;
        }
        let (same_roi, recursive) = self.same_roi_recursive(call_addr as u32);
        let call_addr_ok = if allow_recursive && recursive {
            #[cfg(feature = "debug_call_stack")]
            println!(
                "CALL_STACK_DEBUG: RECURSIVE DETECTED CALL:{:x} RA:{:x} STACK:{}",
                call_addr,
                return_addr,
                self.summary_call_stack()
            );
            true
        } else {
            !same_roi
        };
        call_addr_ok && self.same_roi(return_addr as u32)
    }
    fn check_call(&mut self, return_addr: i64) -> bool {
        let return_addr = return_addr as u64;
        if !self.valid_code_addr(return_addr) {
            return false;
        }
        self.change_roi(return_addr as u32)
    }
    pub fn get_roi_from_pc(&mut self, pc: u32) -> Option<(usize, &RegionsOfInterest)> {
        assert!(pc & 0x01 == 0);
        if let Some((_, index)) = self.rois_by_pc.range(..=pc).next_back() {
            Some((*index as usize, &self.rois[*index as usize]))
        } else {
            None
        }
    }
    pub fn get_cached_roi_index_from_pc(&mut self, pc: u32) -> Option<usize> {
        assert!(pc & 0x01 == 0);
        if let Some(roi_index) = self.cached_roi {
            let roi = &self.rois[roi_index];
            if pc >= roi.from_pc && pc <= roi.to_pc {
                return Some(roi_index);
            }
        }

        if let Some((_, index)) = self.rois_by_pc.range(..=pc).next_back() {
            let roi_index = *index as usize;
            self.cached_roi = Some(roi_index);
            Some(roi_index)
        } else {
            None
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
    pub fn on_op(&mut self, inst: &ZiskInst, inst_ctx: &InstContext, store_value: u64) {
        // Per-instruction execution trace, enabled by ziskemu's `--trace-steps`, or
        // by the change-trace window (`--trace-from` / `--trace-to`) from the requested
        // step onwards (so that each change block is preceded by its instruction). At
        // this point `self.costs.steps` is the current step (it is incremented below).
        if self.trace_steps
            || ((self.trace_from.is_some() || self.trace_to.is_some())
                && self.costs.steps >= self.trace_from.unwrap_or(0)
                && self.trace_to.map_or(true, |to| self.costs.steps <= to))
        {
            println!("### S:{} PC {:x}: {}", self.costs.steps, inst_ctx.pc, inst.verbose);
        }
        let pc = inst_ctx.pc;
        self.costs.steps += 1;
        #[cfg(feature = "handle_stdout")]
        self.check_stdout(inst, inst_ctx);
        self.check_roi(inst, inst_ctx);
        #[cfg(feature = "debug_stats_trace")]
        self.debug_stats_trace(pc);

        if inst.op == ZiskOp::PROFILE {
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
            && (inst.op_type == ZiskOperationType::Arith
                || inst.op_type == ZiskOperationType::Binary
                || inst.op_type == ZiskOperationType::BinaryE)
        {
            // store op, a and b values in file
            self.store_op_data(inst.op, inst_ctx.a, inst_ctx.b);
        }
        if self.is_frops(inst, inst_ctx.a, inst_ctx.b) {
            self.costs.add_fixed_frops_cost_op(inst.op);
        }
        // Otherwise, increase the counter corresponding to this opcode
        else if self.current_variable_cost == 0 {
            self.costs.add_fixed_cost_op(inst.op);
        } else {
            self.costs.add_variable_cost_op(inst.op, self.current_variable_cost);
        }

        // Increase the PC histogram entry for this PC
        self.pc_histogram.entry(pc).and_modify(|count| *count += 1).or_insert(1);

        self.is_call = false;
        self.is_return = false;
        self.call_return_reg = 0;
        self.call_return_value = 0;
        self.is_tail_call = false;
        let _is_rs1_return_reg = if let Some(meta_rs1) = inst.meta_rs1 {
            RETURN_REGS.contains(&meta_rs1)
        } else {
            false
        };
        let non_rd = if let Some(rd) = inst.meta_rd { rd == 0 } else { false };
        if !non_rd {
            // CALL path
            match inst.op {
                ZiskOp::AND
                    if inst.store_pc
                        && inst.set_pc
                        && self.check_call_return(
                            inst_ctx.c as i64 + inst.jmp_offset1,
                            pc as i64 + inst.jmp_offset2,
                            true,
                        ) =>
                {
                    self.is_call = true;
                    self.call_return_reg = inst.store_offset as u8;
                    self.call_return_value = store_value;
                }
                ZiskOp::COPYB
                    if !inst.store_pc
                        && !inst.set_pc
                        && inst.jmp_offset1 == inst.jmp_offset2
                        && self.check_call_return(
                            pc as i64 + inst.jmp_offset2,
                            inst_ctx.b as i64,
                            false,
                        ) =>
                {
                    self.is_call = true;
                    self.call_return_reg = inst.store_offset as u8;
                    self.call_return_value = store_value;
                }
                ZiskOp::FLAG
                    if inst.store_pc
                        && !inst.set_pc
                        && self.check_call_return(
                            pc as i64 + inst.jmp_offset1,
                            pc as i64 + inst.jmp_offset2,
                            true,
                        ) =>
                {
                    self.is_call = true;
                    self.call_return_reg = inst.store_offset as u8;
                    self.call_return_value = store_value;
                }
                _ => {}
            }
        } else if inst.store == STORE_NONE || non_rd {
            // RETURN path

            if let Some(meta_reg) = inst.meta_rs1 {
                if RETURN_REGS.contains(&meta_reg) {
                    self.is_return = match inst.op {
                        ZiskOp::AND => {
                            !inst.store_pc
                                && inst.set_pc
                                && self.check_call(inst_ctx.c as i64 + inst.jmp_offset2)
                        }
                        ZiskOp::COPYB => {
                            !inst.store_pc
                                && !inst.set_pc
                                && inst.jmp_offset1 == inst.jmp_offset2
                                && self.check_call(pc as i64 + inst.jmp_offset2)
                        }
                        _ => false,
                    };
                } else {
                    self.is_tail_call = match inst.op {
                        ZiskOp::AND => {
                            !inst.store_pc
                                && inst.set_pc
                                && self.check_call(inst_ctx.c as i64 + inst.jmp_offset2)
                        }
                        ZiskOp::COPYB => {
                            !inst.store_pc
                                && !inst.set_pc
                                && inst.jmp_offset1 == inst.jmp_offset2
                                && self.check_call(pc as i64 + inst.jmp_offset2)
                        }
                        ZiskOp::FLAG => {
                            !inst.store_pc
                                && !inst.set_pc
                                && self.check_call(pc as i64 + inst.jmp_offset1)
                        }
                        _ => false,
                    };
                }
            }
        }
        self.previous_pc = pc;
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
        top_rois.sort_by_key(|a| std::cmp::Reverse(a.1));

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

    pub fn report_mem(
        &self,
        report: &mut StatsReport,
        mops: &MemoryOperationsStats,
        partial_report: bool,
    ) {
        let previous_hidden_no_cost = report.set_hidden_no_cost(true);
        let (rom_init_count, ram_init_count) = if partial_report {
            (0, 0)
        } else {
            (self.rom_init_count as u64, self.ram_init_count as u64)
        };
        report.set_custom_totals(
            mops.get_count() + rom_init_count + ram_init_count,
            mops.get_cost() + rom_init_count * ROM_READ_COST + ram_init_count * MEM_WRITE_COST,
        );
        report.add_count_cost_perc2_custom(
            "RAM STACK ALIGNED",
            mops.get_ram_stack_aligned_count(),
            mops.get_ram_stack_aligned_cost(),
            "",
        );
        report.add_count_cost_perc2_custom(
            "RAM NO STACK ALIGNED",
            mops.get_ram_no_stack_aligned_count(),
            mops.get_ram_no_stack_aligned_cost(),
            "",
        );
        report.add_count_cost_perc2_custom(
            "RAM INIT",
            ram_init_count,
            ram_init_count * MEM_WRITE_COST,
            "",
        );
        report.add_count_cost_perc2_custom(
            "ROM ALIGNED",
            mops.get_rom_aligned_count(),
            mops.get_rom_aligned_cost(),
            "",
        );
        report.add_count_cost_perc2_custom(
            "ROM INIT",
            rom_init_count,
            rom_init_count * ROM_READ_COST,
            "",
        );
        report.add_count_cost_perc2_custom(
            "INPUT ALIGNED",
            mops.get_input_aligned_count(),
            mops.get_input_aligned_cost(),
            "",
        );
        report.add_count_cost_perc2_custom(
            "RAM STACK UNALIGNED",
            mops.get_ram_stack_unaligned_count(),
            mops.get_ram_stack_unaligned_cost(),
            "",
        );
        report.add_count_cost_perc2_custom(
            "RAM NO STACK UNALIGNED",
            mops.get_ram_no_stack_unaligned_count(),
            mops.get_ram_no_stack_unaligned_cost(),
            "",
        );
        report.add_count_cost_perc2_custom(
            "ROM UNALIGNED",
            mops.get_rom_unaligned_count(),
            mops.get_rom_unaligned_cost(),
            "",
        );
        report.add_count_cost_perc2_custom(
            "INPUT UNALIGNED",
            mops.get_input_unaligned_count(),
            mops.get_input_unaligned_cost(),
            "",
        );
        report.add_total_line();
        report.add_count_cost_perc2_custom(
            "TOTAL ALIGNED",
            mops.get_aligned_count() + ram_init_count + rom_init_count,
            mops.get_aligned_cost()
                + ram_init_count * MEM_WRITE_COST
                + rom_init_count * ROM_READ_COST,
            "",
        );
        report.add_count_cost_perc2_custom(
            "TOTAL UNALIGNED",
            mops.get_unaligned_count(),
            mops.get_unaligned_cost(),
            "",
        );
        report.add_total_line();
        report.add_count_cost_perc2_custom(
            "TOTAL RAM STACK",
            mops.get_ram_stack_count(),
            mops.get_ram_stack_cost(),
            "",
        );
        report.add_count_cost_perc2_custom(
            "TOTAL RAM NO STACK",
            mops.get_ram_no_stack_count(),
            mops.get_ram_no_stack_cost(),
            "",
        );
        report.add_total_line();
        report.add_count_cost_perc2_custom(
            "TOTAL RAM",
            mops.get_ram_count() + ram_init_count,
            mops.get_ram_cost() + ram_init_count * MEM_WRITE_COST,
            "",
        );
        report.add_count_cost_perc2_custom(
            "TOTAL ROM",
            mops.get_rom_count() + rom_init_count,
            mops.get_rom_cost() + rom_init_count * ROM_READ_COST,
            "",
        );
        report.add_count_cost_perc2_custom(
            "TOTAL INPUT",
            mops.get_input_count(),
            mops.get_input_cost(),
            "",
        );
        report.set_hidden_no_cost(previous_hidden_no_cost);
    }

    pub fn report_detailed_mem(
        &self,
        report: &mut StatsReport,
        mops: &MemoryOperationsStats,
        partial_report: bool,
    ) {
        report.set_custom_totals(mops.get_count(), mops.get_cost());
        let (rom_init_count, ram_init_count) = if partial_report {
            (0, 0)
        } else {
            (self.rom_init_count as u64, self.ram_init_count as u64)
        };

        let report_items = mops.get_detailed_items(rom_init_count, ram_init_count);
        for (title, count, cost) in report_items {
            if title.is_empty() {
                report.add_total_line();
            } else {
                report.add_count_cost_perc2_custom(&title, count, cost, "");
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
            cost_frops_opcodes.sort_unstable_by_key(|a| std::cmp::Reverse(a.1));
            for cost_frops in cost_frops_opcodes.iter().take(10) {
                report.sdk_cost_frops_line(&cost_frops.0, cost_frops.1, cost_frops.2);
            }
            report.sdk_cost_frops_separator();
            report.sdk_cost_frops_total_line("Total", ops_cost + precompiled_cost, frops_cost);
            report.sdk_report_footer();
        }

        if self.sdk_top_functions && self.is_rois_enabled() && !self.disable_call_stack {
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
                tags_steps.sort_by_key(|tag| std::cmp::Reverse(tag.total_steps));
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
                tags_cost.sort_by_key(|t| std::cmp::Reverse(t.total_cost));
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
        let mem_cost = self.costs.mops.get_cost()
            + self.rom_init_count as u64 * ROM_READ_COST
            + self.ram_init_count as u64 * MEM_WRITE_COST;
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
        let rom_used_rows =
            self.inst_count + self.rom_init_count.div_ceil(4) + self.ram_init_count.div_ceil(4);
        let rom_size = RomRomTrace::<Goldilocks>::NUM_ROWS;
        report.add_perc("ROM USAGE", rom_used_rows as u64, rom_size as u64);

        if self.mem_stats || self.mem_full_stats {
            report.title_count_cost_perc2("MEM COST BY TYPE", "COUNT", "COST", "");
            self.report_mem(&mut report, &self.costs.mops, false);
        }

        if self.mem_full_stats {
            report.set_and_push_label_width(40);
            report.title_count_cost_perc2("DETAILED MEM COST", "COUNT", "COST", "");
            self.report_detailed_mem(&mut report, &self.costs.mops, false);
            report.pop_label_width();
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

        if show_top_functions && self.is_rois_enabled() && !self.disable_call_stack {
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
                    roi_report.set_identation(1);
                    roi_report.add_perc("STEPS", roi.get_steps(), total_steps);
                    let main_cost = roi.get_steps() * MAIN_COST;
                    let ops_cost = roi.get_ops_cost();
                    let precompiled_cost = roi.get_precompiled_cost();
                    let mem_cost = roi.get_mem_cost();
                    let total_cost = main_cost + ops_cost + precompiled_cost + mem_cost;
                    roi_report.ln();
                    roi_report.add_perc("MAIN COST", main_cost, total_cost);
                    roi_report.add_perc("OPCODES COST", ops_cost, total_cost);
                    roi_report.add_perc("PRECOMPILES COST", precompiled_cost, total_cost);
                    roi_report.add_perc("MEMORY COST", mem_cost, total_cost);
                    roi_report.add_perc_total_line();
                    roi_report.add_perc("TOTAL COST", total_cost, total_cost);

                    if self.mem_stats {
                        roi_report.title_count_cost_perc2("MEM COST BY TYPE", "COUNT", "COST", "");
                        self.report_mem(&mut roi_report, &roi.costs.mops, true);
                    }

                    if self.mem_full_stats {
                        roi_report.set_and_push_label_width(40);
                        roi_report.title_count_cost_perc2("DETAILED MEM COST", "COUNT", "COST", "");
                        self.report_detailed_mem(&mut roi_report, &roi.costs.mops, true);
                        roi_report.pop_label_width();
                    }

                    roi_report.title_count_cost_perc("COST BY OPCODE", "COUNT", "COST", " RANK");
                    self.report_opcodes(&mut roi_report, "OP", roi.ops_costs());

                    roi_report.title_top_count_perc("TOP STEP CALLERS (calls, steps)");
                    let mut callers: Vec<_> = roi.get_callers().collect();
                    callers.sort_by_key(|a| std::cmp::Reverse(a.1.calls));

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
                tags_steps.sort_by_key(|tag| std::cmp::Reverse(tag.total_steps));
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
                tags_cost.sort_by_key(|t| std::cmp::Reverse(t.total_cost));
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
                        self.rois_by_pc.range(..=initial_address as u32).next_back()
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
        self.rois_by_pc.insert(from_pc, index);
    }
    pub fn update_roi_internal_pc_range(
        &mut self,
        roi_index: usize,
        from_pc: u32,
        to_pc: u32,
    ) -> bool {
        if let Some((defined_from_pc, defined_to_pc)) =
            self.rois[roi_index].update_internal_from_to_pc(from_pc, to_pc)
        {
            self.call_stack_error(&format!("Internal from/to PC range is already set for ROI \
                [{roi_index}]. Disabled call stack feature. [0x{defined_from_pc:08x}-0x{defined_to_pc:08x}],\
                [0x{from_pc:08x}-0x{to_pc:08x}]"));
            false
        } else {
            true
        }
    }
    pub fn load_rom_data(&mut self, rom: &ZiskRom) {
        self.inst_count = rom.get_instruction_count();
        self.rom_init_count = rom.get_rom_init_64bit_words();
        self.ram_init_count = rom.get_ram_init_64bit_words();
        if self.is_rois_enabled() {
            self.load_internal_pc_ranges(rom);
        }
    }
    pub fn is_rois_enabled(&self) -> bool {
        self.rois.len() > 1
    }
    fn load_internal_pc_ranges(&mut self, rom: &ZiskRom) {
        let mut internal_pc = ROM_ADDR + 1;
        let mut current_roi_index = None;
        let mut current_roi_from = 0;
        let mut current_roi_to = 0;
        let mut current_internal_from = None;
        let mut current_internal_to = None;
        while let Some(inst) = rom.get_internal_instruction(internal_pc) {
            let pc = inst.external_ref_addr.unwrap() as u32;
            if pc >= current_roi_from && pc <= current_roi_to {
                current_internal_to = Some(internal_pc as u32);
            } else {
                if let Some(internal_from) = current_internal_from {
                    if let Some(roi_index) = current_roi_index {
                        if !(self.update_roi_internal_pc_range(
                            roi_index,
                            internal_from,
                            current_internal_to.unwrap_or(internal_from),
                        )) {
                            return;
                        }
                    }
                }
                if let Some((roi_index, roi)) = self.get_roi_from_pc(pc) {
                    current_roi_index = Some(roi_index);
                    current_roi_from = roi.from_pc;
                    current_roi_to = roi.to_pc;
                    current_internal_from = Some(internal_pc as u32);
                    current_internal_to = None;
                }
            }

            internal_pc += 2;
        }
        if let Some(internal_from) = current_internal_from {
            if let Some(roi_index) = current_roi_index {
                self.update_roi_internal_pc_range(
                    roi_index,
                    internal_from,
                    current_internal_to.unwrap_or(internal_from),
                );
            }
        }
    }
    pub fn mark_roi_as_selected(&mut self, from_pc: u32, track_calls: usize) {
        if let Some(&index) = self.rois_by_pc.get(&from_pc) {
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
    pub fn set_trace_steps(&mut self, value: bool) {
        self.trace_steps = value;
    }
    pub fn set_trace_changes(&mut self, from: Option<u64>, to: Option<u64>) {
        self.trace_from = from;
        self.trace_to = to;
    }

    /// Print a register write as `reg name: prev (0xhex) => post (0xhex)`.
    /// Called from `store_c` right after the register is updated, only when change
    /// tracing is active. No-op when the value did not actually change.
    pub fn trace_register_change(&self, reg: usize, prev: u64, new: u64) {
        if prev == new {
            return;
        }
        let name = RiscVRegisters::name_from_usize(reg).unwrap_or("?");
        println!("    reg x{reg} ({name}): {prev} (0x{prev:x}) => {new} (0x{new:x})");
    }

    /// Print a stack write as `abs_addr [sp+/-off]: prev (0xhex) => post (0xhex)`.
    /// A "stack" write is any RAM write in the range [RAM_ADDR, SYS_ADDR); writes
    /// outside that range are ignored. `sp` is the current value of register x2.
    pub fn trace_stack_change(&self, addr: u64, prev: u64, new: u64, sp: u64) {
        if !(RAM_ADDR..SYS_ADDR).contains(&addr) {
            return;
        }
        let off = addr as i64 - sp as i64;
        let rel = if off >= 0 { format!("sp+0x{off:x}") } else { format!("sp-0x{:x}", -off) };
        println!("    stack 0x{addr:x} [{rel}]: {prev} (0x{prev:x}) => {new} (0x{new:x})");
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
    pub fn set_mem_stats(&mut self, value: bool) {
        self.mem_stats = value;
    }
    pub fn set_mem_full_stats(&mut self, value: bool) {
        self.mem_full_stats = value;
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
                "#T: {:>10} {:>7} {jmp_type} {:>10} PC {:x}: ({}) => {pc:x}: ({}) {func_name}",
                self.costs.steps,
                self.call_stack.len(),
                if down { self.costs.steps - self.debug_step_stack[stack_depth - 1] } else { 0 },
                self.previous_pc,
                self.previous_roi.unwrap_or(0),
                self.current_roi.unwrap_or(0),
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
