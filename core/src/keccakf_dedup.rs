//! Table of the distinct Keccak-f\[1600\] permutations of an execution.
//!
//! Every executed Keccak-f is looked up by its input state. The first one with a given input
//! state is a *unique* permutation, numbered in order of first appearance; any later Keccak-f
//! with the same input state is a *replica* of it. The prover proves a unique permutation once
//! (Keccakf air) and every invocation with one cheap row (KeccakfProxy air), so every Keccak-f
//! carries a *dedup word* in the minimal trace, after its 25 input words: the index of its
//! unique permutation, plus [`KECCAKF_DEDUP_REPLICA_FLAG`] when it is a replica.
//!
//! This is the Rust emulator's table; the assembly emulator keeps an equivalent one
//! (`lib-c/c/src/keccakf_cache/keccakf_dedup.cpp`). Both number the unique permutations in the
//! same order, so they produce the same dedup words.

use std::collections::HashMap;

/// Number of 64-bit words of a Keccak-f\[1600\] state.
pub const KECCAKF_DEDUP_STATE_WORDS: usize = 25;

/// Number of 64-bit words a Keccak-f adds to the minimal trace: its input state and its dedup
/// word.
pub const KECCAKF_MT_WORDS: usize = KECCAKF_DEDUP_STATE_WORDS + 1;

/// Bytes a Keccak-f adds to the minimal trace, i.e. the input size of the instruction.
pub const KECCAKF_MT_BYTES: u64 = (KECCAKF_MT_WORDS * 8) as u64;

/// Flag of the dedup word of a Keccak-f that repeats the input state of an earlier one.
pub const KECCAKF_DEDUP_REPLICA_FLAG: u64 = 1 << 63;

/// Most replicas one unique permutation can have: the KeccakfProxy air commits the count in a
/// 24-bit column. A Keccak-f that repeats a permutation with this many replicas is not a replica
/// of it: it becomes a new unique permutation, which takes the replicas that follow.
pub const KECCAKF_DEDUP_MAX_REPLICAS: u64 = (1 << 24) - 1;

/// A Keccak-f state.
pub type KeccakfState = [u64; KECCAKF_DEDUP_STATE_WORDS];

/// Returns the index of the unique permutation a dedup word refers to.
#[inline(always)]
pub const fn keccakf_dedup_index(dedup_word: u64) -> u64 {
    dedup_word & !KECCAKF_DEDUP_REPLICA_FLAG
}

/// Returns true if a dedup word belongs to a replica, i.e. to a Keccak-f that is not the first
/// one with its input state.
#[inline(always)]
pub const fn keccakf_dedup_is_replica(dedup_word: u64) -> bool {
    dedup_word & KECCAKF_DEDUP_REPLICA_FLAG != 0
}

/// A unique permutation: its output state and the number of replicas seen so far.
#[derive(Debug)]
struct KeccakfDedupEntry {
    output: KeccakfState,
    replicas: u64,
}

/// Maps Keccak-f input states to their unique permutation.
#[derive(Debug)]
pub struct KeccakfDedup {
    /// Input state -> index of its unique permutation: the last one registered for that state,
    /// which is the only one that can still take replicas.
    index: HashMap<KeccakfState, u64>,

    /// Unique permutations, in order of first appearance.
    entries: Vec<KeccakfDedupEntry>,

    /// Total number of replicas.
    replica_count: u64,

    /// Replicas a unique permutation can take, [`KECCAKF_DEDUP_MAX_REPLICAS`] outside tests.
    max_replicas: u64,
}

impl Default for KeccakfDedup {
    fn default() -> Self {
        Self::with_max_replicas(KECCAKF_DEDUP_MAX_REPLICAS)
    }
}

impl KeccakfDedup {
    fn with_max_replicas(max_replicas: u64) -> Self {
        Self { index: HashMap::new(), entries: Vec::new(), replica_count: 0, max_replicas }
    }

