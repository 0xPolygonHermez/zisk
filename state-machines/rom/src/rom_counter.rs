use std::any::Any;

use sm_common::{CounterStats, Metrics};
use zisk_core::{InstContext, ZiskInst};

#[derive(Default)]
pub struct RomCounter {
    pub rom: CounterStats,
}

impl Metrics for RomCounter {
    fn measure(&mut self, _: &ZiskInst, inst_ctx: &InstContext) {
        self.rom.update(inst_ctx.pc, 1);
    }

    fn add(&mut self, other: &dyn Metrics) {
        if let Some(other) = other.as_any().downcast_ref::<RomCounter>() {
            for (k, v) in &other.rom.inst_count {
                let count = self.rom.inst_count.entry(*k).or_default();
                *count += *v;
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
