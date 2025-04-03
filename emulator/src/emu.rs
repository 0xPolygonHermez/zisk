use std::{mem, sync::atomic::AtomicU32};

use crate::{EmuContext, EmuFullTraceStep, EmuOptions, EmuRegTrace, ParEmuOptions};
use data_bus::{
    BusDevice, ExtOperationData, OperationBusData, RomBusData, MEM_BUS_ID, OPERATION_BUS_ID,
    ROM_BUS_ID,
};
use p3_field::PrimeField;
use riscv::RiscVRegisters;
use sm_mem::MemHelpers;
// #[cfg(feature = "sp")]
// use zisk_core::SRC_SP;
use data_bus::DataBus;
use zisk_common::{EmuTrace, EmuTraceStart};
use zisk_core::zisk_ops::ZiskOp;
use zisk_core::{
    EmulationMode, InstContext, Mem, ZiskInst, ZiskRom, OUTPUT_ADDR, ROM_ENTRY, SRC_C, SRC_IMM,
    SRC_IND, SRC_MEM, SRC_REG, SRC_STEP, STORE_IND, STORE_MEM, STORE_NONE, STORE_REG,
};

/// ZisK emulator structure, containing the ZisK rom, the list of ZisK operations, and the
/// execution context
pub struct Emu<'a> {
    /// ZisK rom, containing the program to execute, which is constant for this program except for
    /// the input data
    pub rom: &'a ZiskRom,
    /// Context, where the state of the execution is stored and modified at every execution step
    pub ctx: EmuContext,
}

