//! The `DmaStrategy` module decides which DMA airs are instantiated and how many of each.
//!
//! # The airs
//!
//! Four independent groups prove the DMA operations, and each op — `memcpy`, `memset`, `memcmp`,
//! `inputcpy` — is proved once in every group it applies to. Within the 64-bit-aligned group the
//! specialised airs pack more operations per row than the general one, so the same work takes fewer
//! rows there; and two of the airs come in a taller `Large` sibling that commits the same columns
//! over twice as many rows:
//!
//! | group          | airs                                                                        |
//! |----------------|-----------------------------------------------------------------------------|
//! | `Dma`          | `Dma`                                                                       |
//! | `DmaPrePost`   | `DmaPrePost`                                                                |
//! | `Dma64Aligned` | `Dma64Aligned` / `…Large`, `Dma64AlignedMem` / `…Large`, `…MemCpy`, `…MemSet` |
//! | `DmaUnaligned` | `DmaUnaligned`                                                              |
//!
//! # The criterion
//!
//! [`zisk_common::select_airs`] places the operations under the shared criterion: **fewest instances
//! first, least area to break a tie.** The tall airs are what that first term buys — one
//! `Dma64AlignedLarge` instance holds what four short ones would — and the specialised airs are what
//! the second term buys once the count is settled.
//!
//! Each operation type is routed to a single air, because the per-operation row distribution within
//! a chunk is not known at this stage. A per-chunk split could shave a little more, but it would need
//! that distribution.

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

use proofman_fields::PrimeField64;
use zisk_common::{select_airs, AirChoice, BusDeviceMetrics, BusDeviceMode, CheckPoint, ChunkId};

use zisk_pil::{
    Dma64AlignedLargeTrace, Dma64AlignedMemCpyTrace, Dma64AlignedMemLargeTrace,
    Dma64AlignedMemSetTrace, Dma64AlignedMemTrace, Dma64AlignedTrace, DmaPrePostTrace, DmaTrace,
    DmaUnalignedTrace, DMA_64_ALIGNED_INSTANCE_COST, DMA_64_ALIGNED_LARGE_INSTANCE_COST,
    DMA_64_ALIGNED_MEM_CPY_INSTANCE_COST, DMA_64_ALIGNED_MEM_INSTANCE_COST,
    DMA_64_ALIGNED_MEM_LARGE_INSTANCE_COST, DMA_64_ALIGNED_MEM_SET_INSTANCE_COST,
};

/// Airs of the 64-bit-aligned group, in the order the strategy and the hand-out both use.
mod air {
    /// `Dma64AlignedLarge`: the general air, tall.
    pub const FULL_LARGE: usize = 0;
    /// `Dma64Aligned`: the general air.
    pub const FULL: usize = 1;
    /// `Dma64AlignedMemLarge`: memcpy/memcmp/memset, tall.
    pub const MEM_LARGE: usize = 2;
    /// `Dma64AlignedMem`: memcpy/memcmp/memset.
    pub const MEM: usize = 3;
    /// `Dma64AlignedMemCpy`: memcpy only, packed.
    pub const MEMCPY: usize = 4;
    /// `Dma64AlignedMemSet`: memset only, packed.
    pub const MEMSET: usize = 5;
    /// Number of airs in the group.
    pub const COUNT: usize = 6;
}

/// Operation kinds of the 64-bit-aligned group, in the order [`Dma64AlignedInstances`] reports them.
mod kind {
    pub const MEMCPY: usize = 0;
    pub const MEMSET: usize = 1;
    pub const MEMCMP: usize = 2;
    pub const INPUTCPY: usize = 3;
    /// Number of kinds.
    pub const COUNT: usize = 4;
}

/// How many instances of each 64-bit-aligned air to create, and how many rows of each operation kind
/// each of them receives.
#[derive(Debug, Default, Clone)]
pub struct Dma64AlignedInstances {
    /// Instances of each air, in [`air`] order.
    pub instances: [usize; air::COUNT],

