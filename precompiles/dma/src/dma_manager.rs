use std::sync::Arc;

use pil2_std_lib::Std;
use proofman_common::ProofCtx;
use proofman_fields::PrimeField64;
use zisk_common::{
    BusDeviceMode, ComponentBuilder, ComponentPlanBuilder, Instance, InstanceCtx, Plan, Planner,
};
use zisk_pil::{
    Dma64AlignedLargeTrace, Dma64AlignedMemCpyTrace, Dma64AlignedMemLargeTrace,
    Dma64AlignedMemSetTrace, Dma64AlignedMemTrace, Dma64AlignedTrace, DmaPrePostTrace, DmaTrace,
    DmaUnalignedTrace, ZiskProofValues,
};

use crate::{
    Dma64AlignedInstance, Dma64AlignedMemCpySM, Dma64AlignedMemSM, Dma64AlignedMemSetSM,
    Dma64AlignedSM, DmaCounterInputGen, DmaInstance, DmaPlanner, DmaPrePostInstance, DmaPrePostSM,
    DmaSM, DmaUnalignedInstance, DmaUnalignedSM,
};

/// The `DmaManager` struct represents the Dma manager,
/// which is responsible for managing the Dma state machine and its table state machine.
#[allow(dead_code)]
pub struct DmaManager<F: PrimeField64> {
    /// Dma state machine
    dma_sm: Arc<DmaSM<F>>,
    dma_pre_post_sm: Arc<DmaPrePostSM<F>>,
    /// One state machine per height of the `Dma64Aligned` air.
    dma_64_aligned_sm: Arc<Dma64AlignedSM<F>>,
    dma_64_aligned_large_sm: Arc<Dma64AlignedSM<F>>,
    /// One state machine per height of the `Dma64AlignedMem` air.
    dma_64_aligned_mem_sm: Arc<Dma64AlignedMemSM<F>>,
    dma_64_aligned_mem_large_sm: Arc<Dma64AlignedMemSM<F>>,
    dma_64_aligned_memcpy_sm: Arc<Dma64AlignedMemCpySM<F>>,
    dma_64_aligned_memset_sm: Arc<Dma64AlignedMemSetSM<F>>,
    dma_unaligned_sm: Arc<DmaUnalignedSM<F>>,
}

impl<F: PrimeField64> DmaManager<F> {
    /// Creates a new instance of `DmaManager`.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `DmaManager`.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        let dma_sm = DmaSM::new(std.clone());
        let dma_pre_post_sm = DmaPrePostSM::new(std.clone());
        let dma_64_aligned_sm = Dma64AlignedSM::new(std.clone(), Dma64AlignedTrace::<()>::AIR_ID);
        let dma_64_aligned_large_sm =
            Dma64AlignedSM::new(std.clone(), Dma64AlignedLargeTrace::<()>::AIR_ID);
        let dma_64_aligned_mem_sm =
            Dma64AlignedMemSM::new(std.clone(), Dma64AlignedMemTrace::<()>::AIR_ID);
        let dma_64_aligned_mem_large_sm =
            Dma64AlignedMemSM::new(std.clone(), Dma64AlignedMemLargeTrace::<()>::AIR_ID);
        let dma_64_aligned_memcpy_sm = Dma64AlignedMemCpySM::new(std.clone());
        let dma_64_aligned_memset_sm = Dma64AlignedMemSetSM::new(std.clone());
        let dma_unaligned_sm = DmaUnalignedSM::new(std);

        Arc::new(Self {
            dma_sm,
            dma_pre_post_sm,
            dma_64_aligned_sm,
            dma_64_aligned_large_sm,
            dma_64_aligned_mem_sm,
            dma_64_aligned_mem_large_sm,
            dma_64_aligned_memcpy_sm,
            dma_64_aligned_memset_sm,
            dma_unaligned_sm,
        })
    }
}

impl<F: PrimeField64> ComponentPlanBuilder<F> for DmaManager<F> {
    type Counter = DmaCounterInputGen;

