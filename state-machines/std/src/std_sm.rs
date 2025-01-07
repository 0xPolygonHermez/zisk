use std::sync::Arc;

use p3_field::PrimeField;
use pil_std_lib::Std;
use sm_common::{
    BusDeviceInstance, BusDeviceMetrics, ComponentBuilder, DummyCounter, InstanceCtx, Planner,
};

use crate::{StdInstance, StdPlanner};

pub struct StdSM<F: PrimeField> {
    /// PIL2 standard library
    std: Arc<Std<F>>,
}

impl<F: PrimeField> StdSM<F> {
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        Arc::new(Self { std: std.clone() })
    }
}

impl<F: PrimeField> ComponentBuilder<F> for StdSM<F> {
    fn build_counter(&self) -> Box<dyn BusDeviceMetrics> {
        Box::new(DummyCounter::default())
    }

    fn build_planner(&self) -> Box<dyn Planner> {
        Box::new(StdPlanner::new(self.std.clone()))
    }

    fn build_inputs_collector(&self, ictx: InstanceCtx) -> Box<dyn BusDeviceInstance<F>> {
        Box::new(StdInstance::new(self.std.clone(), ictx))
    }
}
