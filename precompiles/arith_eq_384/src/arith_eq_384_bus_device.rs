//! The `ArithEq384Counter` module defines a counter for tracking arith_eq_384-related operations
//! sent over the data bus. It connects to the bus and gathers metrics for specific
//! `ZiskOperationType::ArithEq384` instructions.

use std::{collections::VecDeque, ops::Add};

use zisk_common::{
    BusDevice, BusDeviceMode, BusId, Counter, MemCollectorInfo, Metrics, B, OP, OPERATION_BUS_ID,
    OP_TYPE, STEP,
};
use zisk_core::{zisk_ops::ZiskOp, ZiskOperationType};

use crate::mem_inputs::{
    generate_arith384_mod_mem_inputs, generate_bls12_381_complex_add_mem_inputs,
    generate_bls12_381_complex_mul_mem_inputs, generate_bls12_381_complex_sub_mem_inputs,
    generate_bls12_381_curve_add_mem_inputs, generate_bls12_381_curve_dbl_mem_inputs,
    skip_arith384_mod_mem_inputs, skip_bls12_381_complex_add_mem_inputs,
    skip_bls12_381_complex_mul_mem_inputs, skip_bls12_381_complex_sub_mem_inputs,
    skip_bls12_381_curve_add_mem_inputs, skip_bls12_381_curve_dbl_mem_inputs,
};

const ARITH384_MOD_OP: u8 = ZiskOp::Arith384Mod.code();
const BLS12_381_CURVE_ADD_OP: u8 = ZiskOp::Bls12_381CurveAdd.code();
const BLS12_381_CURVE_DBL_OP: u8 = ZiskOp::Bls12_381CurveDbl.code();
const BLS12_381_COMPLEX_ADD_OP: u8 = ZiskOp::Bls12_381ComplexAdd.code();
const BLS12_381_COMPLEX_SUB_OP: u8 = ZiskOp::Bls12_381ComplexSub.code();
const BLS12_381_COMPLEX_MUL_OP: u8 = ZiskOp::Bls12_381ComplexMul.code();

/// The `ArithEq384Counter` struct represents a counter that monitors and measures
/// arith_eq_384-related operations on the data bus.
///
/// It tracks specific operation types (`ZiskOperationType`) and updates counters for each
/// accepted operation type whenever data is processed on the bus.
pub struct ArithEq384CounterInputGen {
    /// ArithEq384 counter.
    counter: Counter,

    /// Bus device mode (counter or input generator).
    mode: BusDeviceMode,
}

impl ArithEq384CounterInputGen {
    /// Creates a new instance of `ArithEq384Counter`.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus to which this counter is connected.
    /// * `op_type` - A vector of `ZiskOperationType` instructions to monitor.
    ///
    /// # Returns
    /// A new `ArithEq384Counter` instance.
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
        (op_type == ZiskOperationType::ArithEq384).then_some(self.counter.inst_count)
    }

    fn skip_data(&self, data: &[u64], mem_collectors_info: &[MemCollectorInfo]) -> bool {
        let addr_main = data[B] as u32;

        match data[OP] as u8 {
            ARITH384_MOD_OP => skip_arith384_mod_mem_inputs(addr_main, data, mem_collectors_info),
            BLS12_381_CURVE_ADD_OP => {
                skip_bls12_381_curve_add_mem_inputs(addr_main, data, mem_collectors_info)
            }
            BLS12_381_CURVE_DBL_OP => {
                skip_bls12_381_curve_dbl_mem_inputs(addr_main, data, mem_collectors_info)
            }
            BLS12_381_COMPLEX_ADD_OP => {
                skip_bls12_381_complex_add_mem_inputs(addr_main, data, mem_collectors_info)
            }
            BLS12_381_COMPLEX_SUB_OP => {
                skip_bls12_381_complex_sub_mem_inputs(addr_main, data, mem_collectors_info)
            }
            BLS12_381_COMPLEX_MUL_OP => {
                skip_bls12_381_complex_mul_mem_inputs(addr_main, data, mem_collectors_info)
            }
            _ => {
                panic!("ArithEq384CounterInputGen: Unsupported data length {}", data.len());
            }
        }
    }
}

impl Metrics for ArithEq384CounterInputGen {
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

impl Add for ArithEq384CounterInputGen {
    type Output = ArithEq384CounterInputGen;

    /// Combines two `Arith384Counter` instances by summing their counters.
    ///
    /// # Arguments
    /// * `self` - The first `Arith384Counter` instance.
    /// * `other` - The second `Arith384Counter` instance.
    ///
    /// # Returns
    /// A new `Arith384Counter` with combined counters.
    fn add(self, other: Self) -> ArithEq384CounterInputGen {
        ArithEq384CounterInputGen { counter: &self.counter + &other.counter, mode: self.mode }
    }
}

impl BusDevice<u64> for ArithEq384CounterInputGen {
    /// Processes data received on the bus, updating counters and generating inputs when applicable.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus sending the data.
    /// * `data` - The data received from the bus.
    /// * `pending` – A queue of pending bus operations used to send derived inputs.
    ///
    /// # Returns
    /// A boolean indicating whether the program should continue execution or terminate.
    /// Returns `true` to continue execution, `false` to stop.
    #[inline(always)]
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[u64],
        pending: &mut VecDeque<(BusId, Vec<u64>)>,
        mem_collector_info: Option<&[MemCollectorInfo]>,
    ) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        const ARITH_EQ_384: u64 = ZiskOperationType::ArithEq384 as u64;

        if data[OP_TYPE] != ARITH_EQ_384 {
            return true;
        }

        if let Some(mem_collectors_info) = mem_collector_info {
            if self.skip_data(data, mem_collectors_info) {
                return true;
            }
        }

        let op = data[OP] as u8;
        let step_main = data[STEP];
        let addr_main = data[B] as u32;

        let only_counters = self.mode == BusDeviceMode::Counter;
        if only_counters {
            self.measure(data);
        }

        match op {
            ARITH384_MOD_OP => {
                generate_arith384_mod_mem_inputs(
                    addr_main,
                    step_main,
                    data,
                    only_counters,
                    pending,
                );
            }
            BLS12_381_CURVE_ADD_OP => {
                generate_bls12_381_curve_add_mem_inputs(
                    addr_main,
                    step_main,
                    data,
                    only_counters,
                    pending,
                );
            }
            BLS12_381_CURVE_DBL_OP => {
                generate_bls12_381_curve_dbl_mem_inputs(
                    addr_main,
                    step_main,
                    data,
                    only_counters,
                    pending,
                );
            }
            BLS12_381_COMPLEX_ADD_OP => {
                generate_bls12_381_complex_add_mem_inputs(
                    addr_main,
                    step_main,
                    data,
                    only_counters,
                    pending,
                );
            }
            BLS12_381_COMPLEX_SUB_OP => {
                generate_bls12_381_complex_sub_mem_inputs(
                    addr_main,
                    step_main,
                    data,
                    only_counters,
                    pending,
                );
            }
            BLS12_381_COMPLEX_MUL_OP => {
                generate_bls12_381_complex_mul_mem_inputs(
                    addr_main,
                    step_main,
                    data,
                    only_counters,
                    pending,
                );
            }
            _ => {
                panic!("ArithEq384CounterInputGen: Unsupported data length {}", data.len());
            }
        }

        true
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