    /// The air each operation kind was routed to, in [`kind`] order.
    pub assignment: [usize; kind::COUNT],

    /// Rows of each kind that air receives, in [`kind`] order. Counted down as the chunks are handed
    /// out, so what is left is what the remaining chunks still owe.
    pub rows: [usize; kind::COUNT],
}

impl fmt::Display for Dma64AlignedInstances {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "  full      {:>3}   full_large {:>3}\n  \
               mem       {:>3}   mem_large  {:>3}\n  \
               memcpy    {:>3}   memset     {:>3}",
            self.instances[air::FULL],
            self.instances[air::FULL_LARGE],
            self.instances[air::MEM],
            self.instances[air::MEM_LARGE],
            self.instances[air::MEMCPY],
            self.instances[air::MEMSET],
        )?;
        for (k, name) in ["memcpy", "memset", "memcmp", "inputcpy"].iter().enumerate() {
            writeln!(f, "  {name:<9} {:>12} rows → air {}", self.rows[k], self.assignment[k])?;
        }
        Ok(())
    }
}

/// The `DmaStrategy` struct selects the assignment of DMA operation types to airs and generates the
/// execution plans for each instance.
#[derive(Default)]
pub struct DmaStrategy<F> {
    /// Instances of the single-air `Dma` group.
    pub dma: usize,
    /// Instances of the single-air `DmaPrePost` group.
    pub dma_pre_post: usize,
    /// The 64-bit-aligned group's assignment.
    pub dma_64_aligned: Dma64AlignedInstances,
    /// Instances of the single-air `DmaUnaligned` group.
    pub dma_unaligned: usize,
    _marker: std::marker::PhantomData<F>,
}

impl<F> fmt::Display for DmaStrategy<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "────────────────────────────────────────── DMA\n  \
             full      {:>3}\n\
             ───────────────────────────────── DMA_PRE_POST\n  \
             full      {:>3}\n\
             ─────────────────────────────── DMA_64_ALIGNED\n\
             {}\
             ──────────────────────────────── DMA_UNALIGNED\n  \
             full      {:>3}\n\n",
            self.dma, self.dma_pre_post, self.dma_64_aligned, self.dma_unaligned,
        )
    }
}

impl<F: PrimeField64> DmaStrategy<F> {
    /// Creates a new `DmaStrategy` with default (zero) counters.
    pub fn new() -> Self {
        Self::default()
    }

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
    const DMA_PRE_POST_ROWS: usize = DmaPrePostTrace::<()>::NUM_ROWS;
    const DMA_UNALIGNED_ROWS: usize = DmaUnalignedTrace::<()>::NUM_ROWS;
    const DMA_64_ALIGNED_ROWS: usize = Dma64AlignedTrace::<()>::NUM_ROWS;
    const DMA_64_ALIGNED_LARGE_ROWS: usize = Dma64AlignedLargeTrace::<()>::NUM_ROWS;
    const DMA_64_ALIGNED_MEM_ROWS: usize = Dma64AlignedMemTrace::<()>::NUM_ROWS;
    const DMA_64_ALIGNED_MEM_LARGE_ROWS: usize = Dma64AlignedMemLargeTrace::<()>::NUM_ROWS;
    const DMA_64_ALIGNED_MEMCPY_ROWS: usize = Dma64AlignedMemCpyTrace::<()>::NUM_ROWS;
    const DMA_64_ALIGNED_MEMSET_ROWS: usize = Dma64AlignedMemSetTrace::<()>::NUM_ROWS;

