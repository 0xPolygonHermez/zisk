//! The `DmaStrategy` module defines a strategy planner for generating execution plans specific to
//! DMA operations.
//!
//! It organizes execution plans for DMA instance types (full, memcpy, memset, inputcpy, mem),
//! leveraging operation counters to select the assignment that minimises total proving cost.

use core::panic;
use std::fmt;

use crate::{
    DmaCheckPoint, DmaCounterInputGen, DmaInstancesBuilder, DMA_64_ALIGNED_INPUTS_OFFSET,
    DMA_64_ALIGNED_OFFSET, DMA_COUNTER_INPUTCPY, DMA_COUNTER_MEMCMP, DMA_COUNTER_MEMCPY,
    DMA_COUNTER_MEMCPY_8, DMA_COUNTER_MEMSET, DMA_COUNTER_MEMSET_8, DMA_COUNTER_OPS,
    DMA_COUNTER_OPS_EXT, DMA_INPUT_GEN_COUNTERS, DMA_OFFSET, DMA_PRE_POST_OFFSET,
    DMA_UNALIGNED_INPUTS_OFFSET, DMA_UNALIGNED_OFFSET,
};

#[cfg(feature = "save_dma_plans")]
use crate::get_dma_air_name;

use fields::PrimeField64;
use zisk_common::{BusDeviceMetrics, BusDeviceMode, CheckPoint, ChunkId};
use zisk_core::{
    DMA_64_ALIGNED_COST, DMA_64_ALIGNED_INPUTCPY_COST, DMA_64_ALIGNED_MEMCPY_COST,
    DMA_64_ALIGNED_MEMSET_COST, DMA_64_ALIGNED_MEM_COST,
};

use zisk_pil::{
    Dma64AlignedInputCpyTrace, Dma64AlignedMemCpyTrace, Dma64AlignedMemSetTrace,
    Dma64AlignedMemTrace, Dma64AlignedTrace, DmaInputCpyTrace, DmaMemCpyTrace,
    DmaPrePostInputCpyTrace, DmaPrePostMemCpyTrace, DmaPrePostTrace, DmaTrace, DmaUnalignedTrace,
};

#[derive(Debug, Default, Clone)]
pub struct DmaInstances {
    // memcpy: memcpy ==> full
    // memcmp: full
    // memset: full
    // inputcpy: input_cpy ==> full
    pub full: usize,
    pub memcpy: usize,
    pub inputcpy: usize,
    pub rows_memcpy_to_full: usize,
    pub rows_inputcpy_to_full: usize,
}

impl fmt::Display for DmaInstances {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "  full      {:>3}\n  \
               memcpy    {:>3} {:>6} → full\n  \
               inputcpy  {:>3} {:>6} → full\n",
            self.full,
            self.memcpy,
            self.rows_memcpy_to_full,
            self.inputcpy,
            self.rows_inputcpy_to_full
        )
    }
}

#[derive(Debug, Default, Clone)]
pub struct Dma64AlignedInstances {
    // memcpy: memcpy ==> mem ==> full
    // memcmp: mem ==> full
    // memset: memset ==> mem ==> full
    // inputcpy: input_cpy ==> full
    pub full: usize,
    pub memcpy: usize,
    pub inputcpy: usize,
    pub mem: usize,
    pub memset: usize,

    pub rows_memcpy_to_mem: usize,
    pub rows_memcpy_to_full: usize,
    pub rows_inputcpy_to_full: usize,
    pub rows_memset_to_mem: usize,
    pub rows_memset_to_full: usize,
    pub rows_memcmp_to_full: usize,
}

impl fmt::Display for Dma64AlignedInstances {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "  full      {:>3}\n  \
               memcpy    {:>3} {:>12} → mem   {:>12} → full\n  \
               inputcpy  {:>3} {:>12} → full\n  \
               mem       {:>3}\n  \
               memset    {:>3} {:>12} → mem   {:>12} → full\n  \
               memcmp      - {:>12} → full\n",
            self.full,
            self.memcpy,
            self.rows_memcpy_to_mem,
            self.rows_memcpy_to_full,
            self.inputcpy,
            self.rows_inputcpy_to_full,
            self.mem,
            self.memset,
            self.rows_memset_to_mem,
            self.rows_memset_to_full,
            self.rows_memcmp_to_full
        )
    }
}

