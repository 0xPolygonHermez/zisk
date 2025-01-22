use crate::{
    MemHelpers, MemInput, MemModule, MemModuleSegmentCheckPoint, MemPreviousSegment,
    STEP_MEMORY_MAX_DIFF,
};
use data_bus::{BusDevice, BusId, MemBusData, MEM_BUS_ID};
use p3_field::PrimeField;
use proofman_common::{AirInstance, ProofCtx};
use proofman_util::{timer_start_debug, timer_stop_and_log_debug};
use sm_common::{CheckPoint, Instance, InstanceCtx, InstanceType};
use std::sync::Arc;

pub struct MemModuleInstance<F: PrimeField> {
    /// Binary Basic state machine
    mem_check_point: MemModuleSegmentCheckPoint,
    /// Instance context
    ictx: InstanceCtx,

    /// Collected inputs
    inputs: Vec<MemInput>,
    module: Arc<dyn MemModule<F>>,
}

impl<F: PrimeField> MemModuleInstance<F> {
    pub fn new(module: Arc<dyn MemModule<F>>, ictx: InstanceCtx) -> Self {
        let mem_check_point = ictx
            .plan
            .meta
            .as_ref()
            .unwrap()
            .downcast_ref::<MemModuleSegmentCheckPoint>()
            .unwrap()
            .clone();
        Self { ictx, inputs: Vec::new(), mem_check_point, module: module.clone() }
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

    fn discart_addr_step(&self, addr: u32, step: u64) -> bool {
        if addr < self.mem_check_point.prev_addr || addr > self.mem_check_point.last_addr {
            return true;
        }

        if addr == self.mem_check_point.prev_addr && step <= self.mem_check_point.prev_step {
            return true;
        }

        if addr == self.mem_check_point.last_addr && step > self.mem_check_point.last_step {
            return true;
        }

        false
    }
    fn filtered_inputs_push(&mut self, addr_w: u32, step: u64, is_write: bool, value: u64) {
        if !self.discart_addr_step(addr_w, step) {
            self.inputs.push(MemInput::new(addr_w, is_write, step, value));
        }
    }
    fn prepare_inputs(&mut self) {
        // sort all instance inputs
        timer_start_debug!(MEM_SORT);
        self.inputs.sort_by_key(|input| (input.addr, input.step));
        timer_stop_and_log_debug!(MEM_SORT);
    }
    fn fit_inputs_and_get_prev_segment(&mut self) -> MemPreviousSegment {
        let mut prev_segment = MemPreviousSegment {
            addr: self.mem_check_point.prev_addr,
            step: self.mem_check_point.prev_step,
            value: self.mem_check_point.prev_value,
        };
        #[cfg(feature = "debug_mem")]
        let initial = (self.inputs[0].addr, self.inputs[0].step, self.inputs.len());

        if self.mem_check_point.skip_rows > 0 {
            let mut input_index = 0;
            let mut skip_rows = 0;
            loop {
                while self.inputs[input_index].addr == prev_segment.addr &&
                    (self.inputs[input_index].step - prev_segment.step) > STEP_MEMORY_MAX_DIFF &&
                    skip_rows < self.mem_check_point.skip_rows as usize
                {
                    prev_segment.step += STEP_MEMORY_MAX_DIFF;
                    skip_rows += 1;
                }
                if skip_rows >= self.mem_check_point.skip_rows as usize {
                    break;
                }
                prev_segment.addr = self.inputs[input_index].addr;
                prev_segment.step = self.inputs[input_index].step;
                prev_segment.value = self.inputs[input_index].value;
                input_index += 1;
                skip_rows += 1;
            }
            self.inputs.drain(0..input_index);
        }
        #[cfg(feature = "debug_mem")]
        let original_inputs_len = self.inputs.len();

        self.inputs.truncate(self.mem_check_point.rows as usize);

        #[cfg(feature = "debug_mem")]
        println!(
            "[Mem:{}] #1 INPUT [0x{:X},{}] {} => [0x{:X},{}] {} => {} F [0x{:X},{},skip:{}]-[0x{:X},{}]",
            self.ictx.plan.segment_id.unwrap(),
            initial.0,
            initial.1,
            initial.2,
            self.inputs[0].addr,
            self.inputs[0].step,
            original_inputs_len,
            self.inputs.len(),
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
    fn compute_witness(&mut self, _pctx: &ProofCtx<F>) -> Option<AirInstance<F>> {
        if self.inputs.is_empty() {
            return None;
        }
        self.prepare_inputs();
        let prev_segment = self.fit_inputs_and_get_prev_segment();

        let segment_id = self.ictx.plan.segment_id.unwrap();
        Some(self.module.compute_witness(
            &self.inputs,
            segment_id,
            self.mem_check_point.is_last_segment,
            &prev_segment,
        ))
    }

    fn check_point(&self) -> CheckPoint {
        self.ictx.plan.check_point.clone()
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }
}

impl<F: PrimeField> BusDevice<u64> for MemModuleInstance<F> {
    fn process_data(&mut self, _bus_id: &BusId, data: &[u64]) -> (bool, Vec<(BusId, Vec<u64>)>) {
        // info!("MemModuleInstance process_data bus_id:{} len: {}", _bus_id, data.len());
        // info!(
        //     "MemModuleInstance process_data len: {:X},{:X},{:X},{:X},{:X}",
        //     data[0], data[1], data[2], data[3], data[4]
        // );
        assert!(*_bus_id == MEM_BUS_ID);

        let addr = MemBusData::get_addr(data);
        let step = MemBusData::get_step(data);
        let bytes = MemBusData::get_bytes(data);
        if !MemHelpers::is_aligned(addr, bytes) {
            self.process_unaligned_data(data);
            return (false, vec![]);
        }
        // info!("MemModuleInstance process_data addr: {:x} bytes: {:x}", addr, bytes);
        let addr_w = MemHelpers::get_addr_w(addr);
        let is_write = MemHelpers::is_write(MemBusData::get_op(data));
        if is_write {
            self.filtered_inputs_push(addr_w, step, true, MemBusData::get_value(data));
        } else {
            self.filtered_inputs_push(addr_w, step, false, MemBusData::get_mem_values(data)[0]);
        }

        (false, vec![])
    }

    fn bus_id(&self) -> Vec<BusId> {
        vec![MEM_BUS_ID]
    }
}
