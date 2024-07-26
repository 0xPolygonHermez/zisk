use crate::{EmuTrace, Mem, MemTrace};
use riscv2zisk::{write_u64_le, INPUT_ADDR, MAX_INPUT_SIZE, RAM_ADDR, RAM_SIZE, ROM_ENTRY};

/// ZisK emulator context data container, storing the state of the emuulation
pub struct EmuContext {
    pub mem: Mem,
    pub a: u64,
    pub b: u64,
    pub c: u64,
    pub flag: bool,
    pub sp: u64,
    pub pc: u64,
    pub step: u64,
    pub end: bool,
    pub tracerv: Vec<String>,
    pub tracerv_on: bool,
    pub tracerv_step: u64,
    pub tracerv_current_regs: [u64; 32],
    pub trace_pc: u64,
    pub mem_trace: Vec<MemTrace>,
    pub emu_trace: Vec<EmuTrace>,
}

/// RisK emulator context implementation
impl EmuContext {
    /// RisK emulator context constructor
    pub fn new(input: Vec<u8>) -> EmuContext {
        let mut ctx = EmuContext {
            mem: Mem::new(),
            a: 0,
            b: 0,
            c: 0,
            flag: false,
            sp: 0,
            pc: ROM_ENTRY,
            step: 0,
            end: false,
            tracerv: Vec::new(),
            tracerv_on: false,
            tracerv_step: 0,
            tracerv_current_regs: [0; 32],
            trace_pc: 0,
            mem_trace: Vec::new(),
            emu_trace: Vec::new(),
        };

        // Check the input data size is inside the proper range
        if input.len() > (MAX_INPUT_SIZE - 8) as usize {
            panic!("EmuContext::new() input size too big size={}", input.len());
        }

        // Create a new empty vector
        let mut buffer: Vec<u8> = vec![0; 8];
        write_u64_le(&mut buffer, 0, input.len() as u64);

        // Add the length and input data read sections
        ctx.mem.add_read_section(INPUT_ADDR, &buffer);
        ctx.mem.add_read_section(INPUT_ADDR + 8, &input);

        // Add the write section
        ctx.mem.add_write_section(RAM_ADDR, RAM_SIZE);

        ctx
    }
}