/// The `DmaStrategy` struct selects the optimal assignment of DMA operation types to instance
/// types and generates the execution plans for each instance.
#[derive(Default)]
pub struct DmaStrategy<F> {
    pub dma: DmaInstances,
    pub dma_pre_post: DmaInstances,
    pub dma_64_aligned: Dma64AlignedInstances,
    pub dma_unaligned: usize,
    _marker: std::marker::PhantomData<F>,
}

impl<F> fmt::Display for DmaStrategy<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "────────────────────────────────────────── DMA\n\
             {}\
             ───────────────────────────────── DMA_PRE_POST\n\
             {}\
             ─────────────────────────────── DMA_64_ALIGNED\n\
             {}\
             ──────────────────────────────── DMA_UNALIGNED\n  \
             full      {:>3}\n\n",
            self.dma, self.dma_pre_post, self.dma_64_aligned, self.dma_unaligned,
        )
    }
}

/// Describes an instance type for use in the alignment strategy optimizer.
pub struct AlignmentInstanceInfo {
    /// Number of rows available per instance.
    pub rows: usize,
    /// Cost per instance (applies to the whole instance, regardless of fill level).
    pub cost: usize,
}

impl<F: PrimeField64> DmaStrategy<F> {
    /// Creates a new `DmaStrategy` with default (zero) counters.
    pub fn new() -> Self {
        Self::default()
    }

    // define_plan_for_field!(plan_dma_controller, DmaCounterInputGen, dma_ops);
    // define_plan_for_field!(plan_dma_pre_post, DmaCounterInputGen, dma_pre_post_ops);
    // define_plan_for_field!(
    //     plan_dma_unaligned,
    //     DmaCounterInputGen,
    //     dma_unaligned_rows,
    //     dma_unaligned_inputs
    // );
    // define_plan_for_field!(
    //     plan_dma_64_aligned,
    //     DmaCounterInputGen,
    //     dma_64_aligned_rows,
    //     dma_64_aligned_inputs
    // );

    fn calculate_totals(
        &self,
        counters: &Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>,
    ) -> DmaCounterInputGen {
        let mut totals = DmaCounterInputGen::new(BusDeviceMode::Counter);
        for (_, counter) in counters.iter() {
            let counter = (**counter).as_any().downcast_ref::<DmaCounterInputGen>().unwrap();
            for i in 0..DMA_INPUT_GEN_COUNTERS {
                totals.counters[i] += counter.counters[i];
            }
        }
        totals
    }

    const DMA_ROWS: usize = DmaTrace::<()>::NUM_ROWS;
    const DMA_MEMCPY_ROWS: usize = DmaMemCpyTrace::<()>::NUM_ROWS;
    const DMA_INPUTCPY_ROWS: usize = DmaInputCpyTrace::<()>::NUM_ROWS;
    const DMA_PRE_POST_ROWS: usize = DmaPrePostTrace::<()>::NUM_ROWS;
    const DMA_PRE_POST_MEMCPY_ROWS: usize = DmaPrePostMemCpyTrace::<()>::NUM_ROWS;
    const DMA_PRE_POST_INPUTCPY_ROWS: usize = DmaPrePostInputCpyTrace::<()>::NUM_ROWS;
    const DMA_64_ALIGNED_ROWS: usize = Dma64AlignedTrace::<()>::NUM_ROWS;
    const DMA_64_ALIGNED_MEMCPY_ROWS: usize = Dma64AlignedMemCpyTrace::<()>::NUM_ROWS;
    const DMA_64_ALIGNED_MEMSET_ROWS: usize = Dma64AlignedMemSetTrace::<()>::NUM_ROWS;
    const DMA_64_ALIGNED_INPUTCPY_ROWS: usize = Dma64AlignedInputCpyTrace::<()>::NUM_ROWS;
    const DMA_64_ALIGNED_MEM_ROWS: usize = Dma64AlignedMemTrace::<()>::NUM_ROWS;
    const DMA_UNALIGNED_ROWS: usize = DmaUnalignedTrace::<()>::NUM_ROWS;
    // Dma
    // DmaMemCpy
    // DmaInputCpy
    pub fn calculate_dma_strategy(
        rows: &[usize],
        rows_x_full_instance: usize,
        rows_x_memcpy_instance: usize,
        rows_x_inputcpy_instance: usize,
        info: &mut DmaInstances,
    ) {
        let rows_full = rows[DMA_COUNTER_MEMSET] + rows[DMA_COUNTER_MEMCMP];
        let rows_memcpy = rows[DMA_COUNTER_MEMCPY];
        let rows_inputcpy = rows[DMA_COUNTER_INPUTCPY];

        info.full = rows_full.div_ceil(rows_x_full_instance);
        info.memcpy = rows_memcpy.div_ceil(rows_x_memcpy_instance);
        info.inputcpy = rows_inputcpy.div_ceil(rows_x_inputcpy_instance);

        let remain_dma = rows_full % rows_x_full_instance;
        let available_on_dma = if rows_full == 0 { 0 } else { rows_x_full_instance - remain_dma };
        let remain_dma_memcpy = rows_memcpy % rows_x_memcpy_instance;
        let remain_dma_inputcpy = rows_inputcpy % rows_x_inputcpy_instance;
        let remain = remain_dma_memcpy + remain_dma_inputcpy;

        if remain <= available_on_dma {
            if remain_dma_memcpy > 0 {
                info.memcpy -= 1;
                info.rows_memcpy_to_full = remain_dma_memcpy;
            }
            if remain_dma_inputcpy > 0 {
                info.inputcpy -= 1;
                info.rows_inputcpy_to_full = remain_dma_inputcpy;
            }
        } else if remain_dma_memcpy <= available_on_dma {
            if remain_dma_memcpy > 0 {
                info.memcpy -= 1;
                info.rows_memcpy_to_full = remain_dma_memcpy;
            }
        } else if remain_dma_inputcpy <= available_on_dma {
            if remain_dma_inputcpy > 0 {
                info.inputcpy -= 1;
                info.rows_inputcpy_to_full = remain_dma_inputcpy;
            }
        } else if remain_dma_memcpy > 0
            && remain_dma_inputcpy > 0
            && remain <= (available_on_dma + rows_x_full_instance)
        {
            // COST(Dma) < COST(DmaMemCpy) + COST(DmaInputCpy)
            info.memcpy -= 1;
            info.inputcpy -= 1;
            info.full += 1;
            info.rows_memcpy_to_full = remain_dma_memcpy;
            info.rows_inputcpy_to_full = remain_dma_inputcpy;
        }
    }

