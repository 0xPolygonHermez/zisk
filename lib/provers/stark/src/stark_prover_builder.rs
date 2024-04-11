use goldilocks::AbstractField;

use proofman::provers_manager::{Prover, ProverBuilder};
use crate::stark_prover_settings::StarkProverSettings;
use crate::stark_prover::StarkProver;

pub struct StarkProverBuilder<T> {
    config: StarkProverSettings,
    p_starkinfo: *mut std::os::raw::c_void,
    p_chelpers: *mut std::os::raw::c_void,
    p_steps: *mut std::os::raw::c_void,
    ptr: *mut std::os::raw::c_void,
    phantom: std::marker::PhantomData<T>,
}

impl<T> StarkProverBuilder<T> {
    pub fn new(
        config: StarkProverSettings,
        p_starkinfo: *mut std::os::raw::c_void,
        p_chelpers: *mut std::os::raw::c_void,
        p_steps: *mut std::os::raw::c_void,
        ptr: *mut std::os::raw::c_void,
    ) -> Self {
        Self { config, p_starkinfo, p_chelpers, p_steps, ptr, phantom: std::marker::PhantomData }
    }
}

impl<T: 'static + AbstractField> ProverBuilder<T> for StarkProverBuilder<T> {
    fn build(&mut self) -> Box<dyn Prover<T>> {
        let mut prover =
            Box::new(StarkProver::new(self.config.clone(), self.p_starkinfo, self.p_chelpers, self.p_steps, self.ptr));
        prover.build();

        prover
    }
}
