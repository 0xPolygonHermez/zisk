use std::sync::Arc;

use fields::PrimeField64;
use pil_std_lib::Std;
use proofman_common::ProofCtx;
use zisk_common::{BusDeviceMode, ComponentBuilder, Instance, InstanceCtx, Plan, Planner};
use zisk_pil::{
    Dma64AlignedInputCpyTrace, Dma64AlignedMemCpyTrace, Dma64AlignedMemSetTrace,
    Dma64AlignedMemTrace, Dma64AlignedTrace, DmaInputCpyTrace, DmaMemCpyTrace,
    DmaPrePostInputCpyTrace, DmaPrePostMemCpyTrace, DmaPrePostTrace, DmaTrace, DmaUnalignedTrace,
    ZiskProofValues,
};

use crate::{
    Dma64AlignedInputCpySM, Dma64AlignedInstance, Dma64AlignedMemCpySM, Dma64AlignedMemSM,
    Dma64AlignedMemSetSM, Dma64AlignedSM, DmaCounterInputGen, DmaInputCpySM, DmaInstance,
    DmaMemCpySM, DmaPlanner, DmaPrePostInputCpySM, DmaPrePostInstance, DmaPrePostMemCpySM,
    DmaPrePostSM, DmaSM, DmaUnalignedInstance, DmaUnalignedSM,
};

/// The `DmaManager` struct represents the Dma manager,
/// which is responsible for managing the Dma state machine and its table state machine.
#[allow(dead_code)]
pub struct DmaManager<F: PrimeField64> {
    /// Dma state machine
    dma_sm: Arc<DmaSM<F>>,
    dma_memcpy_sm: Arc<DmaMemCpySM<F>>,
    dma_inputcpy_sm: Arc<DmaInputCpySM<F>>,
    dma_pre_post_sm: Arc<DmaPrePostSM<F>>,
    dma_pre_post_memcpy_sm: Arc<DmaPrePostMemCpySM<F>>,
    dma_pre_post_inputcpy_sm: Arc<DmaPrePostInputCpySM<F>>,
    dma_64_aligned_sm: Arc<Dma64AlignedSM<F>>,
    dma_64_aligned_mem_sm: Arc<Dma64AlignedMemSM<F>>,
    dma_64_aligned_memcpy_sm: Arc<Dma64AlignedMemCpySM<F>>,
    dma_64_aligned_memset_sm: Arc<Dma64AlignedMemSetSM<F>>,
    dma_64_aligned_inputcpy_sm: Arc<Dma64AlignedInputCpySM<F>>,
    dma_unaligned_sm: Arc<DmaUnalignedSM<F>>,
}

impl<F: PrimeField64> DmaManager<F> {
    /// Creates a new instance of `DmaManager`.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `DmaManager`.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        let dma_sm = DmaSM::new(std.clone());
        let dma_memcpy_sm = DmaMemCpySM::new(std.clone());
        let dma_inputcpy_sm = DmaInputCpySM::new(std.clone());
        let dma_pre_post_sm = DmaPrePostSM::new(std.clone());
        let dma_pre_post_inputcpy_sm = DmaPrePostInputCpySM::new(std.clone());
        let dma_pre_post_memcpy_sm = DmaPrePostMemCpySM::new(std.clone());
        let dma_64_aligned_sm = Dma64AlignedSM::new(std.clone());
        let dma_64_aligned_mem_sm = Dma64AlignedMemSM::new(std.clone());
        let dma_64_aligned_memcpy_sm = Dma64AlignedMemCpySM::new(std.clone());
        let dma_64_aligned_memset_sm = Dma64AlignedMemSetSM::new(std.clone());
        let dma_64_aligned_inputcpy_sm = Dma64AlignedInputCpySM::new(std.clone());
        let dma_unaligned_sm = DmaUnalignedSM::new(std);

