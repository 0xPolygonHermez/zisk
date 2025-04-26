use crate::{
    MemHelpers, MemInput, MemModule, MemModuleCheckPoint, MemModuleSegmentCheckPoint,
    MemPreviousSegment, STEP_MEMORY_MAX_DIFF,
};
use data_bus::{BusDevice, BusId, MemBusData, PayloadType, MEM_BUS_ID};
use p3_field::PrimeField;
use proofman_common::{AirInstance, ProofCtx, SetupCtx};
use proofman_util::{timer_start_debug, timer_stop_and_log_debug};
use sm_common::{BusDeviceWrapper, CheckPoint, Instance, InstanceCtx, InstanceType};
use std::ops::Add;
use std::sync::Arc;
use zisk_common::{ChunkId, SegmentId};

#[derive(Debug, Clone, Copy)]
pub struct MemLastValue {
    pub segment_id: SegmentId,
    pub checkpoint_addr: u32,
    pub checkpoint_step: u64,
    pub value: u64,
    pub step: u64,
    pub addr: u32,
    pub is_set: bool,
}

impl MemLastValue {
    pub fn new(segment_id: SegmentId, checkpoint_addr: u32, checkpoint_step: u64) -> Self {
        Self {
            segment_id,
            checkpoint_addr,
            checkpoint_step,
            value: 0,
            step: 0,
            addr: 0,
            is_set: false,
        }
    }
    pub fn set_once(&mut self, value: u64, addr_w: u32, step: u64) {
        if self.is_set {
            return;
        }
        self.set(value, addr_w, step);
    }
    pub fn update(&mut self, value: u64, addr_w: u32, step: u64) {
        if addr_w > self.checkpoint_addr {
            return;
        }
        #[allow(clippy::comparison_chain)]
        if addr_w > self.addr {
            if addr_w < self.checkpoint_addr || step <= self.checkpoint_step {
                println!(
                    "[MemLastValue] update1({}, 0x{:X},{}) [C:0x{:X},{} 0x{:X},{}]",
                    value,
                    addr_w * 8,
                    step,
                    self.checkpoint_addr * 8,
                    self.checkpoint_step,
                    self.addr * 8,
                    self.step
                );
                self.set(value, addr_w, step);
            }
        } else if addr_w == self.addr && step > self.step && step <= self.checkpoint_step {
            println!(
                "[MemLastValue] update2({}, 0x{:X},{}) [C:0x{:X},{} 0x{:X},{}]",
                value,
                addr_w * 8,
                step,
                self.checkpoint_addr * 8,
                self.checkpoint_step,
                self.addr * 8,
                self.step
            );
            self.set(value, addr_w, step);
        }
    }
    pub fn set(&mut self, value: u64, addr_w: u32, step: u64) {
        self.is_set = true;
        self.value = value;
        self.step = step;
        self.addr = addr_w;
    }
}

impl Add for MemLastValue {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        if self.checkpoint_addr != 0 {
            assert_eq!(self.checkpoint_addr, other.checkpoint_addr);
            assert_eq!(self.checkpoint_step, other.checkpoint_step);
            assert_eq!(self.segment_id, other.segment_id);
        }
        if self.addr > other.addr || (self.addr == other.addr && self.step > other.step) {
            self
        } else {
            other
        }
    }
}
