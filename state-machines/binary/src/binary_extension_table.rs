//! The `BinaryExtensionTableSM` module implements the logic for managing the Binary Extension
//! Table.
//!
//! This state machine handles operations like shift-left logical (`Sll`), shift-right logical
//! (`Srl`), arithmetic shifts, and sign extensions.

use std::sync::{Arc, Mutex};

use p3_field::Field;
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

pub struct BinaryExtensionTableAgent {
    /// Binary Basic Table
    binary_extension_table_sm: Arc<BinaryExtensionTableSM>,

    // Multiplicity table
    table: Vec<u64>,
}

impl BinaryExtensionTableAgent {
    pub fn new(binary_extension_table_sm: Arc<BinaryExtensionTableSM>) -> Self {
        Self {
            binary_extension_table_sm,
            table: vec![0; BinaryExtensionTableTrace::<u64>::NUM_ROWS],
        }
    }

    #[inline(always)]
    pub fn process_input(&mut self, idx: usize, val: u64) {
        self.table[idx] += val;
    }

    pub fn finalize(&self) {
        self.binary_extension_table_sm.process_slice(&self.table);
    }
}

/// The `BinaryExtensionTableSM` struct encapsulates the Binary Extension Table's logic.
///
/// This state machine manages multiplicity for table rows and processes operations such as shifts
/// and sign extensions.
pub struct BinaryExtensionTableSM {
    /// The multiplicity table, protected by a mutex for thread-safe access.
    multiplicity: Mutex<Vec<u64>>,
}

impl BinaryExtensionTableSM {
    /// Creates a new `BinaryExtensionTableSM` instance.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `BinaryExtensionTableSM` with an initialized multiplicity
    /// table.
    pub fn new<F: Field>() -> Arc<Self> {
        let binary_extension_table =
            Self { multiplicity: Mutex::new(vec![0; BinaryExtensionTableTrace::<F>::NUM_ROWS]) };

        Arc::new(binary_extension_table)
    }

    /// Processes a slice of input data and updates the multiplicity table.
    ///
    /// # Arguments
    /// * `input` - A slice of `u64` values to process.
    pub fn process_slice(&self, input: &[u64]) {
        let mut multiplicity = self.multiplicity.lock().unwrap();

        for (i, val) in input.iter().enumerate() {
            multiplicity[i] += *val;
        }
    }

    /// Detaches the current multiplicity table, returning its contents and resetting it.
    ///
    /// # Returns
    /// A `Vec<u64>` containing the multiplicity table's current values.
    pub fn detach_multiplicity(&self) -> Vec<u64> {
        let mut multiplicity = self.multiplicity.lock().unwrap();
        std::mem::take(&mut *multiplicity)
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
