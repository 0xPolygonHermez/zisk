//! The `DmaStrategy` module decides which DMA airs are instantiated and how many of each.
//!
//! # The airs
//!
//! A DMA operation has up to two parts: the controller with its PRE/POST sub-operations, and the
//! 64-bit loop. Each part is proved once, by one of the airs able to prove it:
//!
//! | part           | airs                                                                     |
//! |----------------|--------------------------------------------------------------------------|
//! | controller     | `DmaWithPrePost`, the `wpp_` block of `CompactDma`                       |
//! | aligned loop   | `Dma64Aligned` / `…Large`, `Dma64AlignedMem` / `…Large`, `…MemCpy`,      |
//! |                | `…MemSet`, `DmaLoop`, the `loop_` block of `CompactDma`                  |
//! | unaligned loop | `DmaUnaligned`, `DmaLoop`, the `loop_` block of `CompactDma`             |
//!
//! The controller used to be two airs -- `Dma` for the controller and `DmaPrePost` for its
//! sub-operations -- and is two again whenever [`DmaStrategy::USE_DMA_WITH_PRE_POST`] is turned
//! off; then it stays out of the selection below and both of them take every operation.
//!
//! `DmaLoop` is `Dma64Aligned` and `DmaUnaligned` in one air, and `CompactDma` is `DmaWithPrePost`
//! and `DmaLoop` side by side on the same rows: a fused air, whose two blocks enter the selection as
//! two entries tied by [`AirChoice::block_of`] -- what it opens is what its fullest block needs.
//! [`DmaStrategy::USE_DMA_LOOP`] and [`DmaStrategy::USE_COMPACT_DMA`] take them out of it.
//!
//! # The criterion
//!
//! [`zisk_common::select_airs`] places the work under the shared criterion: **fewest instances
//! first, least memory to break a tie.** The tall airs are what that first term buys -- one
//! `Dma64AlignedLarge` instance holds what four short ones would -- the fused airs are what it buys
//! on a light workload -- one `CompactDma` instance instead of a controller instance and one or two
//! loop instances -- and the specialised airs are what the second term buys once the count is
//! settled.
//!
//! Each kind of work is routed to a single air, because the per-operation row distribution within a
//! chunk is not known at this stage. The kinds are the controller and the six loop classes of
//! [`crate::DMA_LOOP_CLASSES`] (the four opcodes, aligned, plus memcpy and memcmp unaligned), so the
//! aligned and the unaligned loops of the same opcode can go to different airs.

use core::panic;
use std::fmt;

use crate::{
    CompactDmaCheckPoint, DmaCheckPoint, DmaCounterInputGen, DmaInstancesBuilder,
    DmaLoopCheckPoint, DmaLoopInstancesBuilder, DmaWithPrePostCheckPoint,
    DmaWithPrePostInstancesBuilder, DMA_64_ALIGNED_INPUTS_OFFSET, DMA_64_ALIGNED_OFFSET,
    DMA_COUNTER_INPUTCPY, DMA_COUNTER_MEMCMP, DMA_COUNTER_MEMCPY, DMA_COUNTER_MEMCPY_8,
    DMA_COUNTER_MEMSET, DMA_COUNTER_MEMSET_8, DMA_COUNTER_OPS, DMA_INPUT_GEN_COUNTERS, DMA_OFFSET,
    DMA_PRE_POST_OFFSET, DMA_UNALIGNED_INPUTS_OFFSET, DMA_UNALIGNED_OFFSET,
    DMA_WITH_PRE_POST_OFFSET, DMA_WPP_CLASSES, DMA_WPP_CLASS_ROWS,
};

#[cfg(feature = "save_dma_plans")]
use crate::get_dma_air_name;

use proofman_fields::PrimeField64;
use zisk_common::{select_airs, AirChoice, BusDeviceMetrics, BusDeviceMode, CheckPoint, ChunkId};

use zisk_pil::{
    CompactDmaTrace, Dma64AlignedLargeTrace, Dma64AlignedMemCpyTrace, Dma64AlignedMemLargeTrace,
    Dma64AlignedMemSetTrace, Dma64AlignedMemTrace, Dma64AlignedTrace, DmaLoopTrace,
    DmaPrePostTrace, DmaTrace, DmaUnalignedTrace, DmaWithPrePostTrace, COMPACT_DMA_INSTANCE_COST,
    DMA_64_ALIGNED_INSTANCE_COST, DMA_64_ALIGNED_LARGE_INSTANCE_COST,
    DMA_64_ALIGNED_MEM_CPY_INSTANCE_COST, DMA_64_ALIGNED_MEM_INSTANCE_COST,
    DMA_64_ALIGNED_MEM_LARGE_INSTANCE_COST, DMA_64_ALIGNED_MEM_SET_INSTANCE_COST,
    DMA_LOOP_INSTANCE_COST, DMA_UNALIGNED_INSTANCE_COST, DMA_WITH_PRE_POST_INSTANCE_COST,
};