    /// Finds the assignment of operation types to instance types that minimises total cost.
    ///
    /// # Parameters
    /// - `ops`       – for each operation type, a list of valid `(instance_index, total_rows)`
    ///   pairs. `total_rows` is the number of rows this operation would occupy in
    ///   that instance and may differ per instance alternative.
    /// - `instances` – for each instance type, its row capacity and per-instance cost.
    ///
    /// # Returns
    /// A tuple `(instance_counts, op_assignments)` where:
    /// - `instance_counts[j]` = number of instances of type `j` needed (`ceil(rows/capacity)`).
    /// - `op_assignments[i]`  = index into `instances` of the selected instance type for op `i`.
    ///
    /// # Cost model
    /// For every instance type `j` that receives at least one operation:
    ///
    /// ```text
    /// cost_j = ceil(total_rows_j / instances[j].rows) * instances[j].cost
    /// ```
    ///
    /// where `total_rows_j = Σ rows` for all operations `i` assigned to `j`.
    /// Instance types with zero total rows incur no cost.
    pub fn calculate_alignment_strategy(
        ops: &[Vec<(usize, usize)>],
        instances: &[AlignmentInstanceInfo],
    ) -> (Vec<usize>, Vec<usize>) {
        let num_ops = ops.len();
        if num_ops == 0 {
            return (vec![0; instances.len()], vec![]);
        }

        let mut best_cost = usize::MAX;
        let mut best_assignment: Vec<usize> = ops.iter().map(|op| op[0].0).collect();
        let mut best_instance_rows = vec![0usize; instances.len()];

        // Mixed-radix counter: combo_indices[i] indexes into ops[i].
        let mut combo_indices = vec![0usize; num_ops];

        loop {
            // Accumulate rows per instance type for this combination.
            let mut instance_rows = vec![0usize; instances.len()];
            for (op_idx, &ci) in combo_indices.iter().enumerate() {
                let (inst_idx, rows) = ops[op_idx][ci];
                instance_rows[inst_idx] += rows;
            }

            // Evaluate total cost.
            let total_cost: usize = instances
                .iter()
                .enumerate()
                .map(|(j, inst)| {
                    if instance_rows[j] == 0 {
                        0
                    } else {
                        instance_rows[j].div_ceil(inst.rows) * inst.cost
                    }
                })
                .sum();

            if total_cost < best_cost {
                best_cost = total_cost;
                best_assignment =
                    combo_indices.iter().zip(ops.iter()).map(|(&ci, op)| op[ci].0).collect();
                best_instance_rows = instance_rows;
            }

            // Advance the mixed-radix counter from the rightmost digit.
            let mut pos = num_ops;
            loop {
                if pos == 0 {
                    let instance_counts = instances
                        .iter()
                        .enumerate()
                        .map(|(j, inst)| {
                            if best_instance_rows[j] == 0 {
                                0
                            } else {
                                best_instance_rows[j].div_ceil(inst.rows)
                            }
                        })
                        .collect();
                    return (instance_counts, best_assignment);
                }
                pos -= 1;
                combo_indices[pos] += 1;
                if combo_indices[pos] < ops[pos].len() {
                    break;
                }
                combo_indices[pos] = 0;
            }
        }
    }