        Arc::new(Self {
            dma_sm,
            dma_memcpy_sm,
            dma_inputcpy_sm,
            dma_pre_post_sm,
            dma_pre_post_inputcpy_sm,
            dma_pre_post_memcpy_sm,
            dma_64_aligned_sm,
            dma_64_aligned_mem_sm,
            dma_64_aligned_memcpy_sm,
            dma_64_aligned_memset_sm,
            dma_64_aligned_inputcpy_sm,
            dma_unaligned_sm,
        })
    }

    pub fn build_dma_counter(&self, asm_execution: bool) -> DmaCounterInputGen {
        match asm_execution {
            true => DmaCounterInputGen::new(BusDeviceMode::CounterAsm),
            false => DmaCounterInputGen::new(BusDeviceMode::Counter),
        }
    }

    pub fn build_dma_input_generator(&self) -> DmaCounterInputGen {
        DmaCounterInputGen::new(BusDeviceMode::InputGenerator)
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for DmaManager<F> {
    /// Builds a planner to plan Dma-related instances.
    ///
    /// # Returns
    /// A boxed implementation of `RegularPlanner`.
    fn build_planner(&self) -> Box<dyn Planner> {
        // Get the number of Dmas that a single Dma instance can handle
        Box::new(DmaPlanner::<F>::new())
    }

    /// Builds an inputs data collector for Dma operations.
    ///
    /// # Arguments
    /// * `ictx` - The context of the instance, containing the plan and its associated
    ///   configurations.
    ///
    /// # Returns
    /// A boxed implementation of `BusDeviceInstance` specific to the requested `air_id` instance.
    ///
    /// # Panics
    /// Panics if the provided `air_id` is not supported.
    fn build_instance(&self, ictx: InstanceCtx) -> Box<dyn Instance<F>> {
        match ictx.plan.air_id {
            // DMA controller instances
            DmaTrace::<F>::AIR_ID => Box::new(DmaInstance::new(self.dma_sm.clone(), ictx)),
            DmaMemCpyTrace::<F>::AIR_ID => {
                Box::new(DmaInstance::new(self.dma_memcpy_sm.clone(), ictx))
            }
            DmaInputCpyTrace::<F>::AIR_ID => {
                Box::new(DmaInstance::new(self.dma_inputcpy_sm.clone(), ictx))
            }
            // DMA pre post instances
            DmaPrePostTrace::<F>::AIR_ID => {
                Box::new(DmaPrePostInstance::new(self.dma_pre_post_sm.clone(), ictx))
            }
            DmaPrePostMemCpyTrace::<F>::AIR_ID => {
                Box::new(DmaPrePostInstance::new(self.dma_pre_post_memcpy_sm.clone(), ictx))
            }
            DmaPrePostInputCpyTrace::<F>::AIR_ID => {
                Box::new(DmaPrePostInstance::new(self.dma_pre_post_inputcpy_sm.clone(), ictx))
            }
            // DMA 64 aligned instances
            Dma64AlignedTrace::<F>::AIR_ID => {
                Box::new(Dma64AlignedInstance::new(self.dma_64_aligned_sm.clone(), ictx))
            }
            Dma64AlignedMemCpyTrace::<F>::AIR_ID => {
                Box::new(Dma64AlignedInstance::new(self.dma_64_aligned_memcpy_sm.clone(), ictx))
            }
            Dma64AlignedInputCpyTrace::<F>::AIR_ID => {
                Box::new(Dma64AlignedInstance::new(self.dma_64_aligned_inputcpy_sm.clone(), ictx))
            }
            Dma64AlignedMemSetTrace::<F>::AIR_ID => {
                Box::new(Dma64AlignedInstance::new(self.dma_64_aligned_memset_sm.clone(), ictx))
            }
            Dma64AlignedMemTrace::<F>::AIR_ID => {
                Box::new(Dma64AlignedInstance::new(self.dma_64_aligned_mem_sm.clone(), ictx))
            }
            // DMA unaligned instances
            DmaUnalignedTrace::<F>::AIR_ID => {
                Box::new(DmaUnalignedInstance::new(self.dma_unaligned_sm.clone(), ictx))
            }
            _ => {
                panic!("DmaBuilder::get_instance() Unsupported air_id: {:?}", ictx.plan.air_id)
            }
        }
    }

    fn configure_instances(&self, pctx: &ProofCtx<F>, plannings: &[Plan]) {
        let enable_dma_64_aligned =
            plannings.iter().any(|p| p.air_id == Dma64AlignedTrace::<F>::AIR_ID);
        let enable_dma_64_aligned_memcpy =
            plannings.iter().any(|p| p.air_id == Dma64AlignedMemCpyTrace::<F>::AIR_ID);
        let enable_dma_64_aligned_memset =
            plannings.iter().any(|p| p.air_id == Dma64AlignedMemSetTrace::<F>::AIR_ID);
        let enable_dma_64_aligned_inputcpy =
            plannings.iter().any(|p| p.air_id == Dma64AlignedInputCpyTrace::<F>::AIR_ID);
        let enable_dma_64_aligned_mem =
            plannings.iter().any(|p| p.air_id == Dma64AlignedMemTrace::<F>::AIR_ID);
        let enable_dma_unaligned =
            plannings.iter().any(|p| p.air_id == DmaUnalignedTrace::<F>::AIR_ID);
        let mut proof_values = ZiskProofValues::from_vec_guard(pctx.get_proof_values());
        proof_values.enable_dma_64_aligned = F::from_bool(enable_dma_64_aligned);
        proof_values.enable_dma_unaligned = F::from_bool(enable_dma_unaligned);
        proof_values.enable_dma_64_aligned_memcpy = F::from_bool(enable_dma_64_aligned_memcpy);
        proof_values.enable_dma_64_aligned_memset = F::from_bool(enable_dma_64_aligned_memset);
        proof_values.enable_dma_64_aligned_inputcpy = F::from_bool(enable_dma_64_aligned_inputcpy);
        proof_values.enable_dma_64_aligned_mem = F::from_bool(enable_dma_64_aligned_mem);
    }
}
