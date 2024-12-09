use std::any::Any;

use sm_common::Metrics;
use zisk_core::{InstContext, ZiskInst};

#[derive(Default)]
pub struct StdCounter {}

impl Metrics for StdCounter {
    fn measure(&mut self, _: &ZiskInst, _: &InstContext) {}

    fn add(&mut self, _: &dyn Metrics) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}
