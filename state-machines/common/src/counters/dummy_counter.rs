use std::any::Any;

use crate::Metrics;
use zisk_common::{BusDevice, Opid};
use zisk_core::ZiskOperationType;

#[derive(Default)]
pub struct DummyCounter {}

impl Metrics for DummyCounter {
    fn measure(&mut self, _: &Opid, _: &[u64]) -> Vec<(Opid, Vec<u64>)> {
        vec![]
    }

    fn add(&mut self, _: &dyn Metrics) {}

    fn op_type(&self) -> Vec<ZiskOperationType> {
        vec![]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl BusDevice<u64> for DummyCounter {
    #[inline]
    fn process_data(&mut self, _: &Opid, _: &[u64]) -> Vec<(Opid, Vec<u64>)> {
        vec![]
    }
}
