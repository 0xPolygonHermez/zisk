//! Emulator execution statistics
//!
//! Statistics include:
//! * Memory read/write counters (aligned and not aligned)
//! * Registers read/write counters (total and per register)
//! * Operations counters (total and per opcode)

use fields::Goldilocks;
use riscv::RiscVRegisters;
use sm_arith::{ArithFrops, ArithLegacyFrops};
use sm_binary::{
    BinaryBasicFrops, BinaryBasicLegacyFrops, BinaryExtensionFrops, BinaryExtensionLegacyFrops,
};
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
    CallPathProfiler, MemoryOperationsStats, OpsCosts, OpsCount, RamMonitor, RegionsOfInterest,
    StatsCosts, StatsCoverageReport, StatsReport, BASE_COST, BINARY_ADD_HI_COST, MAIN_COST,
    MEM_ACCESS_INVALID, MEM_ACCESS_MONITOR, MEM_WRITE_COST, NO_ROI_ID, ROM_READ_COST,
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
const CAT_MASK: [u64; 3] = [0xFFFF_FFFF_0000_0000, 0xFFFF_FFFF_FFFF_0000, 0x0000_0000_FFFF_FFFF];
const OP_MASK_CATEGORIES: usize = 4 * 3;
const OP_BIN_EXT_LT_64_CATS: usize = 2;
const OP_CATEGORIES: usize = OP_MASK_CATEGORIES + OP_BIN_EXT_LT_64_CATS;
const OP_BIN_EXT_A_LT_64_CAT: usize = OP_MASK_CATEGORIES;
const OP_BIN_EXT_B_LT_64_CAT: usize = OP_BIN_EXT_A_LT_64_CAT + 1;

const REG_RA_IDX: u8 = 1;
const REG_T0_IDX: u8 = 5;
const RETURN_REGS: [u8; 2] = [REG_RA_IDX, REG_T0_IDX];

// ------------------------------------------------------------------------------------------------
// Cheap opcode variants: opcodes whose operands meet a condition that makes them cheaper to prove,
// counted separately so their reduced cost can be accounted for and shown as a per-opcode
// breakdown. A general mechanism — currently the BinaryAddHi shapes of ADD:
//   add_hi0 : hi32(a)=hi32(c)=0 and hi32(b)=0            (both operands fit in 32 bits)
//   add_hif : hi32(a)=hi32(c)=0 and hi32(b)=0xFFFF_FFFF  (a subtraction encoded as an addition)
// ------------------------------------------------------------------------------------------------
const ADD_CODE: u8 = ZiskOp::Add.code();
const HI32_MASK: u64 = 0xFFFF_FFFF_0000_0000;
const CHEAP_ADD_HI0: usize = 0;
const CHEAP_ADD_HIF: usize = 1;
const CHEAP_VARIANT_COUNT: usize = 2;
/// (base opcode, label, reduced cost) for each cheap variant, indexed by `CHEAP_*`.
const CHEAP_VARIANTS: [(u8, &str, u64); CHEAP_VARIANT_COUNT] =
    [(ADD_CODE, "add_hi0", BINARY_ADD_HI_COST), (ADD_CODE, "add_hif", BINARY_ADD_HI_COST)];

// ------------------------------------------------------------------------------------------------
// Precompile duplicate analysis: detect precompile calls that repeat the same computation (same
// input content → same output) so the redundant proving cost (the potential deduplication saving)
// can be reported. Works for every precompile except DMA — the ones with a content descriptor
// below. The key is the operand *content*, not the `b` address, so identical inputs stored at
// different memory buffers are still detected as duplicates.
// ------------------------------------------------------------------------------------------------
/// Maximum number of innermost call-stack ROIs (leaf + callers) recorded per precompile call, so
/// the detail report can show where the duplicates come from. The actual depth is configurable
/// (`--duplicates-depth`) up to this maximum.
const DUP_MAX_ROI_DEPTH: usize = 8;

/// One segment of a precompile's input content, describing where to read operand words from,
/// relative to the parameter block at `ctx.b` (all addresses are 8-byte words).
#[derive(Clone, Copy)]
enum DupSeg {
    /// In-place operands: read `words` consecutive words starting at `ctx.b` (no indirection),
    /// appended after any previous `Direct` segment.
    Direct { words: usize },
    /// Indirected operand: the param at `ctx.b + 8*param` is a pointer; read `words` words from it.
    Indirect { param: usize, words: usize },
    /// Inline scalar: the param at `ctx.b + 8*param` is a literal value (e.g. add256 carry-in,
    /// blake2 block index), included as a single content word.
    Literal { param: usize },
}

