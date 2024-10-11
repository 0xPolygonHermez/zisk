use std::{mem, time::Instant};

use crate::{
    EmuContext, EmuFullTraceStep, EmuOptions, EmuSlice, EmuTrace, EmuTraceEnd, EmuTraceStart,
    EmuTraceStep, ParEmuExecutionType, ParEmuOptions,
};
use p3_field::{AbstractField, PrimeField};
use riscv::RiscVRegisters;
// #[cfg(feature = "sp")]
// use zisk_core::SRC_SP;
use zisk_core::{
    ZiskInst, ZiskOperationType, ZiskRequired, ZiskRequiredMemory, ZiskRequiredOperation, ZiskRom,
    OUTPUT_ADDR, ROM_ENTRY, SRC_C, SRC_IMM, SRC_IND, SRC_MEM, SRC_STEP, STORE_IND, STORE_MEM,
    STORE_NONE, SYS_ADDR,
};

/// ZisK emulator structure, containing the ZisK rom, the list of ZisK operations, and the
/// execution context
pub struct Emu<'a> {
    /// ZisK rom, containing the program to execute, which is constant for this program except for
    /// the input data
    pub rom: &'a ZiskRom,
    /// Context, where the state of the execution is stored and modified at every execution step
    ctx: EmuContext,
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
            SRC_MEM => {
                let mut addr = instruction.a_offset_imm0;
                if instruction.a_use_sp_imm1 != 0 {
                    addr += self.ctx.inst_ctx.sp;
                }
                self.ctx.inst_ctx.a = self.ctx.inst_ctx.mem.read(addr, 8);
                if self.ctx.do_stats {
                    self.ctx.stats.on_memory_read(addr, 8);
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

    /// Copy the 'a' register and log memory access if specified by the current instruction
    #[inline(always)]
    pub fn source_a_slice(&mut self, instruction: &ZiskInst, a: u64, required: &mut ZiskRequired) {
        self.ctx.inst_ctx.a = a;
        match instruction.a_src {
            SRC_C => (),
            SRC_MEM => {
                // let mut addr = instruction.a_offset_imm0;
                // if instruction.a_use_sp_imm1 != 0 {
                //     addr += self.ctx.inst_ctx.sp;
                // }
                // let required_memory = ZiskRequiredMemory {
                //     step: self.ctx.inst_ctx.step,
                //     is_write: false,
                //     address: addr,
                //     width: 8,
                //     value: a,
                // };
                // required.memory.push(required_memory);
            }
            SRC_IMM => (),
            SRC_STEP => (),
            // #[cfg(feature = "sp")]
            // SRC_SP => (),
            _ => panic!(
                "Emu::source_a_slice() Invalid a_src={} pc={}",
                instruction.a_src, self.ctx.inst_ctx.pc
            ),
        }
    }

    /// Calculate the 'a' register value based on the source specified by the current instruction
    #[inline(always)]
    pub fn par_source_a(&mut self, instruction: &ZiskInst, required: &mut ZiskRequired) {
        match instruction.a_src {
            SRC_C => self.ctx.inst_ctx.a = self.ctx.inst_ctx.c,
            SRC_MEM => {
                let mut addr = instruction.a_offset_imm0;
                if instruction.a_use_sp_imm1 != 0 {
                    addr += self.ctx.inst_ctx.sp;
                }
                self.ctx.inst_ctx.a = self.ctx.inst_ctx.mem.read(addr, 8);
                let required_memory = ZiskRequiredMemory {
                    step: self.ctx.inst_ctx.step,
                    is_write: false,
                    address: addr,
                    width: 8,
                    value: self.ctx.inst_ctx.a,
                };
                // required.memory.push(required_memory);
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

    /// Calculate the 'b' register value based on the source specified by the current instruction
    #[inline(always)]
    pub fn source_b(&mut self, instruction: &ZiskInst) {
        match instruction.b_src {
            SRC_C => self.ctx.inst_ctx.b = self.ctx.inst_ctx.c,
            SRC_MEM => {
                let mut addr = instruction.b_offset_imm0;
                if instruction.b_use_sp_imm1 != 0 {
                    addr += self.ctx.inst_ctx.sp;
                }
                self.ctx.inst_ctx.b = self.ctx.inst_ctx.mem.read(addr, 8);
                if self.ctx.do_stats {
                    self.ctx.stats.on_memory_read(addr, 8);
                }
            }
            SRC_IMM => {
                self.ctx.inst_ctx.b = instruction.b_offset_imm0 | (instruction.b_use_sp_imm1 << 32)
            }
            SRC_IND => {
                let mut addr =
                    (self.ctx.inst_ctx.a as i64 + instruction.b_offset_imm0 as i64) as u64;
                if instruction.b_use_sp_imm1 != 0 {
                    addr += self.ctx.inst_ctx.sp;
                }
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

    /// Copy the 'b' register and log memory access if specified by the current instruction
    #[inline(always)]
    pub fn source_b_slice(&mut self, instruction: &ZiskInst, b: u64, required: &mut ZiskRequired) {
        self.ctx.inst_ctx.b = b;
        match instruction.b_src {
            SRC_C => (),
            SRC_MEM => {
                let mut addr = instruction.b_offset_imm0;
                if instruction.b_use_sp_imm1 != 0 {
                    addr += self.ctx.inst_ctx.sp;
                }
                let required_memory = ZiskRequiredMemory {
                    step: self.ctx.inst_ctx.step,
                    is_write: false,
                    address: addr,
                    width: 8,
                    value: b,
                };
                // required.memory.push(required_memory);
            }
            SRC_IMM => (),
            SRC_IND => {
                let mut addr =
                    (self.ctx.inst_ctx.a as i64 + instruction.b_offset_imm0 as i64) as u64;
                if instruction.b_use_sp_imm1 != 0 {
                    addr += self.ctx.inst_ctx.sp;
                }
                let required_memory = ZiskRequiredMemory {
                    step: self.ctx.inst_ctx.step,
                    is_write: false,
                    address: addr,
                    width: instruction.ind_width,
                    value: b,
                };
                // required.memory.push(required_memory);
            }
            _ => panic!(
                "Emu::source_b_slice() Invalid b_src={} pc={}",
                instruction.b_src, self.ctx.inst_ctx.pc
            ),
        }
    }

    /// Calculate the 'b' register value based on the source specified by the current instruction
    #[inline(always)]
    pub fn par_source_b(&mut self, instruction: &ZiskInst, required: &mut ZiskRequired) {
        match instruction.b_src {
            SRC_C => self.ctx.inst_ctx.b = self.ctx.inst_ctx.c,
            SRC_MEM => {
                let mut addr = instruction.b_offset_imm0;
                if instruction.b_use_sp_imm1 != 0 {
                    addr += self.ctx.inst_ctx.sp;
                }
                self.ctx.inst_ctx.b = self.ctx.inst_ctx.mem.read(addr, 8);
                let required_memory = ZiskRequiredMemory {
                    step: self.ctx.inst_ctx.step,
                    is_write: false,
                    address: addr,
                    width: 8,
                    value: self.ctx.inst_ctx.b,
                };
                // required.memory.push(required_memory);
            }
            SRC_IMM => {
                self.ctx.inst_ctx.b = instruction.b_offset_imm0 | (instruction.b_use_sp_imm1 << 32)
            }
            SRC_IND => {
                let mut addr =
                    (self.ctx.inst_ctx.a as i64 + instruction.b_offset_imm0 as i64) as u64;
                if instruction.b_use_sp_imm1 != 0 {
                    addr += self.ctx.inst_ctx.sp;
                }
                self.ctx.inst_ctx.b = self.ctx.inst_ctx.mem.read(addr, instruction.ind_width);
                let required_memory = ZiskRequiredMemory {
                    step: self.ctx.inst_ctx.step,
                    is_write: false,
                    address: addr,
                    width: instruction.ind_width,
                    value: self.ctx.inst_ctx.b,
                };
                // required.memory.push(required_memory);
            }
            _ => panic!(
                "Emu::source_b() Invalid b_src={} pc={}",
                instruction.b_src, self.ctx.inst_ctx.pc
            ),
        }
    }

    /// Store the 'c' register value based on the storage specified by the current instruction
    #[inline(always)]
    pub fn store_c(&mut self, instruction: &ZiskInst) {
        match instruction.store {
            STORE_NONE => {}
            STORE_MEM => {
                let val: i64 = if instruction.store_ra {
                    self.ctx.inst_ctx.pc as i64 + instruction.jmp_offset2
                } else {
                    self.ctx.inst_ctx.c as i64
                };
                let mut addr: i64 = instruction.store_offset;
                if instruction.store_use_sp {
                    addr += self.ctx.inst_ctx.sp as i64;
                }
                self.ctx.inst_ctx.mem.write(addr as u64, val as u64, 8);
                if self.ctx.do_stats {
                    self.ctx.stats.on_memory_write(addr as u64, 8);
                }
            }
            STORE_IND => {
                let val: i64 = if instruction.store_ra {
                    self.ctx.inst_ctx.pc as i64 + instruction.jmp_offset2
                } else {
                    self.ctx.inst_ctx.c as i64
                };
                let mut addr = instruction.store_offset;
                if instruction.store_use_sp {
                    addr += self.ctx.inst_ctx.sp as i64;
                }
                addr += self.ctx.inst_ctx.a as i64;
                self.ctx.inst_ctx.mem.write(addr as u64, val as u64, instruction.ind_width);
                if self.ctx.do_stats {
                    self.ctx.stats.on_memory_write(addr as u64, instruction.ind_width);
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
    pub fn store_c_slice(&mut self, instruction: &ZiskInst, required: &mut ZiskRequired) {
        match instruction.store {
            STORE_NONE => {}
            STORE_MEM => {
                let val: i64 = if instruction.store_ra {
                    self.ctx.inst_ctx.pc as i64 + instruction.jmp_offset2
                } else {
                    self.ctx.inst_ctx.c as i64
                };
                let mut addr: i64 = instruction.store_offset;
                if instruction.store_use_sp {
                    addr += self.ctx.inst_ctx.sp as i64;
                }
                self.ctx.inst_ctx.mem.write_silent(addr as u64, val as u64, 8);
                // let required_memory = ZiskRequiredMemory {
                //     step: self.ctx.inst_ctx.step,
                //     is_write: true,
                //     address: addr as u64,
                //     width: 8,
                //     value: val as u64,
                // };
                // required.memory.push(required_memory);
            }
            STORE_IND => {
                let val: i64 = if instruction.store_ra {
                    self.ctx.inst_ctx.pc as i64 + instruction.jmp_offset2
                } else {
                    self.ctx.inst_ctx.c as i64
                };
                let mut addr = instruction.store_offset;
                if instruction.store_use_sp {
                    addr += self.ctx.inst_ctx.sp as i64;
                }
                addr += self.ctx.inst_ctx.a as i64;
                self.ctx.inst_ctx.mem.write_silent(addr as u64, val as u64, instruction.ind_width);
                // let required_memory = ZiskRequiredMemory {
                //     step: self.ctx.inst_ctx.step,
                //     is_write: true,
                //     address: addr as u64,
                //     width: instruction.ind_width,
                //     value: val as u64,
                // };
                // required.memory.push(required_memory);
            }
            _ => panic!(
                "Emu::store_c_slice() Invalid store={} pc={}",
                instruction.store, self.ctx.inst_ctx.pc
            ),
        }
    }

    /// Store the 'c' register value based on the storage specified by the current instruction
    #[inline(always)]
    pub fn par_store_c(&mut self, instruction: &ZiskInst, required: &mut ZiskRequired) {
        match instruction.store {
            STORE_NONE => {}
            STORE_MEM => {
                let val: i64 = if instruction.store_ra {
                    self.ctx.inst_ctx.pc as i64 + instruction.jmp_offset2
                } else {
                    self.ctx.inst_ctx.c as i64
                };
                let mut addr: i64 = instruction.store_offset;
                if instruction.store_use_sp {
                    addr += self.ctx.inst_ctx.sp as i64;
                }
                self.ctx.inst_ctx.mem.write(addr as u64, val as u64, 8);
                let required_memory = ZiskRequiredMemory {
                    step: self.ctx.inst_ctx.step,
                    is_write: true,
                    address: addr as u64,
                    width: 8,
                    value: val as u64,
                };
                required.memory.push(required_memory);
            }
            STORE_IND => {
                let val: i64 = if instruction.store_ra {
                    self.ctx.inst_ctx.pc as i64 + instruction.jmp_offset2
                } else {
                    self.ctx.inst_ctx.c as i64
                };
                let mut addr = instruction.store_offset;
                if instruction.store_use_sp {
                    addr += self.ctx.inst_ctx.sp as i64;
                }
                addr += self.ctx.inst_ctx.a as i64;
                self.ctx.inst_ctx.mem.write(addr as u64, val as u64, instruction.ind_width);
                let required_memory = ZiskRequiredMemory {
                    step: self.ctx.inst_ctx.step,
                    is_write: true,
                    address: addr as u64,
                    width: instruction.ind_width,
                    value: val as u64,
                };
                required.memory.push(required_memory);
            }
            _ => panic!(
                "Emu::store_c() Invalid store={} pc={}",
                instruction.store, self.ctx.inst_ctx.pc
            ),
        }
    }

    /// Set SP, if specified by the current instruction
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
        self.source_a(instruction);
        self.source_b(instruction);
        (instruction.func)(&mut self.ctx.inst_ctx);
        self.store_c(instruction);
        // #[cfg(feature = "sp")]
        // self.set_sp(instruction);
        self.set_pc(instruction);
        self.ctx.inst_ctx.end = instruction.end;
        self.ctx.inst_ctx.step += 1;
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
            self.ctx.trace.steps.reserve(self.ctx.callback_steps as usize);

            // Init pc to the rom entry address
            self.ctx.trace.start.pc = ROM_ENTRY;
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
        self.ctx.trace.start.pc = ROM_ENTRY;

        // Store the stats option into the emulator context
        self.ctx.do_stats = options.stats;

        let mut emu_traces = Vec::new();

        println!("thread_id={}", par_options.thread_id);

        while !self.ctx.inst_ctx.end {
            let block_idx = self.ctx.inst_ctx.step / par_options.num_steps as u64;
            let is_my_block =
                block_idx % par_options.num_threads as u64 == par_options.thread_id as u64;

            let is_my_block_required =
                block_idx % par_options.num_threads as u64 == par_options.thread_id as u64 - 8u64;

            if !is_my_block && !is_my_block_required {
                self.step(options, &None::<fn(EmuTrace)>);
            } else {
                // Check if is the first step of a new block
                if self.ctx.inst_ctx.step % par_options.num_steps as u64 == 0 {
                    emu_traces.push(EmuTrace {
                        start: EmuTraceStart {
                            pc: self.ctx.inst_ctx.pc,
                            sp: self.ctx.inst_ctx.sp,
                            c: self.ctx.inst_ctx.c,
                            step: self.ctx.inst_ctx.step,
                        },
                        steps: Vec::with_capacity(par_options.num_steps),
                        end: EmuTraceEnd { end: false },
                    });
                }
                self.par_step::<F>(options, par_options, emu_traces.last_mut().unwrap());
            }
        }

        emu_traces
    }

    /// Performs one single step of the emulation
    #[inline(always)]
    #[allow(unused_variables)]
    pub fn step(&mut self, options: &EmuOptions, callback: &Option<impl Fn(EmuTrace)>) {
        let instruction = self.rom.get_instruction(self.ctx.inst_ctx.pc);

        //println!("Emu::step() executing step={} pc={:x} inst={}", ctx.step, ctx.pc,
        // inst.i.to_string()); println!("Emu::step() step={} pc={}", ctx.step, ctx.pc);

        // Build the 'a' register value  based on the source specified by the current instruction
        self.source_a(instruction);

        // Build the 'b' register value  based on the source specified by the current instruction
        self.source_b(instruction);

        // Call the operation
        (instruction.func)(&mut self.ctx.inst_ctx);

        // Retrieve statistics data
        // if self.ctx.do_stats {
        //     self.ctx.stats.on_op(instruction, self.ctx.inst_ctx.a, self.ctx.inst_ctx.b);
        // }

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
                "step={} pc={} op={}={} a={} b={} c={} flag={} inst={}",
                self.ctx.inst_ctx.step,
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
            let trace_step = EmuTraceStep { a: self.ctx.inst_ctx.a, b: self.ctx.inst_ctx.b };

            self.ctx.trace.steps.push(trace_step);

            // Increment step counter
            self.ctx.inst_ctx.step += 1;

            if self.ctx.inst_ctx.end
                || ((self.ctx.inst_ctx.step - self.ctx.last_callback_step)
                    == self.ctx.callback_steps)
            {
                // In run() we have checked the callback consistency with ctx.do_callback
                let callback = callback.as_ref().unwrap();

                // Set the end-of-trace data
                self.ctx.trace.end.end = self.ctx.inst_ctx.end;

                // Swap the emulator trace to avoid memory copies
                let mut trace = EmuTrace::default();
                trace.steps.reserve(self.ctx.callback_steps as usize);
                mem::swap(&mut self.ctx.trace, &mut trace);
                (callback)(trace);

                // Set the start-of-trace data
                self.ctx.trace.start.pc = self.ctx.inst_ctx.pc;
                self.ctx.trace.start.sp = self.ctx.inst_ctx.sp;
                self.ctx.trace.start.c = self.ctx.inst_ctx.c;
                self.ctx.trace.start.step = self.ctx.inst_ctx.step;

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
    #[allow(unused_variables)]
    pub fn par_step<F: PrimeField>(
        &mut self,
        options: &EmuOptions,
        par_options: &ParEmuOptions,
        emu_full_trace_vec: &mut EmuTrace,
    ) {
        let last_pc = self.ctx.inst_ctx.pc;
        let last_c = self.ctx.inst_ctx.c;

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

        emu_full_trace_vec
            .steps
            .push(EmuTraceStep { a: self.ctx.inst_ctx.a, b: self.ctx.inst_ctx.b });
        // let full_trace_step = Self::par_step_slice::<F>(
        //     self.ctx.inst_ctx.a,
        //     self.ctx.inst_ctx.b,
        //     self.ctx.inst_ctx.c,
        //     self.ctx.inst_ctx.flag,
        //     last_c,
        //     last_pc,
        //     instruction,
        //     self.ctx.inst_ctx.end,
        // );

        // emu_full_trace_vec.push(full_trace_step);

        // Increment step counter
        self.ctx.inst_ctx.step += 1;
    }

    /// Performs one single step of the emulation
    #[inline(always)]
    #[allow(unused_variables)]
    pub fn par_step_required<F: PrimeField>(
        &mut self,
        options: &EmuOptions,
        par_options: &ParEmuOptions,
        required: &mut ZiskRequired,
    ) {
        let last_pc = self.ctx.inst_ctx.pc;
        let last_c = self.ctx.inst_ctx.c;

        let instruction = self.rom.get_instruction(self.ctx.inst_ctx.pc);

        // Build the 'a' register value  based on the source specified by the current instruction
        self.par_source_a(instruction, required);

        // Build the 'b' register value  based on the source specified by the current instruction
        self.par_source_b(instruction, required);

        // Call the operation
        (instruction.func)(&mut self.ctx.inst_ctx);

        // Store the 'c' register value based on the storage specified by the current instruction
        self.par_store_c(instruction, required);

        // Set SP, if specified by the current instruction
        // #[cfg(feature = "sp")]
        // self.set_sp(instruction);

        // Set PC, based on current PC, current flag and current instruction
        self.set_pc(instruction);

        // If this is the last instruction, stop executing
        self.ctx.inst_ctx.end = instruction.end;

        // Build and store the operation required data
        // match instruction.op_type {
        //     ZiskOperationType::Internal => (),
        //     ZiskOperationType::Arith => {
        //         let required_operation = ZiskRequiredOperation {
        //             step: self.ctx.inst_ctx.step,
        //             opcode: instruction.op,
        //             a: if instruction.m32 {
        //                 self.ctx.inst_ctx.a & 0xffffffff
        //             } else {
        //                 self.ctx.inst_ctx.a
        //             },
        //             b: if instruction.m32 {
        //                 self.ctx.inst_ctx.b & 0xffffffff
        //             } else {
        //                 self.ctx.inst_ctx.b
        //             },
        //         };
        //         required.arith.push(required_operation);
        //     }
        //     ZiskOperationType::Binary => {
        //         let required_operation = ZiskRequiredOperation {
        //             step: self.ctx.inst_ctx.step,
        //             opcode: instruction.op,
        //             a: if instruction.m32 {
        //                 self.ctx.inst_ctx.a & 0xffffffff
        //             } else {
        //                 self.ctx.inst_ctx.a
        //             },
        //             b: if instruction.m32 {
        //                 self.ctx.inst_ctx.b & 0xffffffff
        //             } else {
        //                 self.ctx.inst_ctx.b
        //             },
        //         };
        //         required.binary.push(required_operation);
        //     }
        //     ZiskOperationType::BinaryE => {
        //         let required_operation = ZiskRequiredOperation {
        //             step: self.ctx.inst_ctx.step,
        //             opcode: instruction.op,
        //             a: if instruction.m32 {
        //                 self.ctx.inst_ctx.a & 0xffffffff
        //             } else {
        //                 self.ctx.inst_ctx.a
        //             },
        //             b: if instruction.m32 {
        //                 self.ctx.inst_ctx.b & 0xffffffff
        //             } else {
        //                 self.ctx.inst_ctx.b
        //             },
        //         };
        //         required.binary_extension.push(required_operation);
        //     }
        //     _ => panic!("Emu::step_slice() found invalid op_type"),
        // }

        // Increment step counter
        self.ctx.inst_ctx.step += 1;
    }

    /// Run a slice of the program to generate full traces
    #[inline(always)]
    pub fn run_slice<F: AbstractField>(&mut self, trace: &EmuTrace) -> EmuSlice<F> {
        // Create an emulator slice instance
        let mut emu_slice = EmuSlice {
            full_trace: Vec::with_capacity(trace.steps.len()),
            required: ZiskRequired {
                arith: Vec::new(),
                binary: Vec::new(),
                binary_extension: Vec::new(),
                memory: Vec::new(),
            },
        };

        // Set initial state
        self.ctx.inst_ctx.pc = trace.start.pc;
        self.ctx.inst_ctx.sp = trace.start.sp;
        self.ctx.inst_ctx.step = trace.start.step;
        self.ctx.inst_ctx.c = trace.start.c;

        // Loop for every trace to get its corresponding full_trace
        let start = Instant::now();
        for step in &trace.steps {
            self.step_slice(step, &mut emu_slice.full_trace, &mut emu_slice.required);
        }
        println!("run_slice() duration={:.4}", start.elapsed().as_secs_f64());

        // Return emulator slice
        emu_slice
    }

    /// Performs one single step of the emulation
    #[inline(always)]
    pub fn step_slice<F: AbstractField>(
        &mut self,
        trace_step: &EmuTraceStep,
        full_trace: &mut Vec<EmuFullTraceStep<F>>,
        required: &mut ZiskRequired,
    ) {
        let previous_pc = self.ctx.inst_ctx.pc;
        let last_c = self.ctx.inst_ctx.c;
        let instruction = self.rom.get_instruction(self.ctx.inst_ctx.pc);
        // self.source_a_slice(instruction, trace_step.a, required);
        self.ctx.inst_ctx.a = trace_step.a;
        self.ctx.inst_ctx.b = trace_step.b;
        // self.source_b_slice(instruction, trace_step.b, required);
        (instruction.func)(&mut self.ctx.inst_ctx);
        self.store_c_slice(instruction, required);
        // #[cfg(feature = "sp")]
        // self.set_sp(instruction);
        self.set_pc(instruction);
        self.ctx.inst_ctx.end = instruction.end;

        // Build and store the full trace
        let full_trace_step = EmuFullTraceStep {
            a: [
                F::from_canonical_u64(self.ctx.inst_ctx.a & 0xFFFFFFFF),
                F::from_canonical_u64((self.ctx.inst_ctx.a >> 32) & 0xFFFFFFFF),
            ],
            b: [
                F::from_canonical_u64(self.ctx.inst_ctx.b & 0xFFFFFFFF),
                F::from_canonical_u64((self.ctx.inst_ctx.b >> 32) & 0xFFFFFFFF),
            ],
            c: [
                F::from_canonical_u64(self.ctx.inst_ctx.c & 0xFFFFFFFF),
                F::from_canonical_u64((self.ctx.inst_ctx.c >> 32) & 0xFFFFFFFF),
            ],
            last_c: [
                F::from_canonical_u64(last_c & 0xFFFFFFFF),
                F::from_canonical_u64((last_c >> 32) & 0xFFFFFFFF),
            ],
            flag: F::from_bool(self.ctx.inst_ctx.flag),
            pc: F::from_canonical_u64(previous_pc),
            a_src_imm: F::from_bool(instruction.a_src == SRC_IMM),
            a_src_mem: F::from_bool(instruction.a_src == SRC_MEM),
            a_offset_imm0: F::from_canonical_u64(instruction.a_offset_imm0),
            // #[cfg(not(feature = "sp"))]
            a_imm1: F::from_canonical_u64(instruction.a_use_sp_imm1),
            // #[cfg(feature = "sp")]
            // sp: F::from_canonical_u64(self.ctx.inst_ctx.sp),
            // #[cfg(feature = "sp")]
            // a_src_sp: F::from_bool(instruction.a_src == SRC_SP),
            // #[cfg(feature = "sp")]
            // a_use_sp_imm1: F::from_canonical_u64(instruction.a_use_sp_imm1),
            a_src_step: F::from_bool(instruction.a_src == SRC_STEP),
            b_src_imm: F::from_bool(instruction.b_src == SRC_IMM),
            b_src_mem: F::from_bool(instruction.b_src == SRC_MEM),
            b_offset_imm0: F::from_canonical_u64(instruction.b_offset_imm0),
            // #[cfg(not(feature = "sp"))]
            b_imm1: F::from_canonical_u64(instruction.b_use_sp_imm1),
            // #[cfg(feature = "sp")]
            // b_use_sp_imm1: F::from_canonical_u64(instruction.b_use_sp_imm1),
            b_src_ind: F::from_bool(instruction.b_src == SRC_IND),
            ind_width: F::from_canonical_u64(instruction.ind_width),
            is_external_op: F::from_bool(instruction.is_external_op),
            op: F::from_canonical_u8(instruction.op),
            store_ra: F::from_bool(instruction.store_ra),
            store_mem: F::from_bool(instruction.store == STORE_MEM),
            store_ind: F::from_bool(instruction.store == STORE_IND),
            store_offset: F::from_canonical_u64(instruction.store_offset as u64),
            set_pc: F::from_bool(instruction.set_pc),
            // #[cfg(feature = "sp")]
            // store_use_sp: F::from_bool(instruction.store_use_sp),
            // #[cfg(feature = "sp")]
            // set_sp: F::from_bool(instruction.set_sp),
            // #[cfg(feature = "sp")]
            // inc_sp: F::from_canonical_u64(instruction.inc_sp),
            jmp_offset1: F::from_canonical_u64(instruction.jmp_offset1 as u64),
            jmp_offset2: F::from_canonical_u64(instruction.jmp_offset2 as u64),
            main_segment: F::from_canonical_u64(0),
            main_first_segment: F::from_bool(false),
            main_last_segment: F::from_bool(false),
            end: F::from_bool(self.ctx.inst_ctx.end),
            m32: F::from_bool(instruction.m32),
            operation_bus_enabled: F::from_bool(instruction.op_type == ZiskOperationType::Binary),
        };
        full_trace.push(full_trace_step);

        // Build and store the operation required data
        // match instruction.op_type {
        //     ZiskOperationType::Internal => (),
        //     ZiskOperationType::Arith => {
        //         let required_operation = ZiskRequiredOperation {
        //             step: self.ctx.inst_ctx.step,
        //             opcode: instruction.op,
        //             a: if instruction.m32 {
        //                 self.ctx.inst_ctx.a & 0xffffffff
        //             } else {
        //                 self.ctx.inst_ctx.a
        //             },
        //             b: if instruction.m32 {
        //                 self.ctx.inst_ctx.b & 0xffffffff
        //             } else {
        //                 self.ctx.inst_ctx.b
        //             },
        //         };
        //         required.arith.push(required_operation);
        //     }
        //     ZiskOperationType::Binary | ZiskOperationType::BinaryE => {
        //         let required_operation = ZiskRequiredOperation {
        //             step: self.ctx.inst_ctx.step,
        //             opcode: instruction.op,
        //             a: if instruction.m32 {
        //                 self.ctx.inst_ctx.a & 0xffffffff
        //             } else {
        //                 self.ctx.inst_ctx.a
        //             },
        //             b: if instruction.m32 {
        //                 self.ctx.inst_ctx.b & 0xffffffff
        //             } else {
        //                 self.ctx.inst_ctx.b
        //             },
        //         };
        //         required.binary.push(required_operation);
        //     }
        //     _ => panic!("Emu::step_slice() found invalid op_type"),
        // }

        self.ctx.inst_ctx.step += 1;
    }

    /// Performs one single step of the emulation
    #[inline(always)]
    pub fn par_step_slice<F: AbstractField>(
        a: u64,
        b: u64,
        c: u64,
        flag: bool,
        last_c: u64,
        previous_pc: u64,
        instruction: &ZiskInst,
        end: bool,
    ) -> EmuFullTraceStep<F> {
        EmuFullTraceStep {
            a: [
                F::from_canonical_u64(a & 0xFFFFFFFF),
                F::from_canonical_u64((a >> 32) & 0xFFFFFFFF),
            ],
            b: [
                F::from_canonical_u64(b & 0xFFFFFFFF),
                F::from_canonical_u64((b >> 32) & 0xFFFFFFFF),
            ],
            c: [
                F::from_canonical_u64(c & 0xFFFFFFFF),
                F::from_canonical_u64((c >> 32) & 0xFFFFFFFF),
            ],
            last_c: [
                F::from_canonical_u64(last_c & 0xFFFFFFFF),
                F::from_canonical_u64((last_c >> 32) & 0xFFFFFFFF),
            ],
            flag: F::from_bool(flag),
            pc: F::from_canonical_u64(previous_pc),
            a_src_imm: F::from_bool(instruction.a_src == SRC_IMM),
            a_src_mem: F::from_bool(instruction.a_src == SRC_MEM),
            a_offset_imm0: F::from_canonical_u64(instruction.a_offset_imm0),
            // #[cfg(not(feature = "sp"))]
            a_imm1: F::from_canonical_u64(instruction.a_use_sp_imm1),
            // #[cfg(feature = "sp")]
            // sp: F::from_canonical_u64(self.ctx.inst_ctx.sp),
            // #[cfg(feature = "sp")]
            // a_src_sp: F::from_bool(instruction.a_src == SRC_SP),
            // #[cfg(feature = "sp")]
            // a_use_sp_imm1: F::from_canonical_u64(instruction.a_use_sp_imm1),
            a_src_step: F::from_bool(instruction.a_src == SRC_STEP),
            b_src_imm: F::from_bool(instruction.b_src == SRC_IMM),
            b_src_mem: F::from_bool(instruction.b_src == SRC_MEM),
            b_offset_imm0: F::from_canonical_u64(instruction.b_offset_imm0),
            // #[cfg(not(feature = "sp"))]
            b_imm1: F::from_canonical_u64(instruction.b_use_sp_imm1),
            // #[cfg(feature = "sp")]
            // b_use_sp_imm1: F::from_canonical_u64(instruction.b_use_sp_imm1),
            b_src_ind: F::from_bool(instruction.b_src == SRC_IND),
            ind_width: F::from_canonical_u64(instruction.ind_width),
            is_external_op: F::from_bool(instruction.is_external_op),
            op: F::from_canonical_u8(instruction.op),
            store_ra: F::from_bool(instruction.store_ra),
            store_mem: F::from_bool(instruction.store == STORE_MEM),
            store_ind: F::from_bool(instruction.store == STORE_IND),
            store_offset: F::from_canonical_u64(instruction.store_offset as u64),
            set_pc: F::from_bool(instruction.set_pc),
            // #[cfg(feature = "sp")]
            // store_use_sp: F::from_bool(instruction.store_use_sp),
            // #[cfg(feature = "sp")]
            // set_sp: F::from_bool(instruction.set_sp),
            // #[cfg(feature = "sp")]
            // inc_sp: F::from_canonical_u64(instruction.inc_sp),
            jmp_offset1: F::from_canonical_u64(instruction.jmp_offset1 as u64),
            jmp_offset2: F::from_canonical_u64(instruction.jmp_offset2 as u64),
            main_segment: F::from_canonical_u64(0),
            main_first_segment: F::from_bool(false),
            main_last_segment: F::from_bool(false),
            end: F::from_bool(end),
            m32: F::from_bool(instruction.m32),
            operation_bus_enabled: F::from_bool(instruction.op_type == ZiskOperationType::Binary),
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
            *reg = self.ctx.inst_ctx.mem.read(SYS_ADDR + (i as u64) * 8, 8);
        }
        regs_array
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
