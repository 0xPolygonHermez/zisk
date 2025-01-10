use crate::{MemAlignCheckPoint, MemAlignInput, MemAlignSM, MemHelpers};
use log::info;
use p3_field::PrimeField;
use proofman_common::{AirInstance, ProofCtx};
use sm_common::{CheckPoint, Instance, InstanceCtx, InstanceType};
use std::sync::Arc;
use zisk_common::{BusDevice, BusId, MemBusData};

pub struct MemAlignInstance<F: PrimeField> {
    checkpoint: MemAlignCheckPoint,
    /// Instance context
    ictx: InstanceCtx,

    /// Collected inputs
    inputs: Vec<MemAlignInput>,
    mem_align_sm: Arc<MemAlignSM<F>>,
    pending_count: u32,
    skip_pending: u32,
}

impl<F: PrimeField> MemAlignInstance<F> {
    pub fn new(mem_align_sm: Arc<MemAlignSM<F>>, ictx: InstanceCtx) -> Self {
        info!("[Mem]   MemAlignInstance");

        let checkpoint =
            ictx.plan.meta.as_ref().unwrap().downcast_ref::<MemAlignCheckPoint>().unwrap().clone();

        Self {
            ictx,
            inputs: Vec::new(),
            mem_align_sm,
            checkpoint: checkpoint.clone(),
            skip_pending: checkpoint.skip,
            pending_count: checkpoint.count,
        }
    }
}

impl<F: PrimeField> Instance<F> for MemAlignInstance<F> {
    fn compute_witness(&mut self, _pctx: &ProofCtx<F>) -> Option<AirInstance<F>> {
        Some(self.mem_align_sm.prove_instance(&self.inputs, self.checkpoint.rows))
    }

    fn check_point(&self) -> CheckPoint {
        self.ictx.plan.check_point.clone()
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }
}

impl<F: PrimeField> BusDevice<u64> for MemAlignInstance<F> {
    fn process_data(&mut self, _bus_id: &BusId, data: &[u64]) -> (bool, Vec<(BusId, Vec<u64>)>) {
        let addr = MemBusData::get_addr(data);
        let bytes = MemBusData::get_bytes(data);
        if !MemHelpers::is_aligned(addr, bytes) {
            return (false, vec![])
        }
        if self.skip_pending > 0 {
            self.skip_pending -= 1;
            return (false, vec![])
        }

        if self.pending_count == 0 {
            return (true, vec![])
        }
        self.pending_count -= 1;
        self.inputs.push(MemAlignInput {
            addr: MemBusData::get_addr(data),
            is_write: MemHelpers::is_write(MemBusData::get_op(data)),
            width: MemBusData::get_bytes(data),
            step: MemBusData::get_step(data),
            value: MemBusData::get_value(data),
            mem_values: MemBusData::get_mem_values(data),
        });

        (false, vec![])
    }
}
