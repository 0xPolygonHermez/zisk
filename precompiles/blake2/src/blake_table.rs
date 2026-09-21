//! The `BlakeTableSM` module defines the state machine of the XOR ⊕ ROTR byte table shared by
//! the Blake precompiles: Blake2b and Blake2s in this crate, and Blake3f in its own crate.
//!
//! Each table row proves the tuple (a, b, rot, c0, c1), where [c0, c1] are the two byte pieces
//! of the 32-bit value ((a ^ b) as u32) >>> rot, for rot ∈ {0, 12}. With rot = 0 this is the
//! plain byte XOR (c0 = a ^ b, c1 = 0), which is all Blake2b needs.

/// The `BlakeTableSM` struct represents the shared Blake Table State Machine.
pub struct BlakeTableSM;

impl BlakeTableSM {
    /// Must match `BLAKE_TABLE_ID` in `pil/opids.pil`.
    pub const TABLE_ID: usize = 129;

    /// Number of table rows: 8-bit A × 8-bit B × rotation ∈ {0, 12}.
    /// Must match `BLAKE_TABLE_SIZE` in `blake_table.pil`.
    pub const SIZE: usize = 1 << 17;

    /// Calculates the table row for the tuple (a, b, rot).
    ///
    /// The table iterates A fastest, then B, then the rotation (0 first, 12 second),
    /// mirroring the fixed columns of `blake_table.pil`.
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
            _ => panic!("BlakeTableSM::calculate_table_row() rot must be 0 or 12"),
        };
        a as u32 + (b as u32) * 256 + rot_offset
    }
}
