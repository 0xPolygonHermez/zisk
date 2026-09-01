//! The `Blake3fTableSM` module defines the Blake3f Table State Machine.
//!
//! This state machine is responsible for calculating Blake3f XOR⊕ROTR table rows.
//! Each table row proves the tuple (a, b, rot, c0, c1), where [c0, c1] are the two
//! byte pieces of the 32-bit value ((a ^ b) as u32) >>> rot, for rot ∈ {0, 12}.

/// The `Blake3fTableSM` struct represents the Blake3f Table State Machine.
pub struct Blake3fTableSM;

impl Blake3fTableSM {
    /// Must match `BLAKE3F_TABLE_ID` in `pil/opids.pil`.
    pub const TABLE_ID: usize = 131;

    /// Calculates the table row for the tuple (a, b, rot).
    ///
    /// The table iterates A fastest, then B, then the rotation (0 first, 12 second),
    /// mirroring the fixed columns of `blake3f_table.pil`.
    ///
    /// # Arguments
    /// * `a` - The first input byte.
    /// * `b` - The second input byte.
    /// * `rot` - The rotation, either 0 or 12.
    ///
    /// # Returns
    /// The calculated table row offset.
    pub const fn calculate_table_row(a: u8, b: u8, rot: u32) -> u32 {
        let rot_offset = match rot {
            0 => 0,
            12 => 1 << 16,
            _ => panic!("Blake3fTableSM::calculate_table_row() rot must be 0 or 12"),
        };
        a as u32 + (b as u32) * 256 + rot_offset
    }
}
