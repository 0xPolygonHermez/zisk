//! The `KeccakfTableSM` module defines the Keccakf Table State Machine.
//!
//! This state machine is responsible for calculating Keccakf table rows.

use crate::{
    MASK_BITS_A, MASK_BITS_B, MASK_BITS_C, P2_BITS_A, P2_BITS_AB, P2_BITS_ABC, XOR_ANDP_GATE_OP,
    XOR_GATE_OP,
};

/// Represents operations supported by the Keccakf Table.
#[repr(u8)]
pub enum KeccakfTableGateOp {
    /// XOR gate
    Xor = XOR_GATE_OP,

    /// XORANDP gate
    XorAndp = XOR_ANDP_GATE_OP,
}

/// The `KeccakfTableSM` struct represents the Keccakf Table State Machine.
pub struct KeccakfTableSM;

impl KeccakfTableSM {
    pub const TABLE_ID: usize = 126;

    /// Calculates the table row offset based on the provided parameters.
    ///
    /// # Arguments
    /// * `gate_opcode` - The operation code (`KeccakfTableGateOp`).
    /// * `a` - The first operand a.
    /// * `b` - The second operand b.
    ///
    /// # Returns
    /// The calculated table row offset.
    pub fn calculate_table_row(gate_opcode: &KeccakfTableGateOp, a: u64, b: u64, c: u64) -> usize {
        debug_assert!(a <= MASK_BITS_A);
        debug_assert!(b <= MASK_BITS_B);
        debug_assert!(c <= MASK_BITS_C);

        // Calculate the different row offset contributors, according to the PIL
        let offset_a: u64 = a;
        let offset_b: u64 = b * P2_BITS_A;
        let offset_c: u64 = c * P2_BITS_AB;
        let offset_opcode: u64 = Self::offset_opcode(gate_opcode);

        (offset_a + offset_b + offset_c + offset_opcode)
            .try_into()
            .expect("Invalid table row offset")
    }

    /// Computes the opcode offset for the given operation.
    fn offset_opcode(gate_opcode: &KeccakfTableGateOp) -> u64 {
        match gate_opcode {
            KeccakfTableGateOp::Xor => 0,
            KeccakfTableGateOp::XorAndp => P2_BITS_ABC,
        }
    }
}
