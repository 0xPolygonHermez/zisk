use proofman_common::WitnessPilout;
use sm_common::{LayoutPlanner, OutputPlan};
use zisk_core::{InstContext, ZiskInst, ZiskPcHistogram};

#[derive(Debug)]
pub struct RomPlan {
    pub details: String,
}

impl OutputPlan for RomPlan {}

#[derive(Default)]
pub struct RomPlanner {
    histogram: ZiskPcHistogram,
}

impl RomPlanner {
    pub fn new() -> Self {
        RomPlanner { histogram: ZiskPcHistogram::default() }
    }
}

impl LayoutPlanner for RomPlanner {
    fn new_session(&mut self, _: &WitnessPilout) {
        self.histogram = ZiskPcHistogram::default();
    }

    fn on_instruction(&mut self, instruction: &ZiskInst, inst_ctx: &InstContext) {
        let count = self.histogram.map.entry(inst_ctx.pc).or_default();
        *count += 1;

        if instruction.end {
            self.histogram.end_pc = inst_ctx.pc;
            self.histogram.steps = inst_ctx.step + 1;
        }
    }

    fn get_plan(&self) -> Box<dyn OutputPlan> {
        let mut sum = 0;
        for item in self.histogram.map.iter() {
            sum += item.0 * item.1;
        }
        let xxx =
            format!("pc_histogram: {} {} {}", sum, self.histogram.end_pc, self.histogram.steps);

        Box::new(RomPlan { details: xxx })
    }
}
