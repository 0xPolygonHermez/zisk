use goldilocks::AbstractField;

use log::debug;
use proofman::provers_manager::{Prover, ProverBuilder};
use proofman::AirInstanceCtx;
use starks_lib_c::{chelpers_new_c, get_map_totaln_c, init_hints_c, set_mapOffsets_c, stark_info_new_c};
use crate::stark_prover_settings::StarkProverSettings;
use crate::stark_prover::StarkProver;
use std::ffi::c_void;

pub struct StarkProverBuilder<T> {
    config: StarkProverSettings,
    p_starkinfo: Option<*mut c_void>,
    p_chelpers: Option<*mut c_void>,
    p_steps: *mut std::os::raw::c_void,
    // ptr: *mut std::os::raw::c_void,
    phantom: std::marker::PhantomData<T>,
}

impl<T> StarkProverBuilder<T> {
    pub fn new(
        config: StarkProverSettings,
        p_steps: *mut std::os::raw::c_void,
        // ptr: *mut std::os::raw::c_void,
    ) -> Self {
        Self { config, p_starkinfo: None, p_chelpers: None, p_steps, phantom: std::marker::PhantomData }
    }
}

impl<T: 'static + AbstractField> ProverBuilder<T> for StarkProverBuilder<T> {
    fn build(&mut self, air_instance_ctx: &AirInstanceCtx<T>) -> Box<dyn Prover<T>> {
        if self.p_starkinfo.is_none() || self.p_chelpers.is_none() {
            self.init_stark();
        }

        let mut prover = Box::new(StarkProver::new(
            self.config.clone(),
            self.p_starkinfo.unwrap(),
            self.p_chelpers.unwrap(),
            self.p_steps,
            // self.ptr,
        ));
        prover.build(air_instance_ctx);

        prover
    }

    fn create_buffer(&mut self) -> Vec<u8> {
        if self.p_starkinfo.is_none() || self.p_chelpers.is_none() {
            self.init_stark();
        }

        // Allocate memory for the big buffer on the heap
        // The size of the buffer is hardcoded and it depends on the number of polynomials needed to generate the proof
        let map_total_n = get_map_totaln_c(self.p_starkinfo.unwrap());
        let buffer_size = map_total_n * std::mem::size_of::<T>() as u64;

        debug!("strkprvbld: Preallocating a buffer of {}bytes", buffer_size);
        vec![0u8; buffer_size as usize]
    }
}

impl<T> StarkProverBuilder<T> {
    fn init_stark(&mut self) {
        init_hints_c();

        let p_starkinfo = stark_info_new_c(&self.config.stark_info_filename);
        let p_chelpers = chelpers_new_c(&self.config.chelpers_filename.clone());

        set_mapOffsets_c(p_starkinfo, p_chelpers);

        self.p_starkinfo = Some(p_starkinfo);
        self.p_chelpers = Some(p_chelpers);
    }
}
