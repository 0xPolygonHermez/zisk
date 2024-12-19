use std::ops::Add;

use crate::{Counter, Metrics};
use zisk_common::{BusDevice, BusId, OperationBusData, OperationData};
use zisk_core::ZiskOperationType;

pub struct RegularCounters {
    op_type: Vec<ZiskOperationType>,
    bus_id: BusId,
    counter: Vec<Counter>,
}

impl RegularCounters {
    pub fn new(bus_id: BusId, op_type: Vec<ZiskOperationType>) -> Self {
        let counter = vec![Counter::default(); op_type.len()];
        Self { bus_id, op_type, counter }
    }

    pub fn inst_count(&self, op_type: ZiskOperationType) -> Option<u64> {
        if let Some(index) = self.op_type.iter().position(|&_op_type| op_type == _op_type) {
            return Some(self.counter[index].inst_count);
        }
        None
    }
}

impl Metrics for RegularCounters {
    fn measure(&mut self, _: &BusId, data: &[u64]) -> Vec<(BusId, Vec<u64>)> {
        let data: OperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");
        let inst_op_type = OperationBusData::get_op_type(&data);
        if let Some(index) = self.op_type.iter().position(|&op_type| op_type as u64 == inst_op_type)
        {
            self.counter[index].update(1);
        }

        vec![]
    }

    fn add(&mut self, other: &dyn Metrics) {
        let other = other
            .as_any()
            .downcast_ref::<RegularCounters>()
            .expect("Regular Metrics: Failed to downcast");
        for (counter, other_counter) in self.counter.iter_mut().zip(other.counter.iter()) {
            *counter += other_counter;
        }
    }

    fn bus_id(&self) -> Vec<BusId> {
        vec![self.bus_id]
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Add for RegularCounters {
    type Output = RegularCounters;

    fn add(self, other: Self) -> RegularCounters {
        let counter = self
            .counter
            .into_iter()
            .zip(other.counter)
            .map(|(counter, other_counter)| &counter + &other_counter)
            .collect();
        RegularCounters { bus_id: self.bus_id, op_type: self.op_type, counter }
    }
}

impl BusDevice<u64> for RegularCounters {
    #[inline]
    fn process_data(&mut self, bus_id: &BusId, data: &[u64]) -> (bool, Vec<(BusId, Vec<u64>)>) {
        self.measure(bus_id, data);

        (true, vec![])
    }
}
