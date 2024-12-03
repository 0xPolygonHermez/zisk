mod component_provider;
mod expander;
mod operations;
mod planner;
mod planner_helpers;
mod provable;
mod state_machine;
mod surveyor;
mod witness_buffer;

pub use component_provider::*;
pub use expander::*;
pub use operations::*;
use p3_field::PrimeField;
pub use planner::*;
pub use planner_helpers::*;
use proofman_common::AirInstance;
pub use provable::*;
pub use state_machine::*;
pub use surveyor::*;
pub use witness_buffer::*;

pub struct InstanceExpanderCtx<F: PrimeField> {
    pub expander_idx: usize,
    pub buffer: WitnessBuffer<F>,
    pub segment_id: Option<usize>,
    pub instance_global_idx: usize,
    pub air_instance: Option<AirInstance<F>>,
    pub plan: Plan,
}

impl<F: PrimeField> InstanceExpanderCtx<F> {
    pub fn new(
        expander_idx: usize,
        buffer: WitnessBuffer<F>,
        segment_id: Option<usize>,
        instance_global_idx: usize,
        air_instance: Option<AirInstance<F>>,
        plan: Plan,
    ) -> Self {
        Self { expander_idx, buffer, instance_global_idx, segment_id, air_instance, plan }
    }
}

unsafe impl<F: PrimeField> Send for InstanceExpanderCtx<F> {}
