use zisk_core::{REGS_IN_MAIN, REGS_IN_MAIN_FROM, REGS_IN_MAIN_TO};

// TODO: REMOVE THIS !!!
pub fn main_step_to_mem_step(step: u64, step_offset: u8) -> u64 {
    1 + 4 * step + step_offset as u64
}

#[derive(Debug, Clone, Copy)]
pub struct EmuRegTrace {
    pub reg_steps: [u64; REGS_IN_MAIN],
    pub reg_prev_steps: [u64; 3],
    pub store_reg_prev_value: u64,
    pub first_step_uses: [Option<u64>; REGS_IN_MAIN],
}

impl EmuRegTrace {
    pub fn new() -> Self {
        Self::from_init_step(0, false)
    }
    pub fn from_init_step(init_step: u64, init_as_first_uses: bool) -> Self {
        Self {
            reg_steps: [init_step; REGS_IN_MAIN],
            reg_prev_steps: [0; 3],
            store_reg_prev_value: 0,
            first_step_uses: [if init_as_first_uses { Some(init_step) } else { None };
                REGS_IN_MAIN],
        }
    }
    pub fn clear_reg_prev_steps(&mut self) {
        self.reg_prev_steps = [0; 3];
        self.store_reg_prev_value = 0;
    }
    pub fn trace_reg_access(&mut self, reg: usize, step: u64, slot: u8) {
        debug_assert!((REGS_IN_MAIN_FROM..=REGS_IN_MAIN_TO).contains(&reg) && slot < 3);
        let ireg = reg - REGS_IN_MAIN_FROM;
        let current_reg_step = main_step_to_mem_step(step, slot);
        // First use in this chunk: the previous step is only known once the chunks are chained,
        // which is where the caller picks this up.
        if self.first_step_uses[ireg].is_none() {
            self.first_step_uses[ireg] = Some(current_reg_step);
        }
        self.reg_prev_steps[slot as usize] = self.reg_steps[ireg];
        self.reg_steps[ireg] = current_reg_step;
    }
}
impl Default for EmuRegTrace {
    fn default() -> Self {
        Self::new()
    }
}
