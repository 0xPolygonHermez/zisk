use crate::{EmuContext, EmuOptions};
use riscv2zisk::{
    ZiskOperations, ZiskRom, OUTPUT_ADDR, SRC_C, SRC_IMM, SRC_IND, SRC_MEM, SRC_SP, SRC_STEP,
    STORE_IND, STORE_MEM, STORE_NONE, SYS_ADDR,
};
use std::collections::HashMap;

/// Human-readable names of the 32 well-known RISCV registers, to be used in traces
const REG_NAMES: [&str; 32] = [
    "zero", "ra", "sp", "gp", "tp", "t0", "t1", "t2", "s0", "s1", "a0", "a1", "a2", "a3", "a4",
    "a5", "a6", "a7", "s2", "s3", "s4", "s5", "s6", "s7", "s8", "s9", "s10", "s11", "t3", "t4",
    "t5", "t6",
];

/// ZisK emulator structure, containing the ZisK rom, the list of ZisK operations, and the
/// execution context
pub struct Emu<'a> {
    /// ZisK rom, containing the program to execute, which is constant for this program except for
    /// the input data
    pub rom: &'a ZiskRom,

    /// ZisK operations (c, flag) = f(a, b), one per supported opcode
    operations: ZiskOperations,

    /// Context, where the state of the execution is stored and modified at every execution step
    ctx: EmuContext,

    /// Emulator options
    options: EmuOptions,
}

