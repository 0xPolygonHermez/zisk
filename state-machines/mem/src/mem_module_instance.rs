use crate::{MemHelpers, MemInput, MemInstanceCheckPoint, MemModule, MemPreviousSegment};
use log::info;
use p3_field::PrimeField;
use proofman_common::{AirInstance, ProofCtx};
use proofman_util::{timer_start_debug, timer_stop_and_log_debug};
use sm_common::{CheckPoint, Instance, InstanceCtx, InstanceType};
use std::sync::Arc;
use zisk_common::{BusDevice, BusId, MemBusData, MEM_BUS_ID};

pub struct MemModuleInstance<F: PrimeField> {
    /// Binary Basic state machine
    mem_check_point: MemInstanceCheckPoint,
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
            .downcast_ref::<MemInstanceCheckPoint>()
            .unwrap()
            .clone();
        Self { ictx, inputs: Vec::new(), mem_check_point, module }
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
        self.filtered_inputs_push(addr_w + 1, step, true, read_values[1]);
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

        if addr == self.mem_check_point.prev_addr && step < self.mem_check_point.prev_step {
            return true;
        }

        if addr == self.mem_check_point.last_addr && step > self.mem_check_point.last_step {
            return true;
        }

        true
    }
    fn filtered_inputs_push(&mut self, addr_w: u32, step: u64, is_write: bool, value: u64) {
        if !self.discart_addr_step(addr_w, step) {
            self.inputs.push(MemInput::new(addr_w, is_write, step, value));
        }
    }
    fn prepare_inputs(&mut self) {
        // sort all instance inputs
        timer_start_debug!(MEM_SORT);
        self.inputs.sort_by_key(|input| input.addr);
        timer_stop_and_log_debug!(MEM_SORT);
    }
}

impl<F: PrimeField> Instance<F> for MemModuleInstance<F> {
    fn compute_witness(&mut self, _pctx: &ProofCtx<F>) -> Option<AirInstance<F>> {
        let prev_segment = MemPreviousSegment {
            addr: self.mem_check_point.prev_addr,
            step: self.mem_check_point.prev_step,
            value: self.mem_check_point.prev_value,
        };

        self.prepare_inputs();

        Some(self.module.prove_instance(
            &self.inputs,
            0,
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
    fn bus_id(&self) -> Vec<BusId> {
        vec![MEM_BUS_ID]
    }
}

impl<F: PrimeField> BusDevice<u64> for MemModuleInstance<F> {
    fn process_data(&mut self, _bus_id: &BusId, data: &[u64]) -> (bool, Vec<(BusId, Vec<u64>)>) {
        info!("MemModuleInstance process_data bus_id:{} len: {}", _bus_id, data.len());
        info!(
            "MemModuleInstance process_data len: {:X},{:X},{:X},{:X},{:X}",
            data[0], data[1], data[2], data[3], data[4]
        );
        if *_bus_id != MEM_BUS_ID {
            return (false, vec![])
        }

        let addr = MemBusData::get_addr(data);
        let bytes = MemBusData::get_bytes(data);
        if !MemHelpers::is_aligned(addr, bytes) {
            self.process_unaligned_data(data);
            return (false, vec![])
        }
        info!("MemModuleInstance process_data addr: {:x} bytes: {:x}", addr, bytes);
        let addr_w = MemHelpers::get_addr_w(addr);
        let step = MemBusData::get_step(data);
        let is_write = MemHelpers::is_write(MemBusData::get_op(data));
        if is_write {
            self.filtered_inputs_push(addr_w, step, true, MemBusData::get_value(data));
        } else {
            self.filtered_inputs_push(addr_w, step, false, MemBusData::get_mem_values(data)[0]);
        }

        (false, vec![])
    }
}