/// Airs the strategy chooses between, in the order the selection and the hand-out both use.
mod air {
    /// `Dma64AlignedLarge`: the general aligned air, tall.
    pub const FULL_LARGE: usize = 0;
    /// `Dma64Aligned`: the general aligned air.
    pub const FULL: usize = 1;
    /// `Dma64AlignedMemLarge`: memcpy/memcmp/memset, tall.
    pub const MEM_LARGE: usize = 2;
    /// `Dma64AlignedMem`: memcpy/memcmp/memset.
    pub const MEM: usize = 3;
    /// `Dma64AlignedMemCpy`: memcpy only, packed.
    pub const MEMCPY: usize = 4;
    /// `Dma64AlignedMemSet`: memset only, packed.
    pub const MEMSET: usize = 5;
    /// Number of airs of the 64-bit-aligned group, which come first.
    pub const ALIGNED: usize = 6;
    /// `DmaUnaligned`.
    pub const UNALIGNED: usize = 6;
    /// `DmaLoop`: any loop, aligned or not.
    pub const LOOP: usize = 7;
    /// `DmaWithPrePost`: the controller with its PRE/POST sub-operations.
    pub const WPP: usize = 8;
    /// The `wpp_` block of `CompactDma`.
    pub const COMPACT_WPP: usize = 9;
    /// The `loop_` block of `CompactDma`.
    pub const COMPACT_LOOP: usize = 10;
    /// Number of airs.
    pub const COUNT: usize = 11;
}

/// Kinds of work the strategy places. The six loop classes come first and are numbered as
/// [`crate::DMA_LOOP_CLASSES`], so a loop kind IS the class the loop airs collect it under.
mod kind {
    pub const MEMCPY: usize = crate::DMA_LOOP_CLASS_MEMCPY;
    pub const MEMSET: usize = crate::DMA_LOOP_CLASS_MEMSET;
    pub const MEMCMP: usize = crate::DMA_LOOP_CLASS_MEMCMP;
    pub const INPUTCPY: usize = crate::DMA_LOOP_CLASS_INPUTCPY;
    pub const MEMCPY_UNALIGNED: usize = crate::DMA_LOOP_CLASS_MEMCPY_UNALIGNED;
    pub const MEMCMP_UNALIGNED: usize = crate::DMA_LOOP_CLASS_MEMCMP_UNALIGNED;
    /// Number of loop kinds.
    pub const LOOP: usize = crate::DMA_LOOP_CLASSES;
    /// The controller with its PRE/POST sub-operations.
    pub const CTRL: usize = LOOP;
    /// Number of kinds.
    pub const COUNT: usize = LOOP + 1;
}

/// The fused airs, as the groups [`AirChoice::block_of`] ties their blocks together with.
mod fused {
    /// `CompactDma`.
    pub const COMPACT_DMA: usize = 0;
}

/// Which of the optional airs the selection may use. The constants of [`DmaStrategy`] are the
/// production choice; the tests turn them on and off to pin each family on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DmaAirOptions {
    /// The controller is `DmaWithPrePost` (or the `wpp_` block of `CompactDma`), not `Dma` +
    /// `DmaPrePost`.
    pub with_pre_post: bool,
    /// `DmaLoop` may take loop work.
    pub dma_loop: bool,
    /// `CompactDma` may take work. It needs `with_pre_post`: its controller block is that air.
    pub compact_dma: bool,
}

/// The work to place, in the row cost of each family of airs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DmaWork {
    /// Rows of each loop kind, four words per row: what the general aligned airs, `DmaUnaligned`
    /// and the loop airs take, in [`kind`] order.
    pub loop_rows: [usize; kind::LOOP],
    /// Rows an aligned memcpy takes in the packed `Dma64AlignedMemCpy`.
    pub memcpy_packed: usize,
    /// Rows a memset takes in the packed `Dma64AlignedMemSet`.
    pub memset_packed: usize,
    /// Rows of the controller in `DmaWithPrePost`: one per operation, two for the ones with both
    /// a PRE and a POST.
    pub ctrl_rows: usize,
}

