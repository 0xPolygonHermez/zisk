use std::sync::Arc;

use p3_field::PrimeField;

use crate::{InstanceExpanderCtx, InstanceXXXX, Metrics, Planner};

pub trait ComponentProvider<F: PrimeField>: Send + Sync {
    fn get_counter(&self) -> Box<dyn Metrics>;
    fn get_planner(&self) -> Box<dyn Planner>;
    fn get_instance(self: Arc<Self>, iectx: InstanceExpanderCtx<F>) -> Box<dyn InstanceXXXX>;
}

// fn get_instance_expander_ctx(plan, global_idx, witness_buffer) -> Box<dyn InstanceSMCtx<F>
