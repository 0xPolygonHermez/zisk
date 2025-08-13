//! The `FrequentOpsInstance` performs the witness computation for frequent operations.
//!
//! It is responsible for computing witnesses for frequent operations execution plans.

use fields::PrimeField64;
use proofman_common::{AirInstance, FromTrace, ProofCtx, SetupCtx};
use zisk_common::{
    BusDevice, CheckPoint, ChunkId, Instance, InstanceCtx, InstanceType, PayloadType,
};

use zisk_pil::FrequentOpsTrace;

use crate::FrequentOpsCollector;

/// The `FrequentOpsInstance` struct represents an instance to perform the witness computations for
/// frequent operations execution plans.
///
/// It interacts with input data to compute witnesses for the given execution plan.
pub struct FrequentOpsInstance {
    /// The instance context.
    ictx: InstanceCtx,
}

impl FrequentOpsInstance {
    /// Creates a new `FrequentOpsInstance`.
    ///
    /// # Arguments
    /// * `ictx` - The `InstanceCtx` associated with this instance.
    ///
    /// # Returns
    /// A new `FrequentOpsInstance` instance initialized with the context.
    pub fn new(ictx: InstanceCtx) -> Self {
        Self { ictx }
    }
}

impl<F: PrimeField64> Instance<F> for FrequentOpsInstance {
    /// Computes the witness for the frequent operations execution plan.
    ///
    /// This method processes the collected inputs to generate an `AirInstance` based on the
    /// frequent operations table and the provided execution data.
    ///
    /// # Arguments
    /// * `_pctx` - The proof context, unused in this implementation.
    /// * `_sctx` - The setup context, unused in this implementation.
    /// * `collectors` - A vector of input collectors to process and collect data for witness
    ///   computation.
    /// * `trace_buffer` - Pre-allocated buffer for trace data.
    ///
    /// # Returns
    /// An `Option` containing the computed `AirInstance`.
    fn compute_witness(
        &self,
        _pctx: &ProofCtx<F>,
        _sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
        trace_buffer: Vec<F>,
    ) -> Option<AirInstance<F>> {
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, collector)| {
                collector.as_any().downcast::<FrequentOpsCollector>().unwrap().inputs
            })
            .collect();
        let mut frequent_ops_table = vec![0u64; FrequentOpsTrace::<usize>::NUM_ROWS];
        inputs.iter().for_each(|input| {
            input.iter().for_each(|&value| {
                frequent_ops_table[value as usize] += 1;
            });
        });
        let mut frequent_ops_trace = FrequentOpsTrace::new_from_vec(trace_buffer);
        frequent_ops_table.iter().enumerate().for_each(|(i, &value)| {
            if value > 0 {
                frequent_ops_trace[i].multiplicity = F::from_u64(value);
            } else {
                frequent_ops_trace[i].multiplicity = F::ZERO;
            }
        });
        Some(AirInstance::new_from_trace(FromTrace::new(&mut frequent_ops_trace)))
    }

    /// Retrieves the checkpoint associated with this instance.
    ///
    /// # Returns
    /// A `CheckPoint` object representing the checkpoint of the execution plan.
    fn check_point(&self) -> &CheckPoint {
        &self.ictx.plan.check_point
    }

    /// Retrieves the type of this instance.
    ///
    /// # Returns
    /// An `InstanceType` representing the type of this instance (`InstanceType::Instance`).
    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }
    /// Builds an input collector for the instance.
    ///
    /// # Arguments
    /// * `chunk_id` - The chunk ID associated with the input collector.
    ///
    /// # Returns
    /// An `Option` containing the input collector for the instance.
    fn build_inputs_collector(
        &self,
        _chunk_id: ChunkId,
    ) -> Option<Box<dyn BusDevice<PayloadType>>> {
        Some(Box::new(FrequentOpsCollector::new()))
    }
}