    /// The airs of the 64-bit-aligned group, in [`air`] order.
    fn dma_64_aligned_airs() -> [AirChoice; air::COUNT] {
        [
            AirChoice::new(
                Dma64AlignedLargeTrace::<()>::AIRGROUP_ID,
                Dma64AlignedLargeTrace::<()>::AIR_ID,
                Self::DMA_64_ALIGNED_LARGE_ROWS,
                DMA_64_ALIGNED_LARGE_INSTANCE_COST,
            ),
            AirChoice::new(
                Dma64AlignedTrace::<()>::AIRGROUP_ID,
                Dma64AlignedTrace::<()>::AIR_ID,
                Self::DMA_64_ALIGNED_ROWS,
                DMA_64_ALIGNED_INSTANCE_COST,
            ),
            AirChoice::new(
                Dma64AlignedMemLargeTrace::<()>::AIRGROUP_ID,
                Dma64AlignedMemLargeTrace::<()>::AIR_ID,
                Self::DMA_64_ALIGNED_MEM_LARGE_ROWS,
                DMA_64_ALIGNED_MEM_LARGE_INSTANCE_COST,
            ),
            AirChoice::new(
                Dma64AlignedMemTrace::<()>::AIRGROUP_ID,
                Dma64AlignedMemTrace::<()>::AIR_ID,
                Self::DMA_64_ALIGNED_MEM_ROWS,
                DMA_64_ALIGNED_MEM_INSTANCE_COST,
            ),
            AirChoice::new(
                Dma64AlignedMemCpyTrace::<()>::AIRGROUP_ID,
                Dma64AlignedMemCpyTrace::<()>::AIR_ID,
                Self::DMA_64_ALIGNED_MEMCPY_ROWS,
                DMA_64_ALIGNED_MEM_CPY_INSTANCE_COST,
            ),
            AirChoice::new(
                Dma64AlignedMemSetTrace::<()>::AIRGROUP_ID,
                Dma64AlignedMemSetTrace::<()>::AIR_ID,
                Self::DMA_64_ALIGNED_MEMSET_ROWS,
                DMA_64_ALIGNED_MEM_SET_INSTANCE_COST,
            ),
        ]
    }

    /// Routes each 64-bit-aligned operation kind to an air and sizes the instances.
    ///
    /// `rows` holds the counters of the group: the rows each kind takes in the general airs, plus the
    /// packed row counts (`…_8`) it would take in the airs that pack several operations per row.
    pub fn calculate_dma_64_alignment_strategy(rows: &[usize], info: &mut Dma64AlignedInstances) {
        // Each kind lists the airs able to prove it and the rows it takes there. `memcpy` and
        // `memset` also have a packed air, where the same operations take fewer rows.
        let kinds = vec![
            vec![
                (air::FULL_LARGE, rows[DMA_COUNTER_MEMCPY]),
                (air::FULL, rows[DMA_COUNTER_MEMCPY]),
                (air::MEM_LARGE, rows[DMA_COUNTER_MEMCPY]),
                (air::MEM, rows[DMA_COUNTER_MEMCPY]),
                (air::MEMCPY, rows[DMA_COUNTER_MEMCPY_8]),
            ],
            vec![
                (air::FULL_LARGE, rows[DMA_COUNTER_MEMSET]),
                (air::FULL, rows[DMA_COUNTER_MEMSET]),
                (air::MEM_LARGE, rows[DMA_COUNTER_MEMSET]),
                (air::MEM, rows[DMA_COUNTER_MEMSET]),
                (air::MEMSET, rows[DMA_COUNTER_MEMSET_8]),
            ],
            vec![
                (air::FULL_LARGE, rows[DMA_COUNTER_MEMCMP]),
                (air::FULL, rows[DMA_COUNTER_MEMCMP]),
                (air::MEM_LARGE, rows[DMA_COUNTER_MEMCMP]),
                (air::MEM, rows[DMA_COUNTER_MEMCMP]),
            ],
            // Only the general airs prove an input copy.
            vec![
                (air::FULL_LARGE, rows[DMA_COUNTER_INPUTCPY]),
                (air::FULL, rows[DMA_COUNTER_INPUTCPY]),
            ],
        ];
        let kinds: Vec<Vec<(usize, u64)>> = kinds
            .into_iter()
            .map(|options| options.into_iter().map(|(a, r)| (a, r as u64)).collect())
            .collect();

        let selection = select_airs(&kinds, &Self::dma_64_aligned_airs());

        for (a, &count) in selection.instances.iter().enumerate() {
            info.instances[a] = count as usize;
        }
        info.assignment = [
            selection.assignment[kind::MEMCPY],
            selection.assignment[kind::MEMSET],
            selection.assignment[kind::MEMCMP],
            selection.assignment[kind::INPUTCPY],
        ];
        // The rows each kind owes its air, in that air's own row cost.
        info.rows = [
            kinds[kind::MEMCPY]
                .iter()
                .find(|(a, _)| *a == info.assignment[kind::MEMCPY])
                .map_or(0, |(_, r)| *r as usize),
            kinds[kind::MEMSET]
                .iter()
                .find(|(a, _)| *a == info.assignment[kind::MEMSET])
                .map_or(0, |(_, r)| *r as usize),
            rows[DMA_COUNTER_MEMCMP],
            rows[DMA_COUNTER_INPUTCPY],
        ];
    }

