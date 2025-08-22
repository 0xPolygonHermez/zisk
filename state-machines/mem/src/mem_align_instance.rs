use crate::{MemAlignInput, MemAlignSM, MemHelpers};
use mem_common::MemAlignCheckPoint;

use fields::PrimeField64;
use pil_std_lib::Std;
use proofman_common::{AirInstance, BufferPool, ProofCtx, SetupCtx};
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
        buffer_pool: &dyn BufferPool<F>,
    ) -> Option<AirInstance<F>> {
        let mut total_inputs = 0;

        let mut inputs = Vec::with_capacity(collectors.len());

        for (_, collector) in collectors {
            let c: Box<MemAlignCollector<F>> = collector.as_any().downcast().unwrap();
            if !c.calculate_inputs {
                return None;
            }
            total_inputs += c.rows as u32;
            inputs.push(c.inputs);
        }

        self.compute_multiplicity_instance(total_inputs as usize);
        Some(self.mem_align_sm.compute_witness(
            &inputs,
            total_inputs as usize,
            buffer_pool.take_buffer(),
        ))
    }

    fn compute_multiplicity_instance(&self, total_inputs: usize) {
        self.mem_align_sm.compute_multiplicity_instance(total_inputs);
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
    fn build_inputs_collector(
        &self,
        std: Arc<Std<F>>,
        chunk_id: ChunkId,
    ) -> Option<Box<dyn BusDevice<PayloadType>>> {
        Some(Box::new(MemAlignCollector::new(std, &self.checkpoint[&chunk_id])))
    }
}

pub struct MemAlignCollector<F: PrimeField64> {
    std: Arc<Std<F>>,
    /// Collected inputs
    inputs: Vec<MemAlignInput>,

    pending_count: u32,
    skip_pending: u32,
    rows: u32,

    pub calculate_inputs: bool,

    pub calculate_multiplicity: bool,
    inputs_collected: usize,
}

impl<F: PrimeField64> MemAlignCollector<F> {
    pub fn new(std: Arc<Std<F>>, mem_align_checkpoint: &MemAlignCheckPoint) -> Self {
        Self {
            std,
            inputs: Vec::new(),
            skip_pending: mem_align_checkpoint.skip,
            pending_count: mem_align_checkpoint.count,
            rows: mem_align_checkpoint.rows,
            calculate_inputs: true,
            calculate_multiplicity: true,
            inputs_collected: 0,
        }
    }
}

impl<F: PrimeField64> BusDevice<u64> for MemAlignCollector<F> {
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
        let input = MemAlignInput {
            addr,
            is_write,
            width,
            step: MemBusData::get_step(data),
            value,
            mem_values,
        };

        if self.calculate_multiplicity {
            MemAlignSM::process_multiplicity(&self.std, &input);
        }
        if self.calculate_inputs {
            self.inputs.push(input);
        }
        self.inputs_collected += 1;

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
