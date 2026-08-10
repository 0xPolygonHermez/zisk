//! The `MainPlanner` module defines a planner for the Main State Machine.
//!
//! It generates execution plans for segments of the main trace, mapping each segment
//! to a specific `Plan` instance.

use crate::{MainSmError, Result};
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
        Self::plan_count(min_traces.len(), chunk_size)
    }

    /// Same as [`Self::plan`] but from the chunk count alone (plans depend only
    /// on how many minimal-trace chunks exist).
    pub fn plan_count(num_chunks: usize, chunk_size: u64) -> Result<Vec<Plan>> {
        let num_within = Self::traces_per_segment(chunk_size)?;

        // Number of `Main` segments needed to cover all the execution trace.
        let num_instances = num_chunks.div_ceil(num_within);

        Ok((0..num_instances)
            .map(|segment_id| Self::plan_segment(segment_id, segment_id == num_instances - 1))
            .collect())
    }

    /// Number of minimal-trace chunks wrapped in one main trace segment.
    ///
    /// # Errors
    /// Same `chunk_size` validation as [`Self::plan`].
    pub fn traces_per_segment(chunk_size: u64) -> Result<usize> {
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

        Ok(NUM_ROWS / chunk_size)
    }

    /// The Main segment that chunk `chunk_idx` completes, or `None` when that
    /// chunk neither fills a segment nor ends the execution.
    ///
    /// `num_within` comes from [`Self::traces_per_segment`] (always `>= 1`).
    /// Incremental main advancement calls this once per streamed minimal-trace
    /// chunk: a segment is complete either at its own boundary (`chunk_idx + 1`
    /// is a multiple of `num_within`) or on the chunk that ends the execution,
    /// which closes the final — possibly partial — segment.
    pub fn segment_completed_by(
        chunk_idx: usize,
        num_within: usize,
        is_last_chunk: bool,
    ) -> Option<usize> {
        ((chunk_idx + 1) % num_within == 0 || is_last_chunk).then_some(chunk_idx / num_within)
    }

    /// Builds the plan of a single Main segment. Used by the incremental
    /// main-witness advancement: segments become plannable one by one while
    /// the emulation streams chunks (`is_last_segment` for a full segment is
    /// known as soon as one chunk beyond it exists; the final segment is
    /// planned once the emulation ends and the total count is known).
    pub fn plan_segment(segment_id: usize, is_last_segment: bool) -> Plan {
        Plan::new(
            ZISK_AIRGROUP_ID,
            MAIN_AIR_IDS[0],
            Some(SegmentId(segment_id)),
            InstanceType::Instance,
            CheckPoint::Single(ChunkId(segment_id)),
            Some(Box::new(is_last_segment)),
        )
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
    fn segment_completed_by_releases_only_on_boundaries() {
        // num_within = 4: only chunk 3 completes segment 0, only chunk 7 segment 1.
        let released: Vec<Option<usize>> =
            (0..8).map(|idx| MainPlanner::segment_completed_by(idx, 4, false)).collect();
        assert_eq!(
            released,
            vec![None, None, None, Some(0), None, None, None, Some(1)],
            "a segment is released by its last chunk only"
        );
    }

    #[test]
    fn segment_completed_by_closes_partial_segment_on_last_chunk() {
        // Chunk 5 is mid-segment, but it ends the execution: segment 1 is released
        // partial rather than waiting for a boundary that will never arrive.
        assert_eq!(MainPlanner::segment_completed_by(5, 4, true), Some(1));
        // A single chunk that is also the last one closes segment 0.
        assert_eq!(MainPlanner::segment_completed_by(0, 4, true), Some(0));
    }

    #[test]
    fn segment_completed_by_boundary_and_last_release_one_segment() {
        // Chunk 7 both fills segment 1 and ends the execution — it must resolve to
        // exactly one segment, not release segment 1 and then a phantom segment 2.
        assert_eq!(MainPlanner::segment_completed_by(7, 4, true), Some(1));
    }

    #[test]
    fn segment_completed_by_one_chunk_per_segment() {
        // chunk_size == NUM_ROWS ⇒ num_within = 1: every chunk completes its own segment.
        for idx in 0..4 {
            assert_eq!(MainPlanner::segment_completed_by(idx, 1, false), Some(idx));
        }
    }

    #[test]
    fn incremental_release_matches_batch_plan() {
        // Releasing chunk-by-chunk must yield exactly what the batch planner
        // yields — same segment ids, same order, same `is_last_segment` flags —
        // for every chunk count, partial tails included.
        let chunk_size = (NUM_ROWS as u64) / 4;
        let num_within = MainPlanner::traces_per_segment(chunk_size).unwrap();

        for num_chunks in 1..=13usize {
            let incremental: Vec<(usize, bool)> = (0..num_chunks)
                .filter_map(|idx| {
                    let is_last_chunk = idx == num_chunks - 1;
                    MainPlanner::segment_completed_by(idx, num_within, is_last_chunk)
                        .map(|segment| (segment, is_last_chunk))
                })
                .collect();

            let batch: Vec<(usize, bool)> = MainPlanner::plan_count(num_chunks, chunk_size)
                .unwrap()
                .iter()
                .map(|plan| (plan.segment_id.unwrap().as_usize(), is_last(plan)))
                .collect();

            assert_eq!(incremental, batch, "chunk count {num_chunks}");
        }
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