    fn counter(is_asm_emulator: bool) -> Self::Counter {
        let mode = if is_asm_emulator { BusDeviceMode::CounterAsm } else { BusDeviceMode::Counter };
        DmaCounterInputGen::new(mode)
    }

    fn planner(_is_asm_emulator: bool) -> Box<dyn Planner> {
        Box::new(DmaPlanner::<F>::new())
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for DmaManager<F> {
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
            DmaTrace::<()>::AIR_ID => Box::new(DmaInstance::new(self.dma_sm.clone(), ictx)),
            // DMA pre post instances
            DmaPrePostTrace::<()>::AIR_ID => {
                Box::new(DmaPrePostInstance::new(self.dma_pre_post_sm.clone(), ictx))
            }
            // DMA 64 aligned instances
            Dma64AlignedTrace::<()>::AIR_ID => {
                Box::new(Dma64AlignedInstance::new(self.dma_64_aligned_sm.clone(), ictx))
            }
            Dma64AlignedLargeTrace::<()>::AIR_ID => {
                Box::new(Dma64AlignedInstance::new(self.dma_64_aligned_large_sm.clone(), ictx))
            }
            Dma64AlignedMemCpyTrace::<()>::AIR_ID => {
                Box::new(Dma64AlignedInstance::new(self.dma_64_aligned_memcpy_sm.clone(), ictx))
            }
            Dma64AlignedMemSetTrace::<()>::AIR_ID => {
                Box::new(Dma64AlignedInstance::new(self.dma_64_aligned_memset_sm.clone(), ictx))
            }
            Dma64AlignedMemTrace::<()>::AIR_ID => {
                Box::new(Dma64AlignedInstance::new(self.dma_64_aligned_mem_sm.clone(), ictx))
            }
            Dma64AlignedMemLargeTrace::<()>::AIR_ID => {
                Box::new(Dma64AlignedInstance::new(self.dma_64_aligned_mem_large_sm.clone(), ictx))
            }
            // DMA unaligned instances
            DmaUnalignedTrace::<()>::AIR_ID => {
                Box::new(DmaUnalignedInstance::new(self.dma_unaligned_sm.clone(), ictx))
            }
            _ => {
                panic!("DmaBuilder::get_instance() Unsupported air_id: {:?}", ictx.plan.air_id)
            }
        }
    }

    fn configure_instances(&self, pctx: &ProofCtx<F>, plannings: &[Plan]) {
        // One flag per air, never one per family: the flag gates the global seed of *that air's*
        // continuation chain, and every `Dma64Aligned` alias keeps a chain of its own. Turning a
        // flag on for an air the strategy gave no instance to would seed a chain nobody consumes,
        // which shows up as an unbalanced `DMA_64_ALIGNED_CONT_ID` bus.
        let planned = |air_id: usize| plannings.iter().any(|p| p.air_id == air_id);

        let mut proof_values = ZiskProofValues::from_vec_guard(pctx.get_proof_values());
        proof_values.enable_dma_64_aligned = F::from_bool(planned(Dma64AlignedTrace::<()>::AIR_ID));
        proof_values.enable_dma_64_aligned_large =
            F::from_bool(planned(Dma64AlignedLargeTrace::<()>::AIR_ID));
        proof_values.enable_dma_64_aligned_mem =
            F::from_bool(planned(Dma64AlignedMemTrace::<()>::AIR_ID));
        proof_values.enable_dma_64_aligned_mem_large =
            F::from_bool(planned(Dma64AlignedMemLargeTrace::<()>::AIR_ID));
        proof_values.enable_dma_64_aligned_memcpy =
            F::from_bool(planned(Dma64AlignedMemCpyTrace::<()>::AIR_ID));
        proof_values.enable_dma_64_aligned_memset =
            F::from_bool(planned(Dma64AlignedMemSetTrace::<()>::AIR_ID));
        proof_values.enable_dma_unaligned = F::from_bool(planned(DmaUnalignedTrace::<()>::AIR_ID));
        // No air is instantiated for the dedicated inputcpy variant any more.
        proof_values.enable_dma_64_aligned_inputcpy = F::ZERO;
    }
}
