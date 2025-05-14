//! The `plan` module provides utilities for generating checkpoints based on instruction counts
//! distributed across multiple chunks. It defines the `InstCount` structure and the `plan`
//! function to facilitate the creation of checkpoints at specified intervals.

use std::collections::HashMap;

use crate::{CheckPoint, ChunkId, CollectSkipper};

/// Represents the instruction count for a specific chunk.
///
/// This structure is used to define the number of instructions processed in a particular chunk,
/// along with the chunk's unique identifier.
#[derive(Debug)]
pub struct InstCount {
    /// The identifier for the chunk.
    pub chunk_id: ChunkId,

    /// The number of instructions processed within the chunk.
    pub inst_count: u64,
}

impl InstCount {
    /// Creates a new instance of `InstCount`.
    ///
    /// # Arguments
    /// * `chunk_id` - The unique identifier for the chunk.
    /// * `inst_count` - The number of instructions processed in the chunk.
    ///
    /// # Returns
    /// A new `InstCount` instance with the specified chunk ID and instruction count.
    pub fn new(chunk_id: ChunkId, inst_count: u64) -> Self {
        InstCount { chunk_id, inst_count }
    }
}

/// Generates a nested list of checkpoints from instruction counts across multiple chunks.
///
/// Each inner vector corresponds to a scope of the plan and contains tuples of:
/// - A `CheckPoint` representing the checkpoint's location.
/// - The number of instructions for the chunk.
/// - A `CollectSkipper` containing the associated offset for the checkpoint.
///
/// # Arguments
/// * `counts` - A slice of `InstCount` representing instruction counts for each chunk.
/// * `size` - The interval (number of instructions) at which checkpoints are generated.
///
/// # Returns
/// A nested list of tuples containing the checkpoint, instruction count, and offset for each
/// checkpoint.
#[allow(clippy::type_complexity)]
pub fn plan(
    counts: &[InstCount],
    size: u64,
) -> Vec<(CheckPoint, HashMap<ChunkId, (u64, CollectSkipper)>)> {
    if counts.is_empty() || size == 0 {
        return vec![];
    }

    let mut checkpoints = Vec::new();
    let mut current_scope: HashMap<ChunkId, (u64, CollectSkipper)> = HashMap::new();
    let mut remaining_size = size; // Remaining size for the current scope.

    for (current_chunk, count) in counts.iter().enumerate() {
        let mut inst_count = count.inst_count;
        let mut cumulative_offset = 0u64; // Reset cumulative offset for each chunk.

        while inst_count > 0 {
            let checkpoint_size = remaining_size.min(inst_count);

            current_scope.insert(
                ChunkId(current_chunk),
                (checkpoint_size, CollectSkipper::new(cumulative_offset)),
            );

            cumulative_offset += checkpoint_size;
            inst_count -= checkpoint_size;
            remaining_size -= checkpoint_size;

            if remaining_size == 0 {
                let keys = current_scope.keys().cloned().collect::<Vec<_>>();
                checkpoints.push((CheckPoint::Multiple(keys), std::mem::take(&mut current_scope)));
                remaining_size = size;
            }
        }
    }

    // Push any remaining checkpoints into the result.
    if !current_scope.is_empty() {
        let keys = current_scope.keys().cloned().collect::<Vec<_>>();
        checkpoints.push((CheckPoint::Multiple(keys), current_scope));
    }

    checkpoints
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_empty_counts() {
        let result = plan(&[], 10);
        assert!(result.is_empty());
    }

    #[test]
    fn test_size_zero() {
        let counts = [InstCount::new(ChunkId(0), 5)];
        let result = plan(&counts, 0);
        assert!(result.is_empty());
    }

    #[test]
    fn test_single_count_fits_exactly() {
        let counts = [InstCount::new(ChunkId(0), 10)];
        let size = 10;
        let expected = vec![(
            CheckPoint::Multiple(vec![ChunkId(0)]),
            [(ChunkId(0), (10, CollectSkipper::new(0)))].into_iter().collect::<HashMap<_, _>>(),
        )];
        let result = plan(&counts, size);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_single_count_larger_than_size() {
        let counts = [InstCount::new(ChunkId(0), 25)];
        let size = 10;
        let expected = vec![
            (
                CheckPoint::Multiple(vec![ChunkId(0)]),
                [(ChunkId(0), (10, CollectSkipper::new(0)))].into_iter().collect::<HashMap<_, _>>(),
            ),
            (
                CheckPoint::Multiple(vec![ChunkId(0)]),
                [(ChunkId(0), (10, CollectSkipper::new(10)))]
                    .into_iter()
                    .collect::<HashMap<_, _>>(),
            ),
            (
                CheckPoint::Multiple(vec![ChunkId(0)]),
                [(ChunkId(0), (5, CollectSkipper::new(20)))].into_iter().collect::<HashMap<_, _>>(),
            ),
        ];
        let result = plan(&counts, size);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_multiple_chunks() {
        let counts = [InstCount::new(ChunkId(0), 15), InstCount::new(ChunkId(1), 5)];
        let size = 10;
        let mut expected = vec![
            (
                CheckPoint::Multiple(vec![ChunkId(0)]),
                [(ChunkId(0), (10, CollectSkipper::new(0)))].into_iter().collect::<HashMap<_, _>>(),
            ),
            (
                CheckPoint::Multiple(vec![ChunkId(0), ChunkId(1)]),
                [
                    (ChunkId(0), (5, CollectSkipper::new(10))),
                    (ChunkId(1), (5, CollectSkipper::new(0))),
                ]
                .into_iter()
                .collect::<HashMap<_, _>>(),
            ),
        ];

        let mut result = plan(&counts, size);

        // Sort `Multiple` checkpoints to ensure consistent ordering before comparing.
        for (checkpoint, _) in &mut result {
            if let CheckPoint::Multiple(ref mut ids) = checkpoint {
                ids.sort();
            }
        }
        for (checkpoint, _) in &mut expected {
            if let CheckPoint::Multiple(ref mut ids) = checkpoint {
                ids.sort();
            }
        }

        assert_eq!(result, expected);
    }
}
