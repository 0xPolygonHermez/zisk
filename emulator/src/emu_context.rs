use crate::Stats;
use zisk_common::EmuTrace;
use zisk_core::{
    EmulationMode, FcallInstContext, InstContext, Mem, PrecompiledInstContext, INPUT_ADDR,
    MAX_INPUT_SIZE, RAM_ADDR, RAM_SIZE, REGS_IN_MAIN_TOTAL_NUMBER, ROM_ENTRY,
};

/// ZisK emulator context data container, storing the state of the emulation
pub struct EmuContext {
    pub inst_ctx: InstContext,

    pub tracerv: Vec<String>,
    pub tracerv_step: u64,
    pub tracerv_current_regs: [u64; REGS_IN_MAIN_TOTAL_NUMBER],
    pub trace_pc: u64,
    pub do_callback: bool,
    pub callback_steps: u64,
    pub last_callback_step: u64,
    pub trace: EmuTrace,
    pub do_stats: bool,
    pub stats: Stats,
}

/// RisK emulator context implementation
impl EmuContext {
    /// RisK emulator context constructor
    pub fn new(input: Vec<u8>) -> EmuContext {
        let mut ctx = EmuContext {
            inst_ctx: InstContext {
                mem: Mem::default(),
                a: 0,
                b: 0,
                c: 0,
                flag: false,
                sp: 0,
                pc: ROM_ENTRY,
                step: 0,
                end: false,
                regs: [0; REGS_IN_MAIN_TOTAL_NUMBER],
                emulation_mode: EmulationMode::default(),
                precompiled: PrecompiledInstContext::default(),
                fcall: FcallInstContext::default(),
            },
            tracerv: Vec::new(),
            tracerv_step: 0,
            tracerv_current_regs: [0; REGS_IN_MAIN_TOTAL_NUMBER],
            trace_pc: 0,
            trace: EmuTrace::default(),
            do_callback: false,
            callback_steps: 0,
            last_callback_step: 0,
            do_stats: false,
            stats: Stats::default(),
        };

        // Check the input data size is inside the proper range
        if input.len() > (MAX_INPUT_SIZE - 16) as usize {
            panic!("EmuContext::new() input size too big size={}", input.len());
        }

        // Add the length and input data read sections
        let input_len = input.len() as u64;
        let free_input = 0u64;
        ctx.inst_ctx.mem.add_read_section(INPUT_ADDR, &free_input.to_le_bytes());
        ctx.inst_ctx.mem.add_read_section(INPUT_ADDR + 8, &input_len.to_le_bytes());
        ctx.inst_ctx.mem.add_read_section(INPUT_ADDR + 16, &input);

        // Add the write section
        ctx.inst_ctx.mem.add_write_section(RAM_ADDR, RAM_SIZE);

        ctx
    }
}

impl Default for EmuContext {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
