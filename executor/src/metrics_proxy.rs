use sm_common::Metrics;
use zisk_common::InstObserver;
use zisk_core::{InstContext, ZiskInst, ZiskOperationType, ZISK_OP_TYPE_COUNT};

#[derive(Default)]
pub struct MetricsProxy {
    pub metrics: Vec<Box<dyn Metrics>>,
    pub metrics_by_op: [Vec<usize>; ZISK_OP_TYPE_COUNT],
}

impl MetricsProxy {
    pub fn new() -> Self {
        let metrics_by_op: [Vec<_>; ZISK_OP_TYPE_COUNT] = std::array::from_fn(|_| Vec::new());

        Self { metrics: Vec::new(), metrics_by_op }
    }

    pub fn register_metrics(&mut self, observer: Box<dyn Metrics>) {
        let op_types = observer.op_type();
        self.metrics.push(observer);
        let idx = self.metrics.len() - 1;

        for op_type in op_types {
            self.metrics_by_op[op_type as usize].push(idx);
        }
    }
}

impl InstObserver for MetricsProxy {
    #[inline(always)]
    fn on_instruction(&mut self, zisk_inst: &ZiskInst, inst_ctx: &InstContext) -> bool {
        for op_type in [&zisk_inst.op_type, &ZiskOperationType::None] {
            for idx in self.metrics_by_op[*op_type as usize].iter() {
                self.metrics[*idx].measure(zisk_inst, inst_ctx);
            }
        }

        false
    }
}
