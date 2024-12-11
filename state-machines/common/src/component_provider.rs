use p3_field::PrimeField;

use crate::{Instance, InstanceExpanderCtx, Metrics, Planner};

pub trait ComponentProvider<F: PrimeField>: Send + Sync {
    fn get_counter(&self) -> Box<dyn Metrics>;
    fn get_planner(&self) -> Box<dyn Planner>;
    fn get_instance(&self, iectx: InstanceExpanderCtx) -> Box<dyn Instance<F>>;
}