    /// The rows every operation of a single-air group takes together.
    fn single_air_rows(rows: &[usize]) -> usize {
        rows[DMA_COUNTER_MEMCPY]
            + rows[DMA_COUNTER_INPUTCPY]
            + rows[DMA_COUNTER_MEMSET]
            + rows[DMA_COUNTER_MEMCMP]
    }

    fn calculate_strategy(&mut self, totals: &DmaCounterInputGen) {
        self.dma =
            Self::single_air_rows(&totals.counters[DMA_OFFSET..DMA_OFFSET + DMA_COUNTER_OPS])
                .div_ceil(Self::DMA_ROWS);
        self.dma_pre_post = Self::single_air_rows(
            &totals.counters[DMA_PRE_POST_OFFSET..DMA_PRE_POST_OFFSET + DMA_COUNTER_OPS],
        )
        .div_ceil(Self::DMA_PRE_POST_ROWS);
        Self::calculate_dma_64_alignment_strategy(
            &totals.counters[DMA_64_ALIGNED_OFFSET..DMA_64_ALIGNED_OFFSET + DMA_COUNTER_OPS_EXT],
            &mut self.dma_64_aligned,
        );
        self.dma_unaligned = Self::single_air_rows(
            &totals.counters[DMA_UNALIGNED_OFFSET..DMA_UNALIGNED_OFFSET + DMA_COUNTER_OPS],
        )
        .div_ceil(Self::DMA_UNALIGNED_ROWS);
    }

