use std::sync::Arc;

use fields::PrimeField64;
use pil_std_lib::Std;
use proofman_common::ProofCtx;
use zisk_common::{
    BusDevice, BusDeviceMetrics, BusDeviceMode, ComponentBuilder, Instance, InstanceCtx,
    PayloadType, Plan, Planner,
};
use zisk_pil::{Dma64AlignedTrace, DmaPrePostTrace, DmaTrace, DmaUnalignedTrace, ZiskProofValues};

use crate::{
    Dma64AlignedInstance, Dma64AlignedSM, DmaCounterInputGen, DmaInstance, DmaPlanner,
    DmaPrePostInstance, DmaPrePostSM, DmaSM, DmaUnalignedInstance, DmaUnalignedSM,
};

/// The `DmaManager` struct represents the Dma manager,
/// which is responsible for managing the Dma state machine and its table state machine.
#[allow(dead_code)]
pub struct DmaManager<F: PrimeField64> {
    /// Dma state machine
    dma_sm: Arc<DmaSM<F>>,
    dma_pre_post_sm: Arc<DmaPrePostSM<F>>,
    dma_64_aligned_sm: Arc<Dma64AlignedSM<F>>,
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
        let dma_64_aligned_sm = Dma64AlignedSM::new(std.clone());
        let dma_unaligned_sm = DmaUnalignedSM::new(std);

        Arc::new(Self { dma_sm, dma_pre_post_sm, dma_64_aligned_sm, dma_unaligned_sm })
    }

    pub fn build_dma_counter(&self) -> DmaCounterInputGen {
        DmaCounterInputGen::new(BusDeviceMode::Counter)
    }

    pub fn build_dma_input_generator(&self) -> DmaCounterInputGen {
        DmaCounterInputGen::new(BusDeviceMode::InputGenerator)
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for DmaManager<F> {
    /// Builds and returns a new counter for monitoring Dma operations.
    ///
    /// # Returns
    /// A boxed implementation of `RegularCounters` configured for Dma operations.
    fn build_counter(&self) -> Option<Box<dyn BusDeviceMetrics>> {
        Some(Box::new(DmaCounterInputGen::new(BusDeviceMode::Counter)))
    }

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
            DmaTrace::<F>::AIR_ID => Box::new(DmaInstance::new(self.dma_sm.clone(), ictx)),
            DmaPrePostTrace::<F>::AIR_ID => {
                Box::new(DmaPrePostInstance::new(self.dma_pre_post_sm.clone(), ictx))
            }
            Dma64AlignedTrace::<F>::AIR_ID => {
                Box::new(Dma64AlignedInstance::new(self.dma_64_aligned_sm.clone(), ictx))
            }
            DmaUnalignedTrace::<F>::AIR_ID => {
                Box::new(DmaUnalignedInstance::new(self.dma_unaligned_sm.clone(), ictx))
            }
            _ => {
                panic!("DmaBuilder::get_instance() Unsupported air_id: {:?}", ictx.plan.air_id)
            }
        }
    }

    fn build_inputs_generator(&self) -> Option<Box<dyn BusDevice<PayloadType>>> {
        Some(Box::new(DmaCounterInputGen::new(BusDeviceMode::InputGenerator)))
    }

    fn configure_instances(&self, pctx: &ProofCtx<F>, plannings: &[Plan]) {
        let enable_dma_64_aligned =
            plannings.iter().any(|p| p.air_id == Dma64AlignedTrace::<F>::AIR_ID);
        let enable_dma_unaligned =
            plannings.iter().any(|p| p.air_id == DmaUnalignedTrace::<F>::AIR_ID);
        let mut proof_values = ZiskProofValues::from_vec_guard(pctx.get_proof_values());
        proof_values.enable_dma_64_aligned = F::from_bool(enable_dma_64_aligned);
        proof_values.enable_dma_unaligned = F::from_bool(enable_dma_unaligned);
    }
}