/// ZisK emulator structure implementation
/// There are different modes of execution for different purposes:
/// - run -> step -> source_a, source_b, store_c (full functionality, called by main state machine,
///   calls callback with trace)
/// - run -> run_fast -> step_fast -> source_a, source_b, store_c (maximum speed, for benchmarking)
/// - run_slice -> step_slice -> source_a_slice, source_b_slice, store_c_slice (generates full trace
///   and required input data for secondary state machines)
impl<'a> Emu<'a> {
    pub fn new(rom: &ZiskRom) -> Emu {
        Emu { rom, ctx: EmuContext::default() }
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
    pub fn source_a_mem_reads_consume_databus<BD: BusDevice<u64>>(
        &mut self,
        instruction: &ZiskInst,
        mem_reads: &[u64],
        mem_reads_index: &mut usize,
        data_bus: &mut DataBus<u64, BD>,
    ) {
        match instruction.a_src {
            SRC_C => self.ctx.inst_ctx.a = self.ctx.inst_ctx.c,
            SRC_REG => {
                self.ctx.inst_ctx.a = self.get_reg(instruction.a_offset_imm0 as usize);
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

    /// Calculate the 'b' register value based on the source specified by the current instruction
    #[inline(always)]
    pub fn source_b(&mut self, instruction: &ZiskInst) {
        match instruction.b_src {
            SRC_C => self.ctx.inst_ctx.b = self.ctx.inst_ctx.c,
            SRC_REG => {
                // Calculate memory address
                self.ctx.inst_ctx.b = self.get_reg(instruction.b_offset_imm0 as usize);
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
                        println!("ADDITIONAL DATA IS EMPTY 0x{:X}", address);
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
    pub fn source_b_mem_reads_consume_databus<BD: BusDevice<u64>>(
        &mut self,
        instruction: &ZiskInst,
        mem_reads: &[u64],
        mem_reads_index: &mut usize,
        data_bus: &mut DataBus<u64, BD>,
    ) {
        match instruction.b_src {
            SRC_C => self.ctx.inst_ctx.b = self.ctx.inst_ctx.c,
            SRC_REG => {
                self.ctx.inst_ctx.b = self.get_reg(instruction.b_offset_imm0 as usize);
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

    /// Store the 'c' register value based on the storage specified by the current instruction
    #[inline(always)]
    pub fn store_c(&mut self, instruction: &ZiskInst) {
        match instruction.store {
            STORE_NONE => {}
            STORE_REG => {
                if instruction.store_offset >= 32 {
                    println!("instruction ALERT 0 {:?}", instruction);
                }

                self.set_reg(
                    instruction.store_offset as usize,
                    self.get_value_to_store(instruction),
                );
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
                    self.ctx.stats.on_memory_write(addr, 8);
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
                debug_assert!(addr >= 0);
                let addr = addr as u64;

                // Get it from memory
                self.ctx.inst_ctx.mem.write(addr, val, instruction.ind_width);
                if self.ctx.do_stats {
                    self.ctx.stats.on_memory_write(addr, instruction.ind_width);
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
                    println!("instruction ALERT 1 {:?}", instruction);
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
    pub fn store_c_mem_reads_consume_databus<BD: BusDevice<u64>>(
        &mut self,
        instruction: &ZiskInst,
        mem_reads: &[u64],
        mem_reads_index: &mut usize,
        data_bus: &mut DataBus<u64, BD>,
    ) {
        match instruction.store {
            STORE_NONE => {}
            STORE_REG => {
                if instruction.store_offset >= 32 {
                    println!("instruction ALERT 2 {:?}", instruction);
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
    }

    /// Performs one single step of the emulation
    #[inline(always)]
    pub fn step_fast(&mut self) {
        let instruction = self.rom.get_instruction(self.ctx.inst_ctx.pc);
        let debug = instruction.op >= 0xf6;
        let initial_regs = if debug {
            print!(
                "\x1B[1;36m>==IN ==>\x1B[0m SF #{} 0x{:X} ({}) {}",
                self.ctx.inst_ctx.step,
                self.ctx.inst_ctx.pc,
                instruction.op_str,
                instruction.verbose
            );
            for (index, &value) in self.ctx.inst_ctx.regs.iter().enumerate() {
                print!(" {:}:0x{:X}", index, value);
            }
            println!();
            self.ctx.inst_ctx.regs
        } else {
            /* println!(
                "#{} 0x{:X} ({}) {}",
                self.ctx.inst_ctx.step,
                self.ctx.inst_ctx.pc,
                instruction.op_str,
                instruction.verbose
            );*/
            [0u64; 32]
        };
        self.source_a(instruction);
        self.source_b(instruction);
        (instruction.func)(&mut self.ctx.inst_ctx);
        self.store_c(instruction);

        // #[cfg(feature = "sp")]
        // self.set_sp(instruction);

        self.set_pc(instruction);
        self.ctx.inst_ctx.end = instruction.end;
        self.ctx.inst_ctx.step += 1;
        if debug {
            print!(
                ">==OUT==> #{} 0x{:X} ({}) {} {:?}",
                self.ctx.inst_ctx.step,
                self.ctx.inst_ctx.pc,
                instruction.op_str,
                instruction.verbose,
                self.ctx.inst_ctx.regs,
            );
            for (index, &value) in self.ctx.inst_ctx.regs.iter().enumerate() {
                if initial_regs[index] == value {
                    print!(" {:}:0x{:X}", index, value);
                } else {
                    print!(" {:}:\x1B[1;31m0x{:X}\x1B[0m", index, value);
                }
            }
            println!();
        }
    }

    /// Run the whole program
    pub fn run(
        &mut self,
        inputs: Vec<u8>,
        options: &EmuOptions,
        callback: Option<impl Fn(EmuTrace)>,
    ) {
        // Context, where the state of the execution is stored and modified at every execution step
        self.ctx = self.create_emu_context(inputs);

        // Check that callback is provided if trace_steps is specified
        if options.trace_steps.is_some() {
            // Check callback consistency
            if callback.is_none() {
                panic!("Emu::run() called with trace_steps but no callback");
            }

            // Record callback into context
            self.ctx.do_callback = true;
            self.ctx.callback_steps = options.trace_steps.unwrap();

            // Check steps value
            if self.ctx.callback_steps == 0 {
                panic!("Emu::run() called with trace_steps=0");
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
        //println!("Emu::run() full-equipe");

        // Store the stats option into the emulator context
        self.ctx.do_stats = options.stats;

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

        // Print stats report
        if options.stats {
            let report = self.ctx.stats.report();
            println!("{}", report);
        }
    }

    /// Run the whole program
    pub fn par_run<F: PrimeField>(
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
        self.ctx.do_stats = options.stats;

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
                self.par_step_my_block::<F>(emu_traces.last_mut().unwrap());

                if self.ctx.inst_ctx.step >= options.max_steps {
                    panic!("Emu::par_run() reached max_steps");
                }
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
        // println!("Emu::step() step={} pc={}", ctx.step, ctx.pc);

        //println!("PCLOG={}", instruction.to_text());

        // Build the 'a' register value  based on the source specified by the current instruction
        self.source_a(instruction);

        // Build the 'b' register value  based on the source specified by the current instruction
        self.source_b(instruction);

        // Call the operation
        (instruction.func)(&mut self.ctx.inst_ctx);

        // Retrieve statistics data
        if self.ctx.do_stats {
            self.ctx.stats.on_op(instruction, self.ctx.inst_ctx.a, self.ctx.inst_ctx.b);
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
            if options.stats {
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
    pub fn par_step_my_block<F: PrimeField>(&mut self, emu_full_trace_vec: &mut EmuTrace) {
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

        // If this is a precompiled, copy input data to mem_reads
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
        if instruction.op == 0xF8 {
            println!("instruction: {:?}", instruction);
            println!("inst_ctx: {:?}", self.ctx.inst_ctx);
        }

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
    pub fn step_emu_trace<F: PrimeField, BD: BusDevice<u64>>(
        &mut self,
        mem_reads: &[u64],
        mem_reads_index: &mut usize,
        data_bus: &mut DataBus<u64, BD>,
    ) -> bool {
        let instruction = self.rom.get_instruction(self.ctx.inst_ctx.pc);
        let debug = instruction.op >= 0xF6;
        let initial_regs = if debug {
            print!(
                "\x1B[1;36m>==IN ==>\x1B[0m SE #{} 0x{:X} ({}) {}",
                self.ctx.inst_ctx.step,
                self.ctx.inst_ctx.pc,
                instruction.op_str,
                instruction.verbose
            );
            for (index, &value) in self.ctx.inst_ctx.regs.iter().enumerate() {
                print!(" {:}:0x{:X}", index, value);
            }
            println!(
                " self.ctx.inst_ctx.emulation_mode={:?} instruction:{:?}",
                self.ctx.inst_ctx.emulation_mode, instruction
            );
            self.ctx.inst_ctx.regs
        } else {
            /* println!(
                "#{} 0x{:X} ({}) {}",
                self.ctx.inst_ctx.step,
                self.ctx.inst_ctx.pc,
                instruction.op_str,
                instruction.verbose
            );*/
            [0u64; 32]
        };

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

        if debug {
            print!(
                ">==OUT==> #{} 0x{:X} ({}) {} {:?}",
                self.ctx.inst_ctx.step,
                self.ctx.inst_ctx.pc,
                instruction.op_str,
                instruction.verbose,
                self.ctx.inst_ctx.regs,
            );
            for (index, &value) in self.ctx.inst_ctx.regs.iter().enumerate() {
                if initial_regs[index] == value {
                    print!(" {:}:0x{:X}", index, value);
                } else {
                    print!(" {:}:\x1B[1;31m0x{:X}\x1B[0m", index, value);
                }
            }
            println!();
        }
        // Get operation bus data
        let operation_payload = OperationBusData::from_instruction(instruction, &self.ctx.inst_ctx);

        // Write operation bus data to operation bus
        match operation_payload {
            ExtOperationData::OperationData(data) => {
                data_bus.write_to_bus(OPERATION_BUS_ID, &data);
            }
            ExtOperationData::OperationKeccakData(data) => {
                data_bus.write_to_bus(OPERATION_BUS_ID, &data);
            }
            ExtOperationData::OperationArith256Data(data) => {
                data_bus.write_to_bus(OPERATION_BUS_ID, &data);
            }
            ExtOperationData::OperationArith256ModData(data) => {
                data_bus.write_to_bus(OPERATION_BUS_ID, &data);
            }
            ExtOperationData::OperationSecp256k1AddData(data) => {
                data_bus.write_to_bus(OPERATION_BUS_ID, &data);
            }
            ExtOperationData::OperationSecp256k1DblData(data) => {
                data_bus.write_to_bus(OPERATION_BUS_ID, &data);
            }
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
    #[inline(always)]
    pub fn process_emu_trace<F: PrimeField, BD: BusDevice<u64>>(
        &mut self,
        emu_trace: &EmuTrace,
        data_bus: &mut DataBus<u64, BD>,
    ) {
        // Set initial state
        self.ctx.inst_ctx.pc = emu_trace.start_state.pc;
        self.ctx.inst_ctx.sp = emu_trace.start_state.sp;
        self.ctx.inst_ctx.step = emu_trace.start_state.step;
        self.ctx.inst_ctx.c = emu_trace.start_state.c;
        self.ctx.inst_ctx.regs = emu_trace.start_state.regs;
        self.ctx.inst_ctx.emulation_mode = EmulationMode::ConsumeMemReads;

        let mut mem_reads_index: usize = 0;
        for _ in 0..emu_trace.steps {
            self.step_emu_trace::<F, BD>(&emu_trace.mem_reads, &mut mem_reads_index, data_bus);
        }
    }

    /// Run a slice of the program to generate full traces
    #[inline(always)]
    pub fn process_emu_traces<BD: BusDevice<u64>>(
        &mut self,
        vec_traces: &[EmuTrace],
        chunk_id: usize,
        data_bus: &mut DataBus<u64, BD>,
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
            self.step_emu_traces(&vec_traces[chunk_id].mem_reads, &mut mem_reads_index, data_bus);

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
    pub fn step_emu_traces<BD: BusDevice<u64>>(
        &mut self,
        mem_reads: &[u64],
        mem_reads_index: &mut usize,
        data_bus: &mut DataBus<u64, BD>,
    ) {
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
        let operation_payload = OperationBusData::from_instruction(instruction, &self.ctx.inst_ctx);

        // Write operation bus data to operation bus
        match operation_payload {
            ExtOperationData::OperationData(data) => {
                data_bus.write_to_bus(OPERATION_BUS_ID, &data);
            }
            ExtOperationData::OperationKeccakData(data) => {
                data_bus.write_to_bus(OPERATION_BUS_ID, &data);
            }
            ExtOperationData::OperationArith256Data(data) => {
                data_bus.write_to_bus(OPERATION_BUS_ID, &data);
            }
            ExtOperationData::OperationArith256ModData(data) => {
                data_bus.write_to_bus(OPERATION_BUS_ID, &data);
            }
            ExtOperationData::OperationSecp256k1AddData(data) => {
                data_bus.write_to_bus(OPERATION_BUS_ID, &data);
            }
            ExtOperationData::OperationSecp256k1DblData(data) => {
                data_bus.write_to_bus(OPERATION_BUS_ID, &data);
            }
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
    }

    /// Performs one single step of the emulation
    #[inline(always)]
    pub fn step_slice_full_trace<F: PrimeField>(
        &mut self,
        mem_reads: &[u64],
        mem_reads_index: &mut usize,
        reg_trace: &mut EmuRegTrace,
        step_range_check: Option<&[AtomicU32]>,
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

    pub fn intermediate_value<F: PrimeField>(value: u64) -> [F; 2] {
        [F::from_u64(value & 0xFFFFFFFF), F::from_u64((value >> 32) & 0xFFFFFFFF)]
    }

    #[inline(always)]
    pub fn build_full_trace_step<F: PrimeField>(
        inst: &ZiskInst,
        inst_ctx: &InstContext,
        reg_trace: &EmuRegTrace,
    ) -> EmuFullTraceStep<F> {
        // Calculate intermediate values
        let a = [inst_ctx.a & 0xFFFFFFFF, (inst_ctx.a >> 32) & 0xFFFFFFFF];
        let b = [inst_ctx.b & 0xFFFFFFFF, (inst_ctx.b >> 32) & 0xFFFFFFFF];
        let c = [inst_ctx.c & 0xFFFFFFFF, (inst_ctx.c >> 32) & 0xFFFFFFFF];
        let store_prev_value = [
            reg_trace.store_reg_prev_value & 0xFFFFFFFF,
            (reg_trace.store_reg_prev_value >> 32) & 0xFFFFFFFF,
        ];

        let addr1 = (inst.b_offset_imm0 as i64
            + if inst.b_src == SRC_IND { inst_ctx.a as i64 } else { 0 }) as u64;

        let jmp_offset1 = if inst.jmp_offset1 >= 0 {
            F::from_u64(inst.jmp_offset1 as u64)
        } else {
            F::neg(F::from_u64((-inst.jmp_offset1) as u64))
        };

        let jmp_offset2 = if inst.jmp_offset2 >= 0 {
            F::from_u64(inst.jmp_offset2 as u64)
        } else {
            F::neg(F::from_u64((-inst.jmp_offset2) as u64))
        };

        let store_offset = if inst.store_offset >= 0 {
            F::from_u64(inst.store_offset as u64)
        } else {
            F::neg(F::from_u64((-inst.store_offset) as u64))
        };

        let a_offset_imm0 = if inst.a_offset_imm0 as i64 >= 0 {
            F::from_u64(inst.a_offset_imm0)
        } else {
            F::neg(F::from_u64((-(inst.a_offset_imm0 as i64)) as u64))
        };

        let b_offset_imm0 = if inst.b_offset_imm0 as i64 >= 0 {
            F::from_u64(inst.b_offset_imm0)
        } else {
            F::neg(F::from_u64((-(inst.b_offset_imm0 as i64)) as u64))
        };

        EmuFullTraceStep {
            a: [F::from_u64(a[0]), F::from_u64(a[1])],
            b: [F::from_u64(b[0]), F::from_u64(b[1])],
            c: [F::from_u64(c[0]), F::from_u64(c[1])],

            flag: F::from_bool(inst_ctx.flag),
            pc: F::from_u64(inst.paddr),
            a_src_imm: F::from_bool(inst.a_src == SRC_IMM),
            a_src_mem: F::from_bool(inst.a_src == SRC_MEM),
            a_src_reg: F::from_bool(inst.a_src == SRC_REG),
            a_offset_imm0,
            // #[cfg(not(feature = "sp"))]
            a_imm1: F::from_u64(inst.a_use_sp_imm1),
            // #[cfg(feature = "sp")]
            // sp: F::from_u64(inst_ctx.sp),
            // #[cfg(feature = "sp")]
            // a_src_sp: F::from_bool(inst.a_src == SRC_SP),
            // #[cfg(feature = "sp")]
            // a_use_sp_imm1: F::from_u64(inst.a_use_sp_imm1),
            a_src_step: F::from_bool(inst.a_src == SRC_STEP),
            b_src_imm: F::from_bool(inst.b_src == SRC_IMM),
            b_src_mem: F::from_bool(inst.b_src == SRC_MEM),
            b_src_reg: F::from_bool(inst.b_src == SRC_REG),
            b_offset_imm0,
            // #[cfg(not(feature = "sp"))]
            b_imm1: F::from_u64(inst.b_use_sp_imm1),
            // #[cfg(feature = "sp")]
            // b_use_sp_imm1: F::from_u64(inst.b_use_sp_imm1),
            b_src_ind: F::from_bool(inst.b_src == SRC_IND),
            ind_width: F::from_u64(inst.ind_width),
            is_external_op: F::from_bool(inst.is_external_op),
            // IMPORTANT: the opcodes fcall, fcall_get, and fcall_param are really a variant
            // of the copyb, use to get free-input information
            op: if inst.op == ZiskOp::Fcall.code()
                || inst.op == ZiskOp::FcallGet.code()
                || inst.op == ZiskOp::FcallParam.code()
            {
                F::from_u8(ZiskOp::CopyB.code())
            } else {
                F::from_u8(inst.op)
            },
            store_ra: F::from_bool(inst.store_ra),
            store_mem: F::from_bool(inst.store == STORE_MEM),
            store_reg: F::from_bool(inst.store == STORE_REG),
            store_ind: F::from_bool(inst.store == STORE_IND),
            store_offset,
            set_pc: F::from_bool(inst.set_pc),
            // #[cfg(feature = "sp")]
            // store_use_sp: F::from_bool(inst.store_use_sp),
            // #[cfg(feature = "sp")]
            // set_sp: F::from_bool(inst.set_sp),
            // #[cfg(feature = "sp")]
            // inc_sp: F::from_u64(inst.inc_sp),
            jmp_offset1,
            jmp_offset2,
            m32: F::from_bool(inst.m32),
            addr1: F::from_u64(addr1),
            a_reg_prev_mem_step: F::from_u64(reg_trace.reg_prev_steps[0]),
            b_reg_prev_mem_step: F::from_u64(reg_trace.reg_prev_steps[1]),
            store_reg_prev_mem_step: F::from_u64(reg_trace.reg_prev_steps[2]),
            store_reg_prev_value: [
                F::from_u64(store_prev_value[0]),
                F::from_u64(store_prev_value[1]),
            ],
        }
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
            print!("x{}={}={:x} ", i, r, r);
        }
        println!();
    }
}