    const DMA_64_ALIGNED_AGGREGATE_COST: usize = 100;
    const DMA_64_ALIGNED_INSTANCE_INFO: [AlignmentInstanceInfo; 5] = [
        AlignmentInstanceInfo {
            rows: Self::DMA_64_ALIGNED_ROWS,
            cost: DMA_64_ALIGNED_COST as usize * Self::DMA_64_ALIGNED_ROWS
                + Self::DMA_64_ALIGNED_AGGREGATE_COST,
        }, // full
        AlignmentInstanceInfo {
            rows: Self::DMA_64_ALIGNED_MEMCPY_ROWS,
            cost: DMA_64_ALIGNED_MEMCPY_COST as usize * Self::DMA_64_ALIGNED_MEMCPY_ROWS
                + Self::DMA_64_ALIGNED_AGGREGATE_COST,
        }, // memcpy
        AlignmentInstanceInfo {
            rows: Self::DMA_64_ALIGNED_MEMSET_ROWS,
            cost: DMA_64_ALIGNED_MEMSET_COST as usize * Self::DMA_64_ALIGNED_MEMSET_ROWS
                + Self::DMA_64_ALIGNED_AGGREGATE_COST,
        }, // memset
        AlignmentInstanceInfo {
            rows: Self::DMA_64_ALIGNED_INPUTCPY_ROWS,
            cost: DMA_64_ALIGNED_INPUTCPY_COST as usize * Self::DMA_64_ALIGNED_INPUTCPY_ROWS
                + Self::DMA_64_ALIGNED_AGGREGATE_COST,
        }, // inputcpy
        AlignmentInstanceInfo {
            rows: Self::DMA_64_ALIGNED_MEM_ROWS,
            cost: DMA_64_ALIGNED_MEM_COST as usize * Self::DMA_64_ALIGNED_MEM_ROWS
                + Self::DMA_64_ALIGNED_AGGREGATE_COST,
        }, // memcmp
    ];

    // DmaPrePost
    // DmaPrePostMemCpy
    // DmaPrePostInputCpy
    // memcpy: memcpy ==> mem ==> full
    // memcmp: mem ==> full
    // memset: memset ==> mem ==> full
    // inputcpy: input_cpy ==> full

