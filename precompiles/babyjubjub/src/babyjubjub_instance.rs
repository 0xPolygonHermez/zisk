//! The `BabyJubJubInstance` module defines an instance that performs the witness computation
//! for the BabyJubJub State Machine.

use crate::{BabyJubJubAddInput, BabyJubJubInput, BabyJubJubSM};
use fields::PrimeField64;
use proofman_common::{AirInstance, ProofCtx, ProofmanResult, SetupCtx};
use std::{any::Any, collections::HashMap, sync::Arc};
use zisk_common::ChunkId;
use zisk_common::StatsType;
use zisk_common::{
    BusDevice, BusId, CheckPoint, CollectSkipper, ExtOperationData, Instance, InstanceCtx,
    InstanceType, OperationBusData, PayloadType, OPERATION_BUS_ID,
};
use zisk_core::ZiskOperationType;
use zisk_pil::{BabyJubJubTrace, BabyJubJubTraceRow, BabyJubJubTraceRowPacked};

/// The `BabyJubJubInstance` struct represents an instance for the BabyJubJub State Machine.
pub struct BabyJubJubInstance<F: PrimeField64> {
    /// BabyJubJub state machine.
    babyjubjub_sm: Arc<BabyJubJubSM<F>>,

    /// Collect info for each chunk ID, containing the number of rows and a skipper for collection.
    collect_info: HashMap<ChunkId, (u64, CollectSkipper)>,

    /// Instance context.
    ictx: InstanceCtx,
}

impl<F: PrimeField64> BabyJubJubInstance<F> {
    /// Creates a new `BabyJubJubInstance`.
    pub fn new(babyjubjub_sm: Arc<BabyJubJubSM<F>>, mut ictx: InstanceCtx) -> Self {
        assert_eq!(
            ictx.plan.air_id,
            BabyJubJubTrace::<()>::AIR_ID,
            "BabyJubJubInstance: Unsupported air_id: {:?}",
            ictx.plan.air_id
        );

        let meta = ictx.plan.meta.take().expect("Expected metadata in ictx.plan.meta");

        let collect_info = *meta
            .downcast::<HashMap<ChunkId, (u64, CollectSkipper)>>()
            .expect("Failed to downcast ictx.plan.meta to expected type");

        Self { babyjubjub_sm, collect_info, ictx }
    }

    pub fn build_babyjubjub_collector(&self, chunk_id: ChunkId) -> BabyJubJubCollector {
        assert_eq!(
            self.ictx.plan.air_id,
            BabyJubJubTrace::<()>::AIR_ID,
            "BabyJubJubInstance: Unsupported air_id: {:?}",
            self.ictx.plan.air_id
        );

        let (num_ops, collect_skipper) = self.collect_info[&chunk_id];
        BabyJubJubCollector::new(num_ops, collect_skipper)
    }
}

impl<F: PrimeField64> Instance<F> for BabyJubJubInstance<F> {
    fn compute_witness(
        &self,
        _pctx: &ProofCtx<F>,
        sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<Option<AirInstance<F>>> {
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, collector)| {
                collector.as_any().downcast::<BabyJubJubCollector>().unwrap().inputs
            })
            .collect();

        if packed {
            Ok(Some(self.babyjubjub_sm.compute_witness::<BabyJubJubTraceRowPacked<F>>(
                sctx,
                &inputs,
                trace_buffer,
            )?))
        } else {
            Ok(Some(self.babyjubjub_sm.compute_witness::<BabyJubJubTraceRow<F>>(
                sctx,
                &inputs,
                trace_buffer,
            )?))
        }
    }

    fn check_point(&self) -> &CheckPoint {
        &self.ictx.plan.check_point
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }

    fn stats_type(&self) -> StatsType {
        StatsType::Precompiled
    }

    fn build_inputs_collector(&self, chunk_id: ChunkId) -> Option<Box<dyn BusDevice<PayloadType>>> {
        let (num_ops, collect_skipper) = self.collect_info[&chunk_id];
        Some(Box::new(BabyJubJubCollector::new(num_ops, collect_skipper)))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct BabyJubJubCollector {
    /// Collected inputs for witness computation.
    inputs: Vec<BabyJubJubInput>,

    /// The number of operations to collect.
    num_operations: u64,

    /// Helper to skip instructions based on the plan's configuration.
    collect_skipper: CollectSkipper,
}

impl BabyJubJubCollector {
    /// Creates a new `BabyJubJubCollector`.
    pub fn new(num_operations: u64, collect_skipper: CollectSkipper) -> Self {
        Self {
            inputs: Vec::with_capacity(num_operations as usize),
            num_operations,
            collect_skipper,
        }
    }

    /// Processes data received on the bus, collecting the inputs necessary for witness computation.
    #[inline(always)]
    pub fn process_data(&mut self, bus_id: &BusId, data: &[PayloadType]) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if self.inputs.len() == self.num_operations as usize {
            return false;
        }

        let data: ExtOperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");

        if OperationBusData::get_op_type(&data) as u32 != ZiskOperationType::BabyJubJub as u32 {
            return true;
        }

        if self.collect_skipper.should_skip() {
            return true;
        }

        self.inputs.push(match data {
            ExtOperationData::OperationBabyJubJubAddData(bus_data) => {
                BabyJubJubInput::Add(BabyJubJubAddInput::from(&bus_data))
            }
            _ => panic!("Expected ExtOperationData::OperationBabyJubJubAddData"),
        });

        self.inputs.len() < self.num_operations as usize
    }
}

impl BusDevice<PayloadType> for BabyJubJubCollector {
    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}