    pub fn calculate(
        &mut self,
        counters: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>,
    ) -> Vec<(usize, Vec<(CheckPoint, DmaCheckPoint)>)> {
        let totals: DmaCounterInputGen = self.calculate_totals(&counters);
        #[cfg(feature = "save_dma_plans")]
        let totals_debug_info = format!("{}", totals);

        self.calculate_strategy(&totals);

        let mut dma_full = DmaInstancesBuilder::new("dma_full", self.dma, Self::DMA_ROWS);
        let mut dma_pre_post_full = DmaInstancesBuilder::new(
            "dma_pre_post_full",
            self.dma_pre_post,
            Self::DMA_PRE_POST_ROWS,
        );
        let mut dma_unaligned =
            DmaInstancesBuilder::new("dma_unaligned", self.dma_unaligned, Self::DMA_UNALIGNED_ROWS);

        // One builder per air of the 64-bit-aligned group, in `air` order.
        let names = [
            "dma_64_aligned_large",
            "dma_64_aligned_full",
            "dma_64_aligned_mem_large",
            "dma_64_aligned_mem",
            "dma_64_aligned_memcpy",
            "dma_64_aligned_memset",
        ];
        let heights = [
            Self::DMA_64_ALIGNED_LARGE_ROWS,
            Self::DMA_64_ALIGNED_ROWS,
            Self::DMA_64_ALIGNED_MEM_LARGE_ROWS,
            Self::DMA_64_ALIGNED_MEM_ROWS,
            Self::DMA_64_ALIGNED_MEMCPY_ROWS,
            Self::DMA_64_ALIGNED_MEMSET_ROWS,
        ];
        let mut aligned: Vec<DmaInstancesBuilder> = (0..air::COUNT)
            .map(|a| {
                DmaInstancesBuilder::new(names[a], self.dma_64_aligned.instances[a], heights[a])
            })
            .collect();

        for (current_chunk, dyn_counter) in counters.iter() {
            let counters =
                (**dyn_counter).as_any().downcast_ref::<DmaCounterInputGen>().unwrap().counters;

            // DMA and DMA_PRE_POST: one air each, so every operation goes to it.
            for (offset, builder) in
                [(DMA_OFFSET, &mut dma_full), (DMA_PRE_POST_OFFSET, &mut dma_pre_post_full)]
            {
                for op in 0..DMA_COUNTER_OPS {
                    let rows = counters[offset + op];
                    if rows > 0 {
                        builder.add_op_rows(*current_chunk, 0, rows, rows, op);
                    }
                }
            }

            // DMA_64_ALIGNED: every operation of a kind goes to the air the strategy picked for it,
            // in that air's own row cost — the packed airs take the `…_8` counts.
            for op in 0..DMA_COUNTER_OPS {
                let inputs = counters[DMA_64_ALIGNED_INPUTS_OFFSET + op];
                let (kind, packed_counter) = match op {
                    DMA_COUNTER_MEMCPY => (kind::MEMCPY, Some(DMA_COUNTER_MEMCPY_8)),
                    DMA_COUNTER_MEMSET => (kind::MEMSET, Some(DMA_COUNTER_MEMSET_8)),
                    DMA_COUNTER_MEMCMP => (kind::MEMCMP, None),
                    DMA_COUNTER_INPUTCPY => (kind::INPUTCPY, None),
                    _ => panic!("Unexpected op code {op} in DMA 64 aligned counters"),
                };
                let target = self.dma_64_aligned.assignment[kind];
                let packed = matches!(target, air::MEMCPY | air::MEMSET);
                let counter = if packed {
                    packed_counter.expect("only memcpy and memset have a packed air")
                } else {
                    op
                };
                let rows = counters[DMA_64_ALIGNED_OFFSET + counter];
                // Unconditional: the sizing used the totals and this hand-out walks the chunks, so
                // the two disagreeing means rows are about to be routed to an air that was never
                // given room for them. Catching it here names the kind and the air; letting the
                // subtraction wrap in release would surface it much later, as an opaque overflow
                // in `DmaInstancesBuilder`.
                assert!(
                    rows <= self.dma_64_aligned.rows[kind],
                    "chunk {current_chunk:?} owes air {target} {rows} rows of kind {kind}, more \
                     than the {} the strategy routed to it",
                    self.dma_64_aligned.rows[kind],
                );
                self.dma_64_aligned.rows[kind] -= rows;
                aligned[target].add_op_rows(*current_chunk, 0, rows, inputs, op);
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

        let air_ids = [
            Dma64AlignedLargeTrace::<F>::AIR_ID,
            Dma64AlignedTrace::<F>::AIR_ID,
            Dma64AlignedMemLargeTrace::<F>::AIR_ID,
            Dma64AlignedMemTrace::<F>::AIR_ID,
            Dma64AlignedMemCpyTrace::<F>::AIR_ID,
            Dma64AlignedMemSetTrace::<F>::AIR_ID,
        ];
        let mut plans = vec![
            (DmaTrace::<F>::AIR_ID, dma_full.get_plan()),
            (DmaPrePostTrace::<F>::AIR_ID, dma_pre_post_full.get_plan()),
            (DmaUnalignedTrace::<F>::AIR_ID, dma_unaligned.get_plan()),
        ];
        plans.extend(
            aligned.iter_mut().enumerate().map(|(a, builder)| (air_ids[a], builder.get_plan())),
        );

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

#[cfg(test)]
#[path = "tests/dma_strategy_tests.rs"]
mod tests;
