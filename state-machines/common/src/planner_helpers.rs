use crate::CheckPoint;

pub struct InstCount {
    pub chunk_id: usize,
    pub inst_count: u64,
}

/// Generates a list of checkpoints from instruction counts in multiple chunks.
///
/// # Arguments
/// - `counts`: A vector of `InstCount` structs, each representing the number of instructions in a chunk.
/// - `size`: The number of instructions at which to place checkpoints.
///
/// # Returns
/// A vector of `CheckPoint` structs, each representing a checkpoint with its associated chunk ID and offset.
///
/// # Example
/// ```
/// let counts = vec![
///     InstCount { chunk_id: 0, inst_count: 500 },
///     InstCount { chunk_id: 1, inst_count: 700 },
///     InstCount { chunk_id: 2, inst_count: 300 },
/// ];
/// let size = 300;
/// let checkpoints = plan(counts, size);
/// assert_eq!(checkpoints, vec![
///     CheckPoint { chunk_id: 0, offset: 0 },
///     CheckPoint { chunk_id: 0, offset: 300 },
///     CheckPoint { chunk_id: 1, offset: 100 },
///     CheckPoint { chunk_id: 1, offset: 400 },
///     CheckPoint { chunk_id: 2, offset: 0 },
/// ]);
/// ```
pub fn plan(counts: Vec<InstCount>, size: u64) -> Vec<CheckPoint> {
    if counts.is_empty() {
        return vec![];
    }

    let mut checkpoints = vec![CheckPoint::new(0, 0)];

    let mut offset = 0i64;
    let mut current_chunk = 0;

    let size = size as i64;

    for count in counts {
        let inst_count = count.inst_count as i64;

        // Add checkpoints within the current chunk
        while offset + size < inst_count {
            offset += size;
            checkpoints.push(CheckPoint::new(current_chunk, offset as u64));
        }

        // Carry over remaining instructions to the next chunk
        offset -= inst_count;

        current_chunk += 1;
    }

    checkpoints
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_basic() {
        let counts = vec![
            InstCount { chunk_id: 0, inst_count: 500 },
            InstCount { chunk_id: 1, inst_count: 700 },
            InstCount { chunk_id: 2, inst_count: 300 },
        ];
        let size = 300;
        let checkpoints = plan(counts, size);
        assert_eq!(
            checkpoints,
            vec![
                CheckPoint { chunk_id: 0, offset: 0 },
                CheckPoint { chunk_id: 0, offset: 300 },
                CheckPoint { chunk_id: 1, offset: 100 },
                CheckPoint { chunk_id: 1, offset: 400 },
                CheckPoint { chunk_id: 2, offset: 0 },
            ]
        );
    }

    #[test]
    fn test_plan_single_chunk() {
        let counts = vec![InstCount { chunk_id: 0, inst_count: 1000 }];
        let size = 250;
        let checkpoints = plan(counts, size);
        assert_eq!(
            checkpoints,
            vec![
                CheckPoint { chunk_id: 0, offset: 0 },
                CheckPoint { chunk_id: 0, offset: 250 },
                CheckPoint { chunk_id: 0, offset: 500 },
                CheckPoint { chunk_id: 0, offset: 750 },
            ]
        );
    }

    #[test]
    fn test_plan_small_chunks() {
        let counts = vec![
            InstCount { chunk_id: 0, inst_count: 100 },
            InstCount { chunk_id: 1, inst_count: 150 },
        ];
        let size = 200;
        let checkpoints = plan(counts, size);
        assert_eq!(
            checkpoints,
            vec![CheckPoint { chunk_id: 0, offset: 0 }, CheckPoint { chunk_id: 1, offset: 100 },]
        );
    }

    #[test]
    fn test_plan_no_remainder() {
        let counts = vec![
            InstCount { chunk_id: 0, inst_count: 300 },
            InstCount { chunk_id: 1, inst_count: 300 },
        ];
        let size = 300;
        let checkpoints = plan(counts, size);
        assert_eq!(
            checkpoints,
            vec![CheckPoint { chunk_id: 0, offset: 0 }, CheckPoint { chunk_id: 1, offset: 0 },]
        );
    }
}
