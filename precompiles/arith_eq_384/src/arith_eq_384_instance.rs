//! The `ArithEq384Instance` module defines an instance to perform the witness computation
//! for the ArithEq384 State Machine.
//!
//! It manages collected inputs and interacts with the `Arith256SM` to compute witnesses for
//! execution plans.

use fields::PrimeField64;
use proofman_common::{AirInstance, ProofCtx, SetupCtx};
use std::collections::VecDeque;
use std::{any::Any, collections::HashMap, sync::Arc};

use zisk_common::{
    BusDevice, BusId, CheckPoint, CollectSkipper, ExtOperationData, Instance, InstanceCtx,
    InstanceType, OperationBusData, PayloadType, OPERATION_BUS_ID,
};
use zisk_common::{ChunkId, MemCollectorInfo};
use zisk_core::ZiskOperationType;
use zisk_pil::ArithEq384Trace;

use crate::{
    Arith384ModInput, ArithEq384Input, ArithEq384SM, Bls12_381ComplexAddInput,
    Bls12_381ComplexMulInput, Bls12_381ComplexSubInput, Bls12_381CurveAddInput,
    Bls12_381CurveDblInput,
};

/// The `ArithEq384Instance` struct represents an instance for the ArithEq384 State Machine.
///
/// It encapsulates the `ArithEq384SM` and its associated context, and it processes input data
/// to compute witnesses for the ArithEq384 State Machine.
pub struct ArithEq384Instance<F: PrimeField64> {
    /// ArithEq384 state machine.
    arith_eq_384_sm: Arc<ArithEq384SM<F>>,

    /// Collect info for each chunk ID, containing the number of rows and a skipper for collection.
    collect_info: HashMap<ChunkId, (u64, CollectSkipper)>,

    /// Instance context.
    ictx: InstanceCtx,
}

impl<F: PrimeField64> ArithEq384Instance<F> {
    /// Creates a new `ArithEq384Instance`.
    ///
    /// # Arguments
    /// * `arith_eq_384_sm` - An `Arc`-wrapped reference to the ArithEq384 State Machine.
    /// * `ictx` - The `InstanceCtx` associated with this instance, containing the execution plan.
    /// * `bus_id` - The bus ID associated with this instance.
    ///
    /// # Returns
    /// A new `Arith256Instance` instance initialized with the provided state machine and
    /// context.
    pub fn new(arith_eq_384_sm: Arc<ArithEq384SM<F>>, mut ictx: InstanceCtx) -> Self {
        assert_eq!(
            ictx.plan.air_id,
            ArithEq384Trace::<F>::AIR_ID,
            "ArithEq384Instance: Unsupported air_id: {:?}",
            ictx.plan.air_id
        );

        let meta = ictx.plan.meta.take().expect("Expected metadata in ictx.plan.meta");

        let collect_info = *meta
            .downcast::<HashMap<ChunkId, (u64, CollectSkipper)>>()
            .expect("Failed to downcast ictx.plan.meta to expected type");

        Self { arith_eq_384_sm, collect_info, ictx }
    }

    pub fn build_arith_eq_384_collector(&self, chunk_id: ChunkId) -> ArithEq384Collector {
        assert_eq!(
            self.ictx.plan.air_id,
            ArithEq384Trace::<F>::AIR_ID,
            "ArithEq384Instance: Unsupported air_id: {:?}",
            self.ictx.plan.air_id
        );

        let (num_ops, collect_skipper) = self.collect_info[&chunk_id];
        ArithEq384Collector::new(num_ops, collect_skipper)
    }
}

impl<F: PrimeField64> Instance<F> for ArithEq384Instance<F> {
    /// Computes the witness for the arith_eq_384 execution plan.
    ///
    /// This method leverages the `ArithEq384SM` to generate an `AirInstance` using the collected
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
        sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
        trace_buffer: Vec<F>,
    ) -> Option<AirInstance<F>> {
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, collector)| {
                collector.as_any().downcast::<ArithEq384Collector>().unwrap().inputs
            })
            .collect();

        Some(self.arith_eq_384_sm.compute_witness(sctx, &inputs, trace_buffer))
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

    fn build_inputs_collector(&self, chunk_id: ChunkId) -> Option<Box<dyn BusDevice<PayloadType>>> {
        let (num_ops, collect_skipper) = self.collect_info[&chunk_id];
        Some(Box::new(ArithEq384Collector::new(num_ops, collect_skipper)))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct ArithEq384Collector {
    /// Collected inputs for witness computation.
    inputs: Vec<ArithEq384Input>,

    /// The number of operations to collect.
    num_operations: u64,

    /// Helper to skip instructions based on the plan's configuration.
    collect_skipper: CollectSkipper,
}

impl ArithEq384Collector {
    /// Creates a new `ArithEq384Collector`.
    ///
    /// # Arguments
    ///
    /// * `bus_id` - The connected bus ID.
    /// * `num_operations` - The number of operations to collect.
    /// * `collect_skipper` - The helper to skip instructions based on the plan's configuration.
    ///
    /// # Returns
    /// A new `ArithInstanceCollector` instance initialized with the provided parameters.
    pub fn new(num_operations: u64, collect_skipper: CollectSkipper) -> Self {
        Self { inputs: Vec::new(), num_operations, collect_skipper }
    }
}

impl BusDevice<PayloadType> for ArithEq384Collector {
    /// Processes data received on the bus, collecting the inputs necessary for witness computation.
    ///
    /// # Arguments
    /// * `_bus_id` - The ID of the bus (unused in this implementation).
    /// * `data` - The data received from the bus.
    /// * `pending` – A queue of pending bus operations used to send derived inputs.
    ///
    /// # Returns
    /// A boolean indicating whether the program should continue execution or terminate.
    /// Returns `true` to continue execution, `false` to stop.
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[PayloadType],
        _pending: &mut VecDeque<(BusId, Vec<u64>)>,
        _mem_collector_info: Option<&[MemCollectorInfo]>,
    ) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if self.inputs.len() == self.num_operations as usize {
            return false;
        }

        let data: ExtOperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");

        if OperationBusData::get_op_type(&data) as u32 != ZiskOperationType::ArithEq384 as u32 {
            return true;
        }

        if self.collect_skipper.should_skip() {
            return true;
        }

        self.inputs.push(match data {
            ExtOperationData::OperationArith384ModData(bus_data) => {
                ArithEq384Input::Arith384Mod(Arith384ModInput::from(&bus_data))
            }
            ExtOperationData::OperationBls12_381CurveAddData(bus_data) => {
                ArithEq384Input::Bls12_381CurveAdd(Bls12_381CurveAddInput::from(&bus_data))
            }
            ExtOperationData::OperationBls12_381CurveDblData(bus_data) => {
                ArithEq384Input::Bls12_381CurveDbl(Bls12_381CurveDblInput::from(&bus_data))
            }
            ExtOperationData::OperationBls12_381ComplexAddData(bus_data) => {
                ArithEq384Input::Bls12_381ComplexAdd(Bls12_381ComplexAddInput::from(&bus_data))
            }
            ExtOperationData::OperationBls12_381ComplexSubData(bus_data) => {
                ArithEq384Input::Bls12_381ComplexSub(Bls12_381ComplexSubInput::from(&bus_data))
            }
            ExtOperationData::OperationBls12_381ComplexMulData(bus_data) => {
                ArithEq384Input::Bls12_381ComplexMul(Bls12_381ComplexMulInput::from(&bus_data))
            }
            // Add here new operations
            _ => panic!("Expected ExtOperationData::OperationData"),
        });

        self.inputs.len() < self.num_operations as usize
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
