use crate::{
    MemHelpers, MemInput, MemModule, MemModuleSegmentCheckPoint, MemPreviousSegment,
    STEP_MEMORY_MAX_DIFF,
};
use data_bus::{BusDevice, BusId, MemBusData, PayloadType, MEM_BUS_ID};
use p3_field::PrimeField;
use proofman_common::{AirInstance, ProofCtx, SetupCtx};
use proofman_util::{timer_start_debug, timer_stop_and_log_debug};
use sm_common::{BusDeviceWrapper, CheckPoint, Instance, InstanceCtx, InstanceType};
use std::sync::Arc;

pub struct MemModuleInstance<F: PrimeField> {
    /// Instance context
    ictx: InstanceCtx,

    module: Arc<dyn MemModule<F>>,

    mem_check_point: MemModuleSegmentCheckPoint,
}

impl<F: PrimeField> MemModuleInstance<F> {
    pub fn new(module: Arc<dyn MemModule<F>>, ictx: InstanceCtx) -> Self {
        let meta = ictx.plan.meta.as_ref().unwrap();
        let mem_check_point = meta.downcast_ref::<MemModuleSegmentCheckPoint>().unwrap().clone();

        Self { ictx, module: module.clone(), mem_check_point }
    }

    fn prepare_inputs(&mut self, inputs: &mut [MemInput]) {
        // sort all instance inputs
        timer_start_debug!(MEM_SORT);
        inputs.sort_by_key(|input| (input.addr, input.step));
        timer_stop_and_log_debug!(MEM_SORT);
    }

    fn fit_inputs_and_get_prev_segment(
        &mut self,
        inputs: &mut Vec<MemInput>,
        mem_check_point: MemModuleSegmentCheckPoint,
        prev_last_value: u64,
    ) -> MemPreviousSegment {
        let mut prev_segment = MemPreviousSegment {
            addr: mem_check_point.prev_addr,
            step: mem_check_point.prev_step,
            value: mem_check_point.prev_value,
        };
        #[cfg(feature = "debug_mem")]
        let initial = (inputs[0].addr, inputs[0].step, inputs.len());

                match mem_check_point.skip_rows.cmp(&1) {
            std::cmp::Ordering::Equal => {
                prev_segment.value = prev_last_value;
            }
            std::cmp::Ordering::Greater => {
                let check_point_skip_rows = mem_check_point.skip_rows - 1;
                let mut input_index = 0;
                let mut skip_rows = 0;
                loop {
                    while inputs[input_index].addr == prev_segment.addr
                        && (inputs[input_index].step - prev_segment.step) > STEP_MEMORY_MAX_DIFF
                        && skip_rows < check_point_skip_rows as usize
                    {
                        prev_segment.step += STEP_MEMORY_MAX_DIFF;
                        skip_rows += 1;
                    }
                    if skip_rows >= check_point_skip_rows as usize {
                        break;
                    }
                    prev_segment.addr = inputs[input_index].addr;
                    prev_segment.step = inputs[input_index].step;
                    prev_segment.value = inputs[input_index].value;
                    input_index += 1;
                    skip_rows += 1;
                }
                inputs.drain(0..input_index);
            }
            std::cmp::Ordering::Less => {}
        }

        #[cfg(feature = "debug_mem")]
        let original_inputs_len = inputs.len();

        inputs.truncate(mem_check_point.rows as usize);

        #[cfg(feature = "debug_mem")]
        println!(
            "[Mem:{}] #1 INPUT [0x{:X},{}] {} => [0x{:X},{}] {} => {} F [0x{:X},{},skip:{}]-[0x{:X},{}]",
            self.ictx.plan.segment_id.unwrap(),
            initial.0,
            initial.1,
            initial.2,
            inputs[0].addr,
            inputs[0].step,
            original_inputs_len,
            inputs.len(),
            self.mem_check_point.prev_addr,
            self.mem_check_point.prev_step,
            self.mem_check_point.skip_rows,
            self.mem_check_point.last_addr,
            self.mem_check_point.last_step,
        );
        prev_segment
    }
}

impl<F: PrimeField> Instance<F> for MemModuleInstance<F> {
    fn compute_witness(
        &mut self,
        _pctx: &ProofCtx<F>,
        _sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<BusDeviceWrapper<PayloadType>>)>,
    ) -> Option<AirInstance<F>> {
        let mut prev_last_value = 0;
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, mut collector)| {
                let mem_module_collector = collector.detach_device().as_any().downcast::<MemModuleCollector>().unwrap();
                
                if mem_module_collector.prev_last_value != 0 {
                    prev_last_value = mem_module_collector.prev_last_value;
                }
                mem_module_collector.inputs
            })
            .collect();
        let mut inputs = inputs.into_iter().flatten().collect::<Vec<_>>();

        if inputs.is_empty() {
            return None;
        }

        self.prepare_inputs(&mut inputs);
        let prev_segment = self.fit_inputs_and_get_prev_segment(
            &mut inputs,
            self.mem_check_point.clone(),
            prev_last_value,
        );

        let segment_id = self.ictx.plan.segment_id.unwrap();

        let is_last_segment = self.mem_check_point.is_last_segment;
        Some(self.module.compute_witness(&inputs, segment_id, is_last_segment, &prev_segment))
    }

    /// Builds an input collector for the instance.
    ///
    /// # Arguments
    /// * `chunk_id` - The chunk ID associated with the input collector.
    ///
    /// # Returns
    /// An `Option` containing the input collector for the instance.
    fn build_inputs_collector(&self, _chunk_id: usize) -> Option<Box<dyn BusDevice<PayloadType>>> {
        Some(Box::new(MemModuleCollector::new(self.mem_check_point.clone())))
    }

    fn check_point(&self) -> CheckPoint {
        self.ictx.plan.check_point.clone()
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }
}

