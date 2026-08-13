//! The `BinaryCounter` module defines a device for tracking and processing binary-related operations
//! sent over the data bus. It serves a purpose:
//! - Counting different types of binary operations, to decide if uses specific add instances or not.
//!
//! This module implements the `Metrics` and `BusDevice` traits, enabling seamless integration with
//! the system bus for both monitoring and input generation.

use crate::{add_shape, extension_requires_full, AddShape, BinaryBasicFrops, BinaryExtensionFrops};
use zisk_common::{BusDevice, BusId, Counter, Metrics, A, B, OP, OPERATION_BUS_ID, OP_TYPE};
use zisk_core::{zisk_ops::ZiskOp, ZiskOperationType};

/// The `BinaryCounter` struct represents a counter that monitors and measures
/// binary-related operations on the data bus.
///
/// It tracks specific operations and types and updates differents counters for each
/// accepted operation whenever data is processed on the bus.
///
/// The buckets are **disjoint**: every binary / binary-extension operation on the bus lands in
/// exactly one of them, so their sum is the total number of operations. Each bucket corresponds to
/// the air (or set of airs) able to prove that operation, which is what lets the planner size the
/// instances — see [`crate::add_shape`] and [`crate::extension_requires_full`] for the split.
#[derive(Default)]
pub struct BinaryCounter {
    /// Counter for binary add operations needing the full 64-bit add (only add, no addw).
    /// Proven by `BinaryAdd`, or by `Binary` when no dedicated add air is used.
    pub counter_add: Counter,

    /// Counter for add operations whose result fits in the low limb ([`AddShape::Hi`] and
    /// [`AddShape::HiNeg`]). `BinaryAddHi` packs these, ADDS_X_ROW per row, in any of its slots.
    pub counter_add_hi: Counter,

    /// Counter for basic binary operations, but not considering add operations
    pub counter_basic_wo_add: Counter,

    /// Counter for binary extension operations the reduced `BinaryExtension` air can prove.
    pub counter_extension: Counter,

    /// Counter for binary extension operations that need the `BinaryExtensionFull` air.
    pub counter_extension_full: Counter,
}

impl BinaryCounter {
    /// Creates a new instance of `BinaryCounter`.
    ///
    /// # Arguments
    /// * `mode` - The mode of the bus device.
    ///
    /// # Returns
    /// A new `BinaryCounter` instance.
    pub fn new() -> Self {
        Self::default()
    }

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
    pub fn process_data(&mut self, bus_id: &BusId, data: &[u64]) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        self.measure(data);

        true
    }
}

impl Metrics for BinaryCounter {
    /// Tracks activity on the connected bus and updates counters for recognized operations.
    ///
    /// # Arguments
    /// * `data` - The data received from the bus.
    ///
    /// # Returns
    /// An empty vector, as this implementation does not produce any derived inputs for the bus.
    #[inline(always)]
    fn measure(&mut self, data: &[u64]) {
        // Precomputed constants to avoid casting each time
        const BINARY: u64 = ZiskOperationType::Binary as u64;
        const BINARY_E: u64 = ZiskOperationType::BinaryE as u64;
        const ADD_CODE: u64 = ZiskOp::Add.code() as u64;

        let op_type = data[OP_TYPE];
        if op_type == BINARY {
            // Always read the OP index (assume well-formed trace)
            let op = data[OP];
            if op == ADD_CODE {
                // Bucket the addition by operand shape, which decides whether the packed
                // BinaryAddHi air can prove it.
                let counter = match add_shape(data[A], data[B]) {
                    AddShape::Hi | AddShape::HiNeg => &mut self.counter_add_hi,
                    AddShape::Full => &mut self.counter_add,
                };
                if BinaryBasicFrops::is_frequent_op(ADD_CODE as u8, data[A], data[B]) {
                    counter.update_frops(1);
                } else {
                    counter.update(1);
                }
            } else if BinaryBasicFrops::is_frequent_op(op as u8, data[A], data[B]) {
                self.counter_basic_wo_add.update_frops(1);
            } else {
                self.counter_basic_wo_add.update(1);
            }
        } else if op_type == BINARY_E {
            // Operations whose unused operand parts are dirty can only be proven by the full air.
            let counter = if extension_requires_full(data[OP] as u8, data[A], data[B]) {
                &mut self.counter_extension_full
            } else {
                &mut self.counter_extension
            };
            if BinaryExtensionFrops::is_frequent_op(data[OP] as u8, data[A], data[B]) {
                counter.update_frops(1);
            } else {
                counter.update(1);
            }
        }
    }

    /// Provides a dynamic reference for downcasting purposes.
    ///
    /// # Returns
    /// A reference to `self` as `dyn std::any::Any`.
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl BusDevice<u64> for BinaryCounter {
    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
