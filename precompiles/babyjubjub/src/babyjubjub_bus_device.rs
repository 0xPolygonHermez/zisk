//! The `BabyJubJubCounterInputGen` module defines a counter for tracking babyjubjub-related
//! operations sent over the data bus. It connects to the bus and gathers metrics for specific
//! `ZiskOperationType::BabyJubJub` instructions.

use std::ops::Add;

use precompiles_common::MemProcessor;
use zisk_common::STEP;
use zisk_common::{
    BusDevice, BusDeviceMode, BusId, Counter, Metrics, B, OP, OPERATION_BUS_ID, OP_TYPE,
};
use zisk_core::{zisk_ops::ZiskOp, ZiskOperationType};

use crate::mem_inputs::{generate_babyjubjub_add_mem_inputs, skip_babyjubjub_add_mem_inputs};

const BABYJUBJUB_ADD_OP: u8 = ZiskOp::BabyJubJubAdd.code();

/// The `BabyJubJubCounterInputGen` struct represents a counter that monitors and measures
/// babyjubjub-related operations on the data bus.
pub struct BabyJubJubCounterInputGen {
    /// BabyJubJub counter.
    counter: Counter,

    /// Bus device mode (counter or input generator).
    mode: BusDeviceMode,
}

impl BabyJubJubCounterInputGen {
    /// Creates a new instance of `BabyJubJubCounterInputGen`.
    pub fn new(mode: BusDeviceMode) -> Self {
        Self { counter: Counter::default(), mode }
    }

    /// Retrieves the count of instructions for a specific `ZiskOperationType`.
    pub fn inst_count(&self, op_type: ZiskOperationType) -> Option<u64> {
        (op_type == ZiskOperationType::BabyJubJub).then_some(self.counter.inst_count)
    }

    fn skip_data<P: MemProcessor>(&self, data: &[u64], mem_processors: &mut P) -> bool {
        let addr_main = data[B] as u32;

        match data[OP] as u8 {
            BABYJUBJUB_ADD_OP => skip_babyjubjub_add_mem_inputs(addr_main, data, mem_processors),
            _ => {
                panic!("BabyJubJubCounterInputGen: Unsupported data length {}", data.len(),);
            }
        }
    }

    /// Processes data received on the bus, updating counters and generating inputs when applicable.
    #[inline(always)]
    pub fn process_data<P: MemProcessor>(
        &mut self,
        bus_id: &BusId,
        data: &[u64],
        mem_processors: &mut P,
    ) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        const BABYJUBJUB: u64 = ZiskOperationType::BabyJubJub as u64;

        if data[OP_TYPE] != BABYJUBJUB {
            return true;
        }

        let op = data[OP] as u8;
        let step_main = data[STEP];
        let addr_main = data[B] as u32;

        let only_counters = match self.mode {
            BusDeviceMode::Counter => {
                self.measure(data);
                true
            }
            BusDeviceMode::CounterAsm => {
                self.measure(data);
                return true;
            }
            BusDeviceMode::InputGenerator => {
                if self.skip_data(data, mem_processors) {
                    return true;
                }
                false
            }
        };

        match op {
            BABYJUBJUB_ADD_OP => {
                generate_babyjubjub_add_mem_inputs(
                    addr_main,
                    step_main,
                    data,
                    only_counters,
                    mem_processors,
                );
            }
            _ => {
                panic!("BabyJubJubCounterInputGen: Unsupported data length {}", data.len(),);
            }
        }

        true
    }
}

impl Metrics for BabyJubJubCounterInputGen {
    #[inline(always)]
    fn measure(&mut self, _data: &[u64]) {
        self.counter.update(1);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Add for BabyJubJubCounterInputGen {
    type Output = BabyJubJubCounterInputGen;

    fn add(self, other: Self) -> BabyJubJubCounterInputGen {
        BabyJubJubCounterInputGen { counter: &self.counter + &other.counter, mode: self.mode }
    }
}

impl BusDevice<u64> for BabyJubJubCounterInputGen {
    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
