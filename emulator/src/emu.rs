use std::mem;

use crate::{
    ElfSymbolReader, EmuContext, EmuFullTraceStep, EmuOptions, EmuRegTrace, ParEmuOptions,
};
use fields::PrimeField64;
use mem_common::MemHelpers;
use riscv::RiscVRegisters;
use zisk_common::{
    OperationBusData, RomBusData, MAX_OPERATION_DATA_SIZE, MEM_BUS_ID, OPERATION_BUS_ID, ROM_BUS_ID,
};
// #[cfg(feature = "sp")]
// use zisk_core::SRC_SP;
use data_bus::DataBusTrait;
use zisk_common::{EmuTrace, EmuTraceStart};
use zisk_core::zisk_ops::ZiskOp;
use zisk_core::{
    EmulationMode, InstContext, Mem, ZiskInst, ZiskOperationType, ZiskRom, FREG_F0, FREG_INST,
    FREG_RA, FREG_X0, OUTPUT_ADDR, ROM_ENTRY, SRC_C, SRC_IMM, SRC_IND, SRC_MEM, SRC_REG, SRC_STEP,
    STORE_IND, STORE_MEM, STORE_NONE, STORE_REG,
};

/// ZisK emulator structure, containing the ZisK rom, the list of ZisK operations, and the
/// execution context
pub struct Emu<'a> {
    /// ZisK rom, containing the program to execute, which is constant for this program except for
    /// the input data
    pub rom: &'a ZiskRom,
    /// Context, where the state of the execution is stored and modified at every execution step
    pub ctx: EmuContext,
    // This array is used to store static data to avoid heap allocations and speed up the
    // conversion of data to be written to the bus
    static_array: [u64; MAX_OPERATION_DATA_SIZE],
}

