//! The `ArithEqCounter` module defines a counter for tracking arith_eq-related operations
//! sent over the data bus. It connects to the bus and gathers metrics for specific
//! `ZiskOperationType::ArithEq` instructions.

use std::{collections::VecDeque, ops::Add};

use zisk_common::{
    BusDevice, BusDeviceMode, BusId, Counter, Metrics, A, B, OP, OPERATION_BUS_ID, OP_TYPE,
};
use zisk_core::{zisk_ops::ZiskOp, ZiskOperationType};

use crate::mem_inputs::{
    generate_arith256_mem_inputs, generate_arith256_mod_mem_inputs,
    generate_bn254_complex_add_mem_inputs, generate_bn254_complex_mul_mem_inputs,
    generate_bn254_complex_sub_mem_inputs, generate_bn254_curve_add_mem_inputs,
    generate_bn254_curve_dbl_mem_inputs, generate_secp256k1_add_mem_inputs,
    generate_secp256k1_dbl_mem_inputs,
};

const ARITH256_OP: u8 = ZiskOp::Arith256.code();
const ARITH256_MOD_OP: u8 = ZiskOp::Arith256Mod.code();
const SECP256K1_ADD_OP: u8 = ZiskOp::Secp256k1Add.code();
const SECP256K1_DBL_OP: u8 = ZiskOp::Secp256k1Dbl.code();
const BN254_CURVE_ADD_OP: u8 = ZiskOp::Bn254CurveAdd.code();
const BN254_CURVE_DBL_OP: u8 = ZiskOp::Bn254CurveDbl.code();
const BN254_COMPLEX_ADD_OP: u8 = ZiskOp::Bn254ComplexAdd.code();
const BN254_COMPLEX_SUB_OP: u8 = ZiskOp::Bn254ComplexSub.code();
const BN254_COMPLEX_MUL_OP: u8 = ZiskOp::Bn254ComplexMul.code();

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
    ) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        const ARITH_EQ: u64 = ZiskOperationType::ArithEq as u64;

        if data[OP_TYPE] != ARITH_EQ {
            return true;
        }

        let op = data[OP] as u8;
        let step_main = data[A];
        let addr_main = data[B] as u32;

        let only_counters = self.mode == BusDeviceMode::Counter;
        if only_counters {
            self.measure(data);
        }

        match op {
            ARITH256_OP => {
                pending.extend(generate_arith256_mem_inputs(
                    addr_main,
                    step_main,
                    data,
                    only_counters,
                ));
            }
            ARITH256_MOD_OP => {
                pending.extend(generate_arith256_mod_mem_inputs(
                    addr_main,
                    step_main,
                    data,
                    only_counters,
                ));
            }
            SECP256K1_ADD_OP => {
                pending.extend(generate_secp256k1_add_mem_inputs(
                    addr_main,
                    step_main,
                    data,
                    only_counters,
                ));
            }
            SECP256K1_DBL_OP => {
                pending.extend(generate_secp256k1_dbl_mem_inputs(
                    addr_main,
                    step_main,
                    data,
                    only_counters,
                ));
            }
            BN254_CURVE_ADD_OP => {
                pending.extend(generate_bn254_curve_add_mem_inputs(
                    addr_main,
                    step_main,
                    data,
                    only_counters,
                ));
            }
            BN254_CURVE_DBL_OP => {
                pending.extend(generate_bn254_curve_dbl_mem_inputs(
                    addr_main,
                    step_main,
                    data,
                    only_counters,
                ));
            }
            BN254_COMPLEX_ADD_OP => {
                pending.extend(generate_bn254_complex_add_mem_inputs(
                    addr_main,
                    step_main,
                    data,
                    only_counters,
                ));
            }
            BN254_COMPLEX_SUB_OP => {
                pending.extend(generate_bn254_complex_sub_mem_inputs(
                    addr_main,
                    step_main,
                    data,
                    only_counters,
                ));
            }
            BN254_COMPLEX_MUL_OP => {
                pending.extend(generate_bn254_complex_mul_mem_inputs(
                    addr_main,
                    step_main,
                    data,
                    only_counters,
                ));
            }

            _ => {
                panic!("ArithEqCounterInputGen: Unsupported data length {}", data.len(),);
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
