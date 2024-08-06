use std::mem;

use crate::{EmuContext, EmuOptions, EmuTrace, MemTrace};
use riscv::RiscVRegisters;
use zisk_core::{
    ZiskInst, ZiskRom, OUTPUT_ADDR, SRC_C, SRC_IMM, SRC_IND, SRC_MEM, SRC_SP, SRC_STEP, STORE_IND,
    STORE_MEM, STORE_NONE, SYS_ADDR,
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

    /// Calculate a based on a source
    #[inline(always)]
    pub fn source_a(&mut self, instruction: &ZiskInst, tracing_steps: bool) {
        match instruction.a_src {
            SRC_C => self.ctx.a = self.ctx.c,
            SRC_MEM => {
                let mut addr = instruction.a_offset_imm0;
                if instruction.a_use_sp_imm1 != 0 {
                    addr += self.ctx.sp;
                }
                self.ctx.a = self.ctx.mem.read(addr, 8);
                if tracing_steps {
                    self.ctx.mem_trace.push(MemTrace::new(false, addr, 8, self.ctx.a));
                }
            }
            SRC_IMM => self.ctx.a = instruction.a_offset_imm0 | (instruction.a_use_sp_imm1 << 32),
            SRC_STEP => self.ctx.a = self.ctx.step,
            SRC_SP => self.ctx.a = self.ctx.sp,
            _ => panic!("Emu::source_a() Invalid a_src={} pc={}", instruction.a_src, self.ctx.pc),
        }
    }

    /// Calculate b based on b source
    #[inline(always)]
    pub fn source_b(&mut self, instruction: &ZiskInst, tracing_steps: bool) {
        match instruction.b_src {
            SRC_C => self.ctx.b = self.ctx.c,
            SRC_MEM => {
                let mut addr = instruction.b_offset_imm0;
                if instruction.b_use_sp_imm1 != 0 {
                    addr += self.ctx.sp;
                }
                self.ctx.b = self.ctx.mem.read(addr, 8);
                if tracing_steps {
                    self.ctx.mem_trace.push(MemTrace::new(false, addr, 8, self.ctx.b));
                }
            }
            SRC_IMM => self.ctx.b = instruction.b_offset_imm0 | (instruction.b_use_sp_imm1 << 32),
            SRC_IND => {
                let mut addr = (self.ctx.a as i64 + instruction.b_offset_imm0 as i64) as u64;
                if instruction.b_use_sp_imm1 != 0 {
                    addr += self.ctx.sp;
                }
                self.ctx.b = self.ctx.mem.read(addr, instruction.ind_width);
            }
            _ => panic!("Emu::source_b() Invalid b_src={} pc={}", instruction.b_src, self.ctx.pc),
        }
    }

    /// Store c based on c storage
    #[inline(always)]
    pub fn store_c(&mut self, instruction: &ZiskInst, tracing_steps: bool) {
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
                if tracing_steps {
                    self.ctx.mem_trace.push(MemTrace::new(true, addr as u64, 8, val as u64));
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
            }
            _ => panic!("Emu::store_c() Invalid store={} pc={}", instruction.store, self.ctx.pc),
        }
    }

    /// Performs one single step of the emulation
    #[inline(always)]
    pub fn step(&mut self, options: &EmuOptions, callback: &Option<impl Fn(Vec<EmuTrace>)>) {
        // Check if we are tracing steps to improve the execution
        let tracing_steps = options.trace_steps.is_some();

        // Reset memory traces vector
        if self.ctx.do_callback {
            self.ctx.mem_trace.clear();
        }

        let instruction = self.rom.get_instruction(self.ctx.pc);

        //println!("Emu::step() executing step={} pc={:x} inst={}", ctx.step, ctx.pc,
        // inst.i.to_string()); println!("Emu::step() step={} pc={}", ctx.step, ctx.pc);

        // If this is the last instruction, stop executing
        self.ctx.end = instruction.end;

        // Build the 'a' register value  based on the source specified by the current instruction
        self.source_a(instruction, tracing_steps);

        // Build the 'b' register value  based on the source specified by the current instruction
        self.source_b(instruction, tracing_steps);

        // Call the operation
        (self.ctx.c, self.ctx.flag) = (instruction.func)(self.ctx.a, self.ctx.b);

        // Store the 'c' register value based on the storage specified by the current instruction
        self.store_c(instruction, tracing_steps);

        // Set SP, if specified by the current instruction
        if instruction.set_sp {
            self.ctx.sp = self.ctx.c;
        } else {
            self.ctx.sp += instruction.inc_sp;
        }

        // Set PC, based on current PC, current flag and current instruction
        if instruction.set_pc {
            self.ctx.pc = (self.ctx.c as i64 + instruction.jmp_offset1) as u64;
        } else if self.ctx.flag {
            self.ctx.pc = (self.ctx.pc as i64 + instruction.jmp_offset1) as u64;
        } else {
            self.ctx.pc = (self.ctx.pc as i64 + instruction.jmp_offset2) as u64;
        }

        // Log the step, if requested
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
            let emu_trace = EmuTrace {
                opcode: instruction.op,
                a: self.ctx.a,
                b: self.ctx.b,
                c: self.ctx.c,
                flag: self.ctx.flag,
                sp: self.ctx.sp,
                pc: self.ctx.pc,
                step: self.ctx.step,
                end: self.ctx.end,
                mem_trace: mem::take(&mut self.ctx.mem_trace),
            };

            self.ctx.emu_trace.push(emu_trace);

            // Increment step counter
            self.ctx.step += 1;

            if self.ctx.end ||
                ((self.ctx.step - self.ctx.last_callback_step) == self.ctx.callback_steps)
            {
                if callback.is_none() {
                    panic!("Emu::step() found empty callback");
                }
                let callback = callback.as_ref().expect("Emu::step() found empty callback");

                let emu_trace = mem::take(&mut self.ctx.emu_trace);
                (callback)(emu_trace);
                self.ctx.last_callback_step += self.ctx.callback_steps;
            }
        } else {
            // Increment step counter
            self.ctx.step += 1;
        }
    }

    /// Performs one single step of the emulation
    #[inline(always)]
    pub fn step_fast(&mut self) {
        // Get the instruction for this pc
        let instruction = self.rom.get_instruction(self.ctx.pc);

        //println!("Emu::step_fast() executing step={} pc={:x} inst={}", ctx.step, ctx.pc,
        // inst.i.to_string()); println!("Emu::step() step={} pc={}", ctx.step, ctx.pc);

        // If this is the last instruction, stop executing
        self.ctx.end = instruction.end;

        // Build the 'a' register value  based on the source specified by the current instruction
        self.source_a(instruction, false);

        // Build the 'b' register value  based on the source specified by the current instruction
        self.source_b(instruction, false);

        // Call the operation
        (self.ctx.c, self.ctx.flag) = (instruction.func)(self.ctx.a, self.ctx.b);

        // Store the 'c' register value based on the storage specified by the current instruction
        self.store_c(instruction, false);

        // Set SP, if specified by the current instruction
        if instruction.set_sp {
            self.ctx.sp = self.ctx.c;
        } else {
            self.ctx.sp += instruction.inc_sp;
        }

        // Set PC, based on current PC, current flag and current instruction
        if instruction.set_pc {
            self.ctx.pc = (self.ctx.c as i64 + instruction.jmp_offset1) as u64;
        } else if self.ctx.flag {
            self.ctx.pc = (self.ctx.pc as i64 + instruction.jmp_offset1) as u64;
        } else {
            self.ctx.pc = (self.ctx.pc as i64 + instruction.jmp_offset2) as u64;
        }

        // Increment step counter
        self.ctx.step += 1;
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
        callback: Option<impl Fn(Vec<EmuTrace>)>,
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
        }

        // Call run_fast if only essential work is needed
        if options.is_fast() {
            return self.run_fast(options);
        }
        //println!("Emu::run() full-equipe");

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
    }

    /// Run the whole program, fast
    #[inline(always)]
    pub fn run_fast(&mut self, options: &EmuOptions) {
        // While not ended and not reached the maximum number of steps, call step_fast
        while !self.ctx.end && (self.ctx.step < options.max_steps) {
            self.step_fast();
        }
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
