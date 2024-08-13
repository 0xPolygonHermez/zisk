use std::mem;

use crate::{EmuContext, EmuFullTrace, EmuFullTraceStep, EmuOptions, EmuTrace, EmuTraceStep};
use p3_field::AbstractField;
use riscv::RiscVRegisters;
use zisk_core::{
    ZiskInst, ZiskRom, OUTPUT_ADDR, ROM_ENTRY, SRC_C, SRC_IMM, SRC_IND, SRC_MEM, SRC_SP, SRC_STEP,
    STORE_IND, STORE_MEM, STORE_NONE, SYS_ADDR,
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
impl<'a> Emu<'a> {
    pub fn new(rom: &ZiskRom) -> Emu {
        Emu { rom, ctx: EmuContext::default() }
    }

    pub fn create_emu_context(&mut self, inputs: Vec<u8>) -> EmuContext {
        // Initialize an empty instance
        let mut ctx = EmuContext::new(inputs);

        // Create a new read section for every RO data entry of the rom
        for i in 0..self.rom.ro_data.len() {
            ctx.mem.add_read_section(self.rom.ro_data[i].from, &self.rom.ro_data[i].data);
        }

        // Sort read sections by start address to improve performance when using binary search
        ctx.mem.read_sections.sort_by(|a, b| a.start.cmp(&b.start));

        // Get registers
        //emu.get_regs(); // TODO: ask Jordi

        ctx
    }

    /// Calculate the 'a' register value based on the source specified by the current instruction
    #[inline(always)]
    pub fn source_a(&mut self, instruction: &ZiskInst) {
        match instruction.a_src {
            SRC_C => self.ctx.a = self.ctx.c,
            SRC_MEM => {
                let mut addr = instruction.a_offset_imm0;
                if instruction.a_use_sp_imm1 != 0 {
                    addr += self.ctx.sp;
                }
                self.ctx.a = self.ctx.mem.read(addr, 8);
                if self.ctx.do_stats {
                    self.ctx.stats.on_memory_read(addr, 8);
                }
            }
            SRC_IMM => self.ctx.a = instruction.a_offset_imm0 | (instruction.a_use_sp_imm1 << 32),
            SRC_STEP => self.ctx.a = self.ctx.step,
            SRC_SP => self.ctx.a = self.ctx.sp,
            _ => panic!("Emu::source_a() Invalid a_src={} pc={}", instruction.a_src, self.ctx.pc),
        }
    }

    /// Calculate the 'b' register value based on the source specified by the current instruction
    #[inline(always)]
    pub fn source_b(&mut self, instruction: &ZiskInst) {
        match instruction.b_src {
            SRC_C => self.ctx.b = self.ctx.c,
            SRC_MEM => {
                let mut addr = instruction.b_offset_imm0;
                if instruction.b_use_sp_imm1 != 0 {
                    addr += self.ctx.sp;
                }
                self.ctx.b = self.ctx.mem.read(addr, 8);
                if self.ctx.do_stats {
                    self.ctx.stats.on_memory_read(addr, 8);
                }
            }
            SRC_IMM => self.ctx.b = instruction.b_offset_imm0 | (instruction.b_use_sp_imm1 << 32),
            SRC_IND => {
                let mut addr = (self.ctx.a as i64 + instruction.b_offset_imm0 as i64) as u64;
                if instruction.b_use_sp_imm1 != 0 {
                    addr += self.ctx.sp;
                }
                self.ctx.b = self.ctx.mem.read(addr, instruction.ind_width);
                if self.ctx.do_stats {
                    self.ctx.stats.on_memory_read(addr, instruction.ind_width);
                }
            }
            _ => panic!("Emu::source_b() Invalid b_src={} pc={}", instruction.b_src, self.ctx.pc),
        }
    }

    /// Store the 'c' register value based on the storage specified by the current instruction
    #[inline(always)]
    pub fn store_c(&mut self, instruction: &ZiskInst) {
        match instruction.store {
            STORE_NONE => {}
            STORE_MEM => {
                let val: i64 = if instruction.store_ra {
                    self.ctx.pc as i64 + instruction.jmp_offset2
                } else {
                    self.ctx.c as i64
                };
                let mut addr: i64 = instruction.store_offset;
                if instruction.store_use_sp {
                    addr += self.ctx.sp as i64;
                }
                self.ctx.mem.write(addr as u64, val as u64, 8);
                if self.ctx.do_stats {
                    self.ctx.stats.on_memory_write(addr as u64, 8);
                }
            }
            STORE_IND => {
                let val: i64 = if instruction.store_ra {
                    self.ctx.pc as i64 + instruction.jmp_offset2
                } else {
                    self.ctx.c as i64
                };
                let mut addr = instruction.store_offset;
                if instruction.store_use_sp {
                    addr += self.ctx.sp as i64;
                }
                addr += self.ctx.a as i64;
                self.ctx.mem.write(addr as u64, val as u64, instruction.ind_width);
                if self.ctx.do_stats {
                    self.ctx.stats.on_memory_write(addr as u64, instruction.ind_width);
                }
            }
            _ => panic!("Emu::store_c() Invalid store={} pc={}", instruction.store, self.ctx.pc),
        }
    }

    /// Store the 'c' register value based on the storage specified by the current instruction
    #[inline(always)]
    pub fn store_c_silent(&mut self, instruction: &ZiskInst) {
        match instruction.store {
            STORE_NONE => {}
            STORE_MEM => {
                let val: i64 = if instruction.store_ra {
                    self.ctx.pc as i64 + instruction.jmp_offset2
                } else {
                    self.ctx.c as i64
                };
                let mut addr: i64 = instruction.store_offset;
                if instruction.store_use_sp {
                    addr += self.ctx.sp as i64;
                }
                self.ctx.mem.write_silent(addr as u64, val as u64, 8);
            }
            STORE_IND => {
                let val: i64 = if instruction.store_ra {
                    self.ctx.pc as i64 + instruction.jmp_offset2
                } else {
                    self.ctx.c as i64
                };
                let mut addr = instruction.store_offset;
                if instruction.store_use_sp {
                    addr += self.ctx.sp as i64;
                }
                addr += self.ctx.a as i64;
                self.ctx.mem.write_silent(addr as u64, val as u64, instruction.ind_width);
            }
            _ => panic!(
                "Emu::store_c_silent() Invalid store={} pc={}",
                instruction.store, self.ctx.pc
            ),
        }
    }

    /// Set SP, if specified by the current instruction
    #[inline(always)]
    pub fn set_sp(&mut self, instruction: &ZiskInst) {
        if instruction.set_sp {
            self.ctx.sp = self.ctx.c;
        } else {
            self.ctx.sp += instruction.inc_sp;
        }
    }

    /// Set PC, based on current PC, current flag and current instruction
    #[inline(always)]
    pub fn set_pc(&mut self, instruction: &ZiskInst) {
        if instruction.set_pc {
            self.ctx.pc = (self.ctx.c as i64 + instruction.jmp_offset1) as u64;
        } else if self.ctx.flag {
            self.ctx.pc = (self.ctx.pc as i64 + instruction.jmp_offset1) as u64;
        } else {
            self.ctx.pc = (self.ctx.pc as i64 + instruction.jmp_offset2) as u64;
        }
    }

    /// Performs one single step of the emulation
    #[inline(always)]
    #[allow(unused_variables)]
    pub fn step(&mut self, options: &EmuOptions, callback: &Option<impl Fn(EmuTrace)>) {
        let instruction = self.rom.get_instruction(self.ctx.pc);

        //println!("Emu::step() executing step={} pc={:x} inst={}", ctx.step, ctx.pc,
        // inst.i.to_string()); println!("Emu::step() step={} pc={}", ctx.step, ctx.pc);

        // Build the 'a' register value  based on the source specified by the current instruction
        self.source_a(instruction);

        // Build the 'b' register value  based on the source specified by the current instruction
        self.source_b(instruction);

        // Call the operation
        (self.ctx.c, self.ctx.flag) = (instruction.func)(self.ctx.a, self.ctx.b);
        if self.ctx.do_stats {
            self.ctx.stats.on_op(instruction, self.ctx.a, self.ctx.b);
        }

        // Store the 'c' register value based on the storage specified by the current instruction
        self.store_c(instruction);

        // Set SP, if specified by the current instruction
        self.set_sp(instruction);

        // Set PC, based on current PC, current flag and current instruction
        self.set_pc(instruction);

        // If this is the last instruction, stop executing
        if instruction.end {
            self.ctx.end = true;
            if options.stats {
                self.ctx.stats.on_steps(self.ctx.step);
            }
        }

        // Log the step, if requested
        #[cfg(debug_assertions)]
        if options.log_step {
            println!(
                "step={} pc={} op={}={} a={} b={} c={} flag={} inst={}",
                self.ctx.step,
                self.ctx.pc,
                instruction.op,
                instruction.op_str,
                self.ctx.a,
                self.ctx.b,
                self.ctx.c,
                self.ctx.flag,
                instruction.to_text()
            );
        }

        // Store an emulator trace, if requested
        if self.ctx.do_callback {
            let trace_step = EmuTraceStep { a: self.ctx.a, b: self.ctx.b };

            self.ctx.trace.steps.push(trace_step);

            // Increment step counter
            self.ctx.step += 1;

            if self.ctx.end ||
                ((self.ctx.step - self.ctx.last_callback_step) == self.ctx.callback_steps)
            {
                // In run() we have checked the callback consistency with ctx.do_callback
                let callback = callback.as_ref().unwrap();

                // Set the end-of-trace data
                self.ctx.trace.end.end = self.ctx.end;

                // Swap the emulator trace to avoid memory copies
                let mut trace = EmuTrace::default();
                trace.steps.reserve(self.ctx.callback_steps as usize);
                mem::swap(&mut self.ctx.trace, &mut trace);
                (callback)(trace);

                // Set the start-of-trace data
                self.ctx.trace.start.pc = self.ctx.pc;
                self.ctx.trace.start.sp = self.ctx.sp;
                self.ctx.trace.start.c = self.ctx.c;
                self.ctx.trace.start.step = self.ctx.step;

                // Increment the last callback step counter
                self.ctx.last_callback_step += self.ctx.callback_steps;
            }
        } else {
            // Increment step counter
            self.ctx.step += 1;
        }
    }

    /// Get the output as a vector of u64
    pub fn get_output(&self) -> Vec<u64> {
        let n = self.ctx.mem.read(OUTPUT_ADDR, 8);
        let mut addr = OUTPUT_ADDR + 8;

        let mut output: Vec<u64> = Vec::with_capacity(n as usize);
        for _i in 0..n {
            output.push(self.ctx.mem.read(addr, 8));
            addr += 8;
        }
        output
    }

    /// Get the output as a vector of u32
    pub fn get_output_32(&self) -> Vec<u32> {
        let n = self.ctx.mem.read(OUTPUT_ADDR, 4);
        let mut addr = OUTPUT_ADDR + 4;

        let mut output: Vec<u32> = Vec::with_capacity(n as usize);
        for _i in 0..n {
            output.push(self.ctx.mem.read(addr, 4) as u32);
            addr += 4;
        }
        output
    }

    /// Get the output as a vector of u8
    pub fn get_output_8(&self) -> Vec<u8> {
        let n = self.ctx.mem.read(OUTPUT_ADDR, 4);
        let mut addr = OUTPUT_ADDR + 4;

        let mut output: Vec<u8> = Vec::with_capacity(n as usize);
        for _i in 0..n {
            output.push(self.ctx.mem.read(addr, 1) as u8);
            output.push(self.ctx.mem.read(addr + 1, 1) as u8);
            output.push(self.ctx.mem.read(addr + 2, 1) as u8);
            output.push(self.ctx.mem.read(addr + 3, 1) as u8);
            addr += 4;
        }
        output
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
        while !self.ctx.end {
            if options.verbose {
                println!("Emu::run() step={} ctx.pc={}", self.ctx.step, self.ctx.pc);
            }
            // Check trace PC
            if options.tracerv && (self.ctx.pc & 0b11 == 0) {
                self.ctx.trace_pc = self.ctx.pc;
            }

            // Log emulation step, if requested
            if options.print_step.is_some() &&
                (options.print_step.unwrap() != 0) &&
                ((self.ctx.step % options.print_step.unwrap()) == 0)
            {
                println!("step={}", self.ctx.step);
            }

            // Stop the execution if we exceeded the specified running conditions
            if self.ctx.step >= options.max_steps {
                break;
            }

            // Execute the current step
            self.step(options, &callback);

            // Only trace after finishing a riscV instruction
            if options.tracerv && (self.ctx.pc & 0b11) == 0 {
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

    /// Run the whole program, fast
    #[inline(always)]
    pub fn run_fast(&mut self, options: &EmuOptions) {
        while !self.ctx.end && (self.ctx.step < options.max_steps) {
            self.step_fast();
        }
    }

    /// Performs one single step of the emulation
    #[inline(always)]
    pub fn step_fast(&mut self) {
        let instruction = self.rom.get_instruction(self.ctx.pc);
        self.source_a(instruction);
        self.source_b(instruction);
        (self.ctx.c, self.ctx.flag) = (instruction.func)(self.ctx.a, self.ctx.b);
        self.store_c(instruction);
        self.set_sp(instruction);
        self.set_pc(instruction);
        self.ctx.end = instruction.end;
        self.ctx.step += 1;
    }

    /// Run a slice of the program to generate full traces
    #[inline(always)]
    pub fn run_slice<F: AbstractField>(&mut self, trace: &EmuTrace) -> EmuFullTrace<F> {
        // Create a full trace instance
        let mut full_trace = EmuFullTrace::default();

        // Reserve space for the requested number of steps
        full_trace.steps.reserve(trace.steps.len());

        // Set initial state
        self.ctx.pc = trace.start.pc;
        self.ctx.sp = trace.start.sp;
        self.ctx.step = trace.start.step;
        self.ctx.c = trace.start.c;

        // Loop for every trace to get its corresponding full_trace
        for step in &trace.steps {
            self.step_slice(step, &mut full_trace);
        }

        // Return full trace
        full_trace
    }

    /// Performs one single step of the emulation
    #[inline(always)]
    pub fn step_slice<F: AbstractField>(
        &mut self,
        trace_step: &EmuTraceStep,
        full_trace: &mut EmuFullTrace<F>,
    ) {
        let last_c = self.ctx.c;
        let instruction = self.rom.get_instruction(self.ctx.pc);
        self.ctx.a = trace_step.a;
        self.ctx.b = trace_step.b;
        (self.ctx.c, self.ctx.flag) = (instruction.func)(self.ctx.a, self.ctx.b);
        self.store_c_silent(instruction);
        self.set_sp(instruction);
        self.set_pc(instruction);
        self.ctx.end = instruction.end;
        self.ctx.step += 1;
        let full_trace_step = EmuFullTraceStep {
            a: F::from_canonical_u64(self.ctx.a),
            b: F::from_canonical_u64(self.ctx.b),
            c: F::from_canonical_u64(self.ctx.c),
            last_c: F::from_canonical_u64(last_c),
            flag: F::from_bool(self.ctx.flag),
            pc: F::from_canonical_u64(self.ctx.pc),
            a_src_imm: F::from_bool(instruction.a_src == SRC_IMM),
            a_src_mem: F::from_bool(instruction.a_src == SRC_MEM),
            a_offset_imm0: F::from_canonical_u64(instruction.a_offset_imm0),
            sp: F::from_canonical_u64(self.ctx.sp),
            a_src_sp: F::from_bool(instruction.a_src == SRC_SP),
            a_use_sp_imm1: F::from_canonical_u64(instruction.a_use_sp_imm1),
            a_src_step: F::from_bool(instruction.a_src == SRC_STEP),
            b_src_imm: F::from_bool(instruction.b_src == SRC_IMM),
            b_src_mem: F::from_bool(instruction.b_src == SRC_MEM),
            b_offset_imm0: F::from_canonical_u64(instruction.b_offset_imm0),
            b_use_sp_imm1: F::from_canonical_u64(instruction.b_use_sp_imm1),
            b_src_ind: F::from_bool(instruction.b_src == SRC_IND),
            ind_width: F::from_canonical_u64(instruction.ind_width),
            is_external_op: F::from_bool(instruction.is_external_op),
            op: F::from_canonical_u8(instruction.op),
            store_ra: F::from_bool(instruction.store_ra),
            store_mem: F::from_bool(instruction.store == STORE_MEM),
            store_ind: F::from_bool(instruction.store == STORE_IND),
            store_offset: F::from_canonical_u64(instruction.store_offset as u64),
            set_pc: F::from_bool(instruction.set_pc),
            store_use_sp: F::from_bool(instruction.store_use_sp),
            set_sp: F::from_bool(instruction.set_sp),
            inc_sp: F::from_canonical_u64(instruction.inc_sp),
            jmp_offset1: F::from_canonical_u64(instruction.jmp_offset1 as u64),
            jmp_offset2: F::from_canonical_u64(instruction.jmp_offset2 as u64),
        };
        full_trace.steps.push(full_trace_step);
    }

    /// Gets the current values of the 32 registers
    pub fn get_regs_array(&self) -> [u64; 32] {
        let mut regs_array: [u64; 32] = [0; 32];
        for (i, reg) in regs_array.iter_mut().enumerate() {
            *reg = self.ctx.mem.read(SYS_ADDR + (i as u64) * 8, 8);
        }
        regs_array
    }

    /// Gets the log traces
    pub fn get_tracerv(&self) -> Vec<String> {
        self.ctx.tracerv.clone()
    }

    /// Returns if the emulation ended
    pub fn terminated(&self) -> bool {
        self.ctx.end
    }

    /// Returns the number of executed steps
    pub fn number_of_steps(&self) -> u64 {
        self.ctx.step
    }
}
