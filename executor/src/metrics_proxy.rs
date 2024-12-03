use sm_common::Metrics;
use zisk_common::InstObserver;
use zisk_core::{InstContext, ZiskInst};

#[derive(Default)]
pub struct MetricsProxy {
    pub metrics: Vec<Box<dyn Metrics>>,
}

impl MetricsProxy {
    pub fn new() -> Self {
        Self { metrics: Vec::new() }
    }

    pub fn register_metrics(&mut self, observer: Box<dyn Metrics>) {
        self.metrics.push(observer);
    }
}

impl InstObserver for MetricsProxy {
    #[inline(always)]
    fn on_instruction(&mut self, zisk_inst: &ZiskInst, inst_ctx: &InstContext) -> bool {
        for observer in &mut self.metrics {
            (*observer).measure(zisk_inst, inst_ctx);
        }

        false
    }
}
