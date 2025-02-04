use crate::{MemAlignCheckPoint, MemAlignInput, MemAlignSM, MemHelpers};
use core::panic;
use data_bus::{BusDevice, BusId, MemBusData, PayloadType, MEM_BUS_ID};
use p3_field::PrimeField;
use proofman_common::{AirInstance, ProofCtx};
use sm_common::{BusDeviceWrapper, CheckPoint, Instance, InstanceCtx, InstanceType};
use std::sync::Arc;

pub struct MemAlignInstance<F: PrimeField> {
    /// Instance context
    ictx: InstanceCtx,

    mem_align_sm: Arc<MemAlignSM<F>>,
}

impl<F: PrimeField> MemAlignInstance<F> {
    pub fn new(mem_align_sm: Arc<MemAlignSM<F>>, ictx: InstanceCtx) -> Self {
        Self { ictx, mem_align_sm }
    }
}

impl<F: PrimeField> Instance<F> for MemAlignInstance<F> {
    fn compute_witness(
        &mut self,
        _pctx: &ProofCtx<F>,
        collectors: Vec<(usize, Box<BusDeviceWrapper<PayloadType>>)>,
    ) -> Option<AirInstance<F>> {
        let mut used_rows = 0;
        let inputs: Vec<_> = collectors
            .into_iter()
            .enumerate()
            .map(|(i, (_, mut collector))| {
                // Downcast to your specific collector type.
                let collector =
                    collector.detach_device().as_any().downcast::<MemAlignCollector>().unwrap();
                if i == 0 {
                    used_rows = collector.rows;
                }
                // Return the inputs vector for this collector.
                collector.inputs
            })
            .collect();

        Some(self.mem_align_sm.compute_witness(&inputs, used_rows as usize))
    }

    fn check_point(&self) -> CheckPoint {
        self.ictx.plan.check_point.clone()
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }

    fn build_inputs_collector(&self, chunk_id: usize) -> Option<Box<dyn BusDevice<PayloadType>>> {
        let meta = self.ictx.plan.meta.as_ref().unwrap();
        let checkpoint = meta.downcast_ref::<Vec<MemAlignCheckPoint>>().unwrap();

        if let CheckPoint::Multiple(plan_checkpoints) = &self.ictx.plan.check_point {
            // Find in xxx the idx of the chunk_id
            let idx = plan_checkpoints.iter().position(|&x| x == chunk_id).unwrap();
            Some(Box::new(MemAlignCollector::new(&checkpoint[idx])))
        } else {
            panic!("Invalid checkpoint type");
        }
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
    fn process_data(&mut self, _bus_id: &BusId, data: &[u64]) -> Option<Vec<(BusId, Vec<u64>)>> {
        let addr = MemBusData::get_addr(data);
        let bytes = MemBusData::get_bytes(data);
        if MemHelpers::is_aligned(addr, bytes) {
            return None;
        }
        if self.skip_pending > 0 {
            self.skip_pending -= 1;
            return None;
        }

        if self.pending_count == 0 {
            return None;
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

        None
    }

    fn bus_id(&self) -> Vec<BusId> {
        vec![MEM_BUS_ID]
    }

    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