    pub fn calculate_dma_64_alignment_strategy(rows: &[usize], info: &mut Dma64AlignedInstances) {
        // Further optimization by mixing instance types within each chunk is not done here
        // because the per-operation row distribution within a chunk is not known at this stage.

        // INSTANCES: 0-FULL, 1-MEMCPY, 2-MEMSET, 3-INPUTCPY, 4-MEM
        // OPS: MEMCPY, MEMSET, MEMCMP, INPUTCPY
        let ops = [
            vec![
                (0, rows[DMA_COUNTER_MEMCPY]),
                (4, rows[DMA_COUNTER_MEMCPY]),
                (1, rows[DMA_COUNTER_MEMCPY_8]),
            ], // memcpy
            vec![
                (0, rows[DMA_COUNTER_MEMSET]),
                (4, rows[DMA_COUNTER_MEMSET]),
                (2, rows[DMA_COUNTER_MEMSET_8]),
            ], // memset
            vec![(0, rows[DMA_COUNTER_MEMCMP]), (4, rows[DMA_COUNTER_MEMCMP])], // memcmp
            vec![(0, rows[DMA_COUNTER_INPUTCPY]), (3, rows[DMA_COUNTER_INPUTCPY])], // inputcpy
        ];
        let (instances, ops_instance) =
            Self::calculate_alignment_strategy(&ops, &Self::DMA_64_ALIGNED_INSTANCE_INFO);

        info.full = instances[0];
        info.memcpy = instances[1];
        info.memset = instances[2];
        info.inputcpy = instances[3];
        info.mem = instances[4];

        info.rows_memcpy_to_full = if ops_instance[0] == 0 { rows[DMA_COUNTER_MEMCPY] } else { 0 };
        info.rows_memcpy_to_mem = if ops_instance[0] == 4 { rows[DMA_COUNTER_MEMCPY] } else { 0 };
        info.rows_memset_to_full = if ops_instance[1] == 0 { rows[DMA_COUNTER_MEMSET] } else { 0 };
        info.rows_memset_to_mem = if ops_instance[1] == 4 { rows[DMA_COUNTER_MEMSET] } else { 0 };
        info.rows_memcmp_to_full = if ops_instance[2] == 0 { rows[DMA_COUNTER_MEMCMP] } else { 0 };
        info.rows_inputcpy_to_full =
            if ops_instance[3] == 0 { rows[DMA_COUNTER_INPUTCPY] } else { 0 };
    }
    pub fn calculate_dma_unalignment_strategy(rows: &[usize]) -> usize {
        let rows = rows[DMA_COUNTER_MEMCPY]
            + rows[DMA_COUNTER_INPUTCPY]
            + rows[DMA_COUNTER_MEMSET]
            + rows[DMA_COUNTER_MEMCMP];
        rows.div_ceil(Self::DMA_UNALIGNED_ROWS)
    }
    fn calculate_strategy(&mut self, totals: &DmaCounterInputGen) {
        Self::calculate_dma_strategy(
            &totals.counters[DMA_OFFSET..DMA_OFFSET + DMA_COUNTER_OPS],
            Self::DMA_ROWS,
            Self::DMA_MEMCPY_ROWS,
            Self::DMA_INPUTCPY_ROWS,
            &mut self.dma,
        );
        Self::calculate_dma_strategy(
            &totals.counters[DMA_PRE_POST_OFFSET..DMA_PRE_POST_OFFSET + DMA_COUNTER_OPS],
            Self::DMA_PRE_POST_ROWS,
            Self::DMA_PRE_POST_MEMCPY_ROWS,
            Self::DMA_PRE_POST_INPUTCPY_ROWS,
            &mut self.dma_pre_post,
        );
        Self::calculate_dma_64_alignment_strategy(
            &totals.counters[DMA_64_ALIGNED_OFFSET..DMA_64_ALIGNED_OFFSET + DMA_COUNTER_OPS_EXT],
            &mut self.dma_64_aligned,
        );
        self.dma_unaligned = Self::calculate_dma_unalignment_strategy(
            &totals.counters[DMA_UNALIGNED_OFFSET..DMA_UNALIGNED_OFFSET + DMA_COUNTER_OPS],
        );
    }
    // DmaUnaligned =>
    // Dma64Aligned => decision by chunk
    // pub send_memcpy_to_mem: bool,
    // pub send_memcpy_to_full: bool,
    // pub send_inputcpy_to_full: bool,
    // pub send_memset_to_mem: bool,
    // pub send_memset_to_full: bool,
    // pub send_mem_to_full: bool,
    //
    // pub rows_memcpy_to_full: usize,
    // pub rows_inputcpy_to_full: usize,

