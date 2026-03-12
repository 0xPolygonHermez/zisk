//! The `DmaPlanner` module defines a planner for generating execution plans specific to
//! arithmetic operations.
//!
//! It organizes execution plans for both regular instances and table instances,
//! leveraging arithmetic operation counts and metadata to construct detailed plans.

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
#[cfg(not(feature = "packed"))]
use zisk_pil::{
    Dma64AlignedInputCpyTrace, Dma64AlignedMemCpyTrace, Dma64AlignedMemSetTrace,
    Dma64AlignedMemTrace, Dma64AlignedTrace, DmaInputCpyTrace, DmaMemCpyTrace,
    DmaPrePostInputCpyTrace, DmaPrePostMemCpyTrace, DmaPrePostTrace, DmaTrace, DmaUnalignedTrace,
};

#[cfg(feature = "packed")]
use zisk_pil::{
    Dma64AlignedInputCpyTracePacked as Dma64AlignedInputCpyTrace,
    Dma64AlignedMemCpyTracePacked as Dma64AlignedMemCpyTrace,
    Dma64AlignedMemSetTracePacked as Dma64AlignedMemSetTrace,
    Dma64AlignedMemTracePacked as Dma64AlignedMemTrace,
    Dma64AlignedTracePacked as Dma64AlignedTrace, DmaInputCpyTracePacked as DmaInputCpyTrace,
    DmaMemCpyTracePacked as DmaMemCpyTrace,
    DmaPrePostInputCpyTracePacked as DmaPrePostInputCpyTrace,
    DmaPrePostMemCpyTracePacked as DmaPrePostMemCpyTrace, DmaPrePostTracePacked as DmaPrePostTrace,
    DmaTracePacked as DmaTrace, DmaUnalignedTracePacked as DmaUnalignedTrace,
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
               memcpy    {:>3} {:>6} → mem   {:>6} → full\n  \
               inputcpy  {:>3} {:>6} → full\n  \
               mem       {:>3}\n  \
               memset    {:>3} {:>6} → mem   {:>6} → full\n  \
               memcmp      - {:>6} → full\n",
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

/// The `DmaStrategy` struct organizes execution plans for arithmetic instances and tables.
///
/// It allows adding metadata about instances and tables and generates plans
/// based on the provided counters.
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

impl<F: PrimeField64> DmaStrategy<F> {
    /// Creates a new `DmaStrategy`.
    ///
    /// # Returns
    /// A new `DmaStrategy` instance with no preconfigured instances or tables.
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

    const DMA_ROWS: usize = DmaTrace::<F>::NUM_ROWS;
    const DMA_MEMCPY_ROWS: usize = DmaMemCpyTrace::<F>::NUM_ROWS;
    const DMA_INPUTCPY_ROWS: usize = DmaInputCpyTrace::<F>::NUM_ROWS;
    const DMA_PRE_POST_ROWS: usize = DmaPrePostTrace::<F>::NUM_ROWS;
    const DMA_PRE_POST_MEMCPY_ROWS: usize = DmaPrePostMemCpyTrace::<F>::NUM_ROWS;
    const DMA_PRE_POST_INPUTCPY_ROWS: usize = DmaPrePostInputCpyTrace::<F>::NUM_ROWS;
    const DMA_64_ALIGNED_ROWS: usize = Dma64AlignedTrace::<F>::NUM_ROWS;
    const DMA_64_ALIGNED_MEMCPY_ROWS: usize = Dma64AlignedMemCpyTrace::<F>::NUM_ROWS;
    const DMA_64_ALIGNED_MEMSET_ROWS: usize = Dma64AlignedMemSetTrace::<F>::NUM_ROWS;
    const DMA_64_ALIGNED_INPUTCPY_ROWS: usize = Dma64AlignedInputCpyTrace::<F>::NUM_ROWS;
    const DMA_64_ALIGNED_MEM_ROWS: usize = Dma64AlignedMemTrace::<F>::NUM_ROWS;
    const DMA_UNALIGNED_ROWS: usize = DmaUnalignedTrace::<F>::NUM_ROWS;
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
    // DmaPrePost
    // DmaPrePostMemCpy
    // DmaPrePostInputCpy
    // memcpy: memcpy ==> mem ==> full
    // memcmp: mem ==> full
    // memset: memset ==> mem ==> full
    // inputcpy: input_cpy ==> full

    pub fn calculate_dma_64_alignment_strategy(rows: &[usize], info: &mut Dma64AlignedInstances) {
        info.full = 0;
        info.memcpy = rows[DMA_COUNTER_MEMCPY_8].div_ceil(Self::DMA_64_ALIGNED_MEMCPY_ROWS);
        info.memset = rows[DMA_COUNTER_MEMSET_8].div_ceil(Self::DMA_64_ALIGNED_MEMSET_ROWS);
        info.inputcpy = rows[DMA_COUNTER_INPUTCPY].div_ceil(Self::DMA_64_ALIGNED_INPUTCPY_ROWS);

        let rows_mem = rows[DMA_COUNTER_MEMCMP];
        info.mem = rows_mem.div_ceil(Self::DMA_64_ALIGNED_ROWS);
        // TBO: To Be Optimized
        info.rows_inputcpy_to_full = 0;
        info.rows_memcpy_to_mem = 0;
        info.rows_memcpy_to_full = 0;
        info.rows_memset_to_mem = 0;
        info.rows_memset_to_full = 0;
        info.rows_memcmp_to_full = 0;
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

            for op in 0..DMA_COUNTER_OPS {
                let inputs = counters[DMA_64_ALIGNED_INPUTS_OFFSET + op];
                match op {
                    DMA_COUNTER_INPUTCPY => dma_64_aligned_inputcpy.add_op_rows(
                        *current_chunk,
                        0,
                        counters[DMA_64_ALIGNED_OFFSET + op],
                        inputs,
                        op,
                    ),
                    DMA_COUNTER_MEMSET => dma_64_aligned_memset.add_op_rows(
                        *current_chunk,
                        0,
                        counters[DMA_64_ALIGNED_OFFSET + DMA_COUNTER_MEMSET_8],
                        inputs,
                        op,
                    ),
                    DMA_COUNTER_MEMCMP => dma_64_aligned_mem.add_op_rows(
                        *current_chunk,
                        0,
                        counters[DMA_64_ALIGNED_OFFSET + op],
                        inputs,
                        op,
                    ),
                    DMA_COUNTER_MEMCPY => dma_64_aligned_memcpy.add_op_rows(
                        *current_chunk,
                        0,
                        counters[DMA_64_ALIGNED_OFFSET + DMA_COUNTER_MEMCPY_8],
                        inputs,
                        op,
                    ),
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

            // println!("chunk {current_chunk} counter: {counters:?}");
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
            let title = &format!("{}", get_dma_air_name::<F>(*air_id));
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
