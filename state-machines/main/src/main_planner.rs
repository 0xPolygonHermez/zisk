//! The `MainPlanner` module defines a planner for the Main State Machine.
//!
//! It generates execution plans for segments of the main trace, mapping each segment
//! to a specific `Plan` instance.

use crate::{MainSmError, Result};
use std::any::Any;
use zisk_common::{CheckPoint, ChunkId, EmuTrace, InstanceType, Plan, SegmentId};
use zisk_pil::{MainTrace, MAIN_AIR_IDS, ZISK_AIRGROUP_ID};

/// The `MainPlanner` struct generates execution plans for the Main State Machine.
///
/// It organizes the execution flow by creating a `Plan` instance for each segment
/// of the main trace, associating it with the corresponding segment ID.
pub struct MainPlanner {}

impl MainPlanner {
    /// Generates execution plans for the Main State Machine.
    ///
    /// This method creates a `Plan` for each segment of the provided traces, associating
    /// the segment ID with the corresponding execution plan.
    ///
    /// # Arguments
    /// * `min_traces` - A slice of `EmuTrace` instances representing the segments to be planned.
    /// * `chunk_size` - The size of each chunk used for minimal traces.
    ///
    /// # Returns
    /// A vector of `Plan` instances, each corresponding to a segment of the main trace.
    /// # Errors
    /// Returns a `MainSmError` when:
    /// - The `chunk_size` is not a power of two ([`MainSmError::ChunkSizeNotPowerOfTwo`]).
    /// - The `chunk_size` exceeds the row capacity of `MainTrace` ([`MainSmError::ChunkSizeTooBig`]).
    /// - A `u64` quantity could not be converted to `usize` on this target ([`MainSmError::TryFromIntError`]).
    pub fn plan(min_traces: &[EmuTrace], chunk_size: u64) -> Result<Vec<Plan>> {
        const NUM_ROWS: usize = MainTrace::<()>::NUM_ROWS;

        // Compile-time assertion to ensure `MainTrace::NUM_ROWS` is a power of two.
        const _: () =
            assert!(NUM_ROWS.is_power_of_two(), "MainTrace::NUM_ROWS must be a power of two",);

        let chunk_size: usize = chunk_size.try_into()?;

        if !chunk_size.is_power_of_two() {
            return Err(MainSmError::ChunkSizeNotPowerOfTwo { size: chunk_size });
        }

        if NUM_ROWS < chunk_size {
            return Err(MainSmError::ChunkSizeTooBig { chunk_size, num_rows: NUM_ROWS });
        }

        // Number of minimal traces wrapped in a main trace.
        let num_within = NUM_ROWS / chunk_size;

        // Number of `Main` segments needed to cover all the execution trace.
        let num_instances = min_traces.len().div_ceil(num_within);

        Ok((0..num_instances)
            .map(|segment_id| {
                Plan::new(
                    ZISK_AIRGROUP_ID,
                    MAIN_AIR_IDS[0],
                    Some(SegmentId(segment_id)),
                    InstanceType::Instance,
                    CheckPoint::Single(ChunkId(segment_id)),
                    Some(Box::new(segment_id == num_instances - 1) as Box<dyn Any>),
                )
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NUM_ROWS: usize = MainTrace::<()>::NUM_ROWS;

    fn n_default_traces(n: usize) -> Vec<EmuTrace> {
        vec![EmuTrace::default(); n]
    }

    /// Decode the `is_last_segment` bool out of `plan.meta`.
    fn is_last(plan: &Plan) -> bool {
        *plan.meta.as_ref().unwrap().downcast_ref::<bool>().unwrap()
    }

    #[test]
    fn chunk_size_not_power_of_two_errors() {
        let traces = n_default_traces(1);
        let err = MainPlanner::plan(&traces, 3).unwrap_err();
        assert!(matches!(err, MainSmError::ChunkSizeNotPowerOfTwo { size: 3 }));
    }

    #[test]
    fn chunk_size_zero_errors() {
        // 0 is not a power of two per Rust's `is_power_of_two()` definition.
        let traces = n_default_traces(1);
        let err = MainPlanner::plan(&traces, 0).unwrap_err();
        assert!(matches!(err, MainSmError::ChunkSizeNotPowerOfTwo { size: 0 }));
    }

    #[test]
    fn chunk_size_too_big_errors() {
        // 2 * NUM_ROWS is power of two but exceeds the row capacity of MainTrace.
        let traces = n_default_traces(1);
        let oversized = (NUM_ROWS as u64) * 2;
        let err = MainPlanner::plan(&traces, oversized).unwrap_err();
        assert!(matches!(err, MainSmError::ChunkSizeTooBig { .. }));
    }

    #[test]
    fn single_full_segment_when_traces_equal_num_within() {
        // chunk_size = NUM_ROWS → num_within = 1, so 1 trace = 1 segment.
        let traces = n_default_traces(1);
        let plans = MainPlanner::plan(&traces, NUM_ROWS as u64).unwrap();
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].segment_id, Some(SegmentId(0)));
        assert!(is_last(&plans[0]));
    }

    #[test]
    fn multiple_full_segments_have_sequential_ids() {
        // chunk_size = NUM_ROWS / 2 → num_within = 2. With 4 traces → 2 segments.
        let traces = n_default_traces(4);
        let size = (NUM_ROWS as u64) / 2;
        let plans = MainPlanner::plan(&traces, size).unwrap();
        assert_eq!(plans.len(), 2);
        assert_eq!(plans[0].segment_id, Some(SegmentId(0)));
        assert_eq!(plans[1].segment_id, Some(SegmentId(1)));
        assert!(!is_last(&plans[0]));
        assert!(is_last(&plans[1]));
    }

    #[test]
    fn partial_last_segment_uses_ceil_div() {
        // num_within = 2, 3 traces → ceil(3 / 2) = 2 segments. Last is partial.
        let traces = n_default_traces(3);
        let size = (NUM_ROWS as u64) / 2;
        let plans = MainPlanner::plan(&traces, size).unwrap();
        assert_eq!(plans.len(), 2);
        assert!(!is_last(&plans[0]));
        assert!(is_last(&plans[1]));
    }

    #[test]
    fn empty_min_traces_produces_empty_plan() {
        let traces: Vec<EmuTrace> = vec![];
        let plans = MainPlanner::plan(&traces, NUM_ROWS as u64).unwrap();
        assert!(plans.is_empty());
    }

    #[test]
    fn plan_fields_match_main_air_constants() {
        let traces = n_default_traces(1);
        let plans = MainPlanner::plan(&traces, NUM_ROWS as u64).unwrap();
        let plan = &plans[0];
        assert_eq!(plan.airgroup_id, ZISK_AIRGROUP_ID);
        assert_eq!(plan.air_id, MAIN_AIR_IDS[0]);
        assert!(matches!(plan.instance_type, InstanceType::Instance));
        // Checkpoint's ChunkId is the same usize as segment_id.
        assert!(matches!(plan.check_point, CheckPoint::Single(ChunkId(0))));
    }

    #[test]
    fn is_last_segment_metadata_decodes_to_bool() {
        // num_within = 2, 5 traces → ceil(5 / 2) = 3 segments → flags [false, false, true].
        let traces = n_default_traces(5);
        let plans = MainPlanner::plan(&traces, (NUM_ROWS as u64) / 2).unwrap();
        assert_eq!(plans.len(), 3);
        let flags: Vec<bool> = plans.iter().map(is_last).collect();
        assert_eq!(flags, vec![false, false, true]);
    }
}
