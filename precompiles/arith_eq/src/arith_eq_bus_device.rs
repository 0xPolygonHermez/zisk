//! The `ArithEqCounter` module defines a counter for tracking arith_eq-related operations
//! sent over the data bus. It connects to the bus and gathers metrics for specific
//! `ZiskOperationType::ArithEq` instructions.

use std::ops::Add;

use zisk_common::{
    BusDevice, BusDeviceMode, BusId, Counter, Metrics, A, B, OPERATION_BUS_ARITH_256_DATA_SIZE,
    OPERATION_BUS_ARITH_256_MOD_DATA_SIZE, OPERATION_BUS_ID, OPERATION_BUS_SECP256K1_ADD_DATA_SIZE,
    OPERATION_BUS_SECP256K1_DBL_DATA_SIZE, OP_TYPE,
};
use zisk_core::ZiskOperationType;

use crate::mem_inputs::{
    generate_arith256_mem_inputs, generate_arith256_mod_mem_inputs,
    generate_secp256k1_add_mem_inputs, generate_secp256k1_dbl_mem_inputs,
};

/// The `ArithEqCounter` struct represents a counter that monitors and measures
/// arith_eq-related operations on the data bus.
///
/// It tracks specific operation types (`ZiskOperationType`) and updates counters for each
/// accepted operation type whenever data is processed on the bus.
pub struct ArithEqCounterInputGen {
    /// ArithEq counter.
    counter: Counter,

    /// Bus device mode (counter or input generator).
    mode: BusDeviceMode,
}

impl ArithEqCounterInputGen {
    /// Creates a new instance of `ArithEqCounter`.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus to which this counter is connected.
    /// * `op_type` - A vector of `ZiskOperationType` instructions to monitor.
    ///
    /// # Returns
    /// A new `ArithEqCounter` instance.
    pub fn new(mode: BusDeviceMode) -> Self {
        Self { counter: Counter::default(), mode }
    }

    /// Retrieves the count of instructions for a specific `ZiskOperationType`.
    ///
    /// # Arguments
    /// * `op_type` - The operation type to retrieve the count for.
    ///
    /// # Returns
    /// Returns the count of instructions for the specified operation type.
    pub fn inst_count(&self, op_type: ZiskOperationType) -> Option<u64> {
        (op_type == ZiskOperationType::ArithEq).then_some(self.counter.inst_count)
    }
}

impl Metrics for ArithEqCounterInputGen {
    /// Tracks activity on the connected bus and updates counters for recognized operations.
    ///
    /// # Arguments
    /// * `_bus_id` - The ID of the bus (unused in this implementation).
    /// * `_data` - The data received from the bus.
    ///
    /// # Returns
    /// An empty vector, as this implementation does not produce any derived inputs for the bus.
    #[inline(always)]
    fn measure(&mut self, _data: &[u64]) {
        self.counter.update(1);
    }

    /// Provides a dynamic reference for downcasting purposes.
    ///
    /// # Returns
    /// A reference to `self` as `dyn std::any::Any`.
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Add for ArithEqCounterInputGen {
    type Output = ArithEqCounterInputGen;

    /// Combines two `Arith256Counter` instances by summing their counters.
    ///
    /// # Arguments
    /// * `self` - The first `Arith256Counter` instance.
    /// * `other` - The second `Arith256Counter` instance.
    ///
    /// # Returns
    /// A new `Arith256Counter` with combined counters.
    fn add(self, other: Self) -> ArithEqCounterInputGen {
        ArithEqCounterInputGen { counter: &self.counter + &other.counter, mode: self.mode }
    }
}

impl BusDevice<u64> for ArithEqCounterInputGen {
    /// Processes data received on the bus, updating counters and generating inputs when applicable.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus sending the data.
    /// * `data` - The data received from the bus.
    ///
    /// # Returns
    /// A vector of derived inputs to be sent back to the bus.
    #[inline(always)]
    fn process_data(&mut self, bus_id: &BusId, data: &[u64]) -> Option<Vec<(BusId, Vec<u64>)>> {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        const ARITH_EQ: u64 = ZiskOperationType::ArithEq as u64;

        if data[OP_TYPE] != ARITH_EQ {
            return None;
        }

        let step_main = data[A];
        let addr_main = data[B] as u32;

        let only_counters = self.mode == BusDeviceMode::Counter;
        if only_counters {
            self.measure(data);
        }

        match data.len() {
            OPERATION_BUS_ARITH_256_DATA_SIZE => {
                generate_arith256_mem_inputs(addr_main, step_main, data, only_counters)
            }
            OPERATION_BUS_ARITH_256_MOD_DATA_SIZE => {
                generate_arith256_mod_mem_inputs(addr_main, step_main, data, only_counters)
            }
            OPERATION_BUS_SECP256K1_ADD_DATA_SIZE => {
                generate_secp256k1_add_mem_inputs(addr_main, step_main, data, only_counters)
            }
            OPERATION_BUS_SECP256K1_DBL_DATA_SIZE => {
                generate_secp256k1_dbl_mem_inputs(addr_main, step_main, data, only_counters)
            }

            _ => None,
        }
    }

    /// Returns the bus IDs associated with this counter.
    ///
    /// # Returns
    /// A vector containing the connected bus ID.
    fn bus_id(&self) -> Vec<BusId> {
        vec![OPERATION_BUS_ID]
    }

    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