impl DmaWork {
    /// The work the counters of one chunk (or their sum over the execution) describe.
    pub fn from_counters(counters: &[usize]) -> Self {
        let aligned = |op: usize| counters[DMA_64_ALIGNED_OFFSET + op];
        let unaligned = |op: usize| counters[DMA_UNALIGNED_OFFSET + op];
        let mut loop_rows = [0; kind::LOOP];
        loop_rows[kind::MEMCPY] = aligned(DMA_COUNTER_MEMCPY);
        loop_rows[kind::MEMSET] = aligned(DMA_COUNTER_MEMSET);
        loop_rows[kind::MEMCMP] = aligned(DMA_COUNTER_MEMCMP);
        loop_rows[kind::INPUTCPY] = aligned(DMA_COUNTER_INPUTCPY);
        loop_rows[kind::MEMCPY_UNALIGNED] = unaligned(DMA_COUNTER_MEMCPY);
        loop_rows[kind::MEMCMP_UNALIGNED] = unaligned(DMA_COUNTER_MEMCMP);
        Self {
            loop_rows,
            memcpy_packed: aligned(DMA_COUNTER_MEMCPY_8),
            memset_packed: aligned(DMA_COUNTER_MEMSET_8),
            ctrl_rows: (0..DMA_WPP_CLASSES)
                .map(|class| counters[DMA_WITH_PRE_POST_OFFSET + class] * DMA_WPP_CLASS_ROWS[class])
                .sum(),
        }
    }
}

/// How many instances of each air to create, and where each kind of work goes.
#[derive(Debug, Default, Clone)]
pub struct DmaSelection {
    /// Instances of each air, in [`air`] order. The two blocks of a fused air report the same
    /// count: it is one instance of the air.
    pub instances: [usize; air::COUNT],

    /// The air each kind was routed to, in [`kind`] order. Meaningless for a kind with no work.
    pub assignment: [usize; kind::COUNT],

    /// Rows of each kind that air receives, in its own row cost, in [`kind`] order. Counted down
    /// as the chunks are handed out, so what is left is what the remaining chunks still owe.
    pub rows: [usize; kind::COUNT],
}

impl fmt::Display for DmaSelection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const AIRS: [&str; air::COUNT] = [
            "full_large",
            "full",
            "mem_large",
            "mem",
            "memcpy",
            "memset",
            "unaligned",
            "loop",
            "with_pre_post",
            "compact.wpp",
            "compact.loop",
        ];
        const KINDS: [&str; kind::COUNT] =
            ["memcpy", "memset", "memcmp", "inputcpy", "u_memcpy", "u_memcmp", "ctrl"];
        for (a, name) in AIRS.iter().enumerate() {
            writeln!(f, "  {name:<14} {:>3}", self.instances[a])?;
        }
        for (k, name) in KINDS.iter().enumerate() {
            writeln!(
                f,
                "  {name:<9} {:>12} rows → air {}",
                self.rows[k], AIRS[self.assignment[k]]
            )?;
        }
        Ok(())
    }
}

/// The `DmaStrategy` struct selects the assignment of DMA work to airs and generates the execution
/// plans for each instance.
#[derive(Default)]
pub struct DmaStrategy<F> {
    /// Instances of the `Dma` air (only when [`Self::USE_DMA_WITH_PRE_POST`] is off).
    pub dma: usize,
    /// Instances of the `DmaPrePost` air (likewise).
    pub dma_pre_post: usize,
    /// Instances of the fused `DmaWithPrePost` air.
    pub dma_with_pre_post: usize,
    /// Plan of the fused air, filled by [`DmaStrategy::calculate`] and taken by the planner.
    pub dma_with_pre_post_plan: Vec<(CheckPoint, DmaWithPrePostCheckPoint)>,
    /// Instances of `DmaLoop`.
    pub dma_loop: usize,
    /// Plan of `DmaLoop`, filled by [`DmaStrategy::calculate`] and taken by the planner.
    pub dma_loop_plan: Vec<(CheckPoint, DmaLoopCheckPoint)>,
    /// Instances of `CompactDma`.
    pub compact_dma: usize,
    /// Plan of `CompactDma`, one checkpoint per instance carrying the plan of each block.
    pub compact_dma_plan: Vec<(CheckPoint, CompactDmaCheckPoint)>,
    /// Instances of `DmaUnaligned`.
    pub dma_unaligned: usize,
    /// Where the work went, and how many instances of each air it opens.
    pub selection: DmaSelection,
    _marker: std::marker::PhantomData<F>,
}

