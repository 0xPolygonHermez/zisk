//! The `PubOutsCollector` module defines a collector to accumulate public output operations
//! (`PubOut`) sent over the execution data bus. It maintains a vector of public outputs,
//! where each output is represented as a tuple of an index and a 32-bit value.

use zisk_common::{A, B, OP_TYPE};
use zisk_core::ZiskOperationType;

const MAX_PUBOUTS: usize = 64; // Maximum number of public outputs.

/// Public outputs accumulated, one `(index, value32)` pair per low/high half of each `PubOut`.
pub struct PubOutsCollector(pub Vec<(u64, u32)>);

impl Default for PubOutsCollector {
    fn default() -> Self {
        Self(Vec::with_capacity(MAX_PUBOUTS))
    }
}

impl PubOutsCollector {
    /// Creates a new empty `PubOutsCollector`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores a `PubOut` operation, appending its low and high 32-bit halves to the inner vector.
    ///
    /// # Arguments
    /// * `data` - The data received from the execution bus.
    #[inline(always)]
    pub fn process_data(&mut self, data: &[u64]) {
        const PUBOUT: u64 = ZiskOperationType::PubOut as u64;

        if data[OP_TYPE] != PUBOUT {
            return;
        }

        let pub_index = data[A] << 1;
        let pub_value = data[B];

        self.0.push((pub_index, (pub_value & 0xFFFFFFFF) as u32));
        self.0.push((pub_index + 1, ((pub_value >> 32) & 0xFFFFFFFF) as u32));
    }
}