pub struct MemModuleCollector {
    /// Binary Basic state machine
    mem_check_point: MemModuleSegmentCheckPoint,

    /// Collected inputs
    inputs: Vec<MemInput>,

    prev_last_value: u64,
}

impl MemModuleCollector {
    pub fn new(mem_check_point: MemModuleSegmentCheckPoint) -> Self {
        Self { inputs: Vec::new(), mem_check_point, prev_last_value: 0 }
    }

    fn process_unaligned_data(&mut self, data: &[u64]) {
        let addr = MemBusData::get_addr(data);
        let addr_w = MemHelpers::get_addr_w(addr);
        let bytes = MemBusData::get_bytes(data);
        let is_write = MemHelpers::is_write(MemBusData::get_op(data));
        if MemHelpers::is_double(addr, bytes) {
            if is_write {
                self.process_unaligned_double_write(addr_w, bytes, data);
            } else {
                self.process_unaligned_double_read(addr_w, data);
            }
        } else if is_write {
            self.process_unaligned_single_write(addr_w, bytes, data);
        } else {
            self.process_unaligned_single_read(addr_w, data);
        }
    }

    fn process_unaligned_single_read(&mut self, addr_w: u32, data: &[u64]) {
        let value = MemBusData::get_mem_values(data)[0];
        let step = MemBusData::get_step(data);
        self.filtered_inputs_push(addr_w, step, false, value);
    }

    fn process_unaligned_single_write(&mut self, addr_w: u32, bytes: u8, data: &[u64]) {
        let read_values = MemBusData::get_mem_values(data);
        let write_values = MemHelpers::get_write_values(
            MemBusData::get_addr(data),
            bytes,
            MemBusData::get_value(data),
            read_values,
        );
        let step = MemBusData::get_step(data);
        self.filtered_inputs_push(addr_w, MemHelpers::get_read_step(step), false, read_values[0]);
        self.filtered_inputs_push(addr_w, MemHelpers::get_write_step(step), true, write_values[0]);
    }

    fn process_unaligned_double_read(&mut self, addr_w: u32, data: &[u64]) {
        let read_values = MemBusData::get_mem_values(data);
        let step = MemBusData::get_step(data);
        self.filtered_inputs_push(addr_w, step, false, read_values[0]);
        self.filtered_inputs_push(addr_w + 1, step, false, read_values[1]);
    }

    fn process_unaligned_double_write(&mut self, addr_w: u32, bytes: u8, data: &[u64]) {
        let read_values = MemBusData::get_mem_values(data);
        let write_values = MemHelpers::get_write_values(
            MemBusData::get_addr(data),
            bytes,
            MemBusData::get_value(data),
            read_values,
        );
        let step = MemBusData::get_step(data);
        let read_step = MemHelpers::get_read_step(step);
        let write_step = MemHelpers::get_write_step(step);

        // IMPORTANT: inputs must be ordered by step
        self.filtered_inputs_push(addr_w, read_step, false, read_values[0]);
        self.filtered_inputs_push(addr_w + 1, read_step, false, read_values[1]);

        self.filtered_inputs_push(addr_w, write_step, true, write_values[0]);
        self.filtered_inputs_push(addr_w + 1, write_step, true, write_values[1]);
    }

    fn discart_addr_step(&mut self, addr: u32, step: u64, value: u64) -> bool {
        if addr < self.mem_check_point.prev_addr || addr > self.mem_check_point.last_addr {
            return true;
        }

        if addr == self.mem_check_point.prev_addr {
            match step.cmp(&self.mem_check_point.prev_step) {
                std::cmp::Ordering::Less => {
                    return true;
                }
                std::cmp::Ordering::Equal => {
                    self.prev_last_value = value;
                    return true;
                }
                std::cmp::Ordering::Greater => {}
            }
        }

        if addr == self.mem_check_point.last_addr && step > self.mem_check_point.last_step {
            return true;
        }

        false
    }
    fn filtered_inputs_push(&mut self, addr_w: u32, step: u64, is_write: bool, value: u64) {
        if !self.discart_addr_step(addr_w, step, value) {
            self.inputs.push(MemInput::new(addr_w, is_write, step, value));
        }
    }
}

impl BusDevice<u64> for MemModuleCollector {
    fn process_data(&mut self, bus_id: &BusId, data: &[u64]) -> Option<Vec<(BusId, Vec<u64>)>> {
        debug_assert!(*bus_id == MEM_BUS_ID);

        // info!("MemModuleCollector process_data bus_id:{} len: {}", _bus_id, data.len());
        // info!(
        //     "MemModuleCollector process_data len: {:X},{:X},{:X},{:X},{:X}",
        //     data[0], data[1], data[2], data[3], data[4]
        // );
        assert!(*bus_id == MEM_BUS_ID);

        let addr = MemBusData::get_addr(data);
        let step = MemBusData::get_step(data);
        let bytes = MemBusData::get_bytes(data);
        if !MemHelpers::is_aligned(addr, bytes) {
            self.process_unaligned_data(data);
            return None;
        }
        // info!("MemModuleCollector process_data addr: {:x} bytes: {:x}", addr, bytes);
        let addr_w = MemHelpers::get_addr_w(addr);
        let is_write = MemHelpers::is_write(MemBusData::get_op(data));
        if is_write {
            self.filtered_inputs_push(addr_w, step, true, MemBusData::get_value(data));
        } else {
            self.filtered_inputs_push(addr_w, step, false, MemBusData::get_mem_values(data)[0]);
        }

        None
    }

    fn bus_id(&self) -> Vec<BusId> {
        vec![MEM_BUS_ID]
    }

    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
