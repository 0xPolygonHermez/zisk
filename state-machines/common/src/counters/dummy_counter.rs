use std::any::Any;

use crate::Metrics;
use zisk_core::{InstContext, ZiskInst, ZiskOperationType};

#[derive(Default)]
pub struct DummyCounter {}

impl Metrics for DummyCounter {
    fn measure(&mut self, _: &ZiskInst, _: &InstContext) {}

    fn add(&mut self, _: &dyn Metrics) {}

    fn op_type(&self) -> Vec<ZiskOperationType> {
        vec![]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
