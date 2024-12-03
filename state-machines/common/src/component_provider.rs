use std::sync::Arc;

use p3_field::PrimeField;
use ziskemu::EmuTrace;

use crate::{InstanceExpanderCtx, InstanceXXXX, Planner, Surveyor};

pub trait ComponentProvider<F: PrimeField>: Send + Sync {
    fn get_surveyor(&self) -> Box<dyn Surveyor>;
    fn get_planner(&self) -> Box<dyn Planner>;
    fn get_instance(self: Arc<Self>, iectx: InstanceExpanderCtx<F>) -> Box<dyn InstanceXXXX>;
}

// fn get_instance_expander_ctx(plan, global_idx, witness_buffer) -> Box<dyn InstanceSMCtx<F>