impl<F> fmt::Display for DmaStrategy<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "────────────────────────────────────────── DMA\n  \
             dma       {:>3}\n  \
             pre_post  {:>3}\n\
             ─────────────────────────────── DMA SELECTION\n\
             {}\n",
            self.dma, self.dma_pre_post, self.selection,
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

    /// Whether the fused [`DmaWithPrePostTrace`] air proves the DMA controller together with its
    /// PRE/POST sub-operations.
    ///
    /// While this is `true` every non-direct DMA operation goes to the fused air: one row per
    /// operation instead of one `Dma` row plus one or two `DmaPrePost` rows, one air instead of
    /// two, and no DMA_BUS_ID between them — at the price of a wider row, and of one row per
    /// operation that has neither a PRE nor a POST being as wide as the rest. Turning it back to
    /// `false` returns the work to the two separate airs and leaves the fused one with no
    /// instance.
    pub const USE_DMA_WITH_PRE_POST: bool = true;

    /// Whether `DmaLoop` is offered to the selection.
    ///
    /// Its cost in `air_costs.rs` is an estimate until the next setup measures it, and the
    /// selection trusts it: turning this off keeps every loop in `Dma64Aligned*` / `DmaUnaligned`.
    pub const USE_DMA_LOOP: bool = true;

    /// Whether `CompactDma` is offered to the selection. Like [`Self::USE_DMA_LOOP`], its cost is
    /// an estimate until the next setup measures it.
    pub const USE_COMPACT_DMA: bool = true;

    /// The optional airs production uses.
    pub const OPTIONS: DmaAirOptions = DmaAirOptions {
        with_pre_post: Self::USE_DMA_WITH_PRE_POST,
        dma_loop: Self::USE_DMA_LOOP,
        compact_dma: Self::USE_COMPACT_DMA && Self::USE_DMA_WITH_PRE_POST,
    };

    const DMA_ROWS: usize = DmaTrace::<()>::NUM_ROWS;
    const DMA_PRE_POST_ROWS: usize = DmaPrePostTrace::<()>::NUM_ROWS;
    const DMA_WITH_PRE_POST_ROWS: usize = DmaWithPrePostTrace::<()>::NUM_ROWS;
    const DMA_UNALIGNED_ROWS: usize = DmaUnalignedTrace::<()>::NUM_ROWS;
    const DMA_LOOP_ROWS: usize = DmaLoopTrace::<()>::NUM_ROWS;
    const COMPACT_DMA_ROWS: usize = CompactDmaTrace::<()>::NUM_ROWS;
    const DMA_64_ALIGNED_ROWS: usize = Dma64AlignedTrace::<()>::NUM_ROWS;
    const DMA_64_ALIGNED_LARGE_ROWS: usize = Dma64AlignedLargeTrace::<()>::NUM_ROWS;
    const DMA_64_ALIGNED_MEM_ROWS: usize = Dma64AlignedMemTrace::<()>::NUM_ROWS;
    const DMA_64_ALIGNED_MEM_LARGE_ROWS: usize = Dma64AlignedMemLargeTrace::<()>::NUM_ROWS;
    const DMA_64_ALIGNED_MEMCPY_ROWS: usize = Dma64AlignedMemCpyTrace::<()>::NUM_ROWS;
    const DMA_64_ALIGNED_MEMSET_ROWS: usize = Dma64AlignedMemSetTrace::<()>::NUM_ROWS;

    /// Rows one instance of a DMA air holds.
    ///
    /// The planner budgets in rows, so this is the capacity an instance's occupancy is measured
    /// against. Returns `None` for an air this strategy does not plan. For `CompactDma` it is the
    /// rows of the two blocks together, the budget its occupancy adds up.
    pub fn rows_by_air_id(air_id: usize) -> Option<usize> {
        if air_id == DmaTrace::<F>::AIR_ID {
            Some(Self::DMA_ROWS)
        } else if air_id == DmaPrePostTrace::<F>::AIR_ID {
            Some(Self::DMA_PRE_POST_ROWS)
        } else if air_id == DmaWithPrePostTrace::<F>::AIR_ID {
            Some(Self::DMA_WITH_PRE_POST_ROWS)
        } else if air_id == DmaUnalignedTrace::<F>::AIR_ID {
            Some(Self::DMA_UNALIGNED_ROWS)
        } else if air_id == DmaLoopTrace::<F>::AIR_ID {
            Some(Self::DMA_LOOP_ROWS)
        } else if air_id == CompactDmaTrace::<F>::AIR_ID {
            Some(2 * Self::COMPACT_DMA_ROWS)
        } else if air_id == Dma64AlignedTrace::<F>::AIR_ID {
            Some(Self::DMA_64_ALIGNED_ROWS)
        } else if air_id == Dma64AlignedLargeTrace::<F>::AIR_ID {
            Some(Self::DMA_64_ALIGNED_LARGE_ROWS)
        } else if air_id == Dma64AlignedMemTrace::<F>::AIR_ID {
            Some(Self::DMA_64_ALIGNED_MEM_ROWS)
        } else if air_id == Dma64AlignedMemLargeTrace::<F>::AIR_ID {
            Some(Self::DMA_64_ALIGNED_MEM_LARGE_ROWS)
        } else if air_id == Dma64AlignedMemCpyTrace::<F>::AIR_ID {
            Some(Self::DMA_64_ALIGNED_MEMCPY_ROWS)
        } else if air_id == Dma64AlignedMemSetTrace::<F>::AIR_ID {
            Some(Self::DMA_64_ALIGNED_MEMSET_ROWS)
        } else {
            None
        }
    }

    /// The airs the selection chooses between, in [`air`] order.
    fn air_choices() -> [AirChoice; air::COUNT] {
        let choice =
            |airgroup_id, air_id, rows, cost| AirChoice::new(airgroup_id, air_id, rows, cost);
        [
            choice(
                Dma64AlignedLargeTrace::<()>::AIRGROUP_ID,
                Dma64AlignedLargeTrace::<()>::AIR_ID,
                Self::DMA_64_ALIGNED_LARGE_ROWS,
                DMA_64_ALIGNED_LARGE_INSTANCE_COST,
            ),
            choice(
                Dma64AlignedTrace::<()>::AIRGROUP_ID,
                Dma64AlignedTrace::<()>::AIR_ID,
                Self::DMA_64_ALIGNED_ROWS,
                DMA_64_ALIGNED_INSTANCE_COST,
            ),
            choice(
                Dma64AlignedMemLargeTrace::<()>::AIRGROUP_ID,
                Dma64AlignedMemLargeTrace::<()>::AIR_ID,
                Self::DMA_64_ALIGNED_MEM_LARGE_ROWS,
                DMA_64_ALIGNED_MEM_LARGE_INSTANCE_COST,
            ),
            choice(
                Dma64AlignedMemTrace::<()>::AIRGROUP_ID,
                Dma64AlignedMemTrace::<()>::AIR_ID,
                Self::DMA_64_ALIGNED_MEM_ROWS,
                DMA_64_ALIGNED_MEM_INSTANCE_COST,
            ),
            choice(
                Dma64AlignedMemCpyTrace::<()>::AIRGROUP_ID,
                Dma64AlignedMemCpyTrace::<()>::AIR_ID,
                Self::DMA_64_ALIGNED_MEMCPY_ROWS,
                DMA_64_ALIGNED_MEM_CPY_INSTANCE_COST,
            ),
            choice(
                Dma64AlignedMemSetTrace::<()>::AIRGROUP_ID,
                Dma64AlignedMemSetTrace::<()>::AIR_ID,
                Self::DMA_64_ALIGNED_MEMSET_ROWS,
                DMA_64_ALIGNED_MEM_SET_INSTANCE_COST,
            ),
            choice(
                DmaUnalignedTrace::<()>::AIRGROUP_ID,
                DmaUnalignedTrace::<()>::AIR_ID,
                Self::DMA_UNALIGNED_ROWS,
                DMA_UNALIGNED_INSTANCE_COST,
            ),
            choice(
                DmaLoopTrace::<()>::AIRGROUP_ID,
                DmaLoopTrace::<()>::AIR_ID,
                Self::DMA_LOOP_ROWS,
                DMA_LOOP_INSTANCE_COST,
            ),
            // An operation of the controller cannot straddle two instances -- its PRE row has to
            // follow its DMA row -- so the last row of an instance may be left unused. Taking it off
            // the height here is what `DmaWithPrePostInstancesBuilder::instances_needed` budgets.
            choice(
                DmaWithPrePostTrace::<()>::AIRGROUP_ID,
                DmaWithPrePostTrace::<()>::AIR_ID,
                Self::DMA_WITH_PRE_POST_ROWS - 1,
                DMA_WITH_PRE_POST_INSTANCE_COST,
            ),
            // The blocks of the fused air. Each carries the whole air's cost and its own budget,
            // and the two are instantiated together.
            AirChoice::block_of(
                CompactDmaTrace::<()>::AIRGROUP_ID,
                CompactDmaTrace::<()>::AIR_ID,
                Self::COMPACT_DMA_ROWS - 1,
                COMPACT_DMA_INSTANCE_COST,
                fused::COMPACT_DMA,
            ),
            AirChoice::block_of(
                CompactDmaTrace::<()>::AIRGROUP_ID,
                CompactDmaTrace::<()>::AIR_ID,
                Self::COMPACT_DMA_ROWS,
                COMPACT_DMA_INSTANCE_COST,
                fused::COMPACT_DMA,
            ),
        ]
    }

    /// The airs able to prove one kind, with the rows the kind takes in each.
    fn options_of(work: &DmaWork, k: usize, options: DmaAirOptions) -> Vec<(usize, u64)> {
        if k == kind::CTRL {
            if !options.with_pre_post {
                // `Dma` + `DmaPrePost` take the controller, outside the selection.
                return Vec::new();
            }
            let mut airs = vec![(air::WPP, work.ctrl_rows)];
            if options.compact_dma {
                airs.push((air::COMPACT_WPP, work.ctrl_rows));
            }
            return airs.into_iter().map(|(a, r)| (a, r as u64)).collect();
        }

        let rows = work.loop_rows[k];
        let general = [air::FULL_LARGE, air::FULL];
        let mem = [air::FULL_LARGE, air::FULL, air::MEM_LARGE, air::MEM];
        let mut airs: Vec<(usize, usize)> = match k {
            kind::MEMCPY => {
                mem.iter().map(|&a| (a, rows)).chain([(air::MEMCPY, work.memcpy_packed)]).collect()
            }
            kind::MEMSET => {
                mem.iter().map(|&a| (a, rows)).chain([(air::MEMSET, work.memset_packed)]).collect()
            }
            kind::MEMCMP => mem.iter().map(|&a| (a, rows)).collect(),
            // Only the general airs prove an input copy.
            kind::INPUTCPY => general.iter().map(|&a| (a, rows)).collect(),
            kind::MEMCPY_UNALIGNED | kind::MEMCMP_UNALIGNED => vec![(air::UNALIGNED, rows)],
            _ => unreachable!("kind {k}"),
        };
        // The loop airs prove every loop kind, four words per row like the airs they stand in for.
        if options.dma_loop {
            airs.push((air::LOOP, rows));
        }
        if options.compact_dma {
            airs.push((air::COMPACT_LOOP, rows));
        }
        airs.into_iter().map(|(a, r)| (a, r as u64)).collect()
    }

    /// Routes each kind of work to an air and sizes the instances.
    pub fn select(work: &DmaWork, options: DmaAirOptions) -> DmaSelection {
        let kinds: Vec<Vec<(usize, u64)>> =
            (0..kind::COUNT).map(|k| Self::options_of(work, k, options)).collect();
        let selection = select_airs(&kinds, &Self::air_choices());

        let mut out = DmaSelection::default();
        for (a, &count) in selection.instances.iter().enumerate() {
            out.instances[a] = count as usize;
        }
        for (k, options) in kinds.iter().enumerate() {
            out.assignment[k] = selection.assignment[k];
            // The rows each kind owes its air, in that air's own row cost.
            out.rows[k] = options
                .iter()
                .find(|(a, _)| *a == out.assignment[k])
                .map_or(0, |(_, r)| *r as usize);
        }
        out
    }

    /// The rows every operation of a single-air group takes together.
    fn single_air_rows(rows: &[usize]) -> usize {
        rows[DMA_COUNTER_MEMCPY]
            + rows[DMA_COUNTER_INPUTCPY]
            + rows[DMA_COUNTER_MEMSET]
            + rows[DMA_COUNTER_MEMCMP]
    }

    fn calculate_strategy(&mut self, totals: &DmaCounterInputGen, options: DmaAirOptions) {
        let work = DmaWork::from_counters(&totals.counters);
        self.selection = Self::select(&work, options);

        if options.with_pre_post {
            // The fused air proves the DMA controller together with its PRE/POST sub-operations,
            // so the two separate airs get nothing.
            self.dma = 0;
            self.dma_pre_post = 0;
        } else {
            self.dma =
                Self::single_air_rows(&totals.counters[DMA_OFFSET..DMA_OFFSET + DMA_COUNTER_OPS])
                    .div_ceil(Self::DMA_ROWS);
            self.dma_pre_post = Self::single_air_rows(
                &totals.counters[DMA_PRE_POST_OFFSET..DMA_PRE_POST_OFFSET + DMA_COUNTER_OPS],
            )
            .div_ceil(Self::DMA_PRE_POST_ROWS);
        }
        self.dma_with_pre_post = self.selection.instances[air::WPP];
        self.dma_unaligned = self.selection.instances[air::UNALIGNED];
        self.dma_loop = self.selection.instances[air::LOOP];
        self.compact_dma = self.selection.instances[air::COMPACT_WPP];
        debug_assert_eq!(
            self.selection.instances[air::COMPACT_WPP],
            self.selection.instances[air::COMPACT_LOOP],
            "the two blocks of CompactDma are one instance"
        );
    }

    pub fn calculate(
        &mut self,
        counters: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>,
    ) -> Vec<(usize, Vec<(CheckPoint, DmaCheckPoint)>)> {
        self.calculate_with(counters, Self::OPTIONS)
    }

    /// [`Self::calculate`] with the optional airs chosen by the caller.
    pub fn calculate_with(
        &mut self,
        counters: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>,
        options: DmaAirOptions,
    ) -> Vec<(usize, Vec<(CheckPoint, DmaCheckPoint)>)> {
        let totals: DmaCounterInputGen = self.calculate_totals(&counters);
        #[cfg(feature = "save_dma_plans")]
        let totals_debug_info = format!("{}", totals);

        self.calculate_strategy(&totals, options);
        let instances = self.selection.instances;

        let mut dma_full = DmaInstancesBuilder::new("dma_full", self.dma, Self::DMA_ROWS);
        let mut dma_pre_post_full = DmaInstancesBuilder::new(
            "dma_pre_post_full",
            self.dma_pre_post,
            Self::DMA_PRE_POST_ROWS,
        );
        let mut dma_with_pre_post = DmaWithPrePostInstancesBuilder::new(
            "dma_with_pre_post",
            instances[air::WPP],
            Self::DMA_WITH_PRE_POST_ROWS,
        );
        let mut compact_wpp = DmaWithPrePostInstancesBuilder::new(
            "compact_dma.wpp",
            instances[air::COMPACT_WPP],
            Self::COMPACT_DMA_ROWS,
        );
        let mut dma_unaligned = DmaInstancesBuilder::new(
            "dma_unaligned",
            instances[air::UNALIGNED],
            Self::DMA_UNALIGNED_ROWS,
        );
        let mut dma_loop =
            DmaLoopInstancesBuilder::new("dma_loop", instances[air::LOOP], Self::DMA_LOOP_ROWS);
        let mut compact_loop = DmaLoopInstancesBuilder::new(
            "compact_dma.loop",
            instances[air::COMPACT_LOOP],
            Self::COMPACT_DMA_ROWS,
        );

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
        let mut aligned: Vec<DmaInstancesBuilder> = (0..air::ALIGNED)
            .map(|a| DmaInstancesBuilder::new(names[a], instances[a], heights[a]))
            .collect();

        // The opcode counter each loop kind is counted under, and the counter offsets of the
        // family its rows are counted in.
        let loop_kinds = [
            (kind::MEMCPY, DMA_COUNTER_MEMCPY, DMA_64_ALIGNED_OFFSET, DMA_64_ALIGNED_INPUTS_OFFSET),
            (kind::MEMSET, DMA_COUNTER_MEMSET, DMA_64_ALIGNED_OFFSET, DMA_64_ALIGNED_INPUTS_OFFSET),
            (kind::MEMCMP, DMA_COUNTER_MEMCMP, DMA_64_ALIGNED_OFFSET, DMA_64_ALIGNED_INPUTS_OFFSET),
            (
                kind::INPUTCPY,
                DMA_COUNTER_INPUTCPY,
                DMA_64_ALIGNED_OFFSET,
                DMA_64_ALIGNED_INPUTS_OFFSET,
            ),
            (
                kind::MEMCPY_UNALIGNED,
                DMA_COUNTER_MEMCPY,
                DMA_UNALIGNED_OFFSET,
                DMA_UNALIGNED_INPUTS_OFFSET,
            ),
            (
                kind::MEMCMP_UNALIGNED,
                DMA_COUNTER_MEMCMP,
                DMA_UNALIGNED_OFFSET,
                DMA_UNALIGNED_INPUTS_OFFSET,
            ),
        ];

        for (current_chunk, dyn_counter) in counters.iter() {
            let counters =
                (**dyn_counter).as_any().downcast_ref::<DmaCounterInputGen>().unwrap().counters;

            if options.with_pre_post {
                // The controller: operations, not rows -- an operation cannot be split between
                // two instances, its PRE row has to stay next to its DMA row.
                let builder = if self.selection.assignment[kind::CTRL] == air::COMPACT_WPP {
                    &mut compact_wpp
                } else {
                    &mut dma_with_pre_post
                };
                for class in 0..DMA_WPP_CLASSES {
                    let ops = counters[DMA_WITH_PRE_POST_OFFSET + class];
                    builder.add_ops(*current_chunk, class, ops);
                }
            } else {
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
            }

            // The loop: every kind goes to the air the selection picked for it, in that air's own
            // row cost -- the packed airs take the `…_8` counts.
            for (k, op, offset, inputs_offset) in loop_kinds {
                let target = self.selection.assignment[k];
                let rows = match target {
                    air::MEMCPY => counters[DMA_64_ALIGNED_OFFSET + DMA_COUNTER_MEMCPY_8],
                    air::MEMSET => counters[DMA_64_ALIGNED_OFFSET + DMA_COUNTER_MEMSET_8],
                    _ => counters[offset + op],
                };
                if rows == 0 {
                    continue;
                }
                let inputs = counters[inputs_offset + op];
                // Unconditional: the sizing used the totals and this hand-out walks the chunks, so
                // the two disagreeing means rows are about to be routed to an air that was never
                // given room for them. Catching it here names the kind and the air; letting the
                // subtraction wrap in release would surface it much later, as an opaque overflow
                // in a builder.
                assert!(
                    rows <= self.selection.rows[k],
                    "chunk {current_chunk:?} owes air {target} {rows} rows of kind {k}, more than \
                     the {} the strategy routed to it",
                    self.selection.rows[k],
                );
                self.selection.rows[k] -= rows;
                match target {
                    a if a < air::ALIGNED => {
                        aligned[a].add_op_rows(*current_chunk, 0, rows, inputs, op)
                    }
                    air::UNALIGNED => {
                        dma_unaligned.add_op_rows(*current_chunk, 0, rows, inputs, op)
                    }
                    air::LOOP => dma_loop.add_class_rows(*current_chunk, k, rows, inputs),
                    air::COMPACT_LOOP => {
                        compact_loop.add_class_rows(*current_chunk, k, rows, inputs)
                    }
                    _ => panic!("kind {k} routed to air {target}, which cannot prove it"),
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
        self.dma_with_pre_post_plan = dma_with_pre_post.get_plan();
        self.dma_loop_plan = dma_loop.get_plan();
        self.compact_dma_plan = fuse_compact_plans(compact_wpp.get_plan(), compact_loop.get_plan());

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
        // The airs with a checkpoint type of their own come back apart from the rest.
        let wpp_title = get_dma_air_name::<F>(DmaWithPrePostTrace::<F>::AIR_ID);
        for (segment_id, (_, cp)) in self.dma_with_pre_post_plan.iter().enumerate() {
            debug_info += &cp.get_debug_info(wpp_title, segment_id as u64);
            debug_info += "\n";
        }
        let loop_title = get_dma_air_name::<F>(DmaLoopTrace::<F>::AIR_ID);
        for (segment_id, (_, cp)) in self.dma_loop_plan.iter().enumerate() {
            debug_info += &cp.get_debug_info(loop_title, segment_id as u64);
            debug_info += "\n";
        }
        let compact_title = get_dma_air_name::<F>(CompactDmaTrace::<F>::AIR_ID);
        for (segment_id, (_, cp)) in self.compact_dma_plan.iter().enumerate() {
            debug_info += &cp.get_debug_info(compact_title, segment_id as u64);
            debug_info += "\n";
        }
        use std::fs;

        let path = std::env::var("DEBUG_OUTPUT_PATH").unwrap_or_else(|_| "tmp/".to_string());
        let full_path = format!("{}{}", path, filename);

        fs::write(&full_path, debug_info)?;
        Ok(())
    }
}

/// One plan per instance of `CompactDma`, from the plans its two blocks were given.
///
/// The blocks keep budgets of their own, so they may not run out at the same time: when one has
/// fewer plans than the other, its half of the last instances is empty and the fill pads it. The
/// `loop_` block is a segment of its chain in every instance, the empty ones included, so its last
/// segment is the air's last instance whatever its own builder said.
pub(crate) fn fuse_compact_plans(
    wpp: Vec<(CheckPoint, DmaWithPrePostCheckPoint)>,
    lp: Vec<(CheckPoint, DmaLoopCheckPoint)>,
) -> Vec<(CheckPoint, CompactDmaCheckPoint)> {
    let instances = wpp.len().max(lp.len());
    let mut wpp = wpp.into_iter();
    let mut lp = lp.into_iter();
    (0..instances)
        .map(|segment| {
            let is_last_segment = segment + 1 == instances;
            let mut wpp = wpp.next().map(|(_, cp)| cp).unwrap_or_default();
            let mut lp = lp.next().map(|(_, cp)| cp).unwrap_or_default();
            wpp.is_last_segment = is_last_segment;
            lp.is_last_segment = is_last_segment;

            // Sorted and deduplicated: the collect phase indexes an instance's collectors by the
            // position of the chunk in this list.
            let mut chunks: Vec<ChunkId> =
                wpp.chunks.keys().chain(lp.chunks.keys()).copied().collect();
            chunks.sort_unstable();
            chunks.dedup();
            (CheckPoint::Multiple(chunks), CompactDmaCheckPoint { wpp, lp })
        })
        .collect()
}

#[cfg(test)]
#[path = "tests/dma_strategy_tests.rs"]
mod tests;
