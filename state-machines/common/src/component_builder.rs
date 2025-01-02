use crate::{BusDeviceInstance, BusDeviceMetrics, InstanceCtx, Planner};
use p3_field::PrimeField;

pub trait ComponentBuilder<F: PrimeField>: Send + Sync {
    fn build_counter(&self) -> Box<dyn BusDeviceMetrics>;
    fn build_planner(&self) -> Box<dyn Planner>;
    fn build_inputs_collector(&self, iectx: InstanceCtx) -> Box<dyn BusDeviceInstance<F>>;
    fn build_inputs_generator(&self) -> Option<Box<dyn BusDeviceInstance<F>>> {
        None
    }
}
