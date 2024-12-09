use std::ops::Add;

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
        let other = other
            .as_any()
            .downcast_ref::<BinaryCounter>()
            .expect("Binary Metrics: Failed to downcast");
        self.binary += &other.binary;
        self.binary_extension += &other.binary_extension;
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Add for BinaryCounter {
    type Output = BinaryCounter;

    fn add(self, other: Self) -> BinaryCounter {
        BinaryCounter {
            binary: &self.binary + &other.binary,
            binary_extension: &self.binary_extension + &other.binary_extension,
        }
    }
}
