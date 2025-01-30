//! The `plan` module provides utilities for generating checkpoints based on instruction counts
//! distributed across multiple chunks. It defines the `InstCount` structure and the `plan`
//! function to facilitate the creation of checkpoints at specified intervals.

use std::collections::HashMap;

use crate::{CheckPoint, CollectSkipper};

/// Represents the instruction count for a specific chunk.
///
/// This structure is used to define the number of instructions processed in a particular chunk,
/// along with the chunk's unique identifier.
#[derive(Debug)]
pub struct InstCount {
    /// The identifier for the chunk.
    pub chunk_id: usize,

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
    pub fn new(chunk_id: usize, inst_count: u64) -> Self {
        InstCount { chunk_id, inst_count }
    }
}

/// Generates a list of checkpoints from instruction counts across multiple chunks.
///
/// This function calculates checkpoints based on a specified interval (`size`) of instructions
/// and generates a `CheckPoint` for each interval.
///
/// # Arguments
/// * `counts` - A slice of `InstCount` representing instruction counts for each chunk.
/// * `size` - The interval (number of instructions) at which checkpoints are generated.
///
/// # Returns
/// A vector of tuples, where each tuple contains:
/// - A `CheckPoint` representing the checkpoint's location.
/// - A `Box<CollectSkipper>` containing the associated offset for the checkpoint.
///
/// # Example
/// ```
/// use sm_common::{plan, CheckPoint, CollectSkipper, InstCount};
///
/// let counts = vec![InstCount::new(0, 500), InstCount::new(1, 700), InstCount::new(2, 300)];
/// let size = 300;
/// let checkpoints = plan(&counts, size);
/// assert_eq!(
///     checkpoints,
///     vec![
///         (CheckPoint::Single(0), Box::new(CollectSkipper::new(0))),
///         (CheckPoint::Single(0), Box::new(CollectSkipper::new(300))),
///         (CheckPoint::Single(1), Box::new(CollectSkipper::new(100))),
///         (CheckPoint::Single(1), Box::new(CollectSkipper::new(400))),
///         (CheckPoint::Single(2), Box::new(CollectSkipper::new(0))),
///     ]
/// );
/// ```
pub fn plan(counts: &[InstCount], size: u64) -> Vec<(CheckPoint, CollectSkipper)> {
    if counts.is_empty() {
        return vec![];
    }

    let mut checkpoints = vec![(CheckPoint::Single(0), CollectSkipper::new(0))];
    let mut offset = 0i64;
    let size = size as i64;

    for (current_chunk, count) in counts.iter().enumerate() {
        let inst_count = count.inst_count as i64;

        // Add checkpoints within the current chunk
        while offset + size < inst_count {
            offset += size;
            checkpoints.push((
                CheckPoint::Single(current_chunk),
                CollectSkipper::new(offset as u64),
            ));
        }

        // Carry over remaining instructions to the next chunk
        offset -= inst_count;
    }

    checkpoints
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests the basic functionality of the `plan` function with multiple chunks.
    #[test]
    fn test_plan_basic() {
        let counts = vec![InstCount::new(0, 500), InstCount::new(1, 700), InstCount::new(2, 300)];
        let size = 300;
        let checkpoints = plan(&counts, size);
        assert_eq!(
            checkpoints,
            vec![
                (CheckPoint::Single(0), CollectSkipper::new(0)),
                (CheckPoint::Single(0), CollectSkipper::new(300)),
                (CheckPoint::Single(1), CollectSkipper::new(100)),
                (CheckPoint::Single(1), CollectSkipper::new(400)),
                (CheckPoint::Single(2), CollectSkipper::new(0)),
            ]
        );
    }

    /// Tests the `plan` function with a single chunk containing multiple intervals.
    #[test]
    fn test_plan_single_chunk() {
        let counts = vec![InstCount { chunk_id: 0, inst_count: 1000 }];
        let size = 250;
        let checkpoints = plan(&counts, size);
        assert_eq!(
            checkpoints,
            vec![
                (CheckPoint::Single(0), CollectSkipper::new(0)),
                (CheckPoint::Single(0), CollectSkipper::new(250)),
                (CheckPoint::Single(0), CollectSkipper::new(500)),
                (CheckPoint::Single(0), CollectSkipper::new(750)),
            ]
        );
    }

    /// Tests the `plan` function with small chunks where intervals span across chunks.
    #[test]
    fn test_plan_small_chunks() {
        let counts = vec![InstCount::new(0, 100), InstCount::new(1, 150)];
        let size = 200;
        let checkpoints = plan(&counts, size);
        assert_eq!(
            checkpoints,
            vec![
                (CheckPoint::Single(0), CollectSkipper::new(0)),
                (CheckPoint::Single(1), CollectSkipper::new(100)),
            ]
        );
    }

    /// Tests the `plan` function with chunks whose sizes exactly match the interval size.
    #[test]
    fn test_plan_no_remainder() {
        let counts = vec![
            InstCount { chunk_id: 0, inst_count: 300 },
            InstCount { chunk_id: 1, inst_count: 300 },
        ];
        let size = 300;
        let checkpoints = plan(&counts, size);
        assert_eq!(
            checkpoints,
            vec![
                (CheckPoint::Single(0), CollectSkipper::new(0)),
                (CheckPoint::Single(1), CollectSkipper::new(0)),
            ]
        );
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
/// A nested vector where each inner vector corresponds to a plan scope and contains:
/// - `CheckPoint` for the chunk location.
/// - The number of instructions for the chunk.
/// - `CollectSkipper` with the skip offset.
///
/// # Example
/// ```
/// use sm_common::{plan_2, CheckPoint, CollectSkipper, InstCount};
///
/// let counts = vec![InstCount::new(0, 500), InstCount::new(1, 200), InstCount::new(2, 300)];
/// let size = 400;
/// let nested_checkpoints = plan_2(&counts, size);
/// assert_eq!(
///     nested_checkpoints,
///     vec![
///         vec![
///             (CheckPoint(0), 400, CollectSkipper::new(0)),
///         ],
///         vec![
///             (CheckPoint(0), 100, CollectSkipper::new(400)),
///             (CheckPoint(1), 200, CollectSkipper::new(0)),
///             (CheckPoint(2), 100, CollectSkipper::new(0)),
///         ],
///         vec![
///             (CheckPoint(2), 200, CollectSkipper::new(100)),
///         ],
///     ]
/// );
/// ```
pub fn plan_2(counts: &[InstCount], size: u64) -> Vec<CheckPoint> {
    if counts.is_empty() || size == 0 {
        return vec![];
    }

    let mut checkpoints = Vec::new();
    let mut current_scope: HashMap<usize, (u64, CollectSkipper)> = HashMap::new();
    let mut remaining_size = size; // Remaining size for the current scope.

    for (current_chunk, count) in counts.iter().enumerate() {
        let mut inst_count = count.inst_count;
        let mut cumulative_offset = 0u64; // Reset cumulative offset for each chunk.

        while inst_count > 0 {
            let checkpoint_size = remaining_size.min(inst_count);

            current_scope.insert(current_chunk, (
                checkpoint_size,
                CollectSkipper::new(cumulative_offset),
            ));

            cumulative_offset += checkpoint_size;
            inst_count -= checkpoint_size;
            remaining_size -= checkpoint_size;

            if remaining_size == 0 {
                checkpoints.push(CheckPoint::Multiple2(std::mem::take(&mut current_scope)));
                remaining_size = size;
            }
        }
    }

    // Push any remaining checkpoints into the result.
    if !current_scope.is_empty() {
        checkpoints.push(CheckPoint::Multiple2(current_scope));
    }

    checkpoints
}



// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_plan_2_multiple_chunks() {
//         let counts = vec![InstCount::new(0, 500), InstCount::new(1, 200), InstCount::new(2, 300)];
//         let size = 400;
//         let nested_checkpoints = plan_2(&counts, size);
//         assert_eq!(
//             nested_checkpoints,
//             vec![
//                 vec![
//                     (CheckPoint2(0), 400, CollectSkipper::new(0)),
//                 ],
//                 vec![
//                     (CheckPoint2(0), 100, CollectSkipper::new(400)),
//                     (CheckPoint2(1), 200, CollectSkipper::new(0)),
//                     (CheckPoint2(2), 100, CollectSkipper::new(0)),
//                 ],
//                 vec![
//                     (CheckPoint2(2), 200, CollectSkipper::new(100)),
//                 ],
//             ]
//         );
//     }

//     #[test]
//     fn test_plan_2_single_chunk() {
//         let counts = vec![InstCount { chunk_id: 0, inst_count: 1000 }];
//         let size = 400;
//         let nested_checkpoints = plan_2(&counts, size);
//         assert_eq!(
//             nested_checkpoints,
//             vec![
//                 vec![
//                     (CheckPoint2(0), 400, CollectSkipper::new(0)),
//                 ],
//                 vec![
//                     (CheckPoint2(0), 400, CollectSkipper::new(400)),
//                 ],
//                 vec![
//                     (CheckPoint2(0), 200, CollectSkipper::new(800)),
//                 ],
//             ]
//         );
//     }

//         #[test]
//     fn test_plan_2_multiple_chunk() {
//         let counts = vec![InstCount { chunk_id: 0, inst_count: 100 }, InstCount { chunk_id: 0, inst_count: 200 }, InstCount { chunk_id: 0, inst_count: 90 }];
//         let size = 400;
//         let nested_checkpoints = plan_2(&counts, size);
//         assert_eq!(
//             nested_checkpoints,
//             vec![
//                 vec![
//                     (CheckPoint2(0), 100, CollectSkipper::new(0)),
//                     (CheckPoint2(1), 200, CollectSkipper::new(0)),
//                     (CheckPoint2(2), 90, CollectSkipper::new(0)),
//                 ],
//             ]
//         );
//     }

//     #[test]
//     fn test_plan_2_empty_counts() {
//         let counts = vec![];
//         let size = 400;
//         let nested_checkpoints = plan_2(&counts, size);
//         assert!(nested_checkpoints.is_empty());
//     }

//     #[test]
//     fn test_plan_2_size_zero() {
//         let counts = vec![InstCount::new(0, 500)];
//         let size = 0;
//         let nested_checkpoints = plan_2(&counts, size);
//         assert!(nested_checkpoints.is_empty());
//     }
// }