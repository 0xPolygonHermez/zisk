//! The `DmaPlanner` module defines a planner for generating execution plans specific to
//! arithmetic operations.
//!
//! It organizes execution plans for both regular instances and table instances,
//! leveraging arithmetic operation counts and metadata to construct detailed plans.

use crate::DmaCounterInputGen;
use std::any::Any;
use std::collections::HashMap;

use fields::PrimeField64;
use zisk_common::{
    BusDeviceMetrics, CheckPoint, ChunkId, CollectCounter, InstanceType, Plan, Planner, SegmentId,
};
use zisk_pil::{Dma64AlignedTrace, DmaPrePostTrace, DmaTrace, DmaUnalignedTrace};

/// The `DmaPlanner` struct organizes execution plans for arithmetic instances and tables.
///
/// It allows adding metadata about instances and tables and generates plans
/// based on the provided counters.
#[derive(Default)]
pub struct DmaPlanner<F> {
    _marker: std::marker::PhantomData<F>,
}

#[derive(Default)]
pub struct DmaCheckPoint {
    pub chunks: HashMap<ChunkId, (u64, CollectCounter)>,
    pub last_chunk: Option<ChunkId>,
    pub is_last_segment: bool,
}

/// Macro to generate a plan function for a specific field of a struct.
///
/// This macro creates a function that generates checkpoints from counts across multiple chunks,
/// allowing you to specify which field of the struct to use for counting.
///
/// # Macro Arguments
/// * `$fn_name` - The name of the generated function.
/// * `$type` - The struct type containing the count field (must have a `chunk_id: ChunkId` field).
/// * `$field` - The field name to use as the count value (must be `u64`).
///
/// # Generated Function
/// The generated function has the signature:
/// ```ignore
/// pub fn $fn_name(
///     counts: &[$type],
///     size: u64,
/// ) -> Vec<(CheckPoint, , bool>)>
/// ```
///
/// # Example
/// ```ignore
/// define_plan_for_field!(plan_by_inst_count, InstFropsCount, inst_count);
/// define_plan_for_field!(plan_by_frops_count, InstFropsCount, frops_count);
/// ```
macro_rules! define_plan_for_field {
    ($fn_name:ident, $type:ty, $field:ident) => {
        define_plan_for_field!($fn_name, $type, $field, false, $field);
    };
    ($fn_name:ident, $type:ty, $field:ident, $field_inputs: ident) => {
        define_plan_for_field!($fn_name, $type, $field, true, $field_inputs);
    };
    ($fn_name:ident, $type:ty, $field:ident, $has_input_counter: literal, $field_inputs: ident) => {
        pub fn $fn_name(
            counts: &Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>,
            size: u64,
        ) -> Vec<(CheckPoint, DmaCheckPoint)> {
            if counts.is_empty() || size == 0 {
                return vec![];
            }
            // let tag = stringify!($field);
            // let total = counts.len();
            // println!("plan_{tag} counts:{total} size:{size}");
            let mut checkpoints = Vec::new();
            let mut current_scope =
                DmaCheckPoint { chunks: HashMap::new(), last_chunk: None, is_last_segment: false };
            let mut remaining_size = size; // Remaining size for the current scope.

            for (current_chunk, dyn_counter) in counts.iter() {
                let counter = (**dyn_counter).as_any().downcast_ref::<$type>().unwrap();
                // println!("counter: {:?}", counter);
                let mut inst_count = counter.$field as u64;
                let mut cumulative_offset = 0u64; // Reset cumulative offset for each chunk.

                while inst_count > 0 {
                    let checkpoint_size = remaining_size.min(inst_count);
                    // println!("plan_{tag} C:{}/{total} #{current_chunk} I:{remaining_size}/{size} +{checkpoint_size}/{inst_count} skip:{cumulative_offset} count:{checkpoint_size}", index+1);

                    current_scope.chunks.insert(
                        *current_chunk,
                        (
                            // Use input counter to calculate the capacity of collector inputs vector, used
                            // for state machines that has different number of rows by input.
                            if $has_input_counter {
                                counter.$field_inputs as u64
                            } else {
                                checkpoint_size
                            },
                            CollectCounter::new(cumulative_offset as u32, checkpoint_size as u32),
                        ),
                    );
                    current_scope.last_chunk = Some(*current_chunk);

                    cumulative_offset += checkpoint_size;
                    inst_count -= checkpoint_size;
                    remaining_size -= checkpoint_size;

                    if remaining_size == 0 {
                        // println!("plan_{tag} adding instance .... inst_count = {inst_count}");
                        let keys = current_scope.chunks.keys().cloned().collect::<Vec<_>>();
                        checkpoints
                            .push((CheckPoint::Multiple(keys), std::mem::take(&mut current_scope)));
                        remaining_size = size;
                    }
                }
            }
            // println!("plan_{tag} final counters interation");
            // Push any remaining checkpoints into the result.
            if !current_scope.chunks.is_empty() {
                let keys = current_scope.chunks.keys().cloned().collect::<Vec<_>>();
                current_scope.is_last_segment = true;
                checkpoints.push((CheckPoint::Multiple(keys), std::mem::take(&mut current_scope)));
            } else if let Some(last) = checkpoints.last_mut() {
                last.1.is_last_segment = true;
            }

            checkpoints
        }
    };
}

