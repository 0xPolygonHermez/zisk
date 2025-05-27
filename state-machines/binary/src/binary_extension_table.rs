//! The `BinaryExtensionTableSM` module implements the logic for managing the Binary Extension
//! Table.
//!
//! This state machine handles operations like shift-left logical (`Sll`), shift-right logical
//! (`Srl`), arithmetic shifts, and sign extensions.

use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};

use zisk_common::create_atomic_vec;
use zisk_core::{P2_11, P2_19, P2_8};
use zisk_pil::BinaryExtensionTableTrace;

/// Represents operations supported by the Binary Extension Table.
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum BinaryExtensionTableOp {
    Sll = 0x31,
    Srl = 0x32,
    Sra = 0x33,
    SllW = 0x34,
    SrlW = 0x35,
    SraW = 0x36,
    SignExtendB = 0x37,
    SignExtendH = 0x38,
    SignExtendW = 0x39,
}

/// The `BinaryExtensionTableSM` struct encapsulates the Binary Extension Table's logic.
///
/// This state machine manages multiplicity for table rows and processes operations such as shifts
/// and sign extensions.
pub struct BinaryExtensionTableSM {
    /// The multiplicity table
    multiplicity: Vec<AtomicU64>,
    calculated: AtomicBool,
}

impl BinaryExtensionTableSM {
    /// Creates a new `BinaryExtensionTableSM` instance.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `BinaryExtensionTableSM` with an initialized multiplicity
    /// table.
    pub fn new() -> Arc<Self> {
        let binary_extension_table = Self {
            multiplicity: create_atomic_vec(BinaryExtensionTableTrace::<usize>::NUM_ROWS),
            calculated: AtomicBool::new(false),
        };

        Arc::new(binary_extension_table)
    }

    /// Processes a slice of input data and updates the multiplicity table.
    ///
    /// # Arguments
    /// * `input` - A slice of `u64` values to process.
    pub fn update_multiplicity(&self, row: u64, value: u64) {
        if self.calculated.load(Ordering::Relaxed) {
            return;
        }
        self.multiplicity[row as usize].fetch_add(value, Ordering::Relaxed);
    }

    /// Detaches the current multiplicity table, returning its contents and resetting it.
    ///
    /// # Returns
    /// A `Vec<u64>` containing the multiplicity table's current values.
    pub fn detach_multiplicity(&self) -> &[AtomicU64] {
        &self.multiplicity
    }

    pub fn set_calculated(&self) {
        self.calculated.store(true, Ordering::Relaxed);
    }

    pub fn reset_calculated(&self) {
        self.calculated.store(false, Ordering::Relaxed);
    }

    /// Calculates the row index in the Binary Extension Table based on the operation and its
    /// inputs.
    ///
    /// # Arguments
    /// * `opcode` - The operation code, as a `BinaryExtensionTableOp`.
    /// * `offset` - The offset value.
    /// * `a` - The first operand.
    /// * `b` - The second operand.
    ///
    /// # Returns
    /// A `u64` representing the calculated row index in the table.
    ///
    /// # Panics
    /// In debug mode, it panics if `offset` > 0x07, `a` > 0xFF, or `b` > 0xFF, as these violate
    /// table constraints.
    pub fn calculate_table_row(opcode: BinaryExtensionTableOp, offset: u64, a: u64, b: u64) -> u64 {
        //lookup_proves(BINARY_EXTENSION_TABLE_ID, [OP, OFFSET, A, B, C0, C1], multiplicity);
        debug_assert!(offset <= 0x07);
        debug_assert!(a <= 0xFF);
        debug_assert!(b <= 0xFF);

        // Calculate the different row offset contributors, according to the PIL
        let offset_a: u64 = a;
        let offset_offset: u64 = offset * P2_8;
        let offset_b: u64 = b * P2_11;
        let offset_opcode: u64 = Self::offset_opcode(opcode);

        offset_a + offset_offset + offset_b + offset_opcode
    }

    /// Computes the opcode offset for a given `BinaryExtensionTableOp`.
    ///
    /// # Arguments
    /// * `opcode` - The operation code as a `BinaryExtensionTableOp`.
    ///
    /// # Returns
    /// A `u64` representing the offset contribution of the opcode.
    fn offset_opcode(opcode: BinaryExtensionTableOp) -> u64 {
        match opcode {
            BinaryExtensionTableOp::Sll => 0,
            BinaryExtensionTableOp::Srl => P2_19,
            BinaryExtensionTableOp::Sra => 2 * P2_19,
            BinaryExtensionTableOp::SllW => 3 * P2_19,
            BinaryExtensionTableOp::SrlW => 4 * P2_19,
            BinaryExtensionTableOp::SraW => 5 * P2_19,
            BinaryExtensionTableOp::SignExtendB => 6 * P2_19,
            BinaryExtensionTableOp::SignExtendH => 6 * P2_19 + P2_11,
            BinaryExtensionTableOp::SignExtendW => 6 * P2_19 + 2 * P2_11,
        }
    }
}
