use p3_field::PrimeField;

use crate::{Plan, WitnessBuffer};

pub struct InstanceExpanderCtx<F: PrimeField> {
    pub buffer: WitnessBuffer<F>,
    pub plan: Plan,
    pub instance_global_idx: usize,
}

impl<F: PrimeField> InstanceExpanderCtx<F> {
    pub fn new(buffer: WitnessBuffer<F>, instance_global_idx: usize, plan: Plan) -> Self {
        Self { buffer, plan, instance_global_idx }
    }
}

unsafe impl<F: PrimeField> Send for InstanceExpanderCtx<F> {}