    pub fn calculate(
        &mut self,
        counters: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>,
    ) -> Vec<(usize, Vec<(CheckPoint, DmaCheckPoint)>)> {
        let totals: DmaCounterInputGen = self.calculate_totals(&counters);
        #[cfg(feature = "save_dma_plans")]
        let totals_debug_info = format!("{}", totals);

        self.calculate_strategy(&totals);

        let mut dma_pre_post_full = DmaInstancesBuilder::new(
            "dma_pre_post_full",
            self.dma_pre_post.full,
            Self::DMA_PRE_POST_ROWS,
        );
        let mut dma_pre_post_memcpy = DmaInstancesBuilder::new(
            "dma_pre_post_memcpy",
            self.dma_pre_post.memcpy,
            Self::DMA_PRE_POST_MEMCPY_ROWS,
        );
        let mut dma_pre_post_inputcpy = DmaInstancesBuilder::new(
            "dma_pre_post_inputcpy",
            self.dma_pre_post.inputcpy,
            Self::DMA_PRE_POST_INPUTCPY_ROWS,
        );

        let mut dma_full = DmaInstancesBuilder::new("dma_full", self.dma.full, Self::DMA_ROWS);
        let mut dma_memcpy =
            DmaInstancesBuilder::new("dma_memcpy", self.dma.memcpy, Self::DMA_MEMCPY_ROWS);
        let mut dma_inputcpy =
            DmaInstancesBuilder::new("dma_inputcpy", self.dma.inputcpy, Self::DMA_INPUTCPY_ROWS);

        let mut dma_64_aligned_full = DmaInstancesBuilder::new(
            "dma_64_aligned_full",
            self.dma_64_aligned.full,
            Self::DMA_64_ALIGNED_ROWS,
        );
        let mut dma_64_aligned_memset = DmaInstancesBuilder::new(
            "dma_64_aligned_memset",
            self.dma_64_aligned.memset,
            Self::DMA_64_ALIGNED_MEMSET_ROWS,
        );
        let mut dma_64_aligned_memcpy = DmaInstancesBuilder::new(
            "dma_64_aligned_memcpy",
            self.dma_64_aligned.memcpy,
            Self::DMA_64_ALIGNED_MEMCPY_ROWS,
        );
        let mut dma_64_aligned_inputcpy = DmaInstancesBuilder::new(
            "dma_64_aligned_inputcpy",
            self.dma_64_aligned.inputcpy,
            Self::DMA_64_ALIGNED_INPUTCPY_ROWS,
        );
        let mut dma_64_aligned_mem = DmaInstancesBuilder::new(
            "dma_64_aligned_mem",
            self.dma_64_aligned.mem,
            Self::DMA_64_ALIGNED_MEM_ROWS,
        );

        let mut dma_unaligned =
            DmaInstancesBuilder::new("dma_unaligned", self.dma_unaligned, Self::DMA_UNALIGNED_ROWS);

        for (current_chunk, dyn_counter) in counters.iter() {
            let counters =
                (**dyn_counter).as_any().downcast_ref::<DmaCounterInputGen>().unwrap().counters;

            // DMA

            let rows = counters[DMA_OFFSET + DMA_COUNTER_MEMSET];
            if rows > 0 {
                dma_full.add_op_rows(*current_chunk, 0, rows, rows, DMA_COUNTER_MEMSET);
            }

            let rows = counters[DMA_OFFSET + DMA_COUNTER_MEMCMP];
            if rows > 0 {
                dma_full.add_op_rows(*current_chunk, 0, rows, rows, DMA_COUNTER_MEMCMP);
            }

            let mut rows = counters[DMA_OFFSET + DMA_COUNTER_MEMCPY];
            let skip = if rows > 0 && self.dma.rows_memcpy_to_full > 0 {
                let rows_applicable = std::cmp::min(rows, self.dma.rows_memcpy_to_full);
                dma_full.add_op_rows(
                    *current_chunk,
                    0,
                    rows_applicable,
                    rows_applicable,
                    DMA_COUNTER_MEMCPY,
                );
                rows -= rows_applicable;
                self.dma.rows_memcpy_to_full -= rows_applicable;
                rows_applicable
            } else {
                0
            };
            if rows > 0 {
                dma_memcpy.add_op_rows(*current_chunk, skip, rows, rows, DMA_COUNTER_MEMCPY);
            }

            let mut rows = counters[DMA_OFFSET + DMA_COUNTER_INPUTCPY];
            let skip = if self.dma.rows_inputcpy_to_full > 0 {
                let rows_applicable = std::cmp::min(rows, self.dma.rows_inputcpy_to_full);
                dma_full.add_op_rows(
                    *current_chunk,
                    0,
                    rows_applicable,
                    rows_applicable,
                    DMA_COUNTER_INPUTCPY,
                );
                rows -= rows_applicable;
                self.dma.rows_inputcpy_to_full -= rows_applicable;
                rows_applicable
            } else {
                0
            };
            if rows > 0 {
                dma_inputcpy.add_op_rows(*current_chunk, skip, rows, rows, DMA_COUNTER_INPUTCPY);
            }

            // DMA_PRE_POST

            let rows = counters[DMA_PRE_POST_OFFSET + DMA_COUNTER_MEMSET];
            if rows > 0 {
                dma_pre_post_full.add_op_rows(*current_chunk, 0, rows, rows, DMA_COUNTER_MEMSET);
            }

            let rows = counters[DMA_PRE_POST_OFFSET + DMA_COUNTER_MEMCMP];
            if rows > 0 {
                dma_pre_post_full.add_op_rows(*current_chunk, 0, rows, rows, DMA_COUNTER_MEMCMP);
            }

            let mut rows = counters[DMA_PRE_POST_OFFSET + DMA_COUNTER_MEMCPY];
            let skip = if rows > 0 && self.dma_pre_post.rows_memcpy_to_full > 0 {
                let rows_applicable = std::cmp::min(rows, self.dma_pre_post.rows_memcpy_to_full);
                dma_pre_post_full.add_op_rows(
                    *current_chunk,
                    0,
                    rows_applicable,
                    rows_applicable,
                    DMA_COUNTER_MEMCPY,
                );
                rows -= rows_applicable;
                self.dma_pre_post.rows_memcpy_to_full -= rows_applicable;
                rows_applicable
            } else {
                0
            };
            if rows > 0 {
                dma_pre_post_memcpy.add_op_rows(
                    *current_chunk,
                    skip,
                    rows,
                    rows,
                    DMA_COUNTER_MEMCPY,
                );
            }

            let mut rows = counters[DMA_PRE_POST_OFFSET + DMA_COUNTER_INPUTCPY];
            let skip = if self.dma_pre_post.rows_inputcpy_to_full > 0 {
                let rows_applicable = std::cmp::min(rows, self.dma_pre_post.rows_inputcpy_to_full);
                dma_pre_post_full.add_op_rows(
                    *current_chunk,
                    0,
                    rows_applicable,
                    rows_applicable,
                    DMA_COUNTER_INPUTCPY,
                );
                rows -= rows_applicable;
                self.dma_pre_post.rows_inputcpy_to_full -= rows_applicable;
                rows_applicable
            } else {
                0
            };
            if rows > 0 {
                dma_pre_post_inputcpy.add_op_rows(
                    *current_chunk,
                    skip,
                    rows,
                    rows,
                    DMA_COUNTER_INPUTCPY,
                );
            }

            // DMA_64_ALIGNED
            // Each operation type is routed to a single instance type to avoid the complexity
            // of splitting rows across instance types within a chunk. A per-chunk split
            // could further reduce cost but requires knowledge of intra-chunk distribution.
            for op in 0..DMA_COUNTER_OPS {
                let inputs = counters[DMA_64_ALIGNED_INPUTS_OFFSET + op];
                match op {
                    DMA_COUNTER_INPUTCPY => {
                        let rows = counters[DMA_64_ALIGNED_OFFSET + DMA_COUNTER_INPUTCPY];
                        if self.dma_64_aligned.rows_inputcpy_to_full > 0 {
                            assert!(rows <= self.dma_64_aligned.rows_inputcpy_to_full);
                            dma_64_aligned_full.add_op_rows(*current_chunk, 0, rows, inputs, op);
                            self.dma_64_aligned.rows_inputcpy_to_full -= rows;
                        } else {
                            dma_64_aligned_inputcpy.add_op_rows(
                                *current_chunk,
                                0,
                                rows,
                                inputs,
                                op,
                            );
                        }
                    }
                    DMA_COUNTER_MEMSET => {
                        if self.dma_64_aligned.rows_memset_to_mem > 0 {
                            let rows = counters[DMA_64_ALIGNED_OFFSET + DMA_COUNTER_MEMSET];
                            assert!(rows <= self.dma_64_aligned.rows_memset_to_mem);
                            dma_64_aligned_mem.add_op_rows(*current_chunk, 0, rows, inputs, op);
                            self.dma_64_aligned.rows_memset_to_mem -= rows;
                        } else if self.dma_64_aligned.rows_memset_to_full > 0 {
                            let rows = counters[DMA_64_ALIGNED_OFFSET + DMA_COUNTER_MEMSET];
                            assert!(rows <= self.dma_64_aligned.rows_memset_to_full);
                            dma_64_aligned_full.add_op_rows(*current_chunk, 0, rows, inputs, op);
                            self.dma_64_aligned.rows_memset_to_full -= rows;
                        } else {
                            dma_64_aligned_memset.add_op_rows(
                                *current_chunk,
                                0,
                                counters[DMA_64_ALIGNED_OFFSET + DMA_COUNTER_MEMSET_8],
                                inputs,
                                op,
                            )
                        }
                    }
                    DMA_COUNTER_MEMCMP => {
                        let rows = counters[DMA_64_ALIGNED_OFFSET + DMA_COUNTER_MEMCMP];
                        if self.dma_64_aligned.rows_memcmp_to_full > 0 {
                            assert!(rows <= self.dma_64_aligned.rows_memcmp_to_full);
                            dma_64_aligned_full.add_op_rows(*current_chunk, 0, rows, inputs, op);
                            self.dma_64_aligned.rows_memcmp_to_full -= rows;
                        } else {
                            dma_64_aligned_mem.add_op_rows(*current_chunk, 0, rows, inputs, op)
                        }
                    }
                    DMA_COUNTER_MEMCPY => {
                        if self.dma_64_aligned.rows_memcpy_to_mem > 0 {
                            let rows = counters[DMA_64_ALIGNED_OFFSET + DMA_COUNTER_MEMCPY];
                            assert!(rows <= self.dma_64_aligned.rows_memcpy_to_mem);
                            dma_64_aligned_mem.add_op_rows(*current_chunk, 0, rows, inputs, op);
                            self.dma_64_aligned.rows_memcpy_to_mem -= rows;
                        } else if self.dma_64_aligned.rows_memcpy_to_full > 0 {
                            let rows = counters[DMA_64_ALIGNED_OFFSET + DMA_COUNTER_MEMCPY];
                            assert!(rows <= self.dma_64_aligned.rows_memcpy_to_full);
                            dma_64_aligned_full.add_op_rows(*current_chunk, 0, rows, inputs, op);
                            self.dma_64_aligned.rows_memcpy_to_full -= rows;
                        } else {
                            dma_64_aligned_memcpy.add_op_rows(
                                *current_chunk,
                                0,
                                counters[DMA_64_ALIGNED_OFFSET + DMA_COUNTER_MEMCPY_8],
                                inputs,
                                op,
                            )
                        }
                    }
                    _ => panic!("Unexpected op code {op} in DMA 64 aligned counters"),
                };
            }

            // DMA_UNALIGNED

            for op in 0..DMA_COUNTER_OPS {
                let rows = counters[DMA_UNALIGNED_OFFSET + op];
                let inputs = counters[DMA_UNALIGNED_INPUTS_OFFSET + op];
                if rows > 0 {
                    dma_unaligned.add_op_rows(*current_chunk, 0, rows, inputs, op);
                }
            }
        }

        let plans = vec![
            (DmaTrace::<F>::AIR_ID, dma_full.get_plan()),
            (DmaMemCpyTrace::<F>::AIR_ID, dma_memcpy.get_plan()),
            (DmaInputCpyTrace::<F>::AIR_ID, dma_inputcpy.get_plan()),
            (DmaPrePostTrace::<F>::AIR_ID, dma_pre_post_full.get_plan()),
            (DmaPrePostMemCpyTrace::<F>::AIR_ID, dma_pre_post_memcpy.get_plan()),
            (DmaPrePostInputCpyTrace::<F>::AIR_ID, dma_pre_post_inputcpy.get_plan()),
            (Dma64AlignedTrace::<F>::AIR_ID, dma_64_aligned_full.get_plan()),
            (Dma64AlignedMemSetTrace::<F>::AIR_ID, dma_64_aligned_memset.get_plan()),
            (Dma64AlignedMemCpyTrace::<F>::AIR_ID, dma_64_aligned_memcpy.get_plan()),
            (Dma64AlignedInputCpyTrace::<F>::AIR_ID, dma_64_aligned_inputcpy.get_plan()),
            (Dma64AlignedMemTrace::<F>::AIR_ID, dma_64_aligned_mem.get_plan()),
            (DmaUnalignedTrace::<F>::AIR_ID, dma_unaligned.get_plan()),
        ];

        #[cfg(feature = "save_dma_plans")]
        self.save_plans("dma_plans.txt", totals_debug_info, &plans).unwrap();

        plans
    }

    #[cfg(feature = "save_dma_plans")]
    fn save_plans(
        &self,
        filename: &str,
        totals_debug_info: String,
        plans: &Vec<(usize, Vec<(CheckPoint, DmaCheckPoint)>)>,
    ) -> std::io::Result<()> {
        let mut debug_info = format!(
            "───────────────────────────────────────────────────── TOTALS\n{}\n{}",
            totals_debug_info, self
        );
        for (air_id, plan) in plans {
            if plan.is_empty() {
                continue;
            }
            let title = &get_dma_air_name::<F>(*air_id).to_string();
            debug_info += &plan
                .iter()
                .enumerate()
                .map(|(segment_id, (_checkpoint, dma_checkpoint))| {
                    dma_checkpoint.get_debug_info(title, segment_id as u64)
                })
                .collect::<Vec<_>>()
                .join("\n");
            debug_info += "\n";
        }
        use std::fs;

        let path = std::env::var("DEBUG_OUTPUT_PATH").unwrap_or_else(|_| "tmp/".to_string());
        let full_path = format!("{}{}", path, filename);

        fs::write(&full_path, debug_info)?;
        Ok(())
    }
}
