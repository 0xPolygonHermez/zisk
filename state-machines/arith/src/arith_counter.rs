use std::ops::Add;

use sm_common::{Counter, Metrics};
use zisk_core::{InstContext, ZiskInst, ZiskOperationType};

#[derive(Default)]
pub struct ArithCounter {
    pub arith: Counter,
}

impl Metrics for ArithCounter {
    fn measure(&mut self, inst: &ZiskInst, _: &InstContext) {
        if inst.op_type == ZiskOperationType::Arith {
            self.arith.update(1);
        }
    }

    fn add(&mut self, other: &dyn Metrics) {
        let other = other
            .as_any()
            .downcast_ref::<ArithCounter>()
            .expect("Arith Metrics: Failed to downcast");
        self.arith += &other.arith;
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Add for ArithCounter {
    type Output = ArithCounter;

    fn add(self, other: Self) -> ArithCounter {
        ArithCounter { arith: &self.arith + &other.arith }
    }
}