impl<F: PrimeField64> DmaPlanner<F> {
    /// Creates a new `DmaPlanner`.
    ///
    /// # Returns
    /// A new `DmaPlanner` instance with no preconfigured instances or tables.
    pub fn new() -> Self {
        Self { _marker: std::marker::PhantomData }
    }

    define_plan_for_field!(plan_dma_controller, DmaCounterInputGen, dma_ops);
    define_plan_for_field!(plan_dma_pre_post, DmaCounterInputGen, dma_pre_post_ops);
    define_plan_for_field!(
        plan_dma_unaligned,
        DmaCounterInputGen,
        dma_unaligned_rows,
        dma_unaligned_inputs
    );
    define_plan_for_field!(
        plan_dma_64_aligned,
        DmaCounterInputGen,
        dma_64_aligned_rows,
        dma_64_aligned_inputs
    );
}

impl<F: PrimeField64> Planner for DmaPlanner<F> {
    /// Generates execution plans for Dma instances.
    ///
    /// # Arguments
    /// * `counters` - A vector of counters, each associated with a `ChunkId` and `DmaCounter`
    ///   metrics data.
    ///
    /// # Returns
    /// A vector of `Plan` instances representing execution configurations for the instances
    ///
    /// # Panics
    /// Panics if any counter cannot be downcasted to an `DmaCounter`.
    fn plan(&self, counters: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>) -> Vec<Plan> {
        let mut dma_plans: Vec<Plan> =
            Self::plan_dma_controller(&counters, DmaTrace::<F>::NUM_ROWS as u64)
                .into_iter()
                .map(|(check_point, collect_info)| {
                    let converted: Box<dyn Any> = Box::new(collect_info);
                    Plan::new(
                        DmaTrace::<F>::AIRGROUP_ID,
                        DmaTrace::<F>::AIR_ID,
                        None,
                        InstanceType::Instance,
                        check_point,
                        Some(converted),
                    )
                })
                .collect();

        let pre_post_plans: Vec<Plan> =
            Self::plan_dma_pre_post(&counters, DmaPrePostTrace::<F>::NUM_ROWS as u64)
                .into_iter()
                .map(|(check_point, collect_info)| {
                    let converted: Box<dyn Any> = Box::new(collect_info);
                    Plan::new(
                        DmaPrePostTrace::<F>::AIRGROUP_ID,
                        DmaPrePostTrace::<F>::AIR_ID,
                        None,
                        InstanceType::Instance,
                        check_point,
                        Some(converted),
                    )
                })
                .collect();
        dma_plans.extend(pre_post_plans);

        let aligned_plans: Vec<Plan> =
            Self::plan_dma_64_aligned(&counters, Dma64AlignedTrace::<F>::NUM_ROWS as u64)
                .into_iter()
                .enumerate()
                .map(|(segment_id, (check_point, collect_info))| {
                    let converted: Box<dyn Any> = Box::new(collect_info);
                    Plan::new(
                        Dma64AlignedTrace::<F>::AIRGROUP_ID,
                        Dma64AlignedTrace::<F>::AIR_ID,
                        Some(SegmentId(segment_id)),
                        InstanceType::Instance,
                        check_point,
                        Some(converted),
                    )
                })
                .collect();
        dma_plans.extend(aligned_plans);

        let unaligned_plans: Vec<Plan> =
            Self::plan_dma_unaligned(&counters, DmaUnalignedTrace::<F>::NUM_ROWS as u64)
                .into_iter()
                .enumerate()
                .map(|(segment_id, (check_point, collect_info))| {
                    let converted: Box<dyn Any> = Box::new(collect_info);
                    Plan::new(
                        DmaUnalignedTrace::<F>::AIRGROUP_ID,
                        DmaUnalignedTrace::<F>::AIR_ID,
                        Some(SegmentId(segment_id)),
                        InstanceType::Instance,
                        check_point,
                        Some(converted),
                    )
                })
                .collect();
        dma_plans.extend(unaligned_plans);

        dma_plans
    }
}
