use std::ops::Add;

use crate::{Counter, Metrics};
use zisk_core::{InstContext, ZiskInst, ZiskOperationType};

pub struct RegularCounter {
    op_type: ZiskOperationType,
    counter: Counter,
}

impl RegularCounter {
    pub fn new(op_type: ZiskOperationType) -> Self {
        Self { op_type, counter: Counter::default() }
    }

    pub fn inst_count(&self) -> u64 {
        self.counter.inst_count
    }
}

impl Metrics for RegularCounter {
    fn measure(&mut self, inst: &ZiskInst, _: &InstContext) {
        if inst.op_type == self.op_type {
            self.counter.update(1);
        }
    }

    fn add(&mut self, other: &dyn Metrics) {
        let other = other
            .as_any()
            .downcast_ref::<RegularCounter>()
            .expect("Regular Metrics: Failed to downcast");
        self.counter += &other.counter;
    }

    fn op_type(&self) -> Vec<ZiskOperationType> {
        vec![self.op_type]
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Add for RegularCounter {
    type Output = RegularCounter;

    fn add(self, other: Self) -> RegularCounter {
        RegularCounter { op_type: self.op_type, counter: &self.counter + &other.counter }
    }
}
