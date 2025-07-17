use crate::{MemAlignInput, MemAlignSM, MemHelpers};
use mem_common::MemAlignCheckPoint;

use fields::PrimeField64;
use proofman_common::{AirInstance, ProofCtx, SetupCtx};
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};
use zisk_common::{
    BusDevice, BusId, CheckPoint, ChunkId, Instance, InstanceCtx, InstanceType, MemBusData,
    PayloadType, MEM_BUS_ID,
};

pub struct MemAlignInstance<F: PrimeField64> {
    /// Instance context
    ictx: InstanceCtx,

    /// Checkpoint data for this memory align instance.
    checkpoint: HashMap<ChunkId, MemAlignCheckPoint>,

    mem_align_sm: Arc<MemAlignSM<F>>,
}

impl<F: PrimeField64> MemAlignInstance<F> {
    pub fn new(mem_align_sm: Arc<MemAlignSM<F>>, mut ictx: InstanceCtx) -> Self {
        let meta = ictx.plan.meta.take().expect("Expected metadata in ictx.plan.meta");

        let checkpoint = *meta
            .downcast::<HashMap<ChunkId, MemAlignCheckPoint>>()
            .expect("Failed to downcast ictx.plan.meta to expected type");

        Self { ictx, checkpoint, mem_align_sm }
    }
}

impl<F: PrimeField64> Instance<F> for MemAlignInstance<F> {
    fn compute_witness(
        &self,
        _pctx: &ProofCtx<F>,
        _sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
        trace_buffer: Vec<F>,
    ) -> Option<AirInstance<F>> {
        let mut total_rows = 0;
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, collector)| {
                let collector = collector.as_any().downcast::<MemAlignCollector>().unwrap();

                total_rows += collector.rows;

                collector.inputs
            })
            .collect();
        Some(self.mem_align_sm.compute_witness(&inputs, total_rows as usize, trace_buffer))
    }

    fn check_point(&self) -> &CheckPoint {
        &self.ictx.plan.check_point
    }

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
    fn build_inputs_collector(&self, chunk_id: ChunkId) -> Option<Box<dyn BusDevice<PayloadType>>> {
        Some(Box::new(MemAlignCollector::new(&self.checkpoint[&chunk_id])))
    }
}

pub struct MemAlignCollector {
    /// Collected inputs
    inputs: Vec<MemAlignInput>,

    pending_count: u32,
    skip_pending: u32,
    rows: u32,
}

impl MemAlignCollector {
    pub fn new(mem_align_checkpoint: &MemAlignCheckPoint) -> Self {
        Self {
            inputs: Vec::new(),
            skip_pending: mem_align_checkpoint.skip,
            pending_count: mem_align_checkpoint.count,
            rows: mem_align_checkpoint.rows,
        }
    }
}

impl BusDevice<u64> for MemAlignCollector {
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[u64],
        _pending: &mut VecDeque<(BusId, Vec<u64>)>,
    ) -> bool {
        debug_assert!(*bus_id == MEM_BUS_ID);

        let addr = MemBusData::get_addr(data);
        let bytes = MemBusData::get_bytes(data);
        if MemHelpers::is_aligned(addr, bytes) {
            return true;
        }
        if self.skip_pending > 0 {
            self.skip_pending -= 1;
            return true;
        }

        if self.pending_count == 0 {
            return true;
        }
        self.pending_count -= 1;
        let is_write = MemHelpers::is_write(MemBusData::get_op(data));
        let addr = MemBusData::get_addr(data);
        let width = MemBusData::get_bytes(data);
        let mem_values = MemBusData::get_mem_values(data);
        let value = if is_write {
            MemBusData::get_value(data)
        } else {
            MemHelpers::get_read_value(addr, width, mem_values)
        };
        self.inputs.push(MemAlignInput {
            addr,
            is_write,
            width,
            step: MemBusData::get_step(data),
            value,
            mem_values,
        });

        self.pending_count > 0
    }

    fn bus_id(&self) -> Vec<BusId> {
        vec![MEM_BUS_ID]
    }

    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
