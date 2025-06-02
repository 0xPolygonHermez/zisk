//! The `ArithEqInstance` module defines an instance to perform the witness computation
//! for the ArithEq State Machine.
//!
//! It manages collected inputs and interacts with the `Arith256SM` to compute witnesses for
//! execution plans.

use crate::{
    Arith256Input, Arith256ModInput, ArithEqInput, ArithEqSM, Bn254ComplexAddInput,
    Bn254ComplexMulInput, Bn254ComplexSubInput, Bn254CurveAddInput, Bn254CurveDblInput,
    Secp256k1AddInput, Secp256k1DblInput,
};
use p3_field::PrimeField64;
use proofman_common::{AirInstance, ProofCtx, SetupCtx};
use std::{any::Any, collections::HashMap, sync::Arc};
use zisk_common::ChunkId;
use zisk_common::{
    BusDevice, BusId, CheckPoint, CollectSkipper, ExtOperationData, Instance, InstanceCtx,
    InstanceType, OperationBusData, PayloadType, OPERATION_BUS_ID,
};
use zisk_core::ZiskOperationType;
use zisk_pil::ArithEqTrace;

/// The `ArithEqInstance` struct represents an instance for the ArithEq State Machine.
///
/// It encapsulates the `ArithEqSM` and its associated context, and it processes input data
/// to compute witnesses for the ArithEq State Machine.
pub struct ArithEqInstance<F: PrimeField64> {
    /// ArithEq state machine.
    arith_eq_sm: Arc<ArithEqSM<F>>,

    /// Instance context.
    ictx: InstanceCtx,
}

impl<F: PrimeField64> ArithEqInstance<F> {
    /// Creates a new `ArithEqInstance`.
    ///
    /// # Arguments
    /// * `arith_eq_sm` - An `Arc`-wrapped reference to the ArithEq State Machine.
    /// * `ictx` - The `InstanceCtx` associated with this instance, containing the execution plan.
    /// * `bus_id` - The bus ID associated with this instance.
    ///
    /// # Returns
    /// A new `Arith256Instance` instance initialized with the provided state machine and
    /// context.
    pub fn new(arith_eq_sm: Arc<ArithEqSM<F>>, ictx: InstanceCtx) -> Self {
        Self { arith_eq_sm, ictx }
    }
}

impl<F: PrimeField64> Instance<F> for ArithEqInstance<F> {
    /// Computes the witness for the arith_eq execution plan.
    ///
    /// This method leverages the `ArithEqSM` to generate an `AirInstance` using the collected
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
        collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
    ) -> Option<AirInstance<F>> {
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, collector)| collector.as_any().downcast::<ArithEqCollector>().unwrap().inputs)
            .collect();

        Some(self.arith_eq_sm.compute_witness(sctx, &inputs))
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

    fn build_inputs_collector(&self, chunk_id: ChunkId) -> Option<Box<dyn BusDevice<PayloadType>>> {
        assert_eq!(
            self.ictx.plan.air_id,
            ArithEqTrace::<F>::AIR_ID,
            "ArithEqInstance: Unsupported air_id: {:?}",
            self.ictx.plan.air_id
        );

        let meta = self.ictx.plan.meta.as_ref().unwrap();
        let collect_info = meta.downcast_ref::<HashMap<ChunkId, (u64, CollectSkipper)>>().unwrap();
        let (num_ops, collect_skipper) = collect_info[&chunk_id];
        Some(Box::new(ArithEqCollector::new(num_ops, collect_skipper)))
    }
}

pub struct ArithEqCollector {
    /// Collected inputs for witness computation.
    inputs: Vec<ArithEqInput>,

    /// The number of operations to collect.
    num_operations: u64,

    /// Helper to skip instructions based on the plan's configuration.
    collect_skipper: CollectSkipper,
}

impl ArithEqCollector {
    /// Creates a new `ArithEqCollector`.
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

impl BusDevice<PayloadType> for ArithEqCollector {
    /// Processes data received on the bus, collecting the inputs necessary for witness computation.
    ///
    /// # Arguments
    /// * `_bus_id` - The ID of the bus (unused in this implementation).
    /// * `data` - The data received from the bus.
    ///
    /// # Returns
    /// A tuple where:
    /// - The first element indicates whether further processing should continue.
    /// - The second element contains derived inputs to be sent back to the bus (always empty).
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[PayloadType],
    ) -> Option<Vec<(BusId, Vec<PayloadType>)>> {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if self.inputs.len() == self.num_operations as usize {
            return None;
        }

        let data: ExtOperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");

        if OperationBusData::get_op_type(&data) as u32 != ZiskOperationType::ArithEq as u32 {
            return None;
        }

        if self.collect_skipper.should_skip() {
            return None;
        }

        self.inputs.push(match data {
            ExtOperationData::OperationArith256Data(bus_data) => {
                ArithEqInput::Arith256(Arith256Input::from(&bus_data))
            }
            ExtOperationData::OperationArith256ModData(bus_data) => {
                ArithEqInput::Arith256Mod(Arith256ModInput::from(&bus_data))
            }
            ExtOperationData::OperationSecp256k1AddData(bus_data) => {
                ArithEqInput::Secp256k1Add(Secp256k1AddInput::from(&bus_data))
            }
            ExtOperationData::OperationSecp256k1DblData(bus_data) => {
                ArithEqInput::Secp256k1Dbl(Secp256k1DblInput::from(&bus_data))
            }
            ExtOperationData::OperationBn254CurveAddData(bus_data) => {
                ArithEqInput::Bn254CurveAdd(Bn254CurveAddInput::from(&bus_data))
            }
            ExtOperationData::OperationBn254CurveDblData(bus_data) => {
                ArithEqInput::Bn254CurveDbl(Bn254CurveDblInput::from(&bus_data))
            }
            ExtOperationData::OperationBn254ComplexAddData(bus_data) => {
                ArithEqInput::Bn254ComplexAdd(Bn254ComplexAddInput::from(&bus_data))
            }
            ExtOperationData::OperationBn254ComplexSubData(bus_data) => {
                ArithEqInput::Bn254ComplexSub(Bn254ComplexSubInput::from(&bus_data))
            }
            ExtOperationData::OperationBn254ComplexMulData(bus_data) => {
                ArithEqInput::Bn254ComplexMul(Bn254ComplexMulInput::from(&bus_data))
            }
            // Add here new operations
            _ => panic!("Expected ExtOperationData::OperationData"),
        });
        None
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
