//! The `MainPlanner` module defines a planner for the Main State Machine.
//!
//! It generates execution plans for segments of the main trace, mapping each segment
//! to a specific `Plan` instance.

use sm_common::{CheckPoint, CollectSkipper, InstanceType, Plan};
use zisk_pil::{MAIN_AIR_IDS, ZISK_AIRGROUP_ID};
use ziskemu::EmuTrace;

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
    ///
    /// # Returns
    /// A vector of `Plan` instances, each corresponding to a segment of the main trace.
    pub fn plan(min_traces: &[EmuTrace]) -> Vec<Plan> {
        (0..min_traces.len())
            .map(|segment_id| {
                Plan::new(
                    ZISK_AIRGROUP_ID,
                    MAIN_AIR_IDS[0],
                    Some(segment_id),
                    InstanceType::Instance,
                    CheckPoint::Single(segment_id),
                    Some(Box::new(CollectSkipper::new(0))),
                    None,
                )
            })
            .collect()
    }
}
