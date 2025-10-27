use crate::{mem_module_collector::MemModuleCollector, MemInput, MemModule, MemPreviousSegment};
use fields::PrimeField64;
use mem_common::MemModuleSegmentCheckPoint;
use proofman_common::{AirInstance, ProofCtx, SetupCtx};
use proofman_util::{timer_start_debug, timer_stop_and_log_debug};
use rayon::prelude::*;
use std::sync::Arc;
use zisk_common::{
    BusDevice, CheckPoint, ChunkId, Instance, InstanceCtx, InstanceType, PayloadType,
};
use zisk_pil::MemTrace;

pub struct MemModuleInstance<F: PrimeField64> {
    /// Instance context
    ictx: InstanceCtx,

    module: Arc<dyn MemModule<F>>,

    check_point: MemModuleSegmentCheckPoint,
    min_addr: u32,
    #[allow(dead_code)]
    max_addr: u32,
}

impl<F: PrimeField64> MemModuleInstance<F> {
    pub fn new(module: Arc<dyn MemModule<F>>, ictx: InstanceCtx) -> Self {
        let meta = ictx.plan.meta.as_ref().unwrap();
        let mem_check_point = meta.downcast_ref::<MemModuleSegmentCheckPoint>().unwrap().clone();

        let (min_addr, max_addr) = module.get_addr_range();
        Self { ictx, module: module.clone(), check_point: mem_check_point, min_addr, max_addr }
    }

    fn prepare_inputs(&self, inputs: &mut [MemInput], parallelize: bool) {
        // sort all instance inputs
        timer_start_debug!(MEM_SORT);
        if parallelize {
            inputs.par_sort_by_key(|input| (input.addr, input.step));
        } else {
            inputs.sort_by_key(|input| (input.addr, input.step));
        }
        timer_stop_and_log_debug!(MEM_SORT);
    }

    pub fn build_mem_collector(&self, chunk_id: ChunkId) -> MemModuleCollector {
        let chunk_check_point = self.check_point.chunks.get(&chunk_id).unwrap();
        MemModuleCollector::new(
            chunk_check_point,
            self.min_addr,
            self.ictx.plan.segment_id.unwrap(),
            Some(chunk_id) == self.check_point.first_chunk_id,
            self.module.is_dual(),
        )
    }
}

impl<F: PrimeField64> Instance<F> for MemModuleInstance<F> {
    fn compute_witness(
        &self,
        _pctx: &ProofCtx<F>,
        _sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
        trace_buffer: Vec<F>,
    ) -> Option<AirInstance<F>> {
        // Collect inputs from all collectors. At most, one of them has `prev_last_value` non zero,
        // we take this `prev_last_value`, which represents the last value of the previous segment.

        // let mut last_value = MemLastValue::new(SegmentId(0), 0, 0);
        let mut prev_segment: Option<MemPreviousSegment> = None;
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, collector)| {
                let mem_module_collector =
                    collector.as_any().downcast::<MemModuleCollector>().unwrap();

                if mem_module_collector.prev_segment.is_some() {
                    assert!(prev_segment.is_none());
                    prev_segment = mem_module_collector.prev_segment;
                }
                mem_module_collector.inputs
            })
            .collect();
        let mut inputs = inputs.into_iter().flatten().collect::<Vec<_>>();

        if inputs.is_empty() {
            return None;
        }

        // This method sorts all inputs
        let parallelize = self.ictx.plan.air_id == MemTrace::<F>::AIR_ID
            && self.ictx.plan.airgroup_id == MemTrace::<F>::AIRGROUP_ID;
        self.prepare_inputs(&mut inputs, parallelize);

        // This method calculates intermediate accesses without adding inputs and trims
        // the inputs while considering skipped rows for this instance.
        // Additionally, it computes the necessary information for memory continuations.
        let prev_segment =
            prev_segment.unwrap_or(MemPreviousSegment { addr: self.min_addr, step: 0, value: 0 });

        // Extract segment id from instance context
        let segment_id = self.ictx.plan.segment_id.unwrap();

        let is_last_segment = self.check_point.is_last_segment;
        Some(self.module.compute_witness(
            &inputs,
            segment_id,
            is_last_segment,
            &prev_segment,
            trace_buffer,
        ))
    }

    /// Builds an input collector for the instance.
    ///
    /// # Arguments
    /// * `chunk_id` - The chunk ID associated with the input collector.
    ///
    /// # Returns
    /// An `Option` containing the input collector for the instance.
    fn build_inputs_collector(&self, chunk_id: ChunkId) -> Option<Box<dyn BusDevice<PayloadType>>> {
        let chunk_check_point = self.check_point.chunks.get(&chunk_id).unwrap();
        Some(Box::new(MemModuleCollector::new(
            chunk_check_point,
            self.min_addr,
            self.ictx.plan.segment_id.unwrap(),
            Some(chunk_id) == self.check_point.first_chunk_id,
            self.module.is_dual(),
        )))
    }

    fn check_point(&self) -> &CheckPoint {
        &self.ictx.plan.check_point
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