/// ZisK emulator structure implementation
impl Emu<'_> {
    //// ZisK emulator structure constructor
    pub fn new(rom: &ZiskRom, input: Vec<u8>, options: EmuOptions) -> Emu {
        // Initialize an empty instance
        let mut emu =
            Emu { ctx: EmuContext::new(input), operations: ZiskOperations::new(), rom, options };

        // Create a new read section for every RO data entry of the rom
        for i in 0..emu.rom.ro_data.len() {
            emu.ctx.mem.add_read_section(emu.rom.ro_data[i].from, &emu.rom.ro_data[i].data);
        }

        // Get registers
        //emu.get_regs(); // TODO: ask Jordi

        emu
    }

    /// Performs one single step of the emulation
    pub fn step(&mut self) {
        // Get a mutable reference to the emulation context
        let ctx = &mut self.ctx;

        // Get the ZisK instruction corresponding to the current program counter
        if !self.rom.insts.contains_key(&ctx.pc) {
            panic!("Emu::step() cound not find a rom instruction for pc={}={:x}", ctx.pc, ctx.pc);
        }
        let inst = &self.rom.insts[&ctx.pc];

        //println!("Emu::step() executing step={} pc={:x} inst={}", ctx.step, ctx.pc,
        // inst.i.to_string()); println!("Emu::step() step={} pc={}", ctx.step, ctx.pc);

        // If this is the last instruction, stop executing
        if inst.i.end == 1 {
            ctx.end = true;
        }

        // Build the value of the a register based on the source specified by the current
        // instruction
        match inst.i.a_src {
            SRC_C => ctx.a = ctx.c,
            SRC_MEM => {
                let mut addr = inst.i.a_offset_imm0;
                if inst.i.a_use_sp_imm1 != 0 {
                    addr += ctx.sp;
                }
                ctx.a = ctx.mem.read(addr, 8);
            }
            SRC_IMM => ctx.a = inst.i.a_offset_imm0 | (inst.i.a_use_sp_imm1 << 32),
            SRC_STEP => ctx.a = ctx.step,
            SRC_SP => ctx.a = ctx.sp,
            _ => panic!("Emu::step() Invalid a_src={} pc={}", inst.i.a_src, ctx.pc),
        }

        // Build the value of the b register based on the source specified by the current
        // instruction
        match inst.i.b_src {
            SRC_C => ctx.b = ctx.c,
            SRC_MEM => {
                let mut addr = inst.i.b_offset_imm0;
                if inst.i.b_use_sp_imm1 != 0 {
                    addr += ctx.sp;
                }
                ctx.b = ctx.mem.read(addr, 8);
            }
            SRC_IMM => ctx.b = inst.i.b_offset_imm0 | (inst.i.b_use_sp_imm1 << 32),
            SRC_IND => {
                let mut addr = (ctx.a as i64 + inst.i.b_offset_imm0 as i64) as u64;
                if inst.i.b_use_sp_imm1 != 0 {
                    addr += ctx.sp;
                }
                ctx.b = ctx.mem.read(addr, inst.i.ind_width);
            }
            _ => panic!("Emu::step() Invalid b_src={} pc={}", inst.i.b_src, ctx.pc),
        }

        // Check the instruction opcode range
        if inst.i.op > 0xFF {
            panic!("Emu::step() invalid opcode={}", inst.i.op);
        }

        // Get the ZisK operation for this opcode
        let operation = self.operations.op_from_code.get(&(inst.i.op as u8)).unwrap();

        // Call the operation
        (ctx.c, ctx.flag) = (operation.f)(ctx.a, ctx.b);

        // Store the value of the c register based on the storage specified by the current
        // instruction
        match inst.i.store {
            STORE_NONE => print!(""),
            STORE_MEM => {
                let val: i64 = if inst.i.store_ra != 0 {
                    ctx.pc as i64 + inst.i.jmp_offset2
                } else {
                    ctx.c as i64
                };
                let mut addr: i64 = inst.i.store_offset;
                if inst.i.store_use_sp != 0 {
                    addr += ctx.sp as i64;
                }
                ctx.mem.write(addr as u64, val as u64, 8);
                //println!{"Emu::step() step={} pc={} writing to memory addr={} val={}", ctx.step,
                // ctx.pc, addr, val as u64};
            }
            STORE_IND => {
                let val: i64 = if inst.i.store_ra != 0 {
                    ctx.pc as i64 + inst.i.jmp_offset2
                } else {
                    ctx.c as i64
                };
                let mut addr = inst.i.store_offset;
                if inst.i.store_use_sp != 0 {
                    addr += ctx.sp as i64;
                }
                addr += ctx.a as i64;
                ctx.mem.write(addr as u64, val as u64, inst.i.ind_width);
                //println!{"Emu::step() step={} pc={} writing to memory addr={} val={}", ctx.step,
                // ctx.pc, addr, val as u64};
            }
            _ => panic!("Emu::step() Invalid store={} pc={}", inst.i.store, ctx.pc),
        }

        // Set SP, if specified by the current instruction
        if inst.i.set_sp != 0 {
            ctx.sp = ctx.c;
        } else {
            ctx.sp += inst.i.inc_sp;
        }

        // Set PC, based on current PC, current flag and current instruction
        if inst.i.set_pc != 0 {
            ctx.pc = (ctx.c as i64 + inst.i.jmp_offset1) as u64;
        } else if ctx.flag {
            ctx.pc = (ctx.pc as i64 + inst.i.jmp_offset1) as u64;
        } else {
            ctx.pc = (ctx.pc as i64 + inst.i.jmp_offset2) as u64;
        }

        // Log the step, if requested
        if self.options.log_step {
            println!(
                "step={} pc={} op={}={} a={} b={} c={} flag={}",
                ctx.step, ctx.pc, inst.i.op, inst.i.op_str, ctx.a, ctx.b, ctx.c, ctx.flag
            );
        }
        // Increment step counter
        ctx.step += 1;
    }

    /// Get the output as a vector of u64
    pub fn get_output(&self) -> Vec<u64> {
        let ctx = &self.ctx;
        let n = ctx.mem.read(OUTPUT_ADDR, 8);
        let mut addr = OUTPUT_ADDR + 8;
        let mut output: Vec<u64> = Vec::new();
        for _i in 0..n {
            output.push(ctx.mem.read(addr, 8));
            addr += 8;
        }
        output
    }

    /// Get the output as a vector of u32
    pub fn get_output_32(&self) -> Vec<u32> {
        let ctx = &self.ctx;
        let n = ctx.mem.read(OUTPUT_ADDR, 4);
        let mut addr = OUTPUT_ADDR + 4;
        let mut output: Vec<u32> = Vec::new();
        for _i in 0..n {
            output.push(ctx.mem.read(addr, 4) as u32);
            addr += 4;
        }
        output
    }

    /// Get the output as a vector of u8
    pub fn get_output_8(&self) -> Vec<u8> {
        let ctx = &self.ctx;
        let n = ctx.mem.read(OUTPUT_ADDR, 4);
        let mut addr = OUTPUT_ADDR + 4;
        let mut output: Vec<u8> = Vec::new();
        for _i in 0..n {
            output.push(ctx.mem.read(addr, 1) as u8);
            output.push(ctx.mem.read(addr + 1, 1) as u8);
            output.push(ctx.mem.read(addr + 2, 1) as u8);
            output.push(ctx.mem.read(addr + 3, 1) as u8);
            addr += 4;
        }
        output
    }

    /// Run the whole program
    pub fn run(&mut self) {
        // While not done
        while !self.ctx.end {
            if self.options.verbose {
                println!("Emu::run() step={} ctx.pc={}", self.ctx.step, self.ctx.pc);
            }
            // Check trace PC
            if self.ctx.tracerv_on && (self.ctx.pc % 4 == 0) {
                self.ctx.trace_pc = self.ctx.pc;
            }

            // Log emulation step, if requested
            if self.options.print_step.is_some() &&
                (self.options.print_step.unwrap() != 0) &&
                ((self.ctx.step % self.options.print_step.unwrap()) == 0)
            {
                println!("step={}", self.ctx.step);
            }

            // Stop the execution if we exceeded the specified running conditions
            if self.ctx.step >= self.options.max_steps {
                break;
            }

            // Execute the current step
            self.step();

            // Only trace after finishing a riscV instruction
            if self.ctx.tracerv_on && ((self.ctx.pc % 4) == 0) {
                // Store logs in a vector of strings
                let mut changes: Vec<String> = Vec::new();

                // Get the current state of the registers
                let new_regs_array = self.get_regs_array();

                // For all current registers
                for i in 0..new_regs_array.len() {
                    // If they changed since previous stem, add them to the logs
                    if new_regs_array[i] != self.ctx.tracerv_current_regs[i] {
                        changes.push(format!("{}={:x}", REG_NAMES[i], new_regs_array[i]));
                        self.ctx.tracerv_current_regs[i] = new_regs_array[i];
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

            //println!("Emu::run() done ctx.pc={}", self.ctx.pc); // 2147483828
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

    /// Gets the current values of the 32 registers, mapped to their corresponding register name
    pub fn get_regs(&self) -> HashMap<&str, u64> {
        let regs_array = self.get_regs_array();
        let mut reg_values: HashMap<&str, u64> = HashMap::new();
        for i in 0..32 {
            reg_values.insert(REG_NAMES[i], regs_array[i]);
        }
        reg_values
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
}