/// ZisK emulator structure implementation
/// There are different modes of execution for different purposes:
/// - run -> step -> source_a, source_b, store_c (full functionality, called by main state machine,
///   calls callback with trace)
/// - run -> run_fast -> step_fast -> source_a, source_b, store_c (maximum speed, for benchmarking)
/// - run_slice -> step_slice -> source_a_slice, source_b_slice, store_c_slice (generates full trace
///   and required input data for secondary state machines)
///
/// There are 2 main SM emulation modes that generate memory reads:
///
/// 1.- When called from the Witness Computation library as part of a proof generation process, or similar:
///
/// 1.a.- First, to generate the minimal trace (memory reads):
///
/// ZiskExecutor::execute(&self, pctx: Arc<ProofCtx<F>>, input_data_path: Option<PathBuf>) -> Vec<usize>
///     ZiskExecutor::execute_with_emulator(&self, input_data_path: Option<PathBuf>) -> MinimalTraces
///         ZiskExecutor::run_emulator(&self, num_threads: usize, input_data_path: Option<PathBuf>) -> MinimalTraces
///             ZiskEmulator::compute_minimal_traces(rom: &ZiskRom, inputs: &[u8], options: &EmuOptions, num_threads: usize,) -> Result<Vec<EmuTrace>, ZiskEmulatorErr>
///                 Emu::par_run(&mut self, inputs: Vec<u8>, options: &EmuOptions, par_options: &ParEmuOptions,) -> Vec<EmuTrace>
///                     Emu:: par_step_my_block(&mut self, emu_full_trace_vec: &mut EmuTrace)
///                         Emu::source_a_mem_reads_generate(instruction, &mut emu_full_trace_vec.mem_reads);
///
/// 1.b.- Then, to consume the minimal trace (memory reads):
///
/// ZiskExecutor::calculate_witness(&self, stage: u32, pctx: Arc<ProofCtx<F>>, sctx: Arc<SetupCtx<F>>, global_ids: &[usize], n_cores: usize, buffer_pool: &dyn BufferPool<F>,)
///     ZiskExecutor::witness_main_instance(&self, pctx: &ProofCtx<F>, main_instance: &MainInstance, trace_buffer: Vec<F>,)
///         MainSM::compute_witness<F: PrimeField64>(zisk_rom: &ZiskRom, min_traces: &[EmuTrace], chunk_size: u64, main_instance: &MainInstance, std: Arc<Std<F>>, trace_buffer: Vec<F>,) -> AirInstance<F>
///             MainSM::fill_partial_trace<F: PrimeField64>(zisk_rom: &ZiskRom, main_trace: &mut [MainTraceRow<F>], min_trace: &EmuTrace, chunk_size: u64, reg_trace: &mut EmuRegTrace, step_range_check: &mut [u32], last_reg_values: bool,) -> (u64, Vec<u64>)
///                 Emu::step_slice_full_trace<F: PrimeField64>(&mut self, mem_reads: &[u64], mem_reads_index: &mut usize, reg_trace: &mut EmuRegTrace, step_range_check: Option<&mut [u32]>,) -> EmuFullTraceStep<F>
///                     Emu::source_a_mem_reads_consume(&mut self, instruction: &ZiskInst, mem_reads: &[u64], mem_reads_index: &mut usize, reg_trace: &mut EmuRegTrace,)
///
/// 2.- When called from ZiskEmu to simply emulate a RISC-V ELF file with an input file:
///
/// ZiskEmu::main()
///     ZiskEmulator::emulate(&self, options: &EmuOptions, callback: Option<impl Fn(EmuTrace)>,) -> Result<Vec<u8>, ZiskEmulatorErr>
///         ZiskEmulator::process_elf_file(elf_filename: String, inputs: &[u8], options: &EmuOptions, callback: Option<impl Fn(EmuTrace)>,) -> Result<Vec<u8>, ZiskEmulatorErr>
///             ZiskEmulator::process_rom(rom: &ZiskRom, inputs: &[u8], options: &EmuOptions, callback: Option<impl Fn(EmuTrace)>,) -> Result<Vec<u8>, ZiskEmulatorErr>
///                 Emu::run(&mut self, inputs: Vec<u8>, options: &EmuOptions, callback: Option<impl Fn(EmuTrace)>,)
///                     Emu::run_gen_trace(&mut self, options: &EmuOptions, par_options: &ParEmuOptions,) -> Vec<EmuTrace>
///                         Emu::par_step_my_block(&mut self, emu_full_trace_vec: &mut EmuTrace)
///                             Emu::source_a_mem_reads_generate(instruction, &mut emu_full_trace_vec.mem_reads);
impl<'a> Emu<'a> {
    pub fn new(rom: &ZiskRom) -> Emu<'_> {
        Emu { rom, ctx: EmuContext::default(), static_array: [0; MAX_OPERATION_DATA_SIZE] }
    }

    pub fn from_emu_trace_start(rom: &'a ZiskRom, trace_start: &'a EmuTraceStart) -> Emu<'a> {
        let mut emu = Emu::new(rom);
        emu.ctx.inst_ctx.pc = trace_start.pc;
        emu.ctx.inst_ctx.sp = trace_start.sp;
        emu.ctx.inst_ctx.step = trace_start.step;
        emu.ctx.inst_ctx.c = trace_start.c;
        emu.ctx.inst_ctx.regs = trace_start.regs;

        emu
    }

    pub fn create_emu_context(&mut self, inputs: Vec<u8>) -> EmuContext {
        // Initialize an empty instance
        let mut ctx = EmuContext::new(inputs);

        // Create a new read section for every RO data entry of the rom
        for i in 0..self.rom.ro_data.len() {
            ctx.inst_ctx.mem.add_read_section(self.rom.ro_data[i].from, &self.rom.ro_data[i].data);
        }

        // Sort read sections by start address to improve performance when using binary search
        ctx.inst_ctx.mem.read_sections.sort_by(|a, b| a.start.cmp(&b.start));

        // Get registers
        //emu.get_regs(); // TODO: ask Jordi

        ctx
    }

    /// Calculate the 'a' register value based on the source specified by the current instruction
    #[inline(always)]
    pub fn source_a(&mut self, instruction: &ZiskInst) {
        match instruction.a_src {
            SRC_C => self.ctx.inst_ctx.a = self.ctx.inst_ctx.c,
            SRC_REG => {
                // Calculate memory address
                let reg = instruction.a_offset_imm0 as usize;
                self.ctx.inst_ctx.a = self.get_reg(reg);

                if self.ctx.do_stats {
                    self.ctx.stats.on_register_read(instruction.a_offset_imm0 as usize);
                }
            }
            SRC_MEM => {
                // Calculate memory address
                let mut address = instruction.a_offset_imm0;
                if instruction.a_use_sp_imm1 != 0 {
                    address += self.ctx.inst_ctx.sp;
                }

                if address < 0x200 {
                    println!("ALERT INSTRUCTION: {:?} PC: {:X}", instruction, self.ctx.inst_ctx.pc);
                }
                // get it from memory
                self.ctx.inst_ctx.a = self.ctx.inst_ctx.mem.read(address, 8);

                // Feed the stats
                if self.ctx.do_stats {
                    self.ctx.stats.on_memory_read(address, 8);
                }
            }
            SRC_IMM => {
                self.ctx.inst_ctx.a = instruction.a_offset_imm0 | (instruction.a_use_sp_imm1 << 32)
            }
            SRC_STEP => self.ctx.inst_ctx.a = self.ctx.inst_ctx.step,
            // #[cfg(feature = "sp")]
            // SRC_SP => self.ctx.inst_ctx.a = self.ctx.inst_ctx.sp,
            _ => panic!(
                "Emu::source_a() Invalid a_src={} pc={}",
                instruction.a_src, self.ctx.inst_ctx.pc
            ),
        }
    }

    /// Calculate the 'a' register value based on the source specified by the current instruction
    /// and generate memory reads data
    #[inline(always)]
    pub fn source_a_mem_reads_generate(
        &mut self,
        instruction: &ZiskInst,
        mem_reads: &mut Vec<u64>,
    ) {
        match instruction.a_src {
            SRC_C => self.ctx.inst_ctx.a = self.ctx.inst_ctx.c,
            SRC_REG => {
                // Calculate memory address
                let reg = instruction.a_offset_imm0 as usize;
                self.ctx.inst_ctx.a = self.get_reg(reg);

                if self.ctx.do_stats {
                    self.ctx.stats.on_register_read(instruction.a_offset_imm0 as usize);
                }
            }
            SRC_MEM => {
                // Calculate memory address
                let mut address = instruction.a_offset_imm0;
                if instruction.a_use_sp_imm1 != 0 {
                    address += self.ctx.inst_ctx.sp;
                }

                // If the operation is a register operation, get it from the context registers
                if Mem::is_full_aligned(address, 8) {
                    self.ctx.inst_ctx.a = self.ctx.inst_ctx.mem.read(address, 8);
                    mem_reads.push(self.ctx.inst_ctx.a);
                } else {
                    let mut additional_data: Vec<u64>;
                    (self.ctx.inst_ctx.a, additional_data) =
                        self.ctx.inst_ctx.mem.read_required(address, 8);
                    debug_assert!(!additional_data.is_empty());
                    mem_reads.append(&mut additional_data);
                }
                /*println!(
                    "Emu::source_a_mem_reads_generate() mem_leads.len={} value={:x}",
                    mem_reads.len(),
                    self.ctx.inst_ctx.a
                );*/

                // Feed the stats
                if self.ctx.do_stats {
                    self.ctx.stats.on_memory_read(address, 8);
                }
            }
            SRC_IMM => {
                self.ctx.inst_ctx.a = instruction.a_offset_imm0 | (instruction.a_use_sp_imm1 << 32)
            }
            SRC_STEP => self.ctx.inst_ctx.a = self.ctx.inst_ctx.step,
            // #[cfg(feature = "sp")]
            // SRC_SP => self.ctx.inst_ctx.a = self.ctx.inst_ctx.sp,
            _ => panic!(
                "Emu::source_a_mem_reads_generate() Invalid a_src={} pc={}",
                instruction.a_src, self.ctx.inst_ctx.pc
            ),
        }
    }

    /// Calculate the 'a' register value based on the source specified by the current instruction,
    /// using formerly generated memory reads from a previous emulation
    #[inline(always)]
    pub fn source_a_mem_reads_consume(
        &mut self,
        instruction: &ZiskInst,
        mem_reads: &[u64],
        mem_reads_index: &mut usize,
        reg_trace: &mut EmuRegTrace,
    ) {
        match instruction.a_src {
            SRC_C => self.ctx.inst_ctx.a = self.ctx.inst_ctx.c,
            SRC_REG => {
                self.ctx.inst_ctx.a =
                    self.get_traced_reg(instruction.a_offset_imm0 as usize, 0, reg_trace);

                if self.ctx.do_stats {
                    self.ctx.stats.on_register_read(instruction.a_offset_imm0 as usize);
                }
            }
            SRC_MEM => {
                // Calculate memory address
                let mut address = instruction.a_offset_imm0;
                if instruction.a_use_sp_imm1 != 0 {
                    address += self.ctx.inst_ctx.sp;
                }

                // get it from memory
                if Mem::is_full_aligned(address, 8) {
                    assert!(*mem_reads_index < mem_reads.len());
                    self.ctx.inst_ctx.a = mem_reads[*mem_reads_index];
                    *mem_reads_index += 1;
                } else {
                    let (required_address_1, required_address_2) =
                        Mem::required_addresses(address, 8);
                    debug_assert!(required_address_1 != required_address_2);
                    assert!(*mem_reads_index < mem_reads.len());
                    let raw_data_1 = mem_reads[*mem_reads_index];
                    *mem_reads_index += 1;
                    assert!(*mem_reads_index < mem_reads.len());
                    let raw_data_2 = mem_reads[*mem_reads_index];
                    *mem_reads_index += 1;
                    self.ctx.inst_ctx.a =
                        Mem::get_double_not_aligned_data(address, 8, raw_data_1, raw_data_2);
                }
                /*println!(
                    "Emu::source_a_mem_reads_consume() mem_leads_index={} value={:x}",
                    *mem_reads_index, self.ctx.inst_ctx.a
                );*/

                // Feed the stats
                if self.ctx.do_stats {
                    self.ctx.stats.on_memory_read(address, 8);
                }
            }
            SRC_IMM => {
                self.ctx.inst_ctx.a = instruction.a_offset_imm0 | (instruction.a_use_sp_imm1 << 32)
            }
            SRC_STEP => self.ctx.inst_ctx.a = self.ctx.inst_ctx.step,
            // #[cfg(feature = "sp")]
            // SRC_SP => self.ctx.inst_ctx.a = self.ctx.inst_ctx.sp,
            _ => panic!(
                "Emu::source_a_mem_reads_consume() Invalid a_src={} pc={}",
                instruction.a_src, self.ctx.inst_ctx.pc
            ),
        }
    }

    /// Calculate the 'a' register value based on the source specified by the current instruction,
    /// using formerly generated memory reads from a previous emulation
    #[inline(always)]
    pub fn source_a_mem_reads_consume_databus<T, DB: DataBusTrait<u64, T>>(
        &mut self,
        instruction: &ZiskInst,
        mem_reads: &[u64],
        mem_reads_index: &mut usize,
        data_bus: &mut DB,
    ) {
        match instruction.a_src {
            SRC_C => self.ctx.inst_ctx.a = self.ctx.inst_ctx.c,
            SRC_REG => {
                self.ctx.inst_ctx.a = self.get_reg(instruction.a_offset_imm0 as usize);

                if self.ctx.do_stats {
                    self.ctx.stats.on_register_read(instruction.a_offset_imm0 as usize);
                }
            }
            SRC_MEM => {
                // Calculate memory address
                let mut address = instruction.a_offset_imm0;
                if instruction.a_use_sp_imm1 != 0 {
                    address += self.ctx.inst_ctx.sp;
                }

                // Otherwise, get it from memory
                if Mem::is_full_aligned(address, 8) {
                    assert!(*mem_reads_index < mem_reads.len());
                    self.ctx.inst_ctx.a = mem_reads[*mem_reads_index];
                    *mem_reads_index += 1;
                    let payload = MemHelpers::mem_load(
                        address as u32,
                        self.ctx.inst_ctx.step,
                        0,
                        8,
                        [self.ctx.inst_ctx.a, 0],
                    );
                    data_bus.write_to_bus(MEM_BUS_ID, &payload);
                } else {
                    let (required_address_1, required_address_2) =
                        Mem::required_addresses(address, 8);
                    debug_assert!(required_address_1 != required_address_2);
                    assert!(*mem_reads_index < mem_reads.len());
                    let raw_data_1 = mem_reads[*mem_reads_index];
                    *mem_reads_index += 1;
                    assert!(*mem_reads_index < mem_reads.len());
                    let raw_data_2 = mem_reads[*mem_reads_index];
                    *mem_reads_index += 1;
                    self.ctx.inst_ctx.a =
                        Mem::get_double_not_aligned_data(address, 8, raw_data_1, raw_data_2);
                    let payload = MemHelpers::mem_load(
                        address as u32,
                        self.ctx.inst_ctx.step,
                        0,
                        8,
                        [raw_data_1, raw_data_2],
                    );
                    data_bus.write_to_bus(MEM_BUS_ID, &payload);
                }
                /*println!(
                    "Emu::source_a_mem_reads_consume() mem_leads_index={} value={:x}",
                    *mem_reads_index, self.ctx.inst_ctx.a
                );*/

                // Feed the stats
                if self.ctx.do_stats {
                    self.ctx.stats.on_memory_read(address, 8);
                }
            }
            SRC_IMM => {
                self.ctx.inst_ctx.a = instruction.a_offset_imm0 | (instruction.a_use_sp_imm1 << 32)
            }
            SRC_STEP => self.ctx.inst_ctx.a = self.ctx.inst_ctx.step,
            // #[cfg(feature = "sp")]
            // SRC_SP => self.ctx.inst_ctx.a = self.ctx.inst_ctx.sp,
            _ => panic!(
                "Emu::source_a_mem_reads_consume_databus() Invalid a_src={} pc={}",
                instruction.a_src, self.ctx.inst_ctx.pc
            ),
        }
    }

    /// Calculate the 'a' register value based on the source specified by the current instruction,
    /// using formerly generated memory reads from a previous emulation
    #[inline(always)]
    pub fn source_a_mem_reads_consume_no_mem_ops(
        &mut self,
        instruction: &ZiskInst,
        mem_reads: &[u64],
        mem_reads_index: &mut usize,
    ) {
        match instruction.a_src {
            SRC_C => self.ctx.inst_ctx.a = self.ctx.inst_ctx.c,
            SRC_REG => {
                self.ctx.inst_ctx.a = self.get_reg(instruction.a_offset_imm0 as usize);

                if self.ctx.do_stats {
                    self.ctx.stats.on_register_read(instruction.a_offset_imm0 as usize);
                }
            }
            SRC_MEM => {
                // Calculate memory address
                let mut address = instruction.a_offset_imm0;
                if instruction.a_use_sp_imm1 != 0 {
                    address += self.ctx.inst_ctx.sp;
                }

                // Otherwise, get it from memory
                if Mem::is_full_aligned(address, 8) {
                    assert!(*mem_reads_index < mem_reads.len());
                    self.ctx.inst_ctx.a = mem_reads[*mem_reads_index];
                    *mem_reads_index += 1;
                } else {
                    let (required_address_1, required_address_2) =
                        Mem::required_addresses(address, 8);
                    debug_assert!(required_address_1 != required_address_2);
                    assert!(*mem_reads_index < mem_reads.len());
                    let raw_data_1 = mem_reads[*mem_reads_index];
                    *mem_reads_index += 1;
                    assert!(*mem_reads_index < mem_reads.len());
                    let raw_data_2 = mem_reads[*mem_reads_index];
                    *mem_reads_index += 1;
                    self.ctx.inst_ctx.a =
                        Mem::get_double_not_aligned_data(address, 8, raw_data_1, raw_data_2);
                }
                /*println!(
                    "Emu::source_a_mem_reads_consume() mem_leads_index={} value={:x}",
                    *mem_reads_index, self.ctx.inst_ctx.a
                );*/

                // Feed the stats
                if self.ctx.do_stats {
                    self.ctx.stats.on_memory_read(address, 8);
                }
            }
            SRC_IMM => {
                self.ctx.inst_ctx.a = instruction.a_offset_imm0 | (instruction.a_use_sp_imm1 << 32)
            }
            SRC_STEP => self.ctx.inst_ctx.a = self.ctx.inst_ctx.step,
            // #[cfg(feature = "sp")]
            // SRC_SP => self.ctx.inst_ctx.a = self.ctx.inst_ctx.sp,
            _ => panic!(
                "Emu::source_a_mem_reads_consume_databus() Invalid a_src={} pc={}",
                instruction.a_src, self.ctx.inst_ctx.pc
            ),
        }
    }

    /// Calculate the 'b' register value based on the source specified by the current instruction
    #[inline(always)]
    pub fn source_b(&mut self, instruction: &ZiskInst) {
        match instruction.b_src {
            SRC_C => self.ctx.inst_ctx.b = self.ctx.inst_ctx.c,
            SRC_REG => {
                // Calculate memory address
                self.ctx.inst_ctx.b = self.get_reg(instruction.b_offset_imm0 as usize);

                if self.ctx.do_stats {
                    self.ctx.stats.on_register_read(instruction.b_offset_imm0 as usize);
                }
            }
            SRC_MEM => {
                // Calculate memory address
                let mut addr = instruction.b_offset_imm0;
                if instruction.b_use_sp_imm1 != 0 {
                    addr += self.ctx.inst_ctx.sp;
                }

                // Get it from memory
                self.ctx.inst_ctx.b = self.ctx.inst_ctx.mem.read(addr, 8);

                if self.ctx.do_stats {
                    self.ctx.stats.on_memory_read(addr, 8);
                }
            }
            SRC_IMM => {
                self.ctx.inst_ctx.b = instruction.b_offset_imm0 | (instruction.b_use_sp_imm1 << 32)
            }
            SRC_IND => {
                // Calculate memory address
                let mut addr =
                    (self.ctx.inst_ctx.a as i64 + instruction.b_offset_imm0 as i64) as u64;
                if instruction.b_use_sp_imm1 != 0 {
                    addr += self.ctx.inst_ctx.sp;
                }

                // get it from memory
                self.ctx.inst_ctx.b = self.ctx.inst_ctx.mem.read(addr, instruction.ind_width);

                if self.ctx.do_stats {
                    self.ctx.stats.on_memory_read(addr, instruction.ind_width);
                }
            }
            _ => panic!(
                "Emu::source_b() Invalid b_src={} pc={}",
                instruction.b_src, self.ctx.inst_ctx.pc
            ),
        }
    }

    /// Calculate the 'b' register value based on the source specified by the current instruction
    #[inline(always)]
    pub fn source_b_mem_reads_generate(
        &mut self,
        instruction: &ZiskInst,
        mem_reads: &mut Vec<u64>,
    ) {
        match instruction.b_src {
            SRC_C => self.ctx.inst_ctx.b = self.ctx.inst_ctx.c,
            SRC_REG => {
                self.ctx.inst_ctx.b = self.get_reg(instruction.b_offset_imm0 as usize);

                if self.ctx.do_stats {
                    self.ctx.stats.on_register_read(instruction.b_offset_imm0 as usize);
                }
            }
            SRC_MEM => {
                // Calculate memory address
                let mut address = instruction.b_offset_imm0;
                if instruction.b_use_sp_imm1 != 0 {
                    address += self.ctx.inst_ctx.sp;
                }
                if Mem::is_full_aligned(address, 8) {
                    self.ctx.inst_ctx.b = self.ctx.inst_ctx.mem.read(address, 8);
                    mem_reads.push(self.ctx.inst_ctx.b);
                } else {
                    let mut additional_data: Vec<u64>;
                    (self.ctx.inst_ctx.b, additional_data) =
                        self.ctx.inst_ctx.mem.read_required(address, 8);

                    // debug_assert!(!additional_data.is_empty());
                    if additional_data.is_empty() {
                        println!("ADDITIONAL DATA IS EMPTY 0x{address:X}");
                    }
                    mem_reads.append(&mut additional_data);
                }
                /*println!(
                    "Emu::source_b_mem_reads_generate() mem_leads.len={} value={:x}",
                    mem_reads.len(),
                    self.ctx.inst_ctx.b
                );*/

                if self.ctx.do_stats {
                    self.ctx.stats.on_memory_read(address, 8);
                }
            }
            SRC_IMM => {
                self.ctx.inst_ctx.b = instruction.b_offset_imm0 | (instruction.b_use_sp_imm1 << 32)
            }
            SRC_IND => {
                // Calculate memory address
                let mut address =
                    (self.ctx.inst_ctx.a as i64 + instruction.b_offset_imm0 as i64) as u64;
                if instruction.b_use_sp_imm1 != 0 {
                    address += self.ctx.inst_ctx.sp;
                }

                // If the operation is a register operation, get it from the context registers
                if Mem::is_full_aligned(address, instruction.ind_width) {
                    self.ctx.inst_ctx.b =
                        self.ctx.inst_ctx.mem.read(address, instruction.ind_width);
                    mem_reads.push(self.ctx.inst_ctx.b);
                } else {
                    let mut additional_data: Vec<u64>;
                    (self.ctx.inst_ctx.b, additional_data) =
                        self.ctx.inst_ctx.mem.read_required(address, instruction.ind_width);
                    debug_assert!(!additional_data.is_empty());
                    mem_reads.append(&mut additional_data);
                }
                /*println!(
                    "Emu::source_b_mem_reads_generate() mem_leads.len={} value={:x}",
                    mem_reads.len(),
                    self.ctx.inst_ctx.b
                );*/

                if self.ctx.do_stats {
                    self.ctx.stats.on_memory_read(address, instruction.ind_width);
                }
            }
            _ => panic!(
                "Emu::source_b_mem_reads_generate() Invalid b_src={} pc={}",
                instruction.b_src, self.ctx.inst_ctx.pc
            ),
        }
    }

    /// Calculate the 'b' register value based on the source specified by the current instruction,
    /// using formerly generated memory reads from a previous emulation
    #[inline(always)]
    pub fn source_b_mem_reads_consume(
        &mut self,
        instruction: &ZiskInst,
        mem_reads: &[u64],
        mem_reads_index: &mut usize,
        reg_trace: &mut EmuRegTrace,
    ) {
        match instruction.b_src {
            SRC_C => self.ctx.inst_ctx.b = self.ctx.inst_ctx.c,
            SRC_REG => {
                self.ctx.inst_ctx.b =
                    self.get_traced_reg(instruction.b_offset_imm0 as usize, 1, reg_trace);

                if self.ctx.do_stats {
                    self.ctx.stats.on_register_read(instruction.b_offset_imm0 as usize);
                }
            }
            SRC_MEM => {
                // Calculate memory address
                let mut address = instruction.b_offset_imm0;
                if instruction.b_use_sp_imm1 != 0 {
                    address += self.ctx.inst_ctx.sp;
                }

                // Get it from memory
                if Mem::is_full_aligned(address, 8) {
                    assert!(*mem_reads_index < mem_reads.len());
                    self.ctx.inst_ctx.b = mem_reads[*mem_reads_index];
                    *mem_reads_index += 1;
                } else {
                    let (required_address_1, required_address_2) =
                        Mem::required_addresses(address, 8);
                    if required_address_1 == required_address_2 {
                        assert!(*mem_reads_index < mem_reads.len());
                        let raw_data = mem_reads[*mem_reads_index];
                        *mem_reads_index += 1;
                        self.ctx.inst_ctx.b =
                            Mem::get_single_not_aligned_data(address, 8, raw_data);
                    } else {
                        assert!(*mem_reads_index < mem_reads.len());
                        let raw_data_1 = mem_reads[*mem_reads_index];
                        *mem_reads_index += 1;
                        assert!(*mem_reads_index < mem_reads.len());
                        let raw_data_2 = mem_reads[*mem_reads_index];
                        *mem_reads_index += 1;
                        self.ctx.inst_ctx.b =
                            Mem::get_double_not_aligned_data(address, 8, raw_data_1, raw_data_2);
                    }
                }
                /*println!(
                    "Emu::source_b_mem_reads_consume() mem_leads_index={} value={:x}",
                    *mem_reads_index, self.ctx.inst_ctx.b
                );*/

                if self.ctx.do_stats {
                    self.ctx.stats.on_memory_read(address, 8);
                }
            }
            SRC_IMM => {
                self.ctx.inst_ctx.b = instruction.b_offset_imm0 | (instruction.b_use_sp_imm1 << 32)
            }
            SRC_IND => {
                // Calculate memory address
                let mut address =
                    (self.ctx.inst_ctx.a as i64 + instruction.b_offset_imm0 as i64) as u64;
                if instruction.b_use_sp_imm1 != 0 {
                    address += self.ctx.inst_ctx.sp;
                }

                // Otherwise, get it from memory
                if Mem::is_full_aligned(address, instruction.ind_width) {
                    assert!(*mem_reads_index < mem_reads.len());
                    self.ctx.inst_ctx.b = mem_reads[*mem_reads_index];
                    *mem_reads_index += 1;
                } else {
                    let (required_address_1, required_address_2) =
                        Mem::required_addresses(address, instruction.ind_width);
                    if required_address_1 == required_address_2 {
                        assert!(*mem_reads_index < mem_reads.len());
                        let raw_data = mem_reads[*mem_reads_index];
                        *mem_reads_index += 1;
                        self.ctx.inst_ctx.b = Mem::get_single_not_aligned_data(
                            address,
                            instruction.ind_width,
                            raw_data,
                        );
                    } else {
                        assert!(*mem_reads_index < mem_reads.len());
                        let raw_data_1 = mem_reads[*mem_reads_index];
                        *mem_reads_index += 1;
                        assert!(*mem_reads_index < mem_reads.len());
                        let raw_data_2 = mem_reads[*mem_reads_index];
                        *mem_reads_index += 1;
                        self.ctx.inst_ctx.b = Mem::get_double_not_aligned_data(
                            address,
                            instruction.ind_width,
                            raw_data_1,
                            raw_data_2,
                        );
                    }
                }
                /*println!(
                    "Emu::source_b_mem_reads_consume() mem_leads_index={} value={:x}",
                    *mem_reads_index, self.ctx.inst_ctx.b
                );*/

                if self.ctx.do_stats {
                    self.ctx.stats.on_memory_read(address, instruction.ind_width);
                }
            }
            _ => panic!(
                "Emu::source_b_mem_reads_consume() Invalid b_src={} pc={}",
                instruction.b_src, self.ctx.inst_ctx.pc
            ),
        }
    }

    /// Calculate the 'b' register value based on the source specified by the current instruction,
    /// using formerly generated memory reads from a previous emulation
    #[inline(always)]
    pub fn source_b_mem_reads_consume_databus<T, DB: DataBusTrait<u64, T>>(
        &mut self,
        instruction: &ZiskInst,
        mem_reads: &[u64],
        mem_reads_index: &mut usize,
        data_bus: &mut DB,
    ) {
        match instruction.b_src {
            SRC_C => self.ctx.inst_ctx.b = self.ctx.inst_ctx.c,
            SRC_REG => {
                self.ctx.inst_ctx.b = self.get_reg(instruction.b_offset_imm0 as usize);

                if self.ctx.do_stats {
                    self.ctx.stats.on_register_read(instruction.b_offset_imm0 as usize);
                }
            }
            SRC_MEM => {
                // Calculate memory address
                let mut address = instruction.b_offset_imm0;
                if instruction.b_use_sp_imm1 != 0 {
                    address += self.ctx.inst_ctx.sp;
                }

                // Otherwise, get it from memory
                if Mem::is_full_aligned(address, 8) {
                    assert!(*mem_reads_index < mem_reads.len());
                    self.ctx.inst_ctx.b = mem_reads[*mem_reads_index];

                    *mem_reads_index += 1;
                    let payload = MemHelpers::mem_load(
                        address as u32,
                        self.ctx.inst_ctx.step,
                        1,
                        8,
                        [self.ctx.inst_ctx.b, 0],
                    );
                    data_bus.write_to_bus(MEM_BUS_ID, &payload);
                } else {
                    let (required_address_1, required_address_2) =
                        Mem::required_addresses(address, 8);
                    if required_address_1 == required_address_2 {
                        assert!(*mem_reads_index < mem_reads.len());
                        let raw_data = mem_reads[*mem_reads_index];
                        *mem_reads_index += 1;
                        self.ctx.inst_ctx.b =
                            Mem::get_single_not_aligned_data(address, 8, raw_data);
                        let payload = MemHelpers::mem_load(
                            address as u32,
                            self.ctx.inst_ctx.step,
                            1,
                            8,
                            [raw_data, 0],
                        );
                        data_bus.write_to_bus(MEM_BUS_ID, &payload);
                    } else {
                        assert!(*mem_reads_index < mem_reads.len());
                        let raw_data_1 = mem_reads[*mem_reads_index];
                        *mem_reads_index += 1;
                        assert!(*mem_reads_index < mem_reads.len());
                        let raw_data_2 = mem_reads[*mem_reads_index];
                        *mem_reads_index += 1;
                        self.ctx.inst_ctx.b =
                            Mem::get_double_not_aligned_data(address, 8, raw_data_1, raw_data_2);
                        let payload = MemHelpers::mem_load(
                            address as u32,
                            self.ctx.inst_ctx.step,
                            1,
                            8,
                            [raw_data_1, raw_data_2],
                        );
                        data_bus.write_to_bus(MEM_BUS_ID, &payload);
                    }
                }
                /*println!(
                    "Emu::source_b_mem_reads_consume() mem_leads_index={} value={:x}",
                    *mem_reads_index, self.ctx.inst_ctx.b
                );*/

                if self.ctx.do_stats {
                    self.ctx.stats.on_memory_read(address, 8);
                }
            }
            SRC_IMM => {
                self.ctx.inst_ctx.b = instruction.b_offset_imm0 | (instruction.b_use_sp_imm1 << 32)
            }
            SRC_IND => {
                // Calculate memory address
                let mut address =
                    (self.ctx.inst_ctx.a as i64 + instruction.b_offset_imm0 as i64) as u64;
                if instruction.b_use_sp_imm1 != 0 {
                    address += self.ctx.inst_ctx.sp;
                }

                // Otherwise, get it from memory
                if Mem::is_full_aligned(address, instruction.ind_width) {
                    assert!(*mem_reads_index < mem_reads.len());
                    self.ctx.inst_ctx.b = mem_reads[*mem_reads_index];
                    *mem_reads_index += 1;
                    let payload = MemHelpers::mem_load(
                        address as u32,
                        self.ctx.inst_ctx.step,
                        1,
                        8,
                        [self.ctx.inst_ctx.b, 0],
                    );
                    data_bus.write_to_bus(MEM_BUS_ID, &payload);
                } else {
                    let (required_address_1, required_address_2) =
                        Mem::required_addresses(address, instruction.ind_width);
                    if required_address_1 == required_address_2 {
                        assert!(*mem_reads_index < mem_reads.len());
                        let raw_data = mem_reads[*mem_reads_index];
                        *mem_reads_index += 1;
                        self.ctx.inst_ctx.b = Mem::get_single_not_aligned_data(
                            address,
                            instruction.ind_width,
                            raw_data,
                        );
                        let payload = MemHelpers::mem_load(
                            address as u32,
                            self.ctx.inst_ctx.step,
                            1,
                            instruction.ind_width as u8,
                            [raw_data, 0],
                        );
                        data_bus.write_to_bus(MEM_BUS_ID, &payload);
                    } else {
                        assert!(*mem_reads_index < mem_reads.len());
                        let raw_data_1 = mem_reads[*mem_reads_index];
                        *mem_reads_index += 1;
                        assert!(*mem_reads_index < mem_reads.len());
                        let raw_data_2 = mem_reads[*mem_reads_index];
                        *mem_reads_index += 1;
                        self.ctx.inst_ctx.b = Mem::get_double_not_aligned_data(
                            address,
                            instruction.ind_width,
                            raw_data_1,
                            raw_data_2,
                        );
                        let payload = MemHelpers::mem_load(
                            address as u32,
                            self.ctx.inst_ctx.step,
                            1,
                            8,
                            [raw_data_1, raw_data_2],
                        );
                        data_bus.write_to_bus(MEM_BUS_ID, &payload);
                    }
                }
                /*println!(
                    "Emu::source_b_mem_reads_consume() mem_leads_index={} value={:x}",
                    *mem_reads_index, self.ctx.inst_ctx.b
                );*/

                if self.ctx.do_stats {
                    self.ctx.stats.on_memory_read(address, instruction.ind_width);
                }
            }
            _ => panic!(
                "Emu::source_b_mem_reads_consume_databus() Invalid b_src={} pc={}",
                instruction.b_src, self.ctx.inst_ctx.pc
            ),
        }
    }

    /// Calculate the 'b' register value based on the source specified by the current instruction,
    /// using formerly generated memory reads from a previous emulation
    #[inline(always)]
    pub fn source_b_mem_reads_consume_no_mem_ops(
        &mut self,
        instruction: &ZiskInst,
        mem_reads: &[u64],
        mem_reads_index: &mut usize,
    ) {
        match instruction.b_src {
            SRC_C => self.ctx.inst_ctx.b = self.ctx.inst_ctx.c,
            SRC_REG => {
                self.ctx.inst_ctx.b = self.get_reg(instruction.b_offset_imm0 as usize);

                if self.ctx.do_stats {
                    self.ctx.stats.on_register_read(instruction.b_offset_imm0 as usize);
                }
            }
            SRC_MEM => {
                // Calculate memory address
                let mut address = instruction.b_offset_imm0;
                if instruction.b_use_sp_imm1 != 0 {
                    address += self.ctx.inst_ctx.sp;
                }

                // Otherwise, get it from memory
                if Mem::is_full_aligned(address, 8) {
                    assert!(*mem_reads_index < mem_reads.len());
                    self.ctx.inst_ctx.b = mem_reads[*mem_reads_index];

                    *mem_reads_index += 1;
                } else {
                    let (required_address_1, required_address_2) =
                        Mem::required_addresses(address, 8);
                    if required_address_1 == required_address_2 {
                        assert!(*mem_reads_index < mem_reads.len());
                        let raw_data = mem_reads[*mem_reads_index];
                        *mem_reads_index += 1;
                        self.ctx.inst_ctx.b =
                            Mem::get_single_not_aligned_data(address, 8, raw_data);
                    } else {
                        assert!(*mem_reads_index < mem_reads.len());
                        let raw_data_1 = mem_reads[*mem_reads_index];
                        *mem_reads_index += 1;
                        assert!(*mem_reads_index < mem_reads.len());
                        let raw_data_2 = mem_reads[*mem_reads_index];
                        *mem_reads_index += 1;
                        self.ctx.inst_ctx.b =
                            Mem::get_double_not_aligned_data(address, 8, raw_data_1, raw_data_2);
                    }
                }
                /*println!(
                    "Emu::source_b_mem_reads_consume() mem_leads_index={} value={:x}",
                    *mem_reads_index, self.ctx.inst_ctx.b
                );*/

                if self.ctx.do_stats {
                    self.ctx.stats.on_memory_read(address, 8);
                }
            }
            SRC_IMM => {
                self.ctx.inst_ctx.b = instruction.b_offset_imm0 | (instruction.b_use_sp_imm1 << 32)
            }
            SRC_IND => {
                // Calculate memory address
                let mut address =
                    (self.ctx.inst_ctx.a as i64 + instruction.b_offset_imm0 as i64) as u64;
                if instruction.b_use_sp_imm1 != 0 {
                    address += self.ctx.inst_ctx.sp;
                }

                // Otherwise, get it from memory
                if Mem::is_full_aligned(address, instruction.ind_width) {
                    assert!(*mem_reads_index < mem_reads.len());
                    self.ctx.inst_ctx.b = mem_reads[*mem_reads_index];
                    *mem_reads_index += 1;
                } else {
                    let (required_address_1, required_address_2) =
                        Mem::required_addresses(address, instruction.ind_width);
                    if required_address_1 == required_address_2 {
                        assert!(*mem_reads_index < mem_reads.len());
                        let raw_data = mem_reads[*mem_reads_index];
                        *mem_reads_index += 1;
                        self.ctx.inst_ctx.b = Mem::get_single_not_aligned_data(
                            address,
                            instruction.ind_width,
                            raw_data,
                        );
                    } else {
                        assert!(*mem_reads_index < mem_reads.len());
                        let raw_data_1 = mem_reads[*mem_reads_index];
                        *mem_reads_index += 1;
                        assert!(*mem_reads_index < mem_reads.len());
                        let raw_data_2 = mem_reads[*mem_reads_index];
                        *mem_reads_index += 1;
                        self.ctx.inst_ctx.b = Mem::get_double_not_aligned_data(
                            address,
                            instruction.ind_width,
                            raw_data_1,
                            raw_data_2,
                        );
                    }
                }
                /*println!(
                    "Emu::source_b_mem_reads_consume() mem_leads_index={} value={:x}",
                    *mem_reads_index, self.ctx.inst_ctx.b
                );*/

                if self.ctx.do_stats {
                    self.ctx.stats.on_memory_read(address, instruction.ind_width);
                }
            }
            _ => panic!(
                "Emu::source_b_mem_reads_consume_databus() Invalid b_src={} pc={}",
                instruction.b_src, self.ctx.inst_ctx.pc
            ),
        }
    }

    /// Store the 'c' register value based on the storage specified by the current instruction
    #[inline(always)]
    pub fn store_c(&mut self, instruction: &ZiskInst) {
        match instruction.store {
            STORE_NONE => {}
            STORE_REG => {
                if instruction.store_offset >= 32 {
                    println!("instruction ALERT 0 {instruction:?}");
                }

                self.set_reg(
                    instruction.store_offset as usize,
                    self.get_value_to_store(instruction),
                );

                if self.ctx.do_stats {
                    self.ctx.stats.on_register_write(instruction.store_offset as usize);
                }
            }
            STORE_MEM => {
                // Calculate value
                let val = self.get_value_to_store(instruction);

                // Calculate memory address
                let mut addr: i64 = instruction.store_offset;
                if instruction.store_use_sp {
                    addr += self.ctx.inst_ctx.sp as i64;
                }
                debug_assert!(addr >= 0);
                let addr = addr as u64;

                // get it from memory
                self.ctx.inst_ctx.mem.write(addr, val, 8);
                if self.ctx.do_stats {
                    self.ctx.stats.on_memory_write(addr, 8, val);
                }
            }
            STORE_IND => {
                // Calculate value
                let val: i64 = if instruction.store_ra {
                    self.ctx.inst_ctx.pc as i64 + instruction.jmp_offset2
                } else {
                    self.ctx.inst_ctx.c as i64
                };
                let val = val as u64;

                // Calculate memory address
                let mut addr = instruction.store_offset;
                if instruction.store_use_sp {
                    addr += self.ctx.inst_ctx.sp as i64;
                }
                addr += self.ctx.inst_ctx.a as i64;
                debug_assert!(addr >= 0, "addr is negative: addr={addr}=0x{addr:x}");
                let addr = addr as u64;

                // Get it from memory
                self.ctx.inst_ctx.mem.write(addr, val, instruction.ind_width);
                if self.ctx.do_stats {
                    self.ctx.stats.on_memory_write(addr, instruction.ind_width, val);
                }
            }
            _ => panic!(
                "Emu::store_c() Invalid store={} pc={}",
                instruction.store, self.ctx.inst_ctx.pc
            ),
        }
    }

    /// Store the 'c' register value based on the storage specified by the current instruction and
    /// log memory access if required
    #[inline(always)]
    pub fn store_c_mem_reads_generate(&mut self, instruction: &ZiskInst, mem_reads: &mut Vec<u64>) {
        match instruction.store {
            STORE_NONE => {}
            STORE_REG => {
                if instruction.store_offset >= 32 {
                    println!("instruction ALERT 1 {instruction:?}");
                }

                self.set_reg(
                    instruction.store_offset as usize,
                    self.get_value_to_store(instruction),
                );
            }
            STORE_MEM => {
                // Calculate the value
                let value = self.get_value_to_store(instruction);

                // Calculate the memory address
                let mut address: i64 = instruction.store_offset;
                if instruction.store_use_sp {
                    address += self.ctx.inst_ctx.sp as i64;
                }
                debug_assert!(address >= 0);
                let address = address as u64;

                // If not aligned, get old raw data from memory, then write it
                if Mem::is_full_aligned(address, 8) {
                    self.ctx.inst_ctx.mem.write(address, value, 8);
                } else {
                    let mut additional_data: Vec<u64>;
                    (self.ctx.inst_ctx.b, additional_data) =
                        self.ctx.inst_ctx.mem.read_required(address, 8);
                    debug_assert!(!additional_data.is_empty());
                    mem_reads.append(&mut additional_data);

                    self.ctx.inst_ctx.mem.write(address, value, 8);
                }
            }
            STORE_IND => {
                // Calculate the value
                let value = self.get_value_to_store(instruction);

                // Calculate the memory address
                let mut address = instruction.store_offset;
                if instruction.store_use_sp {
                    address += self.ctx.inst_ctx.sp as i64;
                }
                address += self.ctx.inst_ctx.a as i64;
                debug_assert!(address >= 0);
                let address = address as u64;

                // If the operation is a register operation, write it to the context registers
                // If not aligned, get old raw data from memory, then write it
                if Mem::is_full_aligned(address, instruction.ind_width) {
                    self.ctx.inst_ctx.mem.write(address, value, instruction.ind_width);
                } else {
                    let mut additional_data: Vec<u64>;
                    (self.ctx.inst_ctx.b, additional_data) =
                        self.ctx.inst_ctx.mem.read_required(address, instruction.ind_width);
                    debug_assert!(!additional_data.is_empty());
                    mem_reads.append(&mut additional_data);

                    self.ctx.inst_ctx.mem.write(address, value, instruction.ind_width);
                }
            }
            _ => panic!(
                "Emu::store_c_mem_reads_generate() Invalid store={} pc={}",
                instruction.store, self.ctx.inst_ctx.pc
            ),
        }
    }

    /// Store the 'c' register value based on the storage specified by the current instruction and
    /// log memory access if required
    #[inline(always)]
    pub fn store_c_mem_reads_consume(
        &mut self,
        instruction: &ZiskInst,
        mem_reads: &[u64],
        mem_reads_index: &mut usize,
        reg_trace: &mut EmuRegTrace,
    ) {
        match instruction.store {
            STORE_NONE => {}
            STORE_REG => {
                self.set_traced_reg(
                    instruction.store_offset as usize,
                    self.get_value_to_store(instruction),
                    reg_trace,
                );
            }
            STORE_MEM => {
                // Calculate the memory address
                let mut address: i64 = instruction.store_offset;
                if instruction.store_use_sp {
                    address += self.ctx.inst_ctx.sp as i64;
                }
                debug_assert!(address >= 0);
                let address = address as u64;

                // If not aligned, get old raw data from memory, then write it
                if !Mem::is_full_aligned(address, 8) {
                    let (required_address_1, required_address_2) =
                        Mem::required_addresses(address, 8);
                    if required_address_1 == required_address_2 {
                        assert!(*mem_reads_index < mem_reads.len());
                        *mem_reads_index += 1;
                    } else {
                        assert!(*mem_reads_index < mem_reads.len());
                        *mem_reads_index += 1;
                        assert!(*mem_reads_index < mem_reads.len());
                        *mem_reads_index += 1;
                    }
                }
            }
            STORE_IND => {
                // Calculate the memory address
                let mut address = instruction.store_offset;
                if instruction.store_use_sp {
                    address += self.ctx.inst_ctx.sp as i64;
                }
                address += self.ctx.inst_ctx.a as i64;
                debug_assert!(address >= 0);
                let address = address as u64;

                // If not aligned, get old raw data from memory, then write it
                if !Mem::is_full_aligned(address, instruction.ind_width) {
                    let (required_address_1, required_address_2) =
                        Mem::required_addresses(address, instruction.ind_width);
                    if required_address_1 == required_address_2 {
                        assert!(*mem_reads_index < mem_reads.len());
                        *mem_reads_index += 1;
                    } else {
                        assert!(*mem_reads_index < mem_reads.len());
                        *mem_reads_index += 1;
                        assert!(*mem_reads_index < mem_reads.len());
                        *mem_reads_index += 1;
                    }
                }
            }
            _ => panic!(
                "Emu::store_c_mem_reads_consume() Invalid store={} pc={}",
                instruction.store, self.ctx.inst_ctx.pc
            ),
        }
    }

    /// Store the 'c' register value based on the storage specified by the current instruction and
    /// log memory access if required
    #[inline(always)]
    pub fn store_c_mem_reads_consume_databus<T, DB: DataBusTrait<u64, T>>(
        &mut self,
        instruction: &ZiskInst,
        mem_reads: &[u64],
        mem_reads_index: &mut usize,
        data_bus: &mut DB,
    ) {
        match instruction.store {
            STORE_NONE => {}
            STORE_REG => {
                if instruction.store_offset >= 32 {
                    println!("instruction ALERT 2 {instruction:?}");
                }

                self.set_reg(
                    instruction.store_offset as usize,
                    self.get_value_to_store(instruction),
                );
            }
            STORE_MEM => {
                // Calculate the value
                let value = self.get_value_to_store(instruction);

                // Calculate the memory address
                let mut address: i64 = instruction.store_offset;
                if instruction.store_use_sp {
                    address += self.ctx.inst_ctx.sp as i64;
                }
                debug_assert!(address >= 0);
                let address = address as u64;

                if Mem::is_full_aligned(address, 8) {
                    let payload = MemHelpers::mem_write(
                        address as u32,
                        self.ctx.inst_ctx.step,
                        2,
                        8,
                        value,
                        [value, 0],
                    );
                    data_bus.write_to_bus(MEM_BUS_ID, &payload);
                }
                // Otherwise, if not aligned, get old raw data from memory, then write it
                else {
                    let (required_address_1, required_address_2) =
                        Mem::required_addresses(address, 8);
                    if required_address_1 == required_address_2 {
                        assert!(*mem_reads_index < mem_reads.len());
                        let raw_data = mem_reads[*mem_reads_index];
                        *mem_reads_index += 1;

                        let payload = MemHelpers::mem_write(
                            address as u32,
                            self.ctx.inst_ctx.step,
                            2,
                            8,
                            value,
                            [raw_data, 0],
                        );
                        data_bus.write_to_bus(MEM_BUS_ID, &payload);
                    } else {
                        assert!(*mem_reads_index < mem_reads.len());
                        let raw_data_1 = mem_reads[*mem_reads_index];
                        *mem_reads_index += 1;
                        assert!(*mem_reads_index < mem_reads.len());
                        let raw_data_2 = mem_reads[*mem_reads_index];
                        *mem_reads_index += 1;

                        let payload = MemHelpers::mem_write(
                            address as u32,
                            self.ctx.inst_ctx.step,
                            2,
                            8,
                            value,
                            [raw_data_1, raw_data_2],
                        );
                        data_bus.write_to_bus(MEM_BUS_ID, &payload);
                    }
                }
            }
            STORE_IND => {
                // Calculate the value
                let value = self.get_value_to_store(instruction);

                // Calculate the memory address
                let mut address = instruction.store_offset;
                if instruction.store_use_sp {
                    address += self.ctx.inst_ctx.sp as i64;
                }
                address += self.ctx.inst_ctx.a as i64;
                debug_assert!(address >= 0);
                let address = address as u64;

                // Otherwise, if aligned
                if Mem::is_full_aligned(address, instruction.ind_width) {
                    let payload = MemHelpers::mem_write(
                        address as u32,
                        self.ctx.inst_ctx.step,
                        2,
                        instruction.ind_width as u8,
                        value,
                        [value, 0],
                    );
                    data_bus.write_to_bus(MEM_BUS_ID, &payload);
                }
                // Otherwise, if not aligned, get old raw data from memory, then write it
                else {
                    let (required_address_1, required_address_2) =
                        Mem::required_addresses(address, instruction.ind_width);
                    if required_address_1 == required_address_2 {
                        assert!(*mem_reads_index < mem_reads.len());
                        let raw_data = mem_reads[*mem_reads_index];
                        *mem_reads_index += 1;

                        let payload = MemHelpers::mem_write(
                            address as u32,
                            self.ctx.inst_ctx.step,
                            2,
                            instruction.ind_width as u8,
                            value,
                            [raw_data, 0],
                        );
                        data_bus.write_to_bus(MEM_BUS_ID, &payload);
                    } else {
                        assert!(*mem_reads_index < mem_reads.len());
                        let raw_data_1 = mem_reads[*mem_reads_index];
                        *mem_reads_index += 1;
                        assert!(*mem_reads_index < mem_reads.len());
                        let raw_data_2 = mem_reads[*mem_reads_index];
                        *mem_reads_index += 1;

                        let payload = MemHelpers::mem_write(
                            address as u32,
                            self.ctx.inst_ctx.step,
                            2,
                            instruction.ind_width as u8,
                            value,
                            [raw_data_1, raw_data_2],
                        );
                        data_bus.write_to_bus(MEM_BUS_ID, &payload);
                    }
                }
            }
            _ => panic!(
                "Emu::store_c_mem_reads_consume_databus() Invalid store={} pc={}",
                instruction.store, self.ctx.inst_ctx.pc
            ),
        }
    }

    /// Store the 'c' register value based on the storage specified by the current instruction and
    /// log memory access if required
    #[inline(always)]
    pub fn store_c_mem_reads_consume_no_mem_ops(
        &mut self,
        instruction: &ZiskInst,
        mem_reads: &[u64],
        mem_reads_index: &mut usize,
    ) {
        match instruction.store {
            STORE_NONE => {}
            STORE_REG => {
                if instruction.store_offset >= 32 {
                    println!("instruction ALERT 2 {instruction:?}");
                }

                self.set_reg(
                    instruction.store_offset as usize,
                    self.get_value_to_store(instruction),
                );
            }
            STORE_MEM => {
                // Calculate the memory address
                let mut address: i64 = instruction.store_offset;
                if instruction.store_use_sp {
                    address += self.ctx.inst_ctx.sp as i64;
                }
                debug_assert!(address >= 0);
                let address = address as u64;

                // If not aligned, get old raw data from memory, then write it
                if !Mem::is_full_aligned(address, 8) {
                    let (required_address_1, required_address_2) =
                        Mem::required_addresses(address, 8);
                    if required_address_1 == required_address_2 {
                        assert!(*mem_reads_index < mem_reads.len());
                        *mem_reads_index += 1;
                    } else {
                        assert!(*mem_reads_index < mem_reads.len());
                        *mem_reads_index += 2;
                    }
                }
            }
            STORE_IND => {
                // Calculate the memory address
                let mut address = instruction.store_offset;
                if instruction.store_use_sp {
                    address += self.ctx.inst_ctx.sp as i64;
                }
                address += self.ctx.inst_ctx.a as i64;
                debug_assert!(address >= 0);
                let address = address as u64;

                // If not aligned, get old raw data from memory, then write it
                if !Mem::is_full_aligned(address, instruction.ind_width) {
                    let (required_address_1, required_address_2) =
                        Mem::required_addresses(address, instruction.ind_width);
                    if required_address_1 == required_address_2 {
                        assert!(*mem_reads_index < mem_reads.len());
                        *mem_reads_index += 1;
                    } else {
                        assert!(*mem_reads_index < mem_reads.len());
                        *mem_reads_index += 2;
                    }
                }
            }
            _ => panic!(
                "Emu::store_c_mem_reads_consume_databus() Invalid store={} pc={}",
                instruction.store, self.ctx.inst_ctx.pc
            ),
        }
    }

    // Set SP, if specified by the current instruction
    // #[cfg(feature = "sp")]
    // #[inline(always)]
    // pub fn set_sp(&mut self, instruction: &ZiskInst) {
    //     if instruction.set_sp {
    //         self.ctx.inst_ctx.sp = self.ctx.inst_ctx.c;
    //     } else {
    //         self.ctx.inst_ctx.sp += instruction.inc_sp;
    //     }
    // }
    /// Set PC, based on current PC, current flag and current instruction
    #[inline(always)]
    pub fn set_pc(&mut self, instruction: &ZiskInst) {
        if instruction.set_pc {
            self.ctx.inst_ctx.pc = (self.ctx.inst_ctx.c as i64 + instruction.jmp_offset1) as u64;
        } else if self.ctx.inst_ctx.flag {
            self.ctx.inst_ctx.pc = (self.ctx.inst_ctx.pc as i64 + instruction.jmp_offset1) as u64;
        } else {
            self.ctx.inst_ctx.pc = (self.ctx.inst_ctx.pc as i64 + instruction.jmp_offset2) as u64;
        }
    }

    /// Run the whole program, fast
    #[inline(always)]
    pub fn run_fast(&mut self, options: &EmuOptions) {
        while !self.ctx.inst_ctx.end && (self.ctx.inst_ctx.step < options.max_steps) {
            self.step_fast();
        }

        // Detect and report error
        if self.ctx.inst_ctx.error {
            eprintln!(
                "Emu::run_fast() finished with error at step={} pc=0x{:x}",
                self.ctx.inst_ctx.step, self.ctx.inst_ctx.pc
            );
        }
    }

    /// Performs one single step of the emulation
    #[inline(always)]
    pub fn step_fast(&mut self) {
        let instruction = self.rom.get_instruction(self.ctx.inst_ctx.pc);
        // let debug = instruction.op >= 0xf6;
        // let initial_regs = if debug {
        //     print!(
        //         "\x1B[1;36m>==IN ==>\x1B[0m SF #{} 0x{:X} ({}) {}",
        //         self.ctx.inst_ctx.step,
        //         self.ctx.inst_ctx.pc,
        //         instruction.op_str,
        //         instruction.verbose
        //     );
        //     for (index, &value) in self.ctx.inst_ctx.regs.iter().enumerate() {
        //         print!(" {:}:0x{:X}", index, value);
        //     }
        //     println!();
        //     self.ctx.inst_ctx.regs
        // } else {
        //     /* println!(
        //         "#{} 0x{:X} ({}) {}",
        //         self.ctx.inst_ctx.step,
        //         self.ctx.inst_ctx.pc,
        //         instruction.op_str,
        //         instruction.verbose
        //     );*/
        //     [0u64; 32]
        // };
        self.source_a(instruction);
        self.source_b(instruction);
        (instruction.func)(&mut self.ctx.inst_ctx);
        self.store_c(instruction);

        // #[cfg(feature = "sp")]
        // self.set_sp(instruction);

        self.set_pc(instruction);
        self.ctx.inst_ctx.end = instruction.end;
        self.ctx.inst_ctx.step += 1;
        // if debug {
        //     print!(
        //         ">==OUT==> #{} 0x{:X} ({}) {} {:?}",
        //         self.ctx.inst_ctx.step,
        //         self.ctx.inst_ctx.pc,
        //         instruction.op_str,
        //         instruction.verbose,
        //         self.ctx.inst_ctx.regs,
        //     );
        //     for (index, &value) in self.ctx.inst_ctx.regs.iter().enumerate() {
        //         if initial_regs[index] == value {
        //             print!(" {:}:0x{:X}", index, value);
        //         } else {
        //             print!(" {:}:\x1B[1;31m0x{:X}\x1B[0m", index, value);
        //         }
        //     }
        //     println!();
        // }
    }

    /// Run the whole program
    pub fn run(
        &mut self,
        inputs: Vec<u8>,
        options: &EmuOptions,
        callback: Option<impl Fn(EmuTrace)>,
    ) {
        // Context, where the state of the execution is stored and modified at every execution step
        self.ctx = self.create_emu_context(inputs.clone());

        let mut elf = ElfSymbolReader::new();
        if options.read_symbols {
            if let Some(elf_file) = &options.elf {
                println!("Loading symbols from ELF file: {elf_file}");
                elf.load_from_file(elf_file).unwrap();
                let mut count = 0;
                for symbol in elf.functions() {
                    count += 1;
                    self.ctx.stats.add_roi(
                        symbol.address as u32,
                        (symbol.address + symbol.size - 1) as u32,
                        &symbol.name,
                    );
                }
                println!("Loaded {} function symbols", count);
                self.ctx.stats.set_top_rois(options.top_roi);
                self.ctx.stats.set_roi_callers(options.roi_callers);
                self.ctx.stats.set_top_roi_detail(options.top_roi_detail);
            }
        }

        self.ctx.stats.set_legacy_stats(options.legacy_stats);
        self.ctx.stats.set_store_ops(options.store_op_output.is_some());

        // Check that callback is provided if chunk size is specified
        if options.chunk_size.is_some() {
            // Check callback consistency
            if callback.is_none() {
                panic!("Emu::run() called with chunk size but no callback");
            }

            // Record callback into context
            self.ctx.do_callback = true;
            self.ctx.callback_steps = options.chunk_size.unwrap();

            // Check steps value
            if self.ctx.callback_steps == 0 {
                panic!("Emu::run() called with chunk_size=0");
            }

            // Reserve enough entries for all the requested steps between callbacks
            self.ctx.trace.mem_reads.reserve(self.ctx.callback_steps as usize);

            // Init pc to the rom entry address
            self.ctx.trace.start_state.pc = ROM_ENTRY;
        }

        // Call run_fast if only essential work is needed
        if options.is_fast() {
            return self.run_fast(options);
        }
        if options.generate_minimal_traces {
            let par_emu_options =
                ParEmuOptions { num_steps: 1024 * 1024, num_threads: 1, thread_id: 0 };
            let minimal_trace = self.run_gen_trace(options, &par_emu_options);

            for (c, chunk) in minimal_trace.iter().enumerate() {
                println!("Chunk {c}:");
                println!("\tStart state:");
                println!("\t\tpc=0x{:x}", chunk.start_state.pc);
                println!("\t\tsp=0x{:x}", chunk.start_state.sp);
                println!("\t\tc=0x{:x}", chunk.start_state.c);
                println!("\t\tstep={}", chunk.start_state.step);
                for i in 1..chunk.start_state.regs.len() {
                    println!("\t\tregister[{}]=0x{:x}", i, chunk.start_state.regs[i]);
                }
                println!("\tLast state:");
                println!("\t\tc=0x{:x}", chunk.last_c);
                println!("\tEnd:");
                println!("\t\tend={}", if chunk.end { 1 } else { 0 });
                println!("\tSteps:");
                println!("\t\tsteps={}", chunk.steps);
                println!("\t\tmem_reads_size={}", chunk.mem_reads.len());
                for i in 0..chunk.mem_reads.len() {
                    println!("\t\tchunk[{}].mem_reads[{}]={:08x}", c, i, chunk.mem_reads[i]);
                }
            }
            return;
        }
        //println!("Emu::run() full-equipe");

        // Store the stats option into the emulator context
        self.ctx.do_stats = options.stats || options.legacy_stats;

        // While not done
        while !self.ctx.inst_ctx.end {
            if options.verbose {
                println!(
                    "Emu::run() step={} ctx.pc={}",
                    self.ctx.inst_ctx.step, self.ctx.inst_ctx.pc
                );
            }
            // Check trace PC
            if options.tracerv && (self.ctx.inst_ctx.pc & 0b11 == 0) {
                self.ctx.trace_pc = self.ctx.inst_ctx.pc;
            }

            // Log emulation step, if requested
            if options.print_step.is_some()
                && (options.print_step.unwrap() != 0)
                && ((self.ctx.inst_ctx.step % options.print_step.unwrap()) == 0)
            {
                println!("step={}", self.ctx.inst_ctx.step);
            }

            // Stop the execution if we exceeded the specified running conditions
            if self.ctx.inst_ctx.step >= options.max_steps {
                break;
            }

            // Execute the current step
            self.step(options, &callback);

            // Only trace after finishing a riscV instruction
            if options.tracerv && (self.ctx.inst_ctx.pc & 0b11) == 0 {
                // Store logs in a vector of strings
                let mut changes: Vec<String> = Vec::new();

                // Get the current state of the registers
                let new_regs_array = self.get_regs_array();

                // For all current registers
                for (i, register) in new_regs_array.iter().enumerate() {
                    // If they changed since previous stem, add them to the logs
                    if *register != self.ctx.tracerv_current_regs[i] {
                        changes.push(format!(
                            "{}={:x}",
                            RiscVRegisters::name_from_usize(i).unwrap(),
                            *register
                        ));
                        self.ctx.tracerv_current_regs[i] = *register;
                    }
                }

                // Add a log trace including all modified registers
                self.ctx.tracerv.push(format!(
                    "{}: {} -> {}",
                    self.ctx.tracerv_step,
                    self.ctx.trace_pc,
                    changes.join(", ")
                ));

                // Increase tracer step counter
                self.ctx.tracerv_step += 1;
            }

            // println!("Emu::run() done ctx.pc={}", self.ctx.pc); // 2147483828
        }

        // Detect and report error
        if self.ctx.inst_ctx.error {
            eprintln!(
                "Emu::run() finished with error at step={} pc=0x{:x}",
                self.ctx.inst_ctx.step, self.ctx.inst_ctx.pc
            );
        }

        // Print stats report
        if self.ctx.do_stats {
            self.ctx.stats.update_costs();
            let report = self.ctx.stats.report();
            println!("{report}");
            if let Some(store_op_output_file) = &options.store_op_output {
                self.ctx.stats.flush_op_data_to_file(store_op_output_file).unwrap();
            }
        }
    }

    /// Run the whole program
    pub fn par_run(
        &mut self,
        inputs: Vec<u8>,
        options: &EmuOptions,
        par_options: &ParEmuOptions,
    ) -> Vec<EmuTrace> {
        // Context, where the state of the execution is stored and modified at every execution step
        self.ctx = self.create_emu_context(inputs);

        // Init pc to the rom entry address
        self.ctx.trace.start_state.pc = ROM_ENTRY;

        // Store the stats option into the emulator context
        self.ctx.do_stats = options.stats || options.legacy_stats;

        // Set emulation mode
        self.ctx.inst_ctx.emulation_mode = EmulationMode::GenerateMemReads;

        let mut emu_traces = Vec::new();

        while !self.ctx.inst_ctx.end {
            let block_idx = self.ctx.inst_ctx.step / par_options.num_steps as u64;
            let is_my_block =
                block_idx % par_options.num_threads as u64 == par_options.thread_id as u64;

            if !is_my_block {
                self.par_step();
            } else {
                // Check if is the first step of a new block
                if self.ctx.inst_ctx.step % par_options.num_steps as u64 == 0 {
                    emu_traces.push(EmuTrace {
                        start_state: EmuTraceStart {
                            pc: self.ctx.inst_ctx.pc,
                            sp: self.ctx.inst_ctx.sp,
                            c: self.ctx.inst_ctx.c,
                            step: self.ctx.inst_ctx.step,
                            regs: self.ctx.inst_ctx.regs,
                        },
                        last_c: 0,
                        steps: 0,
                        mem_reads: Vec::with_capacity(par_options.num_steps),
                        end: false,
                    });
                }

                self.par_step_my_block(emu_traces.last_mut().unwrap());

                if self.ctx.inst_ctx.step >= options.max_steps {
                    panic!("Emu::par_run() reached max_steps");
                }
            }
        }

        // Detect and report error
        if self.ctx.inst_ctx.error {
            eprintln!(
                "Emu::par_run() finished with error at step={} pc=0x{:x}",
                self.ctx.inst_ctx.step, self.ctx.inst_ctx.pc
            );
        }

        emu_traces
    }

    /// Run the whole program
    pub fn run_gen_trace(
        &mut self,
        options: &EmuOptions,
        par_options: &ParEmuOptions,
    ) -> Vec<EmuTrace> {
        // Init pc to the rom entry address
        self.ctx.trace.start_state.pc = ROM_ENTRY;

        // Store the stats option into the emulator context
        self.ctx.do_stats = options.stats || options.legacy_stats;

        // Set emulation mode
        self.ctx.inst_ctx.emulation_mode = EmulationMode::GenerateMemReads;

        let mut emu_traces = Vec::new();

        while !self.ctx.inst_ctx.end {
            // Check if is the first step of a new block
            if self.ctx.inst_ctx.step % par_options.num_steps as u64 == 0 {
                emu_traces.push(EmuTrace {
                    start_state: EmuTraceStart {
                        pc: self.ctx.inst_ctx.pc,
                        sp: self.ctx.inst_ctx.sp,
                        c: self.ctx.inst_ctx.c,
                        step: self.ctx.inst_ctx.step,
                        regs: self.ctx.inst_ctx.regs,
                    },
                    last_c: 0,
                    steps: 0,
                    mem_reads: Vec::with_capacity(par_options.num_steps),
                    end: false,
                });
            }

            self.par_step_my_block(emu_traces.last_mut().unwrap());

            if self.ctx.inst_ctx.step >= options.max_steps {
                panic!("Emu::run_gen_trace() reached max_steps");
            }
        }

        emu_traces
    }

    /// Performs one single step of the emulation
    #[inline(always)]
    #[allow(unused_variables)]
    pub fn step(&mut self, options: &EmuOptions, callback: &Option<impl Fn(EmuTrace)>) {
        let pc = self.ctx.inst_ctx.pc;
        let instruction = self.rom.get_instruction(self.ctx.inst_ctx.pc);

        // println!(
        //     "Emu::step() executing step={} pc={:x} inst={}",
        //     self.ctx.inst_ctx.step,
        //     self.ctx.inst_ctx.pc,
        //     instruction.to_text()
        // );

        //println!("PCLOG={}", instruction.to_text());

        // Build the 'a' register value  based on the source specified by the current instruction
        self.source_a(instruction);

        // Build the 'b' register value  based on the source specified by the current instruction
        self.source_b(instruction);

        // Call the operation
        (instruction.func)(&mut self.ctx.inst_ctx);

        // Retrieve statistics data
        if self.ctx.do_stats {
            if instruction.input_size > 0 {
                if let Ok(inst) = ZiskOp::try_from_code(instruction.op) {
                    inst.call_stats(&self.ctx.inst_ctx, &mut self.ctx.stats);
                }
            }
            self.ctx.stats.on_op(
                instruction,
                self.ctx.inst_ctx.a,
                self.ctx.inst_ctx.b,
                pc,
                &[
                    self.ctx.inst_ctx.regs[10], // a0
                    self.ctx.inst_ctx.regs[11], // a1
                    self.ctx.inst_ctx.regs[12], // a2
                ],
            );
        }

        // Store the 'c' register value based on the storage specified by the current instruction
        self.store_c(instruction);

        // Set SP, if specified by the current instruction
        // #[cfg(feature = "sp")]
        // self.set_sp(instruction);

        // Set PC, based on current PC, current flag and current instruction
        self.set_pc(instruction);

        // If this is the last instruction, stop executing
        if instruction.end {
            self.ctx.inst_ctx.end = true;
            if self.ctx.do_stats {
                self.ctx.stats.on_steps(self.ctx.inst_ctx.step);
            }
        }

        // Log the step, if requested
        #[cfg(debug_assertions)]
        if options.log_step {
            println!(
                "step={} pc={:x} next={:x} op={}={} a={:x} b={:x} c={:x} flag={} inst={}",
                self.ctx.inst_ctx.step,
                pc,
                self.ctx.inst_ctx.pc,
                instruction.op,
                instruction.op_str,
                self.ctx.inst_ctx.a,
                self.ctx.inst_ctx.b,
                self.ctx.inst_ctx.c,
                self.ctx.inst_ctx.flag,
                instruction.to_text()
            );
            self.print_regs();
            // self.print_float_regs();
            // self.print_float_saved_regs();
            println!();
        }

        // Store an emulator trace, if requested
        if self.ctx.do_callback {
            // Increment step counter
            self.ctx.inst_ctx.step += 1;

            if self.ctx.inst_ctx.end
                || ((self.ctx.inst_ctx.step - self.ctx.last_callback_step)
                    == self.ctx.callback_steps)
            {
                // In run() we have checked the callback consistency with ctx.do_callback
                let callback = callback.as_ref().unwrap();

                // Set the end-of-trace data
                self.ctx.trace.end = self.ctx.inst_ctx.end;

                // Swap the emulator trace to avoid memory copies
                let mut trace = EmuTrace::default();
                trace.mem_reads.reserve(self.ctx.callback_steps as usize);
                mem::swap(&mut self.ctx.trace, &mut trace);
                (callback)(trace);

                // Set the start-of-trace data
                self.ctx.trace.start_state.pc = self.ctx.inst_ctx.pc;
                self.ctx.trace.start_state.sp = self.ctx.inst_ctx.sp;
                self.ctx.trace.start_state.c = self.ctx.inst_ctx.c;
                self.ctx.trace.start_state.step = self.ctx.inst_ctx.step;
                self.ctx.trace.start_state.regs = self.ctx.inst_ctx.regs;

                // Increment the last callback step counter
                self.ctx.last_callback_step += self.ctx.callback_steps;
            }
        } else {
            // Increment step counter
            self.ctx.inst_ctx.step += 1;
        }
    }

    /// Performs one single step of the emulation
    #[inline(always)]
    pub fn par_step_my_block(&mut self, emu_full_trace_vec: &mut EmuTrace) {
        let instruction = self.rom.get_instruction(self.ctx.inst_ctx.pc);
        // Build the 'a' register value  based on the source specified by the current instruction
        self.source_a_mem_reads_generate(instruction, &mut emu_full_trace_vec.mem_reads);

        // Build the 'b' register value  based on the source specified by the current instruction
        self.source_b_mem_reads_generate(instruction, &mut emu_full_trace_vec.mem_reads);

        // If this is a precompiled, get the required input data to copy it to mem_reads
        if instruction.input_size > 0 {
            self.ctx.inst_ctx.precompiled.input_data.clear();
            self.ctx.inst_ctx.precompiled.output_data.clear();
        }

        // Call the operation
        (instruction.func)(&mut self.ctx.inst_ctx);

        // If this is a precompiled, copy input data generated by precompile call to mem_reads.
        if instruction.input_size > 0 {
            emu_full_trace_vec.mem_reads.append(&mut self.ctx.inst_ctx.precompiled.input_data);
        }

        // Store the 'c' register value based on the storage specified by the current instruction
        self.store_c_mem_reads_generate(instruction, &mut emu_full_trace_vec.mem_reads);

        // Set SP, if specified by the current instruction
        // #[cfg(feature = "sp")]
        // self.set_sp(instruction);

        // Set PC, based on current PC, current flag and current instruction
        self.set_pc(instruction);

        // If this is the last instruction, stop executing
        self.ctx.inst_ctx.end = instruction.end;

        emu_full_trace_vec.last_c = self.ctx.inst_ctx.c;
        emu_full_trace_vec.end = self.ctx.inst_ctx.end;

        // Increment step counter
        self.ctx.inst_ctx.step += 1;
        emu_full_trace_vec.steps += 1;
    }

    /// Performs one single step of the emulation
    #[inline(always)]
    pub fn par_step(&mut self) {
        let instruction = self.rom.get_instruction(self.ctx.inst_ctx.pc);

        // Build the 'a' register value  based on the source specified by the current instruction
        self.source_a(instruction);

        // Build the 'b' register value  based on the source specified by the current instruction
        self.source_b(instruction);

        // Call the operation
        (instruction.func)(&mut self.ctx.inst_ctx);

        // Store the 'c' register value based on the storage specified by the current instruction
        self.store_c(instruction);

        // Set SP, if specified by the current instruction
        // #[cfg(feature = "sp")]
        // self.set_sp(instruction);

        // Set PC, based on current PC, current flag and current instruction
        self.set_pc(instruction);

        // If this is the last instruction, stop executing
        self.ctx.inst_ctx.end = instruction.end;

        // Increment step counter
        self.ctx.inst_ctx.step += 1;
    }

    /// Performs one single step of the emulation
    #[inline(always)]
    pub fn step_emu_trace<T, DB: DataBusTrait<u64, T>>(
        &mut self,
        mem_reads: &[u64],
        mem_reads_index: &mut usize,
        data_bus: &mut DB,
    ) -> bool {
        let instruction = self.rom.get_instruction(self.ctx.inst_ctx.pc);

        self.source_a_mem_reads_consume_databus(instruction, mem_reads, mem_reads_index, data_bus);
        self.source_b_mem_reads_consume_databus(instruction, mem_reads, mem_reads_index, data_bus);
        // If this is a precompiled, get the required input data from mem_reads
        if instruction.input_size > 0 {
            self.ctx.inst_ctx.precompiled.input_data.clear();
            self.ctx.inst_ctx.precompiled.output_data.clear();

            // round_up => (size + 7) >> 3
            let number_of_mem_reads = (instruction.input_size + 7) >> 3;
            for _ in 0..number_of_mem_reads {
                let mem_read = mem_reads[*mem_reads_index];
                *mem_reads_index += 1;
                self.ctx.inst_ctx.precompiled.input_data.push(mem_read);
            }
        }

        (instruction.func)(&mut self.ctx.inst_ctx);

        self.store_c_mem_reads_consume_databus(instruction, mem_reads, mem_reads_index, data_bus);

        // Get operation bus data
        if instruction.op_type > ZiskOperationType::Internal
            && instruction.op_type < ZiskOperationType::FcallParam
        {
            let operation_payload: &[u64] = OperationBusData::write_instruction_payload(
                instruction,
                &self.ctx.inst_ctx,
                &mut self.static_array,
            );
            data_bus.write_to_bus(OPERATION_BUS_ID, operation_payload);
        }

        // #[cfg(feature = "sp")]
        // self.set_sp(instruction);
        self.set_pc(instruction);
        self.ctx.inst_ctx.end = instruction.end;

        self.ctx.inst_ctx.step += 1;
        //trace_step.steps += 1;

        false
    }

    /// Performs one single step of the emulation
    #[inline(always)]
    pub fn step_emu_trace_no_mem_ops<T, DB: DataBusTrait<u64, T>>(
        &mut self,
        mem_reads: &[u64],
        mem_reads_index: &mut usize,
        data_bus: &mut DB,
    ) -> bool {
        let instruction = self.rom.get_instruction(self.ctx.inst_ctx.pc);

        self.source_a_mem_reads_consume_no_mem_ops(instruction, mem_reads, mem_reads_index);
        self.source_b_mem_reads_consume_no_mem_ops(instruction, mem_reads, mem_reads_index);
        // If this is a precompiled, get the required input data from mem_reads
        if instruction.input_size > 0 {
            self.ctx.inst_ctx.precompiled.input_data.clear();
            self.ctx.inst_ctx.precompiled.output_data.clear();

            // round_up => (size + 7) >> 3
            let number_of_mem_reads = (instruction.input_size + 7) >> 3;
            for _ in 0..number_of_mem_reads {
                let mem_read = mem_reads[*mem_reads_index];
                *mem_reads_index += 1;
                self.ctx.inst_ctx.precompiled.input_data.push(mem_read);
            }
        }

        (instruction.func)(&mut self.ctx.inst_ctx);

        self.store_c_mem_reads_consume_no_mem_ops(instruction, mem_reads, mem_reads_index);

        // Get operation bus data
        if instruction.op_type > ZiskOperationType::Internal
            && instruction.op_type < ZiskOperationType::FcallParam
        {
            let operation_payload: &[u64] = OperationBusData::write_instruction_payload(
                instruction,
                &self.ctx.inst_ctx,
                &mut self.static_array,
            );
            data_bus.write_to_bus(OPERATION_BUS_ID, operation_payload);
        }

        // #[cfg(feature = "sp")]
        // self.set_sp(instruction);
        self.set_pc(instruction);
        self.ctx.inst_ctx.end = instruction.end;

        self.ctx.inst_ctx.step += 1;
        //trace_step.steps += 1;

        false
    }

    /// Run a slice of the program to generate full traces
    pub fn process_emu_trace<T, DB: DataBusTrait<u64, T>>(
        &mut self,
        emu_trace: &EmuTrace,
        data_bus: &mut DB,
        with_mem_ops: bool,
    ) {
        // Set initial state
        self.ctx.inst_ctx.pc = emu_trace.start_state.pc;
        self.ctx.inst_ctx.sp = emu_trace.start_state.sp;
        self.ctx.inst_ctx.step = emu_trace.start_state.step;
        self.ctx.inst_ctx.c = emu_trace.start_state.c;
        self.ctx.inst_ctx.regs = emu_trace.start_state.regs;
        self.ctx.inst_ctx.emulation_mode = EmulationMode::ConsumeMemReads;

        let mut mem_reads_index: usize = 0;

        if with_mem_ops {
            for _ in 0..emu_trace.steps {
                self.step_emu_trace(&emu_trace.mem_reads, &mut mem_reads_index, data_bus);
            }
        } else {
            for _ in 0..emu_trace.steps {
                self.step_emu_trace_no_mem_ops(
                    &emu_trace.mem_reads,
                    &mut mem_reads_index,
                    data_bus,
                );
            }
        }
    }

    /// Run a slice of the program to generate full traces
    pub fn process_emu_traces<T, DB: DataBusTrait<u64, T>>(
        &mut self,
        vec_traces: &[EmuTrace],
        chunk_id: usize,
        data_bus: &mut DB,
    ) {
        // Set initial state
        let emu_trace_start = &vec_traces[chunk_id].start_state;
        self.ctx.inst_ctx.pc = emu_trace_start.pc;
        self.ctx.inst_ctx.sp = emu_trace_start.sp;
        self.ctx.inst_ctx.step = emu_trace_start.step;
        self.ctx.inst_ctx.c = emu_trace_start.c;
        self.ctx.inst_ctx.regs = emu_trace_start.regs;
        self.ctx.inst_ctx.emulation_mode = EmulationMode::ConsumeMemReads;

        let mut current_step_idx = 0;
        let mut mem_reads_index: usize = 0;
        loop {
            if !self.step_emu_traces(
                &vec_traces[chunk_id].mem_reads,
                &mut mem_reads_index,
                data_bus,
            ) {
                break;
            }

            if self.ctx.inst_ctx.end {
                break;
            }

            current_step_idx += 1;
            if current_step_idx == vec_traces[chunk_id].steps {
                break;
            }
        }
    }

    /// Performs one single step of the emulation
    #[inline(always)]
    pub fn step_emu_traces<T, DB: DataBusTrait<u64, T>>(
        &mut self,
        mem_reads: &[u64],
        mem_reads_index: &mut usize,
        data_bus: &mut DB,
    ) -> bool {
        let mut _continue = true;
        let instruction = self.rom.get_instruction(self.ctx.inst_ctx.pc);
        self.source_a_mem_reads_consume_databus(instruction, mem_reads, mem_reads_index, data_bus);
        self.source_b_mem_reads_consume_databus(instruction, mem_reads, mem_reads_index, data_bus);
        // If this is a precompiled, get the required input data from mem_reads
        if instruction.input_size > 0 {
            self.ctx.inst_ctx.precompiled.input_data.clear();
            self.ctx.inst_ctx.precompiled.output_data.clear();
            let number_of_mem_reads = (instruction.input_size + 7) >> 3;
            for _ in 0..number_of_mem_reads {
                let mem_read = mem_reads[*mem_reads_index];
                *mem_reads_index += 1;
                self.ctx.inst_ctx.precompiled.input_data.push(mem_read);
            }
        }
        (instruction.func)(&mut self.ctx.inst_ctx);
        self.store_c_mem_reads_consume_databus(instruction, mem_reads, mem_reads_index, data_bus);

        // Get operation bus data
        if instruction.op_type > ZiskOperationType::Internal
            && instruction.op_type < ZiskOperationType::FcallParam
        {
            let operation_payload: &[u64] = OperationBusData::write_instruction_payload(
                instruction,
                &self.ctx.inst_ctx,
                &mut self.static_array,
            );
            _continue = data_bus.write_to_bus(OPERATION_BUS_ID, operation_payload);
        }

        // Get rom bus data
        let rom_payload = RomBusData::from_instruction(instruction, &self.ctx.inst_ctx);

        // Write rom bus data to rom bus
        data_bus.write_to_bus(ROM_BUS_ID, &rom_payload);

        // #[cfg(feature = "sp")]
        // self.set_sp(instruction);
        self.set_pc(instruction);
        self.ctx.inst_ctx.end = instruction.end;

        self.ctx.inst_ctx.step += 1;

        _continue
    }

    /// Performs one single step of the emulation
    #[inline(always)]
    pub fn step_slice_full_trace<F: PrimeField64>(
        &mut self,
        mem_reads: &[u64],
        mem_reads_index: &mut usize,
        reg_trace: &mut EmuRegTrace,
        step_range_check: Option<&mut [u32]>,
    ) -> EmuFullTraceStep<F> {
        if self.ctx.inst_ctx.pc == 0 {
            println!("PC=0 CRASH (step:{})", self.ctx.inst_ctx.step);
        }
        let instruction = self.rom.get_instruction(self.ctx.inst_ctx.pc);

        reg_trace.clear_reg_step_ranges();

        self.source_a_mem_reads_consume(instruction, mem_reads, mem_reads_index, reg_trace);
        self.source_b_mem_reads_consume(instruction, mem_reads, mem_reads_index, reg_trace);
        // If this is a precompiled, get the required input data from mem_reads
        self.ctx.inst_ctx.emulation_mode = EmulationMode::ConsumeMemReads;
        if instruction.input_size > 0 {
            self.ctx.inst_ctx.precompiled.input_data.clear();
            self.ctx.inst_ctx.precompiled.output_data.clear();
            let number_of_mem_reads = (instruction.input_size + 7) >> 3;
            for _ in 0..number_of_mem_reads {
                let mem_read = mem_reads[*mem_reads_index];
                *mem_reads_index += 1;
                self.ctx.inst_ctx.precompiled.input_data.push(mem_read);
            }
        }

        (instruction.func)(&mut self.ctx.inst_ctx);
        self.store_c_mem_reads_consume(instruction, mem_reads, mem_reads_index, reg_trace);

        if let Some(step_range_check) = step_range_check {
            reg_trace.update_step_range_check(step_range_check);
        }

        // #[cfg(feature = "sp")]
        // self.set_sp(instruction);
        self.set_pc(instruction);
        self.ctx.inst_ctx.end = instruction.end;

        // Build and store the full trace
        let full_trace_step =
            Self::build_full_trace_step(instruction, &self.ctx.inst_ctx, reg_trace);

        self.ctx.inst_ctx.step += 1;

        full_trace_step
    }

    pub fn intermediate_value<F: PrimeField64>(value: u64) -> [F; 2] {
        [F::from_u64(value & 0xFFFFFFFF), F::from_u64((value >> 32) & 0xFFFFFFFF)]
    }

    #[inline(always)]
    pub fn build_full_trace_step<F: PrimeField64>(
        inst: &ZiskInst,
        inst_ctx: &InstContext,
        reg_trace: &EmuRegTrace,
    ) -> EmuFullTraceStep<F> {
        // Calculate intermediate values
        let a: [u32; 2] =
            [(inst_ctx.a & 0xFFFFFFFF) as u32, ((inst_ctx.a >> 32) & 0xFFFFFFFF) as u32];
        let b: [u32; 2] =
            [(inst_ctx.b & 0xFFFFFFFF) as u32, ((inst_ctx.b >> 32) & 0xFFFFFFFF) as u32];
        let c: [u32; 2] =
            [(inst_ctx.c & 0xFFFFFFFF) as u32, ((inst_ctx.c >> 32) & 0xFFFFFFFF) as u32];
        let store_prev_value = [
            (reg_trace.store_reg_prev_value & 0xFFFFFFFF) as u32,
            ((reg_trace.store_reg_prev_value >> 32) & 0xFFFFFFFF) as u32,
        ];

        let addr1 = (inst.b_offset_imm0 as i64
            + if inst.b_src == SRC_IND { inst_ctx.a as i64 } else { 0 }) as u32;

        let jmp_offset1 = if inst.jmp_offset1 >= 0 {
            inst.jmp_offset1 as u64
        } else {
            F::neg(F::from_u64((-inst.jmp_offset1) as u64)).as_canonical_u64()
        };

        let jmp_offset2 = if inst.jmp_offset2 >= 0 {
            inst.jmp_offset2 as u64
        } else {
            F::neg(F::from_u64((-inst.jmp_offset2) as u64)).as_canonical_u64()
        };

        let store_offset = if inst.store_offset >= 0 {
            inst.store_offset as u64
        } else {
            F::neg(F::from_u64((-inst.store_offset) as u64)).as_canonical_u64()
        };

        let a_offset_imm0 = if inst.a_offset_imm0 as i64 >= 0 {
            inst.a_offset_imm0
        } else {
            F::neg(F::from_u64((-(inst.a_offset_imm0 as i64)) as u64)).as_canonical_u64()
        };
        let b_offset_imm0 = if inst.b_offset_imm0 as i64 >= 0 {
            inst.b_offset_imm0
        } else {
            F::neg(F::from_u64((-(inst.b_offset_imm0 as i64)) as u64)).as_canonical_u64()
        };

        let mut trace = EmuFullTraceStep::default();
        trace.set_a(0, a[0]);
        trace.set_a(1, a[1]);
        trace.set_b(0, b[0]);
        trace.set_b(1, b[1]);
        trace.set_c(0, c[0]);
        trace.set_c(1, c[1]);
        trace.set_flag(inst_ctx.flag);
        trace.set_pc(inst.paddr as u32);
        trace.set_a_src_imm(inst.a_src == SRC_IMM);
        trace.set_a_src_mem(inst.a_src == SRC_MEM);
        trace.set_a_src_reg(inst.a_src == SRC_REG);
        trace.set_a_offset_imm0(a_offset_imm0);
        // #[cfg(not(feature = "sp"))]
        trace.set_a_imm1(inst.a_use_sp_imm1 as u32);
        // #[cfg(feature = "sp")]
        // trace.set_sp(inst_ctx.sp);
        // #[cfg(feature = "sp")]
        // trace.set_a_src_sp(inst.a_src == SRC_SP),
        // #[cfg(feature = "sp")]
        // trace.set_a_use_sp_imm1(inst.a_use_sp_imm1),
        trace.set_a_src_step(inst.a_src == SRC_STEP);
        trace.set_b_src_imm(inst.b_src == SRC_IMM);
        trace.set_b_src_mem(inst.b_src == SRC_MEM);
        trace.set_b_src_reg(inst.b_src == SRC_REG);
        trace.set_b_offset_imm0(b_offset_imm0);
        // #[cfg(not(feature = "sp"))]
        trace.set_b_imm1(inst.b_use_sp_imm1 as u32);
        // #[cfg(feature = "sp")]
        // trace.set_b_use_sp_imm1(inst.b_use_sp_imm1),
        trace.set_b_src_ind(inst.b_src == SRC_IND);
        trace.set_ind_width(inst.ind_width as u8);
        trace.set_is_external_op(inst.is_external_op);
        // IMPORTANT: the opcodes fcall, fcall_get, and fcall_param are really a variant
        // of the copyb, use to get free-input information
        trace.set_op(
            if inst.op == ZiskOp::Fcall.code()
                || inst.op == ZiskOp::FcallGet.code()
                || inst.op == ZiskOp::FcallParam.code()
            {
                ZiskOp::CopyB.code()
            } else {
                inst.op
            },
        );
        trace.set_store_ra(inst.store_ra);
        trace.set_store_mem(inst.store == STORE_MEM);
        trace.set_store_reg(inst.store == STORE_REG);
        trace.set_store_ind(inst.store == STORE_IND);
        trace.set_store_offset(store_offset);
        trace.set_set_pc(inst.set_pc);
        // #[cfg(feature = "sp")]
        // trace.set_store_use_sp(inst.store_use_sp);
        // #[cfg(feature = "sp")]
        // trace.set_sp(inst_ctx.sp);
        // #[cfg(feature = "sp")]
        // trace.set_inc_sp(inst.inc_sp);
        trace.set_jmp_offset1(jmp_offset1);
        trace.set_jmp_offset2(jmp_offset2);
        trace.set_m32(inst.m32);
        trace.set_addr1(addr1);
        trace.set_a_reg_prev_mem_step(reg_trace.reg_prev_steps[0]);
        trace.set_b_reg_prev_mem_step(reg_trace.reg_prev_steps[1]);
        trace.set_store_reg_prev_mem_step(reg_trace.reg_prev_steps[2]);
        trace.set_store_reg_prev_value(0, store_prev_value[0]);
        trace.set_store_reg_prev_value(1, store_prev_value[1]);
        trace
    }

    /// Returns if the emulation ended
    pub fn terminated(&self) -> bool {
        self.ctx.inst_ctx.end
    }

    /// Returns the number of executed steps
    pub fn number_of_steps(&self) -> u64 {
        self.ctx.inst_ctx.step
    }

    /// Get the output as a vector of u64
    pub fn get_output(&self) -> Vec<u64> {
        let n = self.ctx.inst_ctx.mem.read(OUTPUT_ADDR, 8);
        let mut addr = OUTPUT_ADDR + 8;

        let mut output: Vec<u64> = Vec::with_capacity(n as usize);
        for _i in 0..n {
            output.push(self.ctx.inst_ctx.mem.read(addr, 8));
            addr += 8;
        }
        output
    }

    /// Get the output as a vector of u32
    pub fn get_output_32(&self) -> Vec<u32> {
        let n = self.ctx.inst_ctx.mem.read(OUTPUT_ADDR, 4);
        let mut addr = OUTPUT_ADDR + 4;
        let mut output: Vec<u32> = Vec::with_capacity(n as usize);
        for _i in 0..n {
            output.push(self.ctx.inst_ctx.mem.read(addr, 4) as u32);
            addr += 4;
        }

        // let mut addr = OUTPUT_ADDR;
        // let mut output: Vec<u32> = Vec::with_capacity(32);
        // for _i in 0..32 {
        //     output.push(self.ctx.inst_ctx.mem.read(addr, 4) as u32);
        //     addr += 4;
        // }

        output
    }

    /// Get the output as a vector of u8
    pub fn get_output_8(&self) -> Vec<u8> {
        let n = self.ctx.inst_ctx.mem.read(OUTPUT_ADDR, 4);
        let mut addr = OUTPUT_ADDR + 4;

        let mut output: Vec<u8> = Vec::with_capacity(n as usize);
        for _i in 0..n {
            output.push(self.ctx.inst_ctx.mem.read(addr, 1) as u8);
            output.push(self.ctx.inst_ctx.mem.read(addr + 1, 1) as u8);
            output.push(self.ctx.inst_ctx.mem.read(addr + 2, 1) as u8);
            output.push(self.ctx.inst_ctx.mem.read(addr + 3, 1) as u8);
            addr += 4;
        }
        output
    }

    /// Gets the log traces
    pub fn get_tracerv(&self) -> Vec<String> {
        self.ctx.tracerv.clone()
    }

    /// Gets the current values of the 32 registers
    pub fn get_regs_array(&self) -> [u64; 32] {
        let mut regs_array: [u64; 32] = [0; 32];
        for (i, reg) in regs_array.iter_mut().enumerate() {
            *reg = self.ctx.inst_ctx.regs[i];
        }
        regs_array
    }

    #[inline(always)]
    pub fn get_traced_reg(&mut self, index: usize, slot: u8, reg_trace: &mut EmuRegTrace) -> u64 {
        reg_trace.trace_reg_access(index, self.ctx.inst_ctx.step, slot);
        self.ctx.inst_ctx.regs[index]
    }

    #[inline(always)]
    pub fn set_traced_reg(&mut self, index: usize, value: u64, reg_trace: &mut EmuRegTrace) {
        reg_trace.trace_reg_access(index, self.ctx.inst_ctx.step, 2);
        reg_trace.store_reg_prev_value = self.ctx.inst_ctx.regs[index];
        self.ctx.inst_ctx.regs[index] = value;
    }

    #[inline(always)]
    pub fn get_reg(&self, index: usize) -> u64 {
        debug_assert!(index < 32);
        self.ctx.inst_ctx.regs[index]
    }

    #[inline(always)]
    pub fn set_reg(&mut self, index: usize, value: u64) {
        debug_assert!(index < 32);
        self.ctx.inst_ctx.regs[index] = value;
    }

    #[inline(always)]
    pub fn get_value_to_store(&self, instruction: &ZiskInst) -> u64 {
        if instruction.store_ra {
            (self.ctx.inst_ctx.pc as i64 + instruction.jmp_offset2) as u64
        } else {
            self.ctx.inst_ctx.c
        }
    }

    pub fn print_regs(&self) {
        let regs_array: [u64; 32] = self.get_regs_array();
        print!("Emu::print_regs(): ");
        for (i, r) in regs_array.iter().enumerate() {
            print!("x{i}={r}={r:x} ");
        }
        println!();
    }

    pub fn print_float_regs(&self) {
        print!("Emu::print_float_regs(): ");
        for i in 0..31 {
            let r = self.ctx.inst_ctx.mem.read(FREG_F0 + i * 8, 8);
            print!("f{i}={r}={r:x} ");
        }
        let r = self.ctx.inst_ctx.mem.read(FREG_INST, 8);
        print!("finst={r}={r:x} ");
        let r = self.ctx.inst_ctx.mem.read(FREG_RA, 8);
        print!("fra={r}={r:x} ");
        println!();
    }

    pub fn print_float_saved_regs(&self) {
        print!("Emu::print_float_saved_regs(): ");
        for i in 0..31 {
            let r = self.ctx.inst_ctx.mem.read(FREG_X0 + i * 8, 8);
            print!("fx{i}={r}={r:x} ");
        }
        println!();
    }
}
