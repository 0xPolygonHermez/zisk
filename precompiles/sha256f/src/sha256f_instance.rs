//! The `Sha256fInstance` module defines an instance to perform the witness computation
//! for the Sha256f State Machine.
//!
//! It manages collected inputs and interacts with the `Sha256fSM` to compute witnesses for
//! execution plans.

use crate::{Sha256fInput, Sha256fSM};
use fields::PrimeField64;
use pil_std_lib::Std;
use proofman_common::{AirInstance, BufferPool, ProofCtx, SetupCtx};
use std::collections::VecDeque;
use std::{any::Any, collections::HashMap, sync::Arc};
use zisk_common::ChunkId;
use zisk_common::{
    BusDevice, BusId, CheckPoint, CollectSkipper, ExtOperationData, Instance, InstanceCtx,
    InstanceType, PayloadType, OPERATION_BUS_ID, OP_TYPE,
};
use zisk_core::ZiskOperationType;
use zisk_pil::Sha256fTrace;

/// The `Sha256fInstance` struct represents an instance for the Sha256f State Machine.
///
/// It encapsulates the `Sha256fSM` and its associated context, and it processes input data
/// to compute witnesses for the Sha256f State Machine.
pub struct Sha256fInstance<F: PrimeField64> {
    /// Sha256f state machine.
    sha256f_sm: Arc<Sha256fSM<F>>,

    /// Instance context.
    ictx: InstanceCtx,
}

impl<F: PrimeField64> Sha256fInstance<F> {
    /// Creates a new `Sha256fInstance`.
    ///
    /// # Arguments
    /// * `sha256f_sm` - An `Arc`-wrapped reference to the Sha256f State Machine.
    /// * `ictx` - The `InstanceCtx` associated with this instance, containing the execution plan.
    /// * `bus_id` - The bus ID associated with this instance.
    ///
    /// # Returns
    /// A new `Sha256fInstance` instance initialized with the provided state machine and
    /// context.
    pub fn new(sha256f_sm: Arc<Sha256fSM<F>>, ictx: InstanceCtx) -> Self {
        Self { sha256f_sm, ictx }
    }
}

impl<F: PrimeField64> Instance<F> for Sha256fInstance<F> {
    /// Computes the witness for the sha256f execution plan.
    ///
    /// This method leverages the `Sha256fSM` to generate an `AirInstance` using the collected
    /// inputs.
    ///
    /// # Arguments
    /// * `_pctx` - The proof context, unused in this implementation.
    ///
    /// # Returns
    /// An `Option` containing the computed `AirInstance`.
    fn compute_witness(
        &self,
        _pctx: &ProofCtx<F>,
        _sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
        buffer_pool: &dyn BufferPool<F>,
    ) -> Option<AirInstance<F>> {
        let mut inputs = Vec::with_capacity(collectors.len());

        for (_, collector) in collectors {
            let c: Box<Sha256fCollector<F>> = collector.as_any().downcast().unwrap();
            if !c.calculate_inputs {
                return None;
            }
            inputs.push(c.inputs);
        }

        let total_inputs = inputs.iter().map(|v| v.len()).sum::<usize>();
        self.compute_multiplicity_instance(total_inputs);

        Some(self.sha256f_sm.compute_witness(&inputs, buffer_pool.take_buffer()))
    }

    fn compute_multiplicity_instance(&self, total_inputs: usize) {
        self.sha256f_sm.compute_multiplicity_instance(total_inputs);
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

    fn build_inputs_collector(
        &self,
        std: Arc<Std<F>>,
        chunk_id: ChunkId,
    ) -> Option<Box<dyn BusDevice<PayloadType>>> {
        assert_eq!(
            self.ictx.plan.air_id,
            Sha256fTrace::<F>::AIR_ID,
            "Sha256fInstance: Unsupported air_id: {:?}",
            self.ictx.plan.air_id
        );

        let meta = self.ictx.plan.meta.as_ref().unwrap();
        let collect_info = meta.downcast_ref::<HashMap<ChunkId, (u64, CollectSkipper)>>().unwrap();
        let (num_ops, collect_skipper) = collect_info[&chunk_id];
        Some(Box::new(Sha256fCollector::new(std, num_ops, collect_skipper)))
    }
}

pub struct Sha256fCollector<F: PrimeField64> {
    std: Arc<Std<F>>,
    /// Collected inputs for witness computation.
    inputs: Vec<Sha256fInput>,

    /// The number of operations to collect.
    num_operations: u64,

    /// Helper to skip instructions based on the plan's configuration.
    collect_skipper: CollectSkipper,

    pub calculate_inputs: bool,

    pub calculate_multiplicity: bool,

    inputs_collected: u64,
}

impl<F: PrimeField64> Sha256fCollector<F> {
    /// Creates a new `Sha256fCollector`.
    ///
    /// # Arguments
    ///
    /// * `bus_id` - The connected bus ID.
    /// * `num_operations` - The number of operations to collect.
    /// * `collect_skipper` - The helper to skip instructions based on the plan's configuration.
    ///
    /// # Returns
    /// A new `ArithInstanceCollector` instance initialized with the provided parameters.
    pub fn new(std: Arc<Std<F>>, num_operations: u64, collect_skipper: CollectSkipper) -> Self {
        Self {
            std,
            inputs: Vec::new(),
            num_operations,
            collect_skipper,
            calculate_inputs: true,
            calculate_multiplicity: true,
            inputs_collected: 0,
        }
    }
}

impl<F: PrimeField64> BusDevice<PayloadType> for Sha256fCollector<F> {
    /// Processes data received on the bus, collecting the inputs necessary for witness computation.
    ///
    /// # Arguments
    /// * `_bus_id` - The ID of the bus (unused in this implementation).
    /// * `data` - The data received from the bus.
    /// * `pending` – A queue of pending bus operations used to send derived inputs.
    ///
    /// # Returns
    /// A tuple where:
    /// A boolean indicating whether the program should continue execution or terminate.
    /// Returns `true` to continue execution, `false` to stop.
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[PayloadType],
        _pending: &mut VecDeque<(BusId, Vec<PayloadType>)>,
    ) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if self.inputs.len() == self.num_operations as usize {
            return false;
        }

        if data[OP_TYPE] as u32 != ZiskOperationType::Sha256 as u32 {
            return true;
        }

        if self.collect_skipper.should_skip() {
            return true;
        }

        let data: ExtOperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");
        if let ExtOperationData::OperationSha256Data(data) = data {
            let input = Sha256fInput::from(&data);
            if self.calculate_multiplicity {
                Sha256fSM::process_multiplicity(&self.std, &input);
            }
            if self.calculate_inputs {
                self.inputs.push(input);
            }
            self.inputs_collected += 1;
        } else {
            panic!("Expected ExtOperationData::OperationData");
        }

        self.inputs_collected < self.num_operations
    }

    /// Returns the bus IDs associated with this instance.
    ///
    /// # Returns
    /// A vector containing the connected bus ID.
    fn bus_id(&self) -> Vec<BusId> {
        vec![OPERATION_BUS_ID]
    }

    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}
