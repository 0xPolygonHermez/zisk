use std::ops::Add;

use crate::{Counter, Metrics};
use zisk_common::{BusDevice, DataBusMain, Opid};
use zisk_core::ZiskOperationType;

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
    fn measure(&mut self, opid: &Opid, data: &[u64]) -> Vec<(Opid, Vec<u64>)> {
        if *opid == 5000 {
            let data: &[u64; 8] = data.try_into().expect("Regular Metrics: Failed to convert data");
            let inst_op_type = DataBusMain::get_op_type(data);

            if inst_op_type == self.op_type as u64 {
                self.counter.update(1);
            }
        }

        vec![]
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

impl BusDevice<u64> for RegularCounter {
    #[inline]
    fn process_data(&mut self, opid: &Opid, data: &[u64]) -> Vec<(Opid, Vec<u64>)> {
        self.measure(opid, data);

        vec![]
    }
}
