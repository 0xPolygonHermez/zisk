use goldilocks::AbstractField;

use proofman::provers_manager::{Prover, ProverBuilder};
use crate::estark_prover_settings::EStarkProverSettings;
use crate::estark_prover::EStarkProver;

pub struct EStarkProverBuilder<T> {
    config: EStarkProverSettings,
    p_steps: *mut std::os::raw::c_void,
    ptr: *mut std::os::raw::c_void,
    phantom: std::marker::PhantomData<T>,
}

impl<T> EStarkProverBuilder<T> {
    pub fn new(
        config: EStarkProverSettings,
        p_steps: *mut std::os::raw::c_void,
        ptr: *mut std::os::raw::c_void,
    ) -> Self {
        Self { config, p_steps, ptr, phantom: std::marker::PhantomData }
    }
}

impl<T: 'static + AbstractField> ProverBuilder<T> for EStarkProverBuilder<T> {
    fn build(&mut self) -> Box<dyn Prover<T>> {
        Box::new(EStarkProver::new(self.config.clone(), self.p_steps, self.ptr))
    }
}

