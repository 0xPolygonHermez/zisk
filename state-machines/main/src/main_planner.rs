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
    /// * `min_traces_size` - The size of the minimal traces.
    ///
    /// # Returns
    /// A vector of `Plan` instances, each corresponding to a segment of the main trace.
    /// # Errors
    /// Returns a `MainSmError` when:
    /// - The `min_traces_size` is not a power of two ([`MainSmError::MinTraceSizeNotPowerOfTwo`]).
    /// - The `min_traces_size` exceeds the row capacity of `MainTrace` ([`MainSmError::MinTraceSizeTooBig`]).
    /// - A `u64` quantity could not be converted to `usize` on this target ([`MainSmError::IntConversion`]).
    pub fn plan(min_traces: &[EmuTrace], min_traces_size: u64) -> Result<Vec<Plan>> {
        const NUM_ROWS: u64 = MainTrace::<()>::NUM_ROWS as u64;

        // Compile-time assertion to ensure `MainTrace::NUM_ROWS` is a power of two.
        const _: () =
            assert!(NUM_ROWS.is_power_of_two(), "MainTrace::NUM_ROWS must be a power of two",);

        // Compile-time assertion to ensure `MainTrace::NUM_ROWS` does not exceed `usize::MAX`.
        const _: () = assert!(
            NUM_ROWS <= usize::MAX as u64,
            "MainTrace::NUM_ROWS exceeds usize::MAX on this target",
        );

        if !min_traces_size.is_power_of_two() {
            return Err(MainSmError::MinTraceSizeNotPowerOfTwo { size: min_traces_size });
        }

        if NUM_ROWS < min_traces_size {
            return Err(MainSmError::MinTraceSizeTooBig { min_traces_size, num_rows: NUM_ROWS });
        }

        // Number of minimal traces wrapped in a main trace.
        let num_within = NUM_ROWS / min_traces_size;

        // Number of `Main` segments needed to cover all the execution trace.
        let num_instances: u64 = (min_traces.len() as u64).div_ceil(num_within);
        let num_instances: usize = num_instances.try_into()?;

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
