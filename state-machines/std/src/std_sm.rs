use std::sync::Arc;

use p3_field::PrimeField;
use pil_std_lib::Std;
use sm_common::{
    BusDeviceInstance, BusDeviceMetrics, ComponentProvider, DummyCounter, InstanceCtx, Planner,
};

use crate::{StdInstance, StdPlanner};

pub struct StdSM<F: PrimeField> {
    std: Arc<Std<F>>,
}

impl<F: PrimeField> StdSM<F> {
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        Arc::new(Self { std: std.clone() })
    }
}

impl<F: PrimeField> ComponentProvider<F> for StdSM<F> {
    fn get_counter(&self) -> Box<dyn BusDeviceMetrics> {
        Box::new(DummyCounter::default())
    }

    fn get_planner(&self) -> Box<dyn Planner> {
        Box::new(StdPlanner::new(self.std.clone()))
    }

    fn get_instance(&self, iectx: InstanceCtx) -> Box<dyn BusDeviceInstance<F>> {
        Box::new(StdInstance::new(self.std.clone(), iectx))
    }
}
