use crate::{BusDeviceInstance, BusDeviceMetrics, InstanceExpanderCtx, Planner};
use p3_field::PrimeField;

pub trait ComponentProvider<F: PrimeField>: Send + Sync {
    fn get_counter(&self) -> Box<dyn BusDeviceMetrics>;
    fn get_planner(&self) -> Box<dyn Planner>;
    fn get_instance(&self, iectx: InstanceExpanderCtx) -> Box<dyn BusDeviceInstance<F>>;
    fn get_inputs_generator(&self) -> Option<Box<dyn BusDeviceInstance<F>>> {
        None
    }
}
