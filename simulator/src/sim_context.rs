use crate::Mem;
use riscv2zisk::{INPUT_ADDR, MAX_INPUT_SIZE, RAM_ADDR, RAM_SIZE, ROM_ENTRY};

/// ZisK simulator context data container, storing the state of the simulation
pub struct SimContext {
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
}

/// RisK simulator context implementation
impl SimContext {
    /// RisK simulator context constructor
    pub fn new(input: Vec<u8>) -> SimContext {
        let mut ctx = SimContext {
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
        };

        // Check the input data size is inside the proper range
        if input.len() > (MAX_INPUT_SIZE - 8) as usize {
            panic!("SimContext::new() input size too big size={}", input.len());
        }

        // Create a new empty vector
        let buffer: Vec<u8> = vec![0; 8];

        // Add the length and input data read sections
        ctx.mem.add_read_section(INPUT_ADDR, &buffer);
        ctx.mem.add_read_section(INPUT_ADDR + 8, &input);

        // Add the write section
        ctx.mem.add_write_section(RAM_ADDR, RAM_SIZE);

        ctx
    }
}