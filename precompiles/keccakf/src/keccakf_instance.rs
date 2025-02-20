//! The `KeccakfInstance` module defines an instance to perform the witness computation
//! for the Keccakf State Machine.
//!
//! It manages collected inputs and interacts with the `KeccakfSM` to compute witnesses for
//! execution plans.

use crate::KeccakfSM;
use data_bus::{BusDevice, OperationKeccakData, PayloadType, OPERATION_BUS_ID};
use p3_field::PrimeField64;
use proofman_common::{AirInstance, ProofCtx, SetupCtx};
use sm_common::{
    input_collector, BusDeviceWrapper, CheckPoint, ChunkId, CollectSkipper, Instance, InstanceCtx,
    InstanceType,
};
use std::{collections::HashMap, sync::Arc};
use zisk_core::ZiskOperationType;
use zisk_pil::KeccakfTrace;

input_collector!(
    KeccakfCollector,
    ZiskOperationType::Keccak,
    OperationKeccakData,
    OPERATION_BUS_ID
);

/// The `KeccakfInstance` struct represents an instance for the Keccakf State Machine.
///
/// It encapsulates the `KeccakfSM` and its associated context, and it processes input data
/// to compute witnesses for the Keccakf State Machine.
pub struct KeccakfInstance {
    /// Keccakf state machine.
    keccakf_sm: Arc<KeccakfSM>,

    /// Instance context.
    ictx: InstanceCtx,
}

impl KeccakfInstance {
    /// Creates a new `KeccakfInstance`.
    ///
    /// # Arguments
    /// * `keccakf_sm` - An `Arc`-wrapped reference to the Keccakf State Machine.
    /// * `ictx` - The `InstanceCtx` associated with this instance, containing the execution plan.
    /// * `bus_id` - The bus ID associated with this instance.
    ///
    /// # Returns
    /// A new `KeccakfInstance` instance initialized with the provided state machine and
    /// context.
    pub fn new(keccakf_sm: Arc<KeccakfSM>, ictx: InstanceCtx) -> Self {
        Self { keccakf_sm, ictx }
    }
}

impl<F: PrimeField64> Instance<F> for KeccakfInstance {
    /// Computes the witness for the keccakf execution plan.
    ///
    /// This method leverages the `KeccakfSM` to generate an `AirInstance` using the collected
    /// inputs.
    ///
    /// # Arguments
    /// * `_pctx` - The proof context, unused in this implementation.
    ///
    /// # Returns
    /// An `Option` containing the computed `AirInstance`.
    fn compute_witness(
        &mut self,
        _pctx: &ProofCtx<F>,
        sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<BusDeviceWrapper<PayloadType>>)>,
    ) -> Option<AirInstance<F>> {
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, mut collector)| {
                collector.detach_device().as_any().downcast::<KeccakfCollector>().unwrap().inputs
            })
            .collect();

        Some(self.keccakf_sm.compute_witness(sctx, &inputs))
    }

    /// Retrieves the checkpoint associated with this instance.
    ///
    /// # Returns
    /// A `CheckPoint` object representing the checkpoint of the execution plan.
    fn check_point(&self) -> CheckPoint {
        self.ictx.plan.check_point.clone()
    }

    /// Retrieves the type of this instance.
    ///
    /// # Returns
    /// An `InstanceType` representing the type of this instance (`InstanceType::Instance`).
    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }

    fn build_inputs_collector(&self, chunk_id: usize) -> Option<Box<dyn BusDevice<PayloadType>>> {
        assert_eq!(
            self.ictx.plan.air_id,
            KeccakfTrace::<F>::AIR_ID,
            "KeccakfInstance: Unsupported air_id: {:?}",
            self.ictx.plan.air_id
        );

        let meta = self.ictx.plan.meta.as_ref().unwrap();
        let collect_info = meta.downcast_ref::<HashMap<ChunkId, (u64, CollectSkipper)>>().unwrap();
        let (num_ops, collect_skipper) = collect_info[&chunk_id];
        Some(Box::new(KeccakfCollector::new(num_ops, collect_skipper)))
    }
}
