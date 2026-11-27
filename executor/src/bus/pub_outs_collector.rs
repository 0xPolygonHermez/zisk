//! The `PubOutsCollector` module defines a collector to accumulate public output operations
//! (`PubOut`) sent over the execution data bus. It maintains a vector of public outputs,
//! where each output is represented as a tuple of an index and a 32-bit value.

use zisk_common::{A, B, OP_TYPE};
use zisk_core::ZiskOperationType;

use crate::error::{ExecutorError, ExecutorResult};

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

    /// Fails on an index past the public inputs: the index comes from the guest.
    pub fn check_indexes(&self) -> ExecutorResult<()> {
        match self.0.iter().find(|&&(index, _)| index >= MAX_PUBOUTS as u64) {
            Some(&(index, _)) => Err(ExecutorError::Internal(format!(
                "public output index {index} past the {MAX_PUBOUTS} public inputs"
            ))),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a 4-element data array carrying a `PubOut` op with the given
    /// `a` (pub_index source) and `b` (pub_value).
    fn pubout_data(a: u64, b: u64) -> [u64; 4] {
        let mut data = [0u64; 4];
        data[OP_TYPE] = ZiskOperationType::PubOut as u64;
        data[A] = a;
        data[B] = b;
        data
    }

    #[test]
    fn new_is_empty() {
        let coll = PubOutsCollector::new();
        assert!(coll.0.is_empty());
    }

    #[test]
    fn non_pubout_op_is_ignored() {
        let mut data = [0u64; 4];
        data[OP_TYPE] = ZiskOperationType::Arith as u64;
        data[A] = 5;
        data[B] = u64::MAX;
        let mut coll = PubOutsCollector::new();
        coll.process_data(&data);
        assert!(coll.0.is_empty());
    }

    #[test]
    fn pubout_appends_lo_and_hi_halves() {
        let data = pubout_data(5, 0x1234_5678_9ABC_DEF0);
        let mut coll = PubOutsCollector::new();
        coll.process_data(&data);
        assert_eq!(coll.0.len(), 2);
        // pub_index = a << 1 = 10. Low 32 bits go first, then index + 1 with high 32.
        assert_eq!(coll.0[0], (10, 0x9ABC_DEF0));
        assert_eq!(coll.0[1], (11, 0x1234_5678));
    }

    #[test]
    fn pubout_index_is_left_shifted_by_one() {
        let data = pubout_data(3, 0);
        let mut coll = PubOutsCollector::new();
        coll.process_data(&data);
        assert_eq!(coll.0[0].0, 6);
        assert_eq!(coll.0[1].0, 7);
    }

    #[test]
    fn pubout_handles_zero_value() {
        let data = pubout_data(0, 0);
        let mut coll = PubOutsCollector::new();
        coll.process_data(&data);
        assert_eq!(coll.0, vec![(0, 0), (1, 0)]);
    }

    #[test]
    fn pubout_handles_max_value() {
        let data = pubout_data(0, u64::MAX);
        let mut coll = PubOutsCollector::new();
        coll.process_data(&data);
        assert_eq!(coll.0, vec![(0, u32::MAX), (1, u32::MAX)]);
    }

    #[test]
    fn multiple_pubouts_accumulate_in_order() {
        let mut coll = PubOutsCollector::new();
        coll.process_data(&pubout_data(0, 0xAA));
        coll.process_data(&pubout_data(1, 0xBB << 32));
        assert_eq!(coll.0.len(), 4);
        assert_eq!(coll.0[0], (0, 0xAA));
        assert_eq!(coll.0[1], (1, 0));
        assert_eq!(coll.0[2], (2, 0));
        assert_eq!(coll.0[3], (3, 0xBB));
    }

    #[test]
    fn check_indexes_rejects_an_index_past_the_public_inputs() {
        let mut coll = PubOutsCollector::new();
        coll.process_data(&pubout_data(31, 0)); // indexes 62, 63: the last two
        assert!(coll.check_indexes().is_ok());
        coll.process_data(&pubout_data(32, 0)); // indexes 64, 65
        assert!(coll.check_indexes().is_err());
    }
}
