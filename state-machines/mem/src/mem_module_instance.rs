use crate::{
    mem_module_collector::MemModuleCollector, MemHelpers, MemInput, MemModule,
    MemModuleSegmentCheckPoint, MemPreviousSegment, STEP_MEMORY_MAX_DIFF,
};
use data_bus::{BusDevice, PayloadType};
use p3_field::PrimeField;
use proofman_common::{AirInstance, ProofCtx, SetupCtx};
use proofman_util::{timer_start_debug, timer_stop_and_log_debug};
use sm_common::{BusDeviceWrapper, CheckPoint, Instance, InstanceCtx, InstanceType};
use std::{cmp::min, sync::Arc};
use zisk_common::ChunkId;

pub struct MemModuleInstance<F: PrimeField> {
    /// Instance context
    ictx: InstanceCtx,

    module: Arc<dyn MemModule<F>>,

    check_point: MemModuleSegmentCheckPoint,
    min_addr: u32,
    #[allow(dead_code)]
    max_addr: u32,
    limited_step_distance: bool,
}

impl<F: PrimeField> MemModuleInstance<F> {
    pub fn new(
        module: Arc<dyn MemModule<F>>,
        ictx: InstanceCtx,
        limited_step_distance: bool,
    ) -> Self {
        let meta = ictx.plan.meta.as_ref().unwrap();
        let mem_check_point = meta.downcast_ref::<MemModuleSegmentCheckPoint>().unwrap().clone();

        let (min_addr, max_addr) = module.get_addr_range();
        Self {
            ictx,
            module: module.clone(),
            check_point: mem_check_point,
            min_addr,
            max_addr,
            limited_step_distance,
        }
    }

    fn prepare_inputs(&mut self, inputs: &mut [MemInput]) {
        // sort all instance inputs
        timer_start_debug!(MEM_SORT);
        inputs.sort_by_key(|input| (input.addr, input.step));
        timer_stop_and_log_debug!(MEM_SORT);
    }

    /// This method calculates intermediate accesses without adding inputs and trims
    /// the inputs while considering skipped rows for this instance.
    ///
    /// Additionally, it computes the necessary information for memory continuations.
    /// It returns the previous segment information.
    ///
    /// # Arguments
    /// * `inputs` - The inputs to be processed.
    /// * `mem_check_point` - The memory check point.
    /// * `prev_last_value` - The previous last value.
    ///
    /// # Returns
    /// The previous segment information.
    fn fit_inputs_and_get_prev_segment(
        &mut self,
        inputs: &mut Vec<MemInput>,
        prev_segment: &mut MemPreviousSegment,
        skip_rows: u32,
    ) {
        // println!(
        //     "[Mem:{}] #1 INPUT [0x{:X},{}] {} LV:{:?} S:{}]",
        //     self.ictx.plan.segment_id.unwrap(),
        //     inputs[0].addr * 8,
        //     inputs[0].step,
        //     inputs.len(),
        //     prev_segment,
        //     skip_rows,
        // );

        if skip_rows > 0 && self.limited_step_distance {
            // at this point skip only affects to the intermediate steps, because address
            // skips was resolved previously.

            let last_step = prev_segment.step;
            let step = inputs[0].step;

            if step < last_step {
                for (index, input) in
                    inputs.iter().take(min(20, skip_rows as usize) + 1).enumerate()
                {
                    println!("input[{}]:{:?}", index, input);
                }
                panic!(
                        "MemModuleInstance: step({}) < last_step ({}) skip_rows:{} input[0]:{:?} prev_segment:{:?}",
                        step, last_step, skip_rows, inputs[0], prev_segment
                    );
            }
            if let Some((full_rows, zero_row)) = MemHelpers::get_intermediate_rows(last_step, step)
            {
                if skip_rows <= full_rows as u32 {
                    prev_segment.step = last_step + skip_rows as u64 * STEP_MEMORY_MAX_DIFF;
                } else if skip_rows == (full_rows + zero_row) as u32 {
                    prev_segment.step = last_step + full_rows as u64 * STEP_MEMORY_MAX_DIFF;
                } else {
                    panic!("Invalid skip rows {} > {}", skip_rows, full_rows + zero_row);
                }
            } else {
                panic!(
                    "Expected intermediate rows steps({},{}) prev_segment:{:?}",
                    last_step, step, prev_segment
                );
            }
        }
    }
}

impl<F: PrimeField> Instance<F> for MemModuleInstance<F> {
    fn compute_witness(
        &mut self,
        _pctx: &ProofCtx<F>,
        _sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<BusDeviceWrapper<PayloadType>>)>,
    ) -> Option<AirInstance<F>> {
        // Collect inputs from all collectors. At most, one of them has `prev_last_value` non zero,
        // we take this `prev_last_value`, which represents the last value of the previous segment.

        // let mut last_value = MemLastValue::new(SegmentId(0), 0, 0);
        let mut prev_segment: Option<MemPreviousSegment> = None;
        let mut intermediate_skip: Option<u32> = None;
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, mut collector)| {
                let mem_module_collector =
                    collector.detach_device().as_any().downcast::<MemModuleCollector>().unwrap();

                if mem_module_collector.prev_segment.is_some() {
                    assert!(prev_segment.is_none());
                    prev_segment = mem_module_collector.prev_segment;
                }
                if mem_module_collector.mem_check_point.intermediate_skip.is_some() {
                    assert!(intermediate_skip.is_none());
                    intermediate_skip = mem_module_collector.mem_check_point.intermediate_skip;
                }
                mem_module_collector.inputs
            })
            .collect();
        let mut inputs = inputs.into_iter().flatten().collect::<Vec<_>>();

        if inputs.is_empty() {
            return None;
        }

        // This method sorts all inputs
        self.prepare_inputs(&mut inputs);

        // This method calculates intermediate accesses without adding inputs and trims
        // the inputs while considering skipped rows for this instance.
        // Additionally, it computes the necessary information for memory continuations.
        let skip_rows = intermediate_skip.unwrap_or(0);
        let mut prev_segment =
            prev_segment.unwrap_or(MemPreviousSegment { addr: self.min_addr, step: 0, value: 0 });

        self.fit_inputs_and_get_prev_segment(&mut inputs, &mut prev_segment, skip_rows);

        // Extract segment id from instance context
        let segment_id = self.ictx.plan.segment_id.unwrap();

        let is_last_segment = self.check_point.is_last_segment;
        Some(self.module.compute_witness(&inputs, segment_id, is_last_segment, &prev_segment))
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
            &chunk_check_point,
            self.min_addr,
            self.ictx.plan.segment_id.unwrap(),
        )))
    }

    fn check_point(&self) -> CheckPoint {
        self.ictx.plan.check_point.clone()
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }
}
