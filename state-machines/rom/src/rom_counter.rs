use std::any::Any;

use sm_common::{CounterStats, Metrics};
use zisk_core::{InstContext, ZiskInst, ZiskOperationType};

#[derive(Default)]
pub struct RomCounter {
    pub rom: CounterStats,
    pub end_pc: u64,
    pub steps: u64,
}

impl Metrics for RomCounter {
    fn measure(&mut self, inst: &ZiskInst, inst_ctx: &InstContext) {
        self.rom.update(inst_ctx.pc, 1);
        if inst.end {
            self.end_pc = inst_ctx.pc;
            self.steps = inst_ctx.step + 1;
        }
    }

    fn add(&mut self, other: &dyn Metrics) {
        let other =
            other.as_any().downcast_ref::<RomCounter>().expect("Rom Metrics: Failed to downcast");
        self.rom += &other.rom;

        if other.end_pc != 0 {
            self.end_pc = other.end_pc;
        }

        if other.steps != 0 {
            self.steps = other.steps;
        }
    }

    fn op_type(&self) -> Vec<ZiskOperationType> {
        vec![ZiskOperationType::None]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
