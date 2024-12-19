use std::any::Any;

use crate::Metrics;
use zisk_common::{BusDevice, BusId};

#[derive(Default)]
pub struct DummyCounter {}

impl Metrics for DummyCounter {
    fn measure(&mut self, _: &BusId, _: &[u64]) -> Vec<(BusId, Vec<u64>)> {
        vec![]
    }

    fn add(&mut self, _: &dyn Metrics) {}

    fn bus_id(&self) -> Vec<BusId> {
        vec![]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl BusDevice<u64> for DummyCounter {
    #[inline]
    fn process_data(&mut self, _: &BusId, _: &[u64]) -> (bool, Vec<(BusId, Vec<u64>)>) {
        (true, vec![])
    }
}
