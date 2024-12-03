use sm_common::{Counter, Metrics};
use zisk_core::{InstContext, ZiskInst, ZiskOperationType};

#[derive(Default)]
pub struct BinaryCounter {
    pub binary: Counter,
    pub binary_extension: Counter,
}

impl Metrics for BinaryCounter {
    fn measure(&mut self, inst: &ZiskInst, _: &InstContext) {
        match inst.op_type {
            ZiskOperationType::Binary => {
                self.binary.update(1);
            }
            ZiskOperationType::BinaryE => {
                self.binary_extension.update(1);
            }
            _ => {}
        }
    }

    fn add(&mut self, other: &dyn Metrics) {
        if let Some(other) = other.as_any().downcast_ref::<BinaryCounter>() {
            self.binary.update(other.binary.inst_count);
            self.binary_extension.update(other.binary_extension.inst_count);
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