/// Per-precompile duplicate-analysis state.
#[derive(Debug, Default)]
struct DupStats {
    /// Occurrence count of each input-content fingerprint (the number of words is precompile-fixed).
    states: HashMap<Box<[u64]>, u32>,
    /// Per-call-path stats: the innermost `duplicates_depth` ROIs (leaf first, padded with
    /// `usize::MAX`) → (total calls on that path, of which duplicates). Lets the detail report
    /// attribute the redundant calls to their call paths.
    call_paths: HashMap<[usize; DUP_MAX_ROI_DEPTH], (u64, u64)>,
}

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
    /// Flag to use the legacy FROPS tables when computing FROPS coverage statistics
    legacy_frops: bool,
    /// Sort opcode/precompiled/FROPS report sections by operation count instead of by cost
    sort_by_units: bool,
    costs: StatsCosts,
    // Global costs
    op_categories: OpsCount<OP_CATEGORIES>,
    /// Execution count of each cheap opcode variant (see `CHEAP_VARIANTS`), e.g. add_hi0 / add_hif.
    cheap_variant_counts: [u64; CHEAP_VARIANT_COUNT],
    /// Show the per-opcode breakdown of cheap variants in the report (`--opcode-breakdown`).
    opcode_breakdown: bool,
    /// Analyze duplicate precompile calls: count how many calls repeat the same input content
    /// (same input → same output) and the cost spent on those repeats, per precompile
    /// (`--duplicates`).
    duplicates: bool,
    /// Restrict the duplicate analysis to these opcodes (`--duplicates-ops`). `None` = every
    /// supported precompile (all with a content descriptor, i.e. all except DMA).
    duplicates_ops: Option<HashSet<u8>>,
    /// How many innermost call-stack ROIs to record per precompile call for the detail report
    /// (`--duplicates-depth`), clamped to `1..=DUP_MAX_ROI_DEPTH`.
    duplicates_depth: usize,
    /// Show the per-precompile call-path detail (level 2) in the report (`--duplicates-detail`).
    duplicates_detail: bool,
    /// Per-precompile duplicate-analysis state, keyed by opcode.
    dup_stats: HashMap<u8, DupStats>,
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
    /// Accumulate the per-offset byte read/write counters (MEM_OFFSETS). Enabled by
    /// `--mem-full-stats` for the on-screen table, and by `--save-stats` / `--ref-stats` so the
    /// snapshot always includes them.
    collect_offsets: bool,
    /// Log every costly unaligned memory access (double 4B/8B, i.e. `MEM_ACCESS_MONITOR`) with its
    /// execution context. Off by default; enabled by `--log-costly-unaligned`.
    log_costly_unaligned: bool,
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
    /// Call-stack tracking mode. `false` (auto, default): on a stack mismatch, resync by unwinding
    /// the collapsed self-recursive frames (handles GCC/C++ tail-recursion, e.g. std::sort's
    /// __introsort_loop). `true` (strict): the original behaviour — report the mismatch and disable
    /// the call stack.
    callstack_strict: bool,
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
    byte_reads: [u64; 8],
    byte_clean_writes: [u64; 8],
    byte_dirty_writes: [u64; 8],
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
            legacy_frops: false,
            sort_by_units: false,
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
            callstack_strict: false,
            use_colors: std::io::stdout().is_terminal(),
            compact_cost: true,
            compact_names: None,
            sdk_width: 120,
            sdk_opcodes: false,
            sdk_profile_tags: false,
            sdk_top_functions: false,
            mem_stats: false,
            mem_full_stats: false,
            collect_offsets: false,
            log_costly_unaligned: false,
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
            op_categories: OpsCount::<OP_CATEGORIES>::new(),
            cheap_variant_counts: [0; CHEAP_VARIANT_COUNT],
            opcode_breakdown: false,
            duplicates: false,
            duplicates_ops: None,
            duplicates_depth: 4,
            duplicates_detail: false,
            dup_stats: HashMap::new(),
            byte_reads: [0; 8],
            byte_clean_writes: [0; 8],
            byte_dirty_writes: [0; 8],
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
        if self.collect_offsets && width == 1 {
            self.byte_reads[(address as usize) & 0x7] += 1;
        }
        let status = self.costs.memory_read(address, width);
        self.handle_mem_status(status, false, address, width, 0);
    }

    /// Called every time some data is writen to memory, if statistics are enabled
    pub fn on_memory_write(&mut self, address: u64, width: u64, value: u64) {
        if self.collect_offsets && width == 1 {
            let offset = (address as usize) & 0x7;
            if value < 0x100 {
                self.byte_clean_writes[offset] += 1;
            } else {
                self.byte_dirty_writes[offset] += 1;
            }
        }
        let status = self.costs.memory_write(address, width, value);
        self.handle_mem_status(status, true, address, width, value);
    }

    /// Acts on the status code returned by `memory_read`/`memory_write`: an invalid access is an
    /// error, and a monitored access is logged with its execution context.
    fn handle_mem_status(&self, status: u32, is_write: bool, address: u64, width: u64, value: u64) {
        if status & MEM_ACCESS_INVALID != 0 {
            self.report_invalid_mem_access(is_write, address, width);
        }
        if self.log_costly_unaligned && status & MEM_ACCESS_MONITOR != 0 {
            self.monitor_mem_access(is_write, address, width, value);
        }
    }

    /// Resolves the enclosing function name for `pc` from the ROI map (falls back to `<unknown>`).
    fn func_name_at(&self, pc: u32) -> &str {
        self.rois_by_pc
            .range(..=pc)
            .next_back()
            .map(|(_, &i)| self.rois[i as usize].name.as_str())
            .unwrap_or("<unknown>")
    }

    /// Reports a memory access to an address outside every known region (an unauthorized access, which
    /// would fault on real hardware) with the execution context needed to locate it: the current pc,
    /// step and enclosing function.
    fn report_invalid_mem_access(&self, is_write: bool, address: u64, width: u64) {
        let pc = self.current_pc;
        let kind = if is_write { "write" } else { "read" };
        panic!(
            "Invalid memory {kind} to 0x{address:08x} (width {width}) at pc=0x{pc:08x} \
             step={} fn='{}'",
            self.costs.steps,
            self.func_name_at(pc as u32),
        );
    }

    /// Logs a monitored memory access (e.g. a double 4/8-byte access) with its execution context:
    /// pc, function, address, width, read/write, misalignment offset (`address % 8`) and value.
    #[allow(dead_code)]
    fn monitor_mem_access(&self, is_write: bool, address: u64, width: u64, value: u64) {
        let pc = self.current_pc;
        if is_write {
            println!(
                 "MEM MONITOR pc=0x{pc:08x} fn='{}' addr=0x{address:016x} width={width} write offset={} value=0x{value:016x}",
                 self.func_name_at(pc as u32),
                 address % 8,
             );
        } else {
            println!(
                 "MEM MONITOR pc=0x{pc:08x} fn='{}' addr=0x{address:016x} width={width} read offset={}",
                 self.func_name_at(pc as u32),
                 address % 8,
             );
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
            if let Some(mut return_call) = return_call {
                if return_call.caller_roi_index != Some(roi_index) {
                    // The popped frame does not return into the ROI we landed in. With GCC/C++
                    // tail-recursion (e.g. std::__introsort_loop from std::sort) a single machine
                    // `ret` unwinds several nested self-recursive frames the heuristic tracked as
                    // separate calls. In `auto` mode (default) we resync by discarding the extra
                    // frames until the one that returns into the current ROI; `strict` keeps the
                    // original behaviour (report the mismatch and disable the call stack).
                    if self.callstack_strict {
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

                    // auto: a single machine `ret` unwound several collapsed self-recursive frames
                    // (`return_call`, the top frame, was already popped above). Find the nearest
                    // remaining frame that returns into the current ROI and drop the extra frames
                    // above it, keeping the profiler call path balanced; the dropped frame becomes
                    // the effective return. If no such frame exists the mismatch is not a collapsed
                    // recursion, so fall back to the strict behaviour (report and disable) without
                    // mutating the stack further.
                    match self
                        .call_stack
                        .iter()
                        .rposition(|f| f.caller_roi_index == Some(roi_index))
                    {
                        Some(idx) => {
                            while self.call_stack.len() > idx {
                                if let Some(profiler) = &mut self.profiler {
                                    let ram_usage = self.ram_monitor.get_usage(inst_ctx);
                                    profiler.pop_call_path(self.costs.total_cost(), ram_usage);
                                }
                                return_call = self.call_stack.pop().unwrap();
                            }
                        }
                        None => {
                            self.call_stack_error(
                                "ERROR: STACK MISMATCH DETECTED, disabled call stack feature",
                            );
                            return;
                        }
                    }
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
            if !inst.is_precompiled {
                for (i_mask, mask) in CAT_MASK.iter().enumerate() {
                    if inst_ctx.a & mask == 0 && inst_ctx.b & mask == 0 && inst_ctx.c & mask == 0 {
                        self.op_categories.inc(inst.op, i_mask * 4);
                    }
                    if inst_ctx.a & mask == 0 && inst_ctx.b & mask == *mask {
                        self.op_categories.inc(inst.op, i_mask * 4 + 1);
                    }
                    if inst_ctx.a & mask == *mask && inst_ctx.b & mask == 0 {
                        self.op_categories.inc(inst.op, i_mask * 4 + 2);
                    }
                    if inst_ctx.a & mask == *mask && inst_ctx.b & mask == *mask {
                        self.op_categories.inc(inst.op, i_mask * 4 + 3);
                    }
                }
                if inst.op_type == ZiskOperationType::BinaryE {
                    match inst.op {
                        ZiskOp::PACK | ZiskOp::PACK_H | ZiskOp::PACK_W => {
                            if inst_ctx.a < 0x1_0000_0000 && inst_ctx.b < 0x1_0000_0000 {
                                self.op_categories.inc(inst.op, OP_BIN_EXT_A_LT_64_CAT);
                            }
                        }
                        _ => {
                            if inst_ctx.a < 64 {
                                self.op_categories.inc(inst.op, OP_BIN_EXT_A_LT_64_CAT);
                            }
                            if inst_ctx.b < 64 {
                                self.op_categories.inc(inst.op, OP_BIN_EXT_B_LT_64_CAT);
                            }
                        }
                    }
                }
            }
            // Cheap opcode variants (e.g. BinaryAddHi shapes of ADD) are counted and charged their
            // reduced cost, so the per-opcode and aggregate costs reflect the saving. Only for
            // non-FROPS fixed-cost ops, so the counts are a subset of the opcode's count.
            if let Some(variant) = Self::cheap_variant(inst.op, inst_ctx.a, inst_ctx.b, inst_ctx.c)
            {
                self.cheap_variant_counts[variant] += 1;
                self.costs.add_cost_op(inst.op, CHEAP_VARIANTS[variant].2);
            } else {
                self.costs.add_fixed_cost_op(inst.op);
            }
        } else {
            self.costs.add_variable_cost_op(inst.op, self.current_variable_cost);
        }

        // Precompile duplicate analysis: fingerprint the input content of each precompile call
        // (following its memory layout, dereferencing indirections, so the key is the content —
        // not the `b` address) and mark the call a duplicate if that content was seen before.
        if self.duplicates && self.dup_op_enabled(inst.op) {
            if let Some(content) = Self::precompile_content(inst.op, inst_ctx) {
                let depth = self.duplicates_depth;
                let stats = self.dup_stats.entry(inst.op).or_default();
                let entry = stats.states.entry(content.into_boxed_slice()).or_insert(0);
                let is_duplicate = *entry > 0;
                *entry += 1;

                // Innermost `depth` call-stack ROIs (leaf first): current function plus its
                // callers, so the detail report shows the call path a duplicate comes from. Only
                // the first `depth` levels are filled, so paths aggregate at the configured depth.
                let mut path = [usize::MAX; DUP_MAX_ROI_DEPTH];
                path[0] = self.current_roi.unwrap_or(usize::MAX);
                let n = self.call_stack.len();
                for i in 0..depth.saturating_sub(1) {
                    if i < n {
                        path[i + 1] =
                            self.call_stack[n - 1 - i].caller_roi_index.unwrap_or(usize::MAX);
                    }
                }
                let path_stats = stats.call_paths.entry(path).or_insert((0, 0));
                path_stats.0 += 1;
                if is_duplicate {
                    path_stats.1 += 1;
                }
            }
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
    /// Selects the legacy FROPS tables (pre-overhaul snapshot) for FROPS coverage statistics,
    /// so a new FROPS version can be compared against the previous (legacy) one.
    pub fn set_legacy_frops(&mut self, legacy: bool) {
        self.legacy_frops = legacy;
    }
    /// When true, the opcode/precompiled/FROPS report sections are sorted by operation count
    /// (units) instead of by cost.
    pub fn set_sort_by_units(&mut self, sort_by_units: bool) {
        self.sort_by_units = sort_by_units;
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
        if self.legacy_frops {
            match instruction.op_type {
                ZiskOperationType::Arith => ArithLegacyFrops::is_frequent_op(instruction.op, a, b),
                ZiskOperationType::Binary => {
                    BinaryBasicLegacyFrops::is_frequent_op(instruction.op, a, b)
                }
                ZiskOperationType::BinaryE => {
                    BinaryExtensionLegacyFrops::is_frequent_op(instruction.op, a, b)
                }
                _ => false,
            }
        } else {
            match instruction.op_type {
                ZiskOperationType::Arith => ArithFrops::is_frequent_op(instruction.op, a, b),
                ZiskOperationType::Binary => BinaryBasicFrops::is_frequent_op(instruction.op, a, b),
                ZiskOperationType::BinaryE => {
                    BinaryExtensionFrops::is_frequent_op(instruction.op, a, b)
                }
                _ => false,
            }
        }
    }

    pub fn get_top_rois(&self, by_step: bool) -> Vec<(usize, u64)> {
        if by_step {
            self.get_top_rois_by(|roi| roi.get_steps())
        } else {
            self.get_top_rois_by(|roi| roi.get_cost())
        }
    }

    /// Ranks the ROIs (calls) by an arbitrary per-ROI metric, highest first, applying
    /// the same selected-ROI filter, `main` skipping and `top_rois` truncation as
    /// [`get_top_rois`]. Returns `(roi_index, metric)` pairs.
    pub fn get_top_rois_by<F: Fn(&RegionsOfInterest) -> u64>(
        &self,
        metric: F,
    ) -> Vec<(usize, u64)> {
        let mut top_rois: Vec<(usize, u64)> = self
            .rois
            .iter()
            .enumerate()
            .filter(|(_, roi)| !self.top_rois_filter || roi.is_selected_roi)
            .map(|(index, roi)| (index, metric(roi)))
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

    pub fn report_opcodes(
        &self,
        report: &mut StatsReport,
        title: &str,
        ops: &OpsCosts,
        base: bool,
        precompiled: bool,
        breakdown: bool,
    ) {
        let extended = base && !ops.is_frops();

        // Collect the opcodes to report, then order them from highest to lowest by cost (default) or
        // by operation count (units) when `sort_by_units` is set.
        let mut entries: Vec<(u8, u64, u64)> = Vec::new(); // (opcode, count, cost)
        for opcode in ZiskOp::MIN_OPCODE..=ZiskOp::MAX_OPCODE {
            if let Some((count, cost)) = ops.get_opcode_count_and_cost(opcode) {
                if count == 0 {
                    continue;
                }
                if let Ok(inst) = ZiskOp::try_from_code(opcode) {
                    if !base && !inst.is_precompiled() {
                        continue;
                    }
                    if !precompiled && inst.is_precompiled() {
                        continue;
                    }
                    entries.push((opcode, count as u64, cost));
                }
            }
        }
        if self.sort_by_units {
            entries.sort_by_key(|&(_, count, cost)| std::cmp::Reverse((count, cost)));
        } else {
            entries.sort_by_key(|&(_, count, cost)| std::cmp::Reverse((cost, count)));
        }

        for (opcode, count, cost) in entries {
            let inst = match ZiskOp::try_from_code(opcode) {
                Ok(inst) => inst,
                Err(_) => continue,
            };
            if extended {
                let categories =
                    self.op_categories.get_by_opcode(opcode).map(|c| &c[..]).unwrap_or(&[]);
                report.add_count_cost_perc2_extended(
                    &format!("{title} {:}", inst.name()),
                    count,
                    cost,
                    "",
                    categories,
                );
            } else {
                report.add_count_cost_perc2(&format!("{title} {:}", inst.name()), count, cost, "");
            }

            // Per-opcode breakdown of cheap variants (e.g. add → add_hif / add_hi0), as a tree.
            if breakdown {
                let mut variants: Vec<(usize, &str, u64)> = CHEAP_VARIANTS
                    .iter()
                    .enumerate()
                    .filter(|(i, (vop, _, _))| *vop == opcode && self.cheap_variant_counts[*i] > 0)
                    .map(|(i, (_, label, vcost))| (i, *label, *vcost))
                    .collect();
                variants.sort_by_key(|&(i, _, _)| std::cmp::Reverse(self.cheap_variant_counts[i]));
                for (n, (i, label, vcost)) in variants.iter().enumerate() {
                    let glyph = if n + 1 == variants.len() { "└" } else { "├" };
                    let vcount = self.cheap_variant_counts[*i];
                    report.add_count_cost_perc2(
                        &format!("{glyph} {label}"),
                        vcount,
                        vcount * vcost,
                        "",
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
        let (rom_init_count, ram_init_count) = if partial_report {
            (0, 0)
        } else {
            (self.rom_init_count as u64, self.ram_init_count as u64)
        };
        report.set_custom_totals(
            mops.get_count() + rom_init_count + ram_init_count,
            mops.get_cost() + rom_init_count * ROM_READ_COST + ram_init_count * MEM_WRITE_COST,
        );
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
        // Collect the FROPS opcodes, then order them from highest to lowest by cost (default) or by
        // FROPS count (units) when `sort_by_units` is set.
        let mut entries: Vec<(u8, u64, u64, u64)> = Vec::new(); // (opcode, frops_count, no_frops_count, frops_cost)
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
                    entries.push((opcode, frops_count as u64, no_frops_count as u64, frops_cost));
                }
            }
        }
        if self.sort_by_units {
            entries.sort_by_key(|&(_, frops_count, _, frops_cost)| {
                std::cmp::Reverse((frops_count, frops_cost))
            });
        } else {
            entries.sort_by_key(|&(_, frops_count, _, frops_cost)| {
                std::cmp::Reverse((frops_cost, frops_count))
            });
        }

        for (opcode, frops_count, no_frops_count, frops_cost) in entries {
            let inst = match ZiskOp::try_from_code(opcode) {
                Ok(inst) => inst,
                Err(_) => continue,
            };
            report.add_count_perc_cost_perc(
                &format!("{title} {:}", inst.name()),
                frops_count,
                (frops_count as f64 * 100.0) / ((frops_count + no_frops_count) as f64),
                frops_cost,
                "",
            );
        }
    }

    /// Reports the precompile duplicate analysis. Level 1 (always) is a per-precompile summary
    /// table: total calls, unique inputs, duplicates and the cost spent on those duplicates (what
    /// deduplication could save). Level 2 (`--duplicates-detail`) adds, per precompile with
    /// duplicates, the call paths where the duplicates come from — most costly first.
    fn report_duplicates(&self, report: &mut StatsReport, total_cost: u64) {
        // One summary row per precompile that had calls, ordered by duplicate cost (most first).
        struct DupRow {
            op: u8,
            total: u64,
            unique: u64,
            dup: u64,
            max_dup: u64,
            dup_cost: u64,
        }
        let mut rows: Vec<DupRow> = self
            .dup_stats
            .iter()
            .filter(|(_, s)| !s.states.is_empty())
            .map(|(&op, s)| {
                let total: u64 = s.states.values().map(|&c| c as u64).sum();
                let unique = s.states.len() as u64;
                let dup = total.saturating_sub(unique);
                let max_dup = s.states.values().copied().max().unwrap_or(0) as u64;
                let (count, cost) = self.costs.get_opcode_count_and_cost(op).unwrap_or((0, 0));
                let per_call = if count > 0 { cost / count as u64 } else { 0 };
                DupRow { op, total, unique, dup, max_dup, dup_cost: dup * per_call }
            })
            .collect();
        if rows.is_empty() {
            return;
        }
        rows.sort_by_key(|r| std::cmp::Reverse((r.dup_cost, r.dup)));

        // --- Level 1: summary table ---------------------------------------------------------
        let pct = |num: u64, den: u64| -> String {
            if den == 0 {
                "0.00%".to_string()
            } else {
                format!("{:.2}%", 100.0 * num as f64 / den as f64)
            }
        };
        let fmt_row = |c: &[String; 9]| -> String {
            format!(
                "{:<22} {:>12} {:>12} {:>8} {:>12} {:>8} {:>9} {:>15} {:>8}\n",
                c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7], c[8]
            )
        };
        let header = [
            "PRECOMPILE".to_string(),
            "TOTAL".to_string(),
            "UNIQUE".to_string(),
            "%".to_string(),
            "DUP".to_string(),
            "%".to_string(),
            "MAX DUP".to_string(),
            "DUP COST".to_string(),
            "%".to_string(),
        ];
        let header_line = fmt_row(&header);
        let width = header_line.trim_end().len();
        report.add("\nPRECOMPILE DUPLICATES\n");
        report.add(&header_line);
        report.add(&format!("{}\n", "-".repeat(width)));

        let (mut t_total, mut t_unique, mut t_dup, mut t_cost) = (0u64, 0u64, 0u64, 0u64);
        for r in &rows {
            let name =
                ZiskOp::try_from_code(r.op).map(|z| z.name().to_string()).unwrap_or_default();
            report.add(&fmt_row(&[
                name,
                report.format_number(r.total),
                report.format_number(r.unique),
                pct(r.unique, r.total),
                report.format_number(r.dup),
                pct(r.dup, r.total),
                report.format_number(r.max_dup),
                report.format_number(r.dup_cost),
                pct(r.dup_cost, total_cost),
            ]));
            t_total += r.total;
            t_unique += r.unique;
            t_dup += r.dup;
            t_cost += r.dup_cost;
        }
        if rows.len() > 1 {
            report.add(&format!("{}\n", "-".repeat(width)));
            report.add(&fmt_row(&[
                "TOTAL".to_string(),
                report.format_number(t_total),
                report.format_number(t_unique),
                pct(t_unique, t_total),
                report.format_number(t_dup),
                pct(t_dup, t_total),
                "-".to_string(),
                report.format_number(t_cost),
                pct(t_cost, total_cost),
            ]));
        }

        // --- Level 2: per-precompile call-path detail ---------------------------------------
        if !self.duplicates_detail {
            return;
        }
        let depth = self.duplicates_depth.clamp(1, DUP_MAX_ROI_DEPTH);
        let top_n = self.top_rois.max(1);
        let roi_name = |roi: usize| -> String {
            if roi == usize::MAX {
                "-".to_string()
            } else {
                self.format_roi_name(&self.rois[roi].name)
            }
        };
        for r in &rows {
            if r.dup == 0 {
                continue;
            }
            let stats = match self.dup_stats.get(&r.op) {
                Some(s) => s,
                None => continue,
            };
            let mut paths: Vec<([usize; DUP_MAX_ROI_DEPTH], u64, u64)> = stats
                .call_paths
                .iter()
                .map(|(&p, &(t, d))| (p, t, d))
                .filter(|&(_, _, d)| d > 0)
                .collect();
            if paths.is_empty() {
                continue;
            }
            paths.sort_by_key(|&(_, _, d)| std::cmp::Reverse(d));
            let name =
                ZiskOp::try_from_code(r.op).map(|z| z.name().to_string()).unwrap_or_default();
            report.title(&format!("{} DUPLICATES BY CALL PATH", name.to_uppercase()));
            for (path, path_total, dup) in paths.iter().take(top_n) {
                report.add(&format!(
                    "{} duplicates / {} total  ({:.2}%)\n",
                    report.format_number(*dup),
                    report.format_number(*path_total),
                    100.0 * *dup as f64 / *path_total as f64,
                ));
                for (level, &roi) in path.iter().take(depth).enumerate() {
                    let prefix = if level == 0 { "    " } else { "    <- " };
                    report.add(&format!("{prefix}{}\n", roi_name(roi)));
                }
            }
            if paths.len() > top_n {
                report.add(&format!("... and {} more call paths\n", paths.len() - top_n));
            }
        }
    }

    // ------------------------------------------------------------------------------------------
    // Aggregate stats snapshot (semicolon-separated) — save one run and compare against it later.
    // No per-function/ROI detail is included, only aggregate counters.
    // ------------------------------------------------------------------------------------------

    /// Cost-distribution totals, mirroring [`Self::report`]:
    /// `(steps, main, opcodes, precompiled, memory, base, total, frops)`.
    fn cost_distribution(&self) -> (u64, u64, u64, u64, u64, u64, u64, u64) {
        let ops_cost = self.costs.base_ops_cost();
        let precompiled_cost = self.costs.precompiled_ops_cost();
        let steps = self.costs.steps;
        let mem_cost = self.costs.mops.get_cost()
            + self.rom_init_count as u64 * ROM_READ_COST
            + self.ram_init_count as u64 * MEM_WRITE_COST;
        let main_cost = steps * MAIN_COST;
        let base_cost = BASE_COST as u64;
        let total_cost = base_cost + mem_cost + main_cost + ops_cost + precompiled_cost;
        (
            steps,
            main_cost,
            ops_cost,
            precompiled_cost,
            mem_cost,
            base_cost,
            total_cost,
            self.costs.frops_cost(),
        )
    }

    /// Collects opcode rows `(name, count, cost)` for base or precompiled ops, sorted desc by cost.
    fn csv_opcode_rows(&self, precompiled: bool) -> Vec<(String, u64, u64)> {
        let mut rows: Vec<(String, u64, u64)> = Vec::new();
        for opcode in ZiskOp::MIN_OPCODE..=ZiskOp::MAX_OPCODE {
            if let Some((count, cost)) = self.costs.get_opcode_count_and_cost(opcode) {
                if count == 0 {
                    continue;
                }
                if let Ok(inst) = ZiskOp::try_from_code(opcode) {
                    if inst.is_precompiled() != precompiled {
                        continue;
                    }
                    rows.push((inst.name().to_string(), count as u64, cost));
                }
            }
        }
        rows.sort_by_key(|(_, count, cost)| std::cmp::Reverse((*cost, *count)));
        rows
    }

    /// Collects FROP rows `(name, count, hit_percentage, cost)`, sorted desc by cost.
    fn csv_frop_rows(&self) -> Vec<(String, u64, f64, u64)> {
        let mut rows: Vec<(String, u64, f64, u64)> = Vec::new();
        for opcode in ZiskOp::MIN_OPCODE..=ZiskOp::MAX_OPCODE {
            if let Some((frops_count, frops_cost)) =
                self.costs.get_opcode_frops_count_and_cost(opcode)
            {
                if frops_count == 0 {
                    continue;
                }
                if let Ok(inst) = ZiskOp::try_from_code(opcode) {
                    if inst.is_precompiled() {
                        continue;
                    }
                    let (no_frops_count, _) =
                        self.costs.get_opcode_count_and_cost(opcode).unwrap_or((0, 0));
                    let hit = frops_count as f64 * 100.0 / (frops_count + no_frops_count) as f64;
                    rows.push((inst.name().to_string(), frops_count as u64, hit, frops_cost));
                }
            }
        }
        rows.sort_by_key(|(_, count, _, cost)| std::cmp::Reverse((*cost, *count)));
        rows
    }

    /// Appends the memory sections (`MEM` cost by type, `DETAILED_MEM` and `DETAILED_MEM FULL`) to
    /// `s`. Percentages are relative to the total memory count/cost; zero-cost rows are skipped
    /// (mirroring the pretty report's hidden-no-cost behaviour).
    fn append_mem_csv(&self, s: &mut String) {
        let mops = &self.costs.mops;
        let rom_init = self.rom_init_count as u64;
        let ram_init = self.ram_init_count as u64;
        let mem_count = mops.get_count() + rom_init + ram_init;
        let mem_cost = mops.get_cost() + rom_init * ROM_READ_COST + ram_init * MEM_WRITE_COST;
        let pct = |v: u64, total: u64| -> String {
            if total == 0 {
                "0.00%".to_string()
            } else {
                format!("{:.2}%", 100.0 * v as f64 / total as f64)
            }
        };
        let row = |s: &mut String, tag: &str, label: &str, count: u64, cost: u64| {
            if cost == 0 {
                return; // hidden-no-cost
            }
            s.push_str(&format!(
                "{tag};{label};{count};{};{cost};{}\n",
                pct(count, mem_count),
                pct(cost, mem_cost)
            ));
        };

        // MEM cost by type.
        s.push_str("MEM;COST BY TYPE;COUNT;%;COST;%\n");
        row(
            s,
            "MEM",
            "RAM STACK ALIGNED",
            mops.get_ram_stack_aligned_count(),
            mops.get_ram_stack_aligned_cost(),
        );
        row(
            s,
            "MEM",
            "RAM NO STACK ALIGNED",
            mops.get_ram_no_stack_aligned_count(),
            mops.get_ram_no_stack_aligned_cost(),
        );
        row(s, "MEM", "RAM INIT", ram_init, ram_init * MEM_WRITE_COST);
        row(s, "MEM", "ROM ALIGNED", mops.get_rom_aligned_count(), mops.get_rom_aligned_cost());
        row(s, "MEM", "ROM INIT", rom_init, rom_init * ROM_READ_COST);
        row(
            s,
            "MEM",
            "INPUT ALIGNED",
            mops.get_input_aligned_count(),
            mops.get_input_aligned_cost(),
        );
        row(
            s,
            "MEM",
            "RAM STACK UNALIGNED",
            mops.get_ram_stack_unaligned_count(),
            mops.get_ram_stack_unaligned_cost(),
        );
        row(
            s,
            "MEM",
            "RAM NO STACK UNALIGNED",
            mops.get_ram_no_stack_unaligned_count(),
            mops.get_ram_no_stack_unaligned_cost(),
        );
        row(
            s,
            "MEM",
            "ROM UNALIGNED",
            mops.get_rom_unaligned_count(),
            mops.get_rom_unaligned_cost(),
        );
        row(
            s,
            "MEM",
            "INPUT UNALIGNED",
            mops.get_input_unaligned_count(),
            mops.get_input_unaligned_cost(),
        );
        s.push('\n');
        row(
            s,
            "MEM",
            "TOTAL ALIGNED",
            mops.get_aligned_count() + ram_init + rom_init,
            mops.get_aligned_cost() + ram_init * MEM_WRITE_COST + rom_init * ROM_READ_COST,
        );
        row(s, "MEM", "TOTAL UNALIGNED", mops.get_unaligned_count(), mops.get_unaligned_cost());
        row(s, "MEM", "TOTAL RAM STACK", mops.get_ram_stack_count(), mops.get_ram_stack_cost());
        row(
            s,
            "MEM",
            "TOTAL RAM NO STACK",
            mops.get_ram_no_stack_count(),
            mops.get_ram_no_stack_cost(),
        );
        row(
            s,
            "MEM",
            "TOTAL RAM",
            mops.get_ram_count() + ram_init,
            mops.get_ram_cost() + ram_init * MEM_WRITE_COST,
        );
        row(
            s,
            "MEM",
            "TOTAL ROM",
            mops.get_rom_count() + rom_init,
            mops.get_rom_cost() + rom_init * ROM_READ_COST,
        );
        row(s, "MEM", "TOTAL INPUT", mops.get_input_count(), mops.get_input_cost());
        s.push('\n');

        // Detailed memory: per-subtype rows first, then aggregate totals (after the first empty
        // separator returned by `get_detailed_items`) tagged as DETAILED_MEM FULL.
        s.push_str("DETAILED_MEM;TYPE;COUNT;%;COST;%\n");
        let mut full = false;
        for (title, count, cost) in mops.get_detailed_items(rom_init, ram_init) {
            if title.is_empty() {
                full = true;
                s.push('\n');
                continue;
            }
            let tag = if full { "DETAILED_MEM FULL" } else { "DETAILED_MEM" };
            s.push_str(&format!(
                "{tag};{title};{count};{};{cost};{}\n",
                pct(count, mem_count),
                pct(cost, mem_cost)
            ));
        }
        s.push('\n');
    }

    /// Builds the aggregate snapshot of this run, delimited by `sep`. The first column tags the
    /// section (`STEPS`, `COST`, `MEM`, `DETAILED_MEM`, `MEM_OFFSETS`, `OP_BASE`, `PRECOMPILES`,
    /// `FROP`); sections are separated by a blank line. Numbers are raw (no thousands separators)
    /// for easy parsing.
    pub fn stats_csv(&self, sep: char) -> String {
        let (
            steps,
            main_cost,
            ops_cost,
            precompiled_cost,
            mem_cost,
            base_cost,
            total_cost,
            frops_cost,
        ) = self.cost_distribution();
        let variable = total_cost - base_cost;
        let pct = |v: u64, total: u64| -> String {
            if total == 0 {
                "0.00%".to_string()
            } else {
                format!("{:.2}%", 100.0 * v as f64 / total as f64)
            }
        };

        let mut s = String::new();
        s += &format!("STEPS;{steps}\n\n");

        s += "COST;COST DISTRIBUTION;COST;%\n";
        s += &format!("COST;MAIN;{main_cost};{}\n", pct(main_cost, total_cost));
        s += &format!("COST;OPCODES;{ops_cost};{}\n", pct(ops_cost, total_cost));
        s +=
            &format!("COST;PRECOMPILES;{precompiled_cost};{}\n", pct(precompiled_cost, total_cost));
        s += &format!("COST;MEMORY;{mem_cost};{}\n", pct(mem_cost, total_cost));
        s += &format!("COST;VARIABLE;{variable};{}\n", pct(variable, total_cost));
        s += &format!("COST;BASE;{base_cost};{}\n", pct(base_cost, total_cost));
        s += &format!("COST;TOTAL;{total_cost};{}\n", pct(total_cost, total_cost));
        s += &format!("COST;FROPS;{frops_cost};{}\n\n", pct(frops_cost, variable));

        if self.ram_monitor.ram_size > 0 {
            s += &format!(
                "RAM USAGE;{};{}\n",
                self.ram_monitor.ram_used,
                pct(self.ram_monitor.ram_used, self.ram_monitor.ram_size)
            );
        }
        let rom_used_rows =
            self.inst_count + self.rom_init_count.div_ceil(4) + self.ram_init_count.div_ceil(4);
        let rom_size = RomRomTrace::<Goldilocks>::NUM_ROWS as u64;
        s += &format!("ROM USAGE;{};{}\n\n", rom_used_rows, pct(rom_used_rows as u64, rom_size));

        self.append_mem_csv(&mut s);

        s += "MEM_OFFSETS;offset;0;1;2;3;4;5;6;7;total\n";
        for (label, vals) in [
            ("reads", &self.byte_reads),
            ("clean writes", &self.byte_clean_writes),
            ("dirty writes", &self.byte_dirty_writes),
        ] {
            let total: u64 = vals.iter().sum();
            let joined = vals.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(";");
            s += &format!("MEM_OFFSETS;{label};{joined};{total}\n");
        }
        s.push('\n');

        s += "OP_BASE;OPCODE;COUNT;%;COST;%\n";
        for (name, count, cost) in self.csv_opcode_rows(false) {
            s += &format!(
                "OP_BASE;{name};{count};{};{cost};{}\n",
                pct(count, steps),
                pct(cost, total_cost)
            );
        }
        s.push('\n');

        for (name, count, cost) in self.csv_opcode_rows(true) {
            s += &format!(
                "PRECOMPILES;{name};{count};{};{cost};{}\n",
                pct(count, steps),
                pct(cost, total_cost)
            );
        }
        s.push('\n');

        for (name, count, hit, cost) in self.csv_frop_rows() {
            s += &format!("FROP;{name};{count};{hit:.2}%;{cost};{}\n", pct(cost, variable));
        }

        // Built internally with ';'; swap to the requested separator (field content never
        // contains ';', so this is safe).
        if sep != ';' {
            s = s.replace(';', &sep.to_string());
        }
        s
    }

    /// Compares the current run against a reference snapshot saved with `--save-stats`. With
    /// `csv = false` it returns the human-readable, colour-coded report (red = higher cost/calls,
    /// green = lower; sign always shown); with `csv = true` it returns the previous plain
    /// semicolon-separated view (used by SDK mode / `--legacy-display` / `--diff-format csv`).
    pub fn compare_stats(
        &self,
        ref_path: &str,
        csv: bool,
        color: bool,
        sep: char,
    ) -> std::io::Result<String> {
        let ref_text = std::fs::read_to_string(ref_path)?;
        let cur_text = self.stats_csv(sep);
        Ok(if csv {
            diff_snapshots_csv(ref_path, &ref_text, "current run", &cur_text, sep)
        } else {
            diff_snapshots(ref_path, &ref_text, "current run", &cur_text, color)
        })
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
            report.set_and_push_label_width(55);
            report.title_count_cost_perc2("DETAILED MEM COST", "COUNT", "COST", "");
            self.report_detailed_mem(&mut report, &self.costs.mops, false);
            report.pop_label_width();
        }

        if self.mem_full_stats {
            report.set_and_push_label_width(14);
            report.title("DETAILED OFFSET BYTE MEMORY OPERATIONS");
            report.add_str_cells(
                "offset",
                &["0", "1", "2", "3", "4", "5", "6", "7", "total"].map(String::from),
                12,
            );
            for (label, vals) in [
                ("reads", &self.byte_reads),
                ("clean writes", &self.byte_clean_writes),
                ("dirty writes", &self.byte_dirty_writes),
            ] {
                let total: u64 = vals.iter().sum();
                let mut cells: Vec<String> =
                    vals.iter().map(|v| report.format_number(*v)).collect();
                cells.push(report.format_number(total));
                report.add_str_cells(label, &cells, 12);
            }
            report.pop_label_width();
        }

        if show_opcodes {
            report.title_count_cost_perc2("COST BY BASE OPCODE", "COUNT", "COST", "");
            self.report_opcodes(
                &mut report,
                "OP",
                self.costs.ops_costs(),
                true,
                false,
                self.opcode_breakdown,
            );

            report.title_count_cost_perc2("COST BY PRECOMPILED OPCODE", "COUNT", "COST", "");
            self.report_opcodes(&mut report, "OP", self.costs.ops_costs(), false, true, false);

            report.title_count_perc_cost_perc("FROPS BY OPCODE", "COUNT", "HIT", "COST", "");
            self.report_frops_hit(&mut report, "FROP");
        }

        if self.duplicates {
            self.report_duplicates(&mut report, total_cost);
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

                    if self.mem_stats || self.mem_full_stats {
                        roi_report.title_count_cost_perc2("MEM COST BY TYPE", "COUNT", "COST", "");
                        self.report_mem(&mut roi_report, &roi.costs.mops, true);
                    }

                    if self.mem_full_stats {
                        roi_report.set_and_push_label_width(55);
                        roi_report.title_count_cost_perc2("DETAILED MEM COST", "COUNT", "COST", "");
                        self.report_detailed_mem(&mut roi_report, &roi.costs.mops, true);
                        roi_report.pop_label_width();
                    }

                    roi_report.title_count_cost_perc("COST BY OPCODE", "COUNT", "COST", "");
                    self.report_opcodes(&mut roi_report, "OP", roi.ops_costs(), true, true, false);

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

        // Memory-focused call rankings, shown with `--mem-stats` / `--mem-full-stats`
        // whenever per-function ROIs are available.
        if (self.mem_stats || self.mem_full_stats)
            && self.is_rois_enabled()
            && !self.disable_call_stack
        {
            // Costs are shown in millions by default (the totals are large); the SDK
            // report keeps raw values. `(M)` marks the millions columns in the header.
            let millions = !self.sdk;
            let m = if millions { " (M)" } else { "" };

            // Rank calls by total memory cost.
            report.title_auto_width(&format!(
                "TOP MEMORY COST FUNCTIONS (MEM COST{m}, % MEM COST, CALLS, COST/CALL{m}, FUNCTION)"
            ));
            let mem_divisor = self.costs.mops.get_cost() as f64 / 100.0;
            for (index, _) in self.get_top_rois_by(|roi| roi.get_mem_cost()).iter() {
                let roi = &self.rois[*index];
                let mem_cost = roi.get_mem_cost();
                if mem_cost == 0 {
                    continue;
                }
                let formatted_name = self.format_roi_name(&roi.name);
                report.add_top_mem_cost_calls(
                    &formatted_name,
                    mem_cost,
                    mem_divisor,
                    roi.calls,
                    millions,
                );
            }

            // Rank calls by unaligned memory cost, showing the aligned cost alongside.
            report.title_auto_width(&format!(
                "TOP UNALIGNED MEMORY FUNCTIONS (UNALIGNED{m}, ALIGNED{m}, % UNALIGNED, CALLS, FUNCTION)"
            ));
            for (index, _) in self.get_top_rois_by(|roi| roi.get_mem_unaligned_cost()).iter() {
                let roi = &self.rois[*index];
                let unaligned = roi.get_mem_unaligned_cost();
                let aligned = roi.get_mem_aligned_cost();
                if unaligned + aligned == 0 {
                    continue;
                }
                let formatted_name = self.format_roi_name(&roi.name);
                report.add_top_mem_align_calls(
                    &formatted_name,
                    unaligned,
                    aligned,
                    roi.calls,
                    millions,
                );
            }

            // Rank calls by how far their unaligned cost per step exceeds the global
            // average (unaligned cost / step). Only functions accounting for more than
            // 1% of the total unaligned cost are considered, to filter out low-volume
            // outliers with a high ratio but negligible impact.
            let total_unaligned = self.costs.mops.get_unaligned_cost();
            if total_unaligned > 0 && total_steps > 0 {
                let global_per_step = total_unaligned as f64 / total_steps as f64;
                let min_unaligned = total_unaligned as f64 * 0.01; // 1% threshold

                let mut ranked: Vec<(usize, f64)> = self
                    .rois
                    .iter()
                    .enumerate()
                    .filter(|(_, roi)| !self.top_rois_filter || roi.is_selected_roi)
                    .filter_map(|(index, roi)| {
                        let unaligned = roi.get_mem_unaligned_cost();
                        let steps = roi.get_steps();
                        if steps == 0 || (unaligned as f64) < min_unaligned {
                            return None;
                        }
                        let ratio = (unaligned as f64 / steps as f64) / global_per_step;
                        Some((index, ratio))
                    })
                    .collect();
                ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                ranked.truncate(self.top_rois);

                report.title_auto_width(&format!(
                    "TOP UNALIGNED/STEP RATIO FUNCTIONS (RATIO vs GLOBAL AVG, UNALIGNED{m}, % UNALIGNED, UNALIGNED ACCESSES/CALL, CALLS, FUNCTION)"
                ));
                for (index, ratio) in ranked.iter() {
                    let roi = &self.rois[*index];
                    let unaligned = roi.get_mem_unaligned_cost();
                    let unaligned_perc = unaligned as f64 * 100.0 / total_unaligned as f64;
                    let unaligned_per_call = if roi.calls > 0 {
                        roi.get_mem_unaligned_count() / roi.calls as u64
                    } else {
                        0
                    };
                    let formatted_name = self.format_roi_name(&roi.name);
                    report.add_top_mem_ratio(
                        &formatted_name,
                        *ratio,
                        unaligned,
                        unaligned_perc,
                        unaligned_per_call,
                        roi.calls,
                        millions,
                    );
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
            if let Some(pc) = inst.external_ref_addr {
                if pc as u32 >= current_roi_from && pc as u32 <= current_roi_to {
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
                    if let Some((roi_index, roi)) = self.get_roi_from_pc(pc as u32) {
                        current_roi_index = Some(roi_index);
                        current_roi_from = roi.from_pc;
                        current_roi_to = roi.to_pc;
                        current_internal_from = Some(internal_pc as u32);
                        current_internal_to = None;
                    }
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
    /// Enables accumulation of the per-offset byte read/write counters (MEM_OFFSETS).
    pub fn set_collect_offsets(&mut self, value: bool) {
        self.collect_offsets = value;
    }
    /// Enables logging of costly unaligned memory accesses (double 4B/8B) with execution context.
    pub fn set_log_costly_unaligned(&mut self, value: bool) {
        self.log_costly_unaligned = value;
    }
    /// Selects strict call-stack tracking (original behaviour): a stack mismatch disables the call
    /// stack instead of resyncing by unwinding collapsed recursive frames.
    pub fn set_callstack_strict(&mut self, value: bool) {
        self.callstack_strict = value;
    }
    /// Shows the per-opcode breakdown of cheap variants (e.g. add → add_hi0 / add_hif).
    pub fn set_opcode_breakdown(&mut self, value: bool) {
        self.opcode_breakdown = value;
    }
    /// Enables the precompile duplicate analysis.
    pub fn set_duplicates(&mut self, value: bool) {
        self.duplicates = value;
    }
    /// Restricts the duplicate analysis to the given opcodes (`None` = every supported precompile).
    pub fn set_duplicates_ops(&mut self, value: Option<HashSet<u8>>) {
        self.duplicates_ops = value;
    }
    /// Sets the recorded call-path depth for the duplicate detail report (clamped to a sane range).
    pub fn set_duplicates_depth(&mut self, value: usize) {
        self.duplicates_depth = value.clamp(1, DUP_MAX_ROI_DEPTH);
    }
    /// Enables the per-precompile call-path detail (level 2) of the duplicate report.
    pub fn set_duplicates_detail(&mut self, value: bool) {
        self.duplicates_detail = value;
    }

    /// Classifies an executed operation into a cheap variant (index into `CHEAP_VARIANTS`), or
    /// `None` if it is a regular operation. Currently the BinaryAddHi shapes of ADD:
    /// hi32(a)=hi32(c)=0 with hi32(b)=0 (add_hi0) or hi32(b)=0xFFFF_FFFF (add_hif).
    #[inline(always)]
    fn cheap_variant(op: u8, a: u64, b: u64, c: u64) -> Option<usize> {
        if op == ADD_CODE && a & HI32_MASK == 0 && c & HI32_MASK == 0 {
            if b & HI32_MASK == 0 {
                return Some(CHEAP_ADD_HI0);
            } else if b & HI32_MASK == HI32_MASK {
                return Some(CHEAP_ADD_HIF);
            }
        }
        None
    }

    /// Content descriptor for a precompile opcode, or `None` if the opcode is not a precompile
    /// eligible for duplicate analysis (this excludes DMA, EVM `jump_dest`, `halt`, fcalls, etc.).
    /// See the layout notes in `DupSeg`. The segments read the operand *content* (dereferencing
    /// indirections) so two calls with the same inputs at different buffers are seen as duplicates.
    fn dup_descriptor(op: u8) -> Option<&'static [DupSeg]> {
        use DupSeg::{Direct, Indirect, Literal};
        let z = ZiskOp::try_from_code(op).ok()?;
        Some(match z {
            // In-place operands read directly from `ctx.b`.
            ZiskOp::Keccak => &[Direct { words: 25 }][..],
            ZiskOp::Poseidon1 | ZiskOp::Poseidon2 => &[Direct { words: 16 }][..],
            ZiskOp::Secp256k1Dbl | ZiskOp::Secp256r1Dbl | ZiskOp::Bn254CurveDbl => {
                &[Direct { words: 8 }][..]
            }
            ZiskOp::Bls12_381CurveDbl => &[Direct { words: 12 }][..],
            // Indirected operands: `ctx.b` holds pointers to the operand buffers.
            ZiskOp::Sha256 => {
                &[Indirect { param: 0, words: 4 }, Indirect { param: 1, words: 8 }][..]
            }
            ZiskOp::Blake2 => &[
                Literal { param: 0 },
                Indirect { param: 1, words: 16 },
                Indirect { param: 2, words: 16 },
            ][..],
            ZiskOp::Add256 => &[
                Indirect { param: 0, words: 4 },
                Indirect { param: 1, words: 4 },
                Literal { param: 2 },
            ][..],
            ZiskOp::Arith256 => &[
                Indirect { param: 0, words: 4 },
                Indirect { param: 1, words: 4 },
                Indirect { param: 2, words: 4 },
            ][..],
            ZiskOp::Arith256Mod => &[
                Indirect { param: 0, words: 4 },
                Indirect { param: 1, words: 4 },
                Indirect { param: 2, words: 4 },
                Indirect { param: 3, words: 4 },
            ][..],
            ZiskOp::Secp256k1Add
            | ZiskOp::Secp256r1Add
            | ZiskOp::Bn254CurveAdd
            | ZiskOp::Bn254ComplexAdd
            | ZiskOp::Bn254ComplexSub
            | ZiskOp::Bn254ComplexMul => {
                &[Indirect { param: 0, words: 8 }, Indirect { param: 1, words: 8 }][..]
            }
            ZiskOp::Arith384Mod => &[
                Indirect { param: 0, words: 6 },
                Indirect { param: 1, words: 6 },
                Indirect { param: 2, words: 6 },
                Indirect { param: 3, words: 6 },
            ][..],
            ZiskOp::Bls12_381CurveAdd
            | ZiskOp::Bls12_381ComplexAdd
            | ZiskOp::Bls12_381ComplexSub
            | ZiskOp::Bls12_381ComplexMul => {
                &[Indirect { param: 0, words: 12 }, Indirect { param: 1, words: 12 }][..]
            }
            _ => return None,
        })
    }

    /// Reads the input-content fingerprint of a precompile call from memory, following the layout
    /// in `dup_descriptor`. Returns `None` for opcodes that are not duplicate-analyzed precompiles.
    /// Read post-execution (as `on_op` runs after the op): for in-place ops the words are the
    /// output state, but since the precompile is deterministic, identical inputs still map to an
    /// identical fingerprint — which is all duplicate detection needs.
    fn precompile_content(op: u8, ctx: &InstContext) -> Option<Vec<u64>> {
        let segs = Self::dup_descriptor(op)?;
        let base = ctx.b;
        let mut content = Vec::new();
        let mut direct_off = 0u64;
        for seg in segs {
            match *seg {
                DupSeg::Direct { words } => {
                    for i in 0..words as u64 {
                        content.push(ctx.mem.read(base + 8 * (direct_off + i), 8));
                    }
                    direct_off += words as u64;
                }
                DupSeg::Indirect { param, words } => {
                    let ptr = ctx.mem.read(base + 8 * param as u64, 8);
                    for i in 0..words as u64 {
                        content.push(ctx.mem.read(ptr + 8 * i, 8));
                    }
                }
                DupSeg::Literal { param } => {
                    content.push(ctx.mem.read(base + 8 * param as u64, 8));
                }
            }
        }
        Some(content)
    }

    /// Whether the duplicate analysis should process this opcode: it must be a supported precompile
    /// and, if a restriction list was given (`--duplicates-ops`), be in it.
    fn dup_op_enabled(&self, op: u8) -> bool {
        match &self.duplicates_ops {
            Some(set) => set.contains(&op),
            None => true,
        }
    }

    /// The opcodes eligible for duplicate analysis (every precompile with a content descriptor).
    pub fn dup_supported_opcodes() -> Vec<u8> {
        (ZiskOp::MIN_OPCODE..=ZiskOp::MAX_OPCODE)
            .filter(|&op| Self::dup_descriptor(op).is_some())
            .collect()
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

// ==============================================================================================
// Stats snapshot comparison — human-readable, colour-coded diff of two runs.
// Red  = value went up (more cost / more calls → regression).
// Green = value went down (improvement).  Dim = unchanged.  Magenta = hit% (inverted axis).
// The sign is always printed, so the diff reads correctly with colour disabled.
// ==============================================================================================

const C_RESET: &str = "\x1b[0m";
const C_RED: &str = "\x1b[31m";
const C_GREEN: &str = "\x1b[32m";
const C_DIM: &str = "\x1b[2m";
const C_MAGENTA: &str = "\x1b[35m";
const C_HEAD: &str = "\x1b[36m";

#[derive(Default)]
struct OpRow {
    name: String,
    count: u64,
    count_pct: String,
    cost: u64,
    cost_pct: String,
}

#[derive(Default)]
struct FrRow {
    name: String,
    count: u64,
    hit: f64,
    hit_pct: String,
    cost: u64,
    cost_pct: String,
}

#[derive(Default)]
struct Snap {
    steps: u64,
    cost_rows: Vec<(String, u64, String)>, // (metric, cost, pct string)
    cost_map: HashMap<String, u64>,
    op_rows: Vec<OpRow>,
    op_map: HashMap<String, (u64, u64)>,
    pc_rows: Vec<OpRow>,
    pc_map: HashMap<String, (u64, u64)>,
    fr_rows: Vec<FrRow>,
    fr_map: HashMap<String, (u64, u64, f64)>, // (count, cost, hit)
}

/// Detects the field separator of a snapshot: the character right after the leading `STEPS` tag on
/// the first `STEPS` line. Defaults to `,` (also handles files saved with `;` or any single char).
fn detect_sep(text: &str) -> char {
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("STEPS") {
            if let Some(c) = rest.chars().next() {
                return c;
            }
        }
    }
    ','
}

/// Parses a `--save-stats` snapshot, preserving row order and the original share-of-total strings.
/// The field separator is auto-detected, so snapshots saved with any separator parse correctly.
fn parse_snapshot(text: &str) -> Snap {
    let sep = detect_sep(text);
    let mut snap = Snap::default();
    for line in text.lines() {
        let f: Vec<&str> = line.split(sep).collect();
        match f.first().copied() {
            Some("STEPS") if f.len() >= 2 => {
                snap.steps = f[1].parse().unwrap_or(0);
            }
            Some("COST") if f.len() >= 4 && f[1] != "COST DISTRIBUTION" => {
                if let Ok(c) = f[2].parse::<u64>() {
                    snap.cost_rows.push((f[1].to_string(), c, f[3].to_string()));
                    snap.cost_map.insert(f[1].to_string(), c);
                }
            }
            Some("OP_BASE") if f.len() >= 6 && f[1] != "OPCODE" => {
                if let (Ok(cnt), Ok(cst)) = (f[2].parse::<u64>(), f[4].parse::<u64>()) {
                    snap.op_rows.push(OpRow {
                        name: f[1].into(),
                        count: cnt,
                        count_pct: f[3].into(),
                        cost: cst,
                        cost_pct: f[5].into(),
                    });
                    snap.op_map.insert(f[1].into(), (cnt, cst));
                }
            }
            Some("PRECOMPILES") if f.len() >= 6 && f[1] != "OPCODE" => {
                if let (Ok(cnt), Ok(cst)) = (f[2].parse::<u64>(), f[4].parse::<u64>()) {
                    snap.pc_rows.push(OpRow {
                        name: f[1].into(),
                        count: cnt,
                        count_pct: f[3].into(),
                        cost: cst,
                        cost_pct: f[5].into(),
                    });
                    snap.pc_map.insert(f[1].into(), (cnt, cst));
                }
            }
            Some("FROP") if f.len() >= 6 && f[1] != "OPCODE" => {
                if let (Ok(cnt), Ok(cst)) = (f[2].parse::<u64>(), f[4].parse::<u64>()) {
                    let hit = f[3].trim_end_matches('%').parse::<f64>().unwrap_or(0.0);
                    snap.fr_rows.push(FrRow {
                        name: f[1].into(),
                        count: cnt,
                        hit,
                        hit_pct: f[3].into(),
                        cost: cst,
                        cost_pct: f[5].into(),
                    });
                    snap.fr_map.insert(f[1].into(), (cnt, cst, hit));
                }
            }
            _ => {}
        }
    }
    snap
}

/// Groups a non-negative integer with thousands separators: `1234567` -> `"1,234,567"`.
fn group_u64(n: u64) -> String {
    let s = n.to_string();
    let len = s.len();
    let mut out = String::with_capacity(len + len / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// Signed, grouped delta: `1203` -> `"+1,203"`, `-48720` -> `"-48,720"`, `0` -> `"+0"`.
fn fmt_delta(d: i64) -> String {
    format!("{}{}", if d < 0 { "-" } else { "+" }, group_u64(d.unsigned_abs()))
}

/// Percentage change of `cur` vs `ref`, signed. `n/a` when the reference is zero but current isn't.
fn pct_change(r: u64, c: u64) -> String {
    if r == 0 {
        if c == 0 {
            "0.00%".to_string()
        } else {
            "n/a".to_string()
        }
    } else {
        format!("{:+.2}%", 100.0 * (c as i64 - r as i64) as f64 / r as f64)
    }
}

/// ANSI colour for a delta: red up (worse), green down (better), dim when flat.
fn dir(d: i64) -> &'static str {
    if d > 0 {
        C_RED
    } else if d < 0 {
        C_GREEN
    } else {
        C_DIM
    }
}

fn wrap(on: bool, code: &str, text: &str) -> String {
    if on {
        format!("{code}{text}{C_RESET}")
    } else {
        text.to_string()
    }
}

/// The `Δ (Δ%)` cell for a cost/value delta, tagging brand-new and removed entries.
fn delta_cell(r: u64, c: u64) -> String {
    let d = c as i64 - r as i64;
    if r == 0 && c > 0 {
        format!("{} (new)", fmt_delta(d))
    } else if c == 0 && r > 0 {
        format!("{} (gone)", fmt_delta(d))
    } else {
        format!("{} ({})", fmt_delta(d), pct_change(r, c))
    }
}

/// Joins pre-padded cells `(text, colour_code)` with single spaces, applying colour when enabled.
/// `colour_code` empty means no colour. Widths live in the padded `text`, so alignment is preserved.
fn row_line(color: bool, cells: &[(String, &str)]) -> String {
    cells
        .iter()
        .map(|(text, code)| if code.is_empty() { text.clone() } else { wrap(color, code, text) })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Renders an opcode-style section (base opcodes or precompiles): current value + share%, then a
/// coloured signed delta for count and for cost. Reference-only entries are appended as `(gone)`.
fn render_op_section(
    out: &mut String,
    color: bool,
    title: &str,
    prefix: &str,
    cur_rows: &[OpRow],
    ref_map: &HashMap<String, (u64, u64)>,
) {
    let header: [(String, &str); 7] = [
        (format!("{:<24}", ""), C_DIM),
        (format!("{:>12}", "COUNT"), C_DIM),
        (format!("{:>7}", "%"), C_DIM),
        (format!("{:>12}", "Δ"), C_DIM),
        (format!("{:>15}", "COST"), C_DIM),
        (format!("{:>7}", "%"), C_DIM),
        (format!("{:>24}", "Δ (Δ%)"), C_DIM),
    ];
    out.push('\n');
    out.push_str(&wrap(color, C_HEAD, title));
    out.push('\n');
    let plain_len = header.iter().map(|(t, _)| t.len()).sum::<usize>() + header.len() - 1;
    out.push_str(&row_line(color, &header));
    out.push('\n');
    out.push_str(&wrap(color, C_DIM, &"-".repeat(plain_len)));
    out.push('\n');

    let mut seen: HashSet<String> = HashSet::new();
    for row in cur_rows {
        seen.insert(row.name.clone());
        let (rcount, rcost) = ref_map.get(&row.name).copied().unwrap_or((0, 0));
        let dcount = row.count as i64 - rcount as i64;
        let dcost = row.cost as i64 - rcost as i64;
        let cells: [(String, &str); 7] = [
            (format!("{:<24}", format!("{prefix}{}", row.name)), ""),
            (format!("{:>12}", group_u64(row.count)), ""),
            (format!("{:>7}", row.count_pct), C_DIM),
            (format!("{:>12}", fmt_delta(dcount)), dir(dcount)),
            (format!("{:>15}", group_u64(row.cost)), ""),
            (format!("{:>7}", row.cost_pct), C_DIM),
            (format!("{:>24}", delta_cell(rcost, row.cost)), dir(dcost)),
        ];
        out.push_str(&row_line(color, &cells));
        out.push('\n');
    }
    let mut removed: Vec<(&String, &(u64, u64))> =
        ref_map.iter().filter(|(k, _)| !seen.contains(*k)).collect();
    removed.sort_by_key(|(_, (_, cost))| std::cmp::Reverse(*cost));
    for (rname, (rcount, rcost)) in removed {
        let cells: [(String, &str); 7] = [
            (format!("{:<24}", format!("{prefix}{rname}")), ""),
            (format!("{:>12}", "0"), ""),
            (format!("{:>7}", "-"), C_DIM),
            (format!("{:>12}", fmt_delta(-(*rcount as i64))), dir(-(*rcount as i64))),
            (format!("{:>15}", "0"), ""),
            (format!("{:>7}", "-"), C_DIM),
            (format!("{:>24}", delta_cell(*rcost, 0)), dir(-(*rcost as i64))),
        ];
        out.push_str(&row_line(color, &cells));
        out.push('\n');
    }
}

/// Renders a human-readable, colour-coded comparison of two snapshots (each in the `--save-stats`
/// format). `ref_*` is the baseline, `cur_*` the run being evaluated; deltas are `cur - ref`.
/// Pass `color = false` for plain output (piped / `--color=never`).
pub fn diff_snapshots(
    ref_label: &str,
    ref_text: &str,
    cur_label: &str,
    cur_text: &str,
    color: bool,
) -> String {
    let refs = parse_snapshot(ref_text);
    let cur = parse_snapshot(cur_text);
    let mut out = String::new();

    out.push('\n');
    out.push_str(&wrap(color, C_HEAD, &format!("COMPARISON   {ref_label}  →  {cur_label}")));
    out.push('\n');
    out.push_str(&wrap(
        color,
        C_DIM,
        "red = higher / worse   green = lower / better   % = share of total   sign always shown",
    ));
    out.push('\n');

    // STEPS
    let dsteps = cur.steps as i64 - refs.steps as i64;
    out.push('\n');
    out.push_str(&row_line(
        color,
        &[
            (format!("{:<24}", "STEPS"), ""),
            (format!("{:>12}", group_u64(cur.steps)), ""),
            (format!("{:>7}", ""), ""),
            (format!("{:>24}", delta_cell(refs.steps, cur.steps)), dir(dsteps)),
        ],
    ));
    out.push('\n');

    // COST DISTRIBUTION
    let cd_header: [(String, &str); 4] = [
        (format!("{:<24}", "COST DISTRIBUTION"), C_DIM),
        (format!("{:>15}", "COST"), C_DIM),
        (format!("{:>8}", "%"), C_DIM),
        (format!("{:>26}", "Δ (Δ%)"), C_DIM),
    ];
    out.push('\n');
    let cd_len = cd_header.iter().map(|(t, _)| t.len()).sum::<usize>() + cd_header.len() - 1;
    out.push_str(&row_line(color, &cd_header));
    out.push('\n');
    out.push_str(&wrap(color, C_DIM, &"-".repeat(cd_len)));
    out.push('\n');
    for (metric, cost, pct) in &cur.cost_rows {
        let r = refs.cost_map.get(metric).copied().unwrap_or(0);
        let d = *cost as i64 - r as i64;
        out.push_str(&row_line(
            color,
            &[
                (format!("{metric:<24}"), ""),
                (format!("{:>15}", group_u64(*cost)), ""),
                (format!("{pct:>8}"), C_DIM),
                (format!("{:>26}", delta_cell(r, *cost)), dir(d)),
            ],
        ));
        out.push('\n');
    }

    // Base opcodes and precompiles.
    render_op_section(&mut out, color, "COST BY BASE OPCODE", "OP ", &cur.op_rows, &refs.op_map);
    render_op_section(
        &mut out,
        color,
        "COST BY PRECOMPILED OPCODE",
        "OP ",
        &cur.pc_rows,
        &refs.pc_map,
    );

    // FROPS — hit% is an inverted axis (up is good), shown in magenta with an explicit sign.
    let fr_header: [(String, &str); 7] = [
        (format!("{:<24}", "FROPS BY OPCODE"), C_DIM),
        (format!("{:>12}", "COUNT"), C_DIM),
        (format!("{:>8}", "HIT"), C_DIM),
        (format!("{:>9}", "ΔHIT"), C_DIM),
        (format!("{:>15}", "COST"), C_DIM),
        (format!("{:>7}", "%"), C_DIM),
        (format!("{:>24}", "Δ (Δ%)"), C_DIM),
    ];
    out.push('\n');
    let fr_len = fr_header.iter().map(|(t, _)| t.len()).sum::<usize>() + fr_header.len() - 1
        + "ΔHIT".chars().count(); // ΔHIT header has a 2-byte char; pad-len uses bytes, keep rule long enough
    out.push_str(&row_line(color, &fr_header));
    out.push('\n');
    out.push_str(&wrap(color, C_DIM, &"-".repeat(fr_len.min(120))));
    out.push('\n');
    for row in &cur.fr_rows {
        let (rcount, rcost, rhit) = refs.fr_map.get(&row.name).copied().unwrap_or((0, 0, 0.0));
        let present = refs.fr_map.contains_key(&row.name);
        let dhit = if present { format!("{:+.1}pp", row.hit - rhit) } else { "new".to_string() };
        let dcost = row.cost as i64 - rcost as i64;
        let _ = rcount;
        out.push_str(&row_line(
            color,
            &[
                (format!("{:<24}", format!("FROP {}", row.name)), ""),
                (format!("{:>12}", group_u64(row.count)), ""),
                (format!("{:>8}", row.hit_pct), C_DIM),
                (format!("{dhit:>9}"), C_MAGENTA),
                (format!("{:>15}", group_u64(row.cost)), ""),
                (format!("{:>7}", row.cost_pct), C_DIM),
                (format!("{:>24}", delta_cell(rcost, row.cost)), dir(dcost)),
            ],
        ));
        out.push('\n');
    }

    out
}

/// Renders the comparison of two snapshots in the previous plain, semicolon-separated view
/// (machine-friendly, no colour). Used by SDK mode, `--legacy-display`, and `--diff-format csv`.
pub fn diff_snapshots_csv(
    ref_label: &str,
    ref_text: &str,
    cur_label: &str,
    cur_text: &str,
    sep: char,
) -> String {
    let refs = parse_snapshot(ref_text);
    let cur = parse_snapshot(cur_text);
    let mut s = String::new();
    s += &format!("\nCOMPARISON: {ref_label} (ref) -> {cur_label}\n\n");

    s += "COST;metric;ref;current;delta;%\n";
    for name in ["MAIN", "OPCODES", "PRECOMPILES", "MEMORY", "VARIABLE", "BASE", "TOTAL", "FROPS"] {
        let r = refs.cost_map.get(name).copied().unwrap_or(0);
        let c = cur.cost_map.get(name).copied().unwrap_or(0);
        s += &format!("COST;{name};{r};{c};{:+};{}\n", c as i64 - r as i64, pct_change(r, c));
    }
    s.push('\n');

    // Diffs a (name, count, cost) section: current rows first (order preserved), then
    // reference-only entries (removed in the current run), sorted by cost.
    let diff_cc = |tag: &str,
                   cur_rows: &[(&str, u64, u64)],
                   ref_map: &HashMap<String, (u64, u64)>|
     -> String {
        let mut out =
            format!("{tag};name;ref_count;cur_count;d_count;ref_cost;cur_cost;d_cost;%\n");
        let mut seen: HashSet<String> = HashSet::new();
        for (name, ccount, ccost) in cur_rows {
            seen.insert((*name).to_string());
            let (rcount, rcost) = ref_map.get(*name).copied().unwrap_or((0, 0));
            out += &format!(
                "{tag};{name};{rcount};{ccount};{:+};{rcost};{ccost};{:+};{}\n",
                *ccount as i64 - rcount as i64,
                *ccost as i64 - rcost as i64,
                pct_change(rcost, *ccost),
            );
        }
        let mut removed: Vec<(&String, &(u64, u64))> =
            ref_map.iter().filter(|(k, _)| !seen.contains(*k)).collect();
        removed.sort_by_key(|(_, (_, cost))| std::cmp::Reverse(*cost));
        for (name, (rcount, rcost)) in removed {
            out += &format!(
                "{tag};{name};{rcount};0;{:+};{rcost};0;{:+};{}\n",
                -(*rcount as i64),
                -(*rcost as i64),
                pct_change(*rcost, 0),
            );
        }
        out
    };

    let op_rows: Vec<(&str, u64, u64)> =
        cur.op_rows.iter().map(|r| (r.name.as_str(), r.count, r.cost)).collect();
    let pc_rows: Vec<(&str, u64, u64)> =
        cur.pc_rows.iter().map(|r| (r.name.as_str(), r.count, r.cost)).collect();
    let fr_rows: Vec<(&str, u64, u64)> =
        cur.fr_rows.iter().map(|r| (r.name.as_str(), r.count, r.cost)).collect();
    let fr_ref: HashMap<String, (u64, u64)> =
        refs.fr_map.iter().map(|(k, (c, co, _))| (k.clone(), (*c, *co))).collect();

    s += &diff_cc("OP_BASE", &op_rows, &refs.op_map);
    s.push('\n');
    s += &diff_cc("PRECOMPILES", &pc_rows, &refs.pc_map);
    s.push('\n');
    s += &diff_cc("FROP", &fr_rows, &fr_ref);

    // Built internally with ';'; swap to the requested separator (field content never contains ';').
    if sep != ';' {
        s = s.replace(';', &sep.to_string());
    }
    s
}

/// Compares two aggregate stats snapshots (each saved with `--save-stats`) without running the
/// emulator. `old_path` is the reference; deltas are `new - old`. `csv = true` selects the previous
/// plain view (delimited by `sep`); otherwise the colour-coded view (honouring `color`).
pub fn diff_stats_files(
    old_path: &str,
    new_path: &str,
    csv: bool,
    color: bool,
    sep: char,
) -> std::io::Result<String> {
    let old_text = std::fs::read_to_string(old_path)?;
    let new_text = std::fs::read_to_string(new_path)?;
    Ok(if csv {
        diff_snapshots_csv(old_path, &old_text, new_path, &new_text, sep)
    } else {
        diff_snapshots(old_path, &old_text, new_path, &new_text, color)
    })
}

/// Resolves a `--color=auto|always|never` choice to an on/off flag. `auto` (and any unknown value)
/// enables colour only when stdout is a terminal.
pub fn resolve_color(when: &str) -> bool {
    match when {
        "always" => true,
        "never" => false,
        _ => std::io::stdout().is_terminal(),
    }
}
