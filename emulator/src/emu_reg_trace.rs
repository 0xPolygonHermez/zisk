use zisk_core::{REGS_IN_MAIN, REGS_IN_MAIN_FROM, REGS_IN_MAIN_TO};

// TODO: REMOVE THIS !!!
pub fn main_step_to_mem_step(step: u64, step_offset: u8) -> u64 {
    1 + 4 * step + step_offset as u64
}

#[derive(Debug, Clone, Copy)]
pub struct EmuRegTrace {
    pub reg_steps: [u64; REGS_IN_MAIN],
    pub reg_prev_steps: [u64; 3],
    pub reg_step_ranges: [u32; 3],
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
            reg_step_ranges: [0; 3],
            store_reg_prev_value: 0,
            first_step_uses: [if init_as_first_uses { Some(init_step) } else { None };
                REGS_IN_MAIN],
        }
    }
    pub fn clear_reg_step_ranges(&mut self) {
        self.reg_step_ranges = [0; 3];
        self.reg_prev_steps = [0; 3];
        self.store_reg_prev_value = 0;
    }
    pub fn update_step_range_check(&self, step_range_check: &mut [u32]) {
        for range in self.reg_step_ranges.iter() {
            // 0 isn't a valid range value, 0 is used to mark as no range
            if *range == 0 {
                continue;
            }
            assert!(*range as usize <= step_range_check.len());
            step_range_check[(*range - 1) as usize] += 1;
        }
    }
    pub fn trace_reg_access(&mut self, reg: usize, step: u64, slot: u8) {
        debug_assert!((REGS_IN_MAIN_FROM..=REGS_IN_MAIN_TO).contains(&reg) && slot < 3);
        let ireg = reg - REGS_IN_MAIN_FROM;
        // registry information about use to update later
        let first_reference = self.first_step_uses[ireg].is_none();
        if first_reference {
            self.first_step_uses[ireg] = Some(main_step_to_mem_step(step, slot));
        }

        let current_reg_step = main_step_to_mem_step(step, slot);
        let prev_reg_step = self.reg_steps[ireg];

        // first reference is incorrect because in this point we don't known what is the previous
        // register step, later we update this information and count the range check.
        if !first_reference {
            self.reg_step_ranges[slot as usize] = (current_reg_step - prev_reg_step) as u32;
        }
        self.reg_prev_steps[slot as usize] = prev_reg_step;
        self.reg_steps[ireg] = current_reg_step;
    }
}
impl Default for EmuRegTrace {
    fn default() -> Self {
        Self::new()
    }
}
