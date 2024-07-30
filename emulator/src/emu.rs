use std::mem;

use crate::{EmuContext, EmuOptions, EmuTrace, MemTrace};
use riscv2zisk::{
    opcode_execute, RiscVRegisters, ZiskRom, OUTPUT_ADDR, ROM_ADDR, ROM_ENTRY, SRC_C, SRC_IMM,
    SRC_IND, SRC_MEM, SRC_SP, SRC_STEP, STORE_IND, STORE_MEM, STORE_NONE, SYS_ADDR,
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

        // Get registers
        //emu.get_regs(); // TODO: ask Jordi

        ctx
    }

    /// Performs one single step of the emulation
    #[inline(always)]
    pub fn step(&mut self, options: &EmuOptions, callback: &Option<Box<dyn Fn(Vec<EmuTrace>)>>) {
        // Check if we are tracing steps to improve the execution
        let tracing_steps = options.trace_steps.is_some();

        // Reset memory traces vector
        if tracing_steps {
            self.ctx.mem_trace.clear();
        }

        let instruction = if self.ctx.pc >= ROM_ADDR {
            &self.rom.rom_instructions[(self.ctx.pc - ROM_ADDR) as usize]
        } else if self.ctx.pc >= ROM_ENTRY {
            &self.rom.rom_entry_instructions[(self.ctx.pc - ROM_ENTRY) as usize]
        } else {
            self.ctx.end = true;
            return;
        };

        //println!("Emu::step() executing step={} pc={:x} inst={}", ctx.step, ctx.pc,
        // inst.i.to_string()); println!("Emu::step() step={} pc={}", ctx.step, ctx.pc);

        // If this is the last instruction, stop executing
        if instruction.end {
            self.ctx.end = true;
        }

        // Build the 'a' register value  based on the source specified by the current instruction
        match instruction.a_src {
            SRC_C => self.ctx.a = self.ctx.c,
            SRC_MEM => {
                let mut addr = instruction.a_offset_imm0;
                if instruction.a_use_sp_imm1 != 0 {
                    addr += self.ctx.sp;
                }
                self.ctx.a = self.ctx.mem.read(addr, 8);
                if tracing_steps {
                    let mem_trace = MemTrace::new(false, addr, 8, self.ctx.a);
                    self.ctx.mem_trace.push(mem_trace);
                }
            }
            SRC_IMM => self.ctx.a = instruction.a_offset_imm0 | (instruction.a_use_sp_imm1 << 32),
            SRC_STEP => self.ctx.a = self.ctx.step,
            SRC_SP => self.ctx.a = self.ctx.sp,
            _ => panic!("Emu::step() Invalid a_src={} pc={}", instruction.a_src, self.ctx.pc),
        }

        // Build the 'b' register value  based on the source specified by the current instruction
        match instruction.b_src {
            SRC_C => self.ctx.b = self.ctx.c,
            SRC_MEM => {
                let mut addr = instruction.b_offset_imm0;
                if instruction.b_use_sp_imm1 != 0 {
                    addr += self.ctx.sp;
                }
                self.ctx.b = self.ctx.mem.read(addr, 8);
                if tracing_steps {
                    let mem_trace = MemTrace::new(false, addr, 8, self.ctx.b);
                    self.ctx.mem_trace.push(mem_trace);
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
            _ => panic!("Emu::step() Invalid b_src={} pc={}", instruction.b_src, self.ctx.pc),
        }
        // Call the operation
        (self.ctx.c, self.ctx.flag) = opcode_execute(instruction.op, self.ctx.a, self.ctx.b);

        // Store the value of the c register based on the storage specified by the current
        // instruction
        match instruction.store {
            STORE_NONE => print!(""),
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
                    let mem_trace = MemTrace::new(true, addr as u64, 8, val as u64);
                    self.ctx.mem_trace.push(mem_trace);
                }
                //println!{"Emu::step() step={} pc={} writing to memory addr={} val={}", ctx.step,
                // ctx.pc, addr, val as u64};
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
                //println!{"Emu::step() step={} pc={} writing to memory addr={} val={}", ctx.step,
                // ctx.pc, addr, val as u64};
            }
            _ => panic!("Emu::step() Invalid store={} pc={}", instruction.store, self.ctx.pc),
        }

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
        if tracing_steps {
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

            if self.ctx.end ||
                ((self.ctx.step % options.trace_steps.unwrap()) ==
                    (options.trace_steps.unwrap() - 1))
            {
                if callback.is_none() {
                    panic!("Emu::step() found empty callback");
                }
                let callback = callback.as_ref().unwrap();
                let emu_trace = mem::take(&mut self.ctx.emu_trace);
                (callback)(emu_trace);
            }
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
        callback: Option<Box<dyn Fn(Vec<EmuTrace>)>>,
    ) {
        // Context, where the state of the execution is stored and modified at every execution step
        self.ctx = self.create_emu_context(inputs);

        // While not done
        while !self.ctx.end {
            if options.verbose {
                println!("Emu::run() step={} ctx.pc={}", self.ctx.step, self.ctx.pc);
            }
            // Check trace PC
            if self.ctx.tracerv_on && (self.ctx.pc % 4 == 0) {
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
            if self.ctx.tracerv_on && ((self.ctx.pc % 4) == 0) {
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

    /// Gets the current values of the 32 registers
    pub fn get_regs_array(&self) -> [u64; 32] {
        let mut regs_array: [u64; 32] = [0; 32];
        for (i, reg) in regs_array.iter_mut().enumerate() {
            *reg = self.ctx.mem.read(SYS_ADDR + (i as u64) * 8, 8);
        }
        regs_array
    }

    /// Enables the tracer, initializing the current registers to detect differences from now on
    pub fn tracerv_on(&mut self) {
        self.ctx.tracerv_current_regs = self.get_regs_array();
        self.ctx.tracerv_on = true;
    }

    /// Disables the tracer
    pub fn tracerv_off(&mut self) {
        self.ctx.tracerv_on = false;
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
