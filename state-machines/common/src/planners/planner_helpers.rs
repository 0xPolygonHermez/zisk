use crate::CheckPoint;

#[derive(Debug)]
pub struct InstCount {
    pub chunk_id: usize,
    pub inst_count: u64,
}

impl InstCount {
    pub fn new(chunk_id: usize, inst_count: u64) -> Self {
        InstCount { chunk_id, inst_count }
    }
}

/// Generates a list of checkpoints from instruction counts in multiple chunks.
///
/// # Arguments
/// - `counts`: A vector of `InstCount` structs, each representing the number of instructions in a
///   chunk.
/// - `size`: The number of instructions at which to place checkpoints.
///
/// # Returns
/// A vector of `CheckPoint` structs, each representing a checkpoint with its associated chunk ID
/// and offset.
///
/// # Example
/// ```
/// use sm_common::{plan, CheckPoint, InstCount};
///
/// let counts = vec![InstCount::new(0, 500), InstCount::new(1, 700), InstCount::new(2, 300)];
/// let size = 300;
/// let checkpoints = plan(&counts, size);
/// assert_eq!(
///     checkpoints,
///     vec![
///         CheckPoint::new(0, 0),
///         CheckPoint::new(0, 300),
///         CheckPoint::new(1, 100),
///         CheckPoint::new(1, 400),
///         CheckPoint::new(2, 0),
///     ]
/// );
/// ```
pub fn plan(counts: &[InstCount], size: u64) -> Vec<CheckPoint> {
    if counts.is_empty() {
        return vec![];
    }

    let mut checkpoints = vec![CheckPoint::new(0, 0)];

    let mut offset = 0i64;

    let size = size as i64;

    for (current_chunk, count) in counts.iter().enumerate() {
        let inst_count = count.inst_count as i64;

        // Add checkpoints within the current chunk
        while offset + size < inst_count {
            offset += size;
            checkpoints.push(CheckPoint::new(current_chunk, offset as u64));
        }

        // Carry over remaining instructions to the next chunk
        offset -= inst_count;
    }

    checkpoints
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_basic() {
        let counts = vec![InstCount::new(0, 500), InstCount::new(1, 700), InstCount::new(2, 300)];
        let size = 300;
        let checkpoints = plan(&counts, size);
        assert_eq!(
            checkpoints,
            vec![
                CheckPoint::new(0, 0),
                CheckPoint::new(0, 300),
                CheckPoint::new(1, 100),
                CheckPoint::new(1, 400),
                CheckPoint::new(2, 0),
            ]
        );
    }

    #[test]
    fn test_plan_single_chunk() {
        let counts = vec![InstCount { chunk_id: 0, inst_count: 1000 }];
        let size = 250;
        let checkpoints = plan(&counts, size);
        assert_eq!(
            checkpoints,
            vec![
                CheckPoint::new(0, 0),
                CheckPoint::new(0, 250),
                CheckPoint::new(0, 500),
                CheckPoint::new(0, 750),
            ]
        );
    }

    #[test]
    fn test_plan_small_chunks() {
        let counts = vec![InstCount::new(0, 100), InstCount::new(1, 150)];
        let size = 200;
        let checkpoints = plan(&counts, size);
        assert_eq!(checkpoints, vec![CheckPoint::new(0, 0), CheckPoint::new(1, 100),]);
    }

    #[test]
    fn test_plan_no_remainder() {
        let counts = vec![
            InstCount { chunk_id: 0, inst_count: 300 },
            InstCount { chunk_id: 1, inst_count: 300 },
        ];
        let size = 300;
        let checkpoints = plan(&counts, size);
        assert_eq!(checkpoints, vec![CheckPoint::new(0, 0), CheckPoint::new(1, 0),]);
    }
}
