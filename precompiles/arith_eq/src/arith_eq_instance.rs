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
use fields::PrimeField64;
use proofman_common::{AirInstance, BufferPool, ProofCtx, SetupCtx};
use std::collections::VecDeque;
use std::{any::Any, collections::HashMap, sync::Arc};
use zisk_common::ChunkId;
use zisk_common::{
    BusDevice, BusId, CheckPoint, CollectSkipper, ExtOperationData, Instance, InstanceCtx,
    InstanceType, OperationBusData, PayloadType, OPERATION_BUS_ID,
};

use pil_std_lib::Std;
use zisk_core::ZiskOperationType;
use zisk_pil::ArithEqTrace;

/// The `ArithEqInstance` struct represents an instance for the ArithEq State Machine.
///
/// It encapsulates the `ArithEqSM` and its associated context, and it processes input data
/// to compute witnesses for the ArithEq State Machine.
pub struct ArithEqInstance<F: PrimeField64> {
    /// ArithEq state machine.
    arith_eq_sm: Arc<ArithEqSM<F>>,

    /// Collect info for each chunk ID, containing the number of rows and a skipper for collection.
    collect_info: HashMap<ChunkId, (u64, CollectSkipper)>,

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
    pub fn new(arith_eq_sm: Arc<ArithEqSM<F>>, mut ictx: InstanceCtx) -> Self {
        assert_eq!(
            ictx.plan.air_id,
            ArithEqTrace::<F>::AIR_ID,
            "ArithEqInstance: Unsupported air_id: {:?}",
            ictx.plan.air_id
        );

        let meta = ictx.plan.meta.take().expect("Expected metadata in ictx.plan.meta");

        let collect_info = *meta
            .downcast::<HashMap<ChunkId, (u64, CollectSkipper)>>()
            .expect("Failed to downcast ictx.plan.meta to expected type");

        Self { arith_eq_sm, collect_info, ictx }
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
        &self,
        _pctx: &ProofCtx<F>,
        sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
        buffer_pool: &dyn BufferPool<F>,
    ) -> Option<AirInstance<F>> {
        let mut inputs = Vec::with_capacity(collectors.len());

        for (_, collector) in collectors {
            let c: Box<ArithEqCollector<F>> = collector.as_any().downcast().unwrap();
            if !c.calculate_inputs {
                return None;
            }
            inputs.push(c.inputs);
        }

        Some(self.arith_eq_sm.compute_witness(sctx, &inputs, buffer_pool.take_buffer()))
    }

    fn compute_multiplicity_instance(&self) {
        let num_rows = self.ictx.plan.num_rows.unwrap();
        self.arith_eq_sm.compute_multiplicity_instance(num_rows);
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
        calculate_inputs: bool,
        calculate_multiplicity: bool,
    ) -> Option<Box<dyn BusDevice<PayloadType>>> {
        let (num_ops, collect_skipper) = self.collect_info[&chunk_id];
        Some(Box::new(ArithEqCollector::new(
            std,
            num_ops as usize,
            calculate_inputs,
            calculate_multiplicity,
            collect_skipper,
        )))
    }
}

pub struct ArithEqCollector<F: PrimeField64> {
    std: Arc<Std<F>>,
    /// Collected inputs for witness computation.
    inputs: Vec<ArithEqInput>,

    /// The number of operations to collect.
    num_operations: usize,

    /// Helper to skip instructions based on the plan's configuration.
    collect_skipper: CollectSkipper,

    pub calculate_inputs: bool,

    pub calculate_multiplicity: bool,

    inputs_collected: usize,
}

impl<F: PrimeField64> ArithEqCollector<F> {
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
    pub fn new(
        std: Arc<Std<F>>,
        num_operations: usize,
        calculate_inputs: bool,
        calculate_multiplicity: bool,
        collect_skipper: CollectSkipper,
    ) -> Self {
        Self {
            std,
            inputs: Vec::new(),
            num_operations,
            collect_skipper,
            calculate_inputs,
            calculate_multiplicity,
            inputs_collected: 0,
        }
    }
}

impl<F: PrimeField64> BusDevice<PayloadType> for ArithEqCollector<F> {
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
    ) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if self.inputs.len() == self.num_operations as usize {
            return false;
        }

        let data: ExtOperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");

        if OperationBusData::get_op_type(&data) as u32 != ZiskOperationType::ArithEq as u32 {
            return true;
        }

        if self.collect_skipper.should_skip() {
            return true;
        }

        let input = match data {
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
        };

        if self.calculate_multiplicity {
            ArithEqSM::process_multiplicity(&self.std, &input);
        }

        self.inputs_collected += 1;
        if self.calculate_inputs {
            self.inputs.push(input);
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
