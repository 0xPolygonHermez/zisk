//! The `MainPlanner` module defines a planner for the Main State Machine.
//!
//! It generates execution plans for segments of the main trace, mapping each segment
//! to a specific `Plan` instance.

use p3_field::PrimeField;
use sm_common::{BusDeviceMetrics, CheckPoint, ChunkId, InstanceType, Metrics, Plan};
use zisk_common::EmuTrace;
use zisk_pil::{MainTrace, MAIN_AIR_IDS, ZISK_AIRGROUP_ID};

use crate::MainCounter;

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
    /// * `main_counters` - A vector of main counters, each associated with a specific chunk ID.
    /// * `min_traces_size` - The size of the minimal traces.
    ///
    /// # Returns
    /// A vector of `Plan` instances, each corresponding to a segment of the main trace.
    pub fn plan<F: PrimeField>(
        min_traces: &[EmuTrace],
        main_counters: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>,
        min_traces_size: u64,
    ) -> (Vec<Plan>, Vec<(u64, u32)>) {
        let num_rows = MainTrace::<F>::NUM_ROWS as u64;

        let mut publics = Vec::new();

        main_counters.iter().for_each(|(_, counter)| {
            let reg_counter = Metrics::as_any(&**counter).downcast_ref::<MainCounter>().unwrap();
            publics.extend_from_slice(&reg_counter.publics);
        });

        assert!(num_rows.is_power_of_two());
        assert!(min_traces_size.is_power_of_two());
        assert!(num_rows >= min_traces_size);

        // This is the number of minimal traces wrapped in a main trace
        let num_within = num_rows / min_traces_size;
        let num_instances = (min_traces.len() as f64 / num_within as f64).ceil() as usize;

        let plans = (0..num_instances)
            .map(|segment_id| {
                Plan::new(
                    ZISK_AIRGROUP_ID,
                    MAIN_AIR_IDS[0],
                    Some(segment_id),
                    InstanceType::Instance,
                    CheckPoint::Single(segment_id),
                    None,
                )
            })
            .collect();

        (plans, publics)
    }
}
