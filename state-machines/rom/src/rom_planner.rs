//! The `RomPlanner` module defines a planner for organizing execution
//! plans for ROM-related operations.
//!
//! Unlike other state machines, ROM has no bus-side counter — every
//! executed instruction comes from ROM, so its chunk set is just
//! `ChunkId(0..num_chunks)`. The executor calls
//! [`RomPlanner::plan_for_chunks`] directly with the chunk count;
//! ROM does not participate in the [`zisk_common::Planner`] trait
//! dispatch used by other SMs.

use zisk_common::{CheckPoint, ChunkId, InstanceType, Plan};
use zisk_pil::{ROM_AIR_IDS, ZISK_AIRGROUP_ID};

use crate::{RomError, RomResult};

/// Planner for the ROM state machine. See module docs.
pub struct RomPlanner;

impl RomPlanner {
    /// Builds the ROM plan from the chunk count.
    ///
    /// Returns one `Plan` covering `ChunkId(0..num_chunks)`. Errors if
    /// `num_chunks` is zero — the executor never plans ROM for an empty
    /// run.
    pub fn plan_for_chunks(num_chunks: usize) -> RomResult<Vec<Plan>> {
        if num_chunks == 0 {
            return Err(RomError::Custom(
                "RomPlanner::plan_for_chunks: num_chunks must be > 0".to_string(),
            ));
        }

        let vec_chunk_ids = (0..num_chunks).map(ChunkId).collect::<Vec<_>>();

        Ok(vec![Plan::new(
            ZISK_AIRGROUP_ID,
            ROM_AIR_IDS[0],
            None,
            InstanceType::Instance,
            CheckPoint::Multiple(vec_chunk_ids),
            None,
        )])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_for_chunks_errors_on_zero() {
        let result = RomPlanner::plan_for_chunks(0);
        assert!(result.is_err());
    }

    #[test]
    fn plan_for_chunks_builds_contiguous_checkpoint() {
        let plans = RomPlanner::plan_for_chunks(3)
            .expect("plan_for_chunks should succeed for num_chunks > 0");

        assert_eq!(plans.len(), 1, "always exactly one Plan");
        let p = &plans[0];
        assert_eq!(p.airgroup_id, ZISK_AIRGROUP_ID);
        assert_eq!(p.air_id, ROM_AIR_IDS[0]);
        assert_eq!(p.instance_type, InstanceType::Instance);
        match &p.check_point {
            CheckPoint::Multiple(ids) => {
                assert_eq!(ids, &vec![ChunkId(0), ChunkId(1), ChunkId(2)]);
            }
            other => panic!("expected CheckPoint::Multiple, got {other:?}"),
        }
    }
}