    /// Runs a Keccak-f through the table: `state` holds the input state and receives the output
    /// state, computed with `permute` only if no earlier Keccak-f had the same input state.
    /// Returns the dedup word of this Keccak-f.
    pub fn run(
        &mut self,
        state: &mut KeccakfState,
        permute: impl FnOnce(&mut KeccakfState),
    ) -> u64 {
        if let Some(&index) = self.index.get(state) {
            let entry = &mut self.entries[index as usize];
            if entry.replicas < self.max_replicas {
                entry.replicas += 1;
                self.replica_count += 1;
                *state = entry.output;
                return index | KECCAKF_DEDUP_REPLICA_FLAG;
            }
            // That permutation is full: this Keccak-f is proven again, as a new unique one
        }

        let index = self.entries.len() as u64;
        self.index.insert(*state, index);
        permute(state);
        self.entries.push(KeccakfDedupEntry { output: *state, replicas: 0 });
        index
    }

    /// Number of unique permutations.
    pub fn unique_count(&self) -> u64 {
        self.entries.len() as u64
    }

    /// Total number of replicas: Keccak-f executions = unique count + replica count.
    pub fn replica_count(&self) -> u64 {
        self.replica_count
    }

    /// Number of replicas of the unique permutation `index`.
    pub fn replicas(&self, index: u64) -> u64 {
        self.entries.get(index as usize).map_or(0, |entry| entry.replicas)
    }

    /// Empties the table.
    pub fn clear(&mut self) {
        self.index.clear();
        self.entries.clear();
        self.replica_count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_unique_permutations_in_order_and_flags_replicas() {
        let mut dedup = KeccakfDedup::default();
        let mut calls = 0;
        let mut run = |dedup: &mut KeccakfDedup, seed: u64| {
            let mut state = [seed; KECCAKF_DEDUP_STATE_WORDS];
            let word = dedup.run(&mut state, |s| {
                calls += 1;
                s.iter_mut().for_each(|w| *w = !*w);
            });
            assert_eq!(state, [!seed; KECCAKF_DEDUP_STATE_WORDS]);
            word
        };

        assert_eq!(run(&mut dedup, 7), 0);
        assert_eq!(run(&mut dedup, 9), 1);
        assert_eq!(run(&mut dedup, 7), KECCAKF_DEDUP_REPLICA_FLAG);
        assert_eq!(run(&mut dedup, 7), KECCAKF_DEDUP_REPLICA_FLAG);
        assert_eq!(run(&mut dedup, 9), 1 | KECCAKF_DEDUP_REPLICA_FLAG);
        assert_eq!(run(&mut dedup, 11), 2);

        // The permutation only ran for the unique ones
        assert_eq!(calls, 3);
        assert_eq!(dedup.unique_count(), 3);
        assert_eq!(dedup.replica_count(), 3);
        assert_eq!((dedup.replicas(0), dedup.replicas(1), dedup.replicas(2)), (2, 1, 0));
        assert!(keccakf_dedup_is_replica(1 | KECCAKF_DEDUP_REPLICA_FLAG));
        assert_eq!(keccakf_dedup_index(1 | KECCAKF_DEDUP_REPLICA_FLAG), 1);
    }

    #[test]
    fn a_full_permutation_is_registered_again() {
        let mut dedup = KeccakfDedup::with_max_replicas(2);
        let run = |dedup: &mut KeccakfDedup| {
            let mut state = [5; KECCAKF_DEDUP_STATE_WORDS];
            dedup.run(&mut state, |s| s.iter_mut().for_each(|w| *w = !*w))
        };

        let words: Vec<u64> = (0..7).map(|_| run(&mut dedup)).collect();
        let replica = KECCAKF_DEDUP_REPLICA_FLAG;
        assert_eq!(words, [0, replica, replica, 1, 1 | replica, 1 | replica, 2]);
        assert_eq!((dedup.replicas(0), dedup.replicas(1), dedup.replicas(2)), (2, 2, 0));
        assert_eq!(dedup.unique_count(), 3);
    }
}
