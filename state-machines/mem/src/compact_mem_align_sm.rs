//! Witness of the fused `CompactMemAlign` air: the two mem-align airs, on the same rows.

use std::sync::Arc;

use proofman_common::{AirInstance, FromTrace, GenericTrace, ProofmanResult};
use proofman_fields::PrimeField64;
use rayon::prelude::*;
use zisk_pil::{
    CompactMemAlignAirValues, CompactMemAlignLargeAirValues, CompactMemAlignLargeTrace,
    CompactMemAlignLargeTraceRow, CompactMemAlignLargeTraceRowPacked, CompactMemAlignTrace,
    CompactMemAlignTraceRow, CompactMemAlignTraceRowPacked, MemAlignTraceRow, ZISK_AIRGROUP_ID,
};

use crate::{
    is_byte_align_op, MemAlignByteSM, MemAlignBytesBlockRow, MemAlignFullBlockRow, MemAlignInput,
    MemAlignSM,
};

/// Registers a `MemAlign` row carries and the bits of each, as the air declares them: what the
/// dual-byte range check pairs up.
const CHUNK_NUM: usize = 8;
const CHUNK_BITS: usize = 8;

/// Rows the longest sub-program (`RWVWR`) takes, which is what the fill stages one operation in.
const MAX_OP_ROWS: usize = 5;

/// Fills the `CompactMemAlign` trace by running the two mem-align fills over the same rows.
///
/// There is no proving logic of its own here: each block is filled by the state machine of the air
/// it replaces -- `MemAlignSM` proves a sub-program exactly as it does for `MemAlign`, and the rows
/// it produces are scattered into the lanes of the `full_` block; `MemAlignByteSM` fills the
/// `bytes_` block one virtual row per operation. What this type adds is the one thing the two
/// cannot do separately: putting their results in a single `AirInstance`.
pub struct CompactMemAlignSM<F: PrimeField64> {
    mem_align_sm: Arc<MemAlignSM<F>>,
    mem_align_byte_sm: Arc<MemAlignByteSM<F>>,
}

impl<F: PrimeField64> CompactMemAlignSM<F> {
    /// Shares the two state machines with the standalone instances rather than building its own:
    /// they hold the `Std` range-check ids, and the ids must be the same ones the standalone airs
    /// raise their checks against.
    pub fn new(
        mem_align_sm: Arc<MemAlignSM<F>>,
        mem_align_byte_sm: Arc<MemAlignByteSM<F>>,
    ) -> Arc<Self> {
        Arc::new(Self { mem_align_sm, mem_align_byte_sm })
    }

    /// Builds the instance of one `CompactMemAlign` plan.
    ///
    /// `is_large` picks the tall air, which commits the very same columns -- only the height and
    /// the air id differ.
    pub fn compute_witness(
        &self,
        inputs: &[Vec<MemAlignInput>],
        is_large: bool,
        packed: bool,
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        macro_rules! build {
            ($row:ty, $trace:ident, $values:ident) => {{
                const NUM_ROWS: usize = $trace::<()>::NUM_ROWS;
                const AIR_ID: usize = $trace::<()>::AIR_ID;

                let mut trace =
                    GenericTrace::<$row, NUM_ROWS, ZISK_AIRGROUP_ID, AIR_ID>::new_from_vec(
                        trace_buffer,
                    )?;
                let bytes_padding = self.fill_blocks(inputs, &mut trace.buffer);

                let mut air_values = $values::<F>::new();
                air_values.bytes_padding_size = F::from_usize(bytes_padding);
                Ok(AirInstance::new_from_trace(
                    FromTrace::new(&mut trace).with_air_values(&mut air_values),
                ))
            }};
        }

        match (is_large, packed) {
            (false, false) => {
                build!(CompactMemAlignTraceRow<F>, CompactMemAlignTrace, CompactMemAlignAirValues)
            }
            (false, true) => {
                build!(
                    CompactMemAlignTraceRowPacked<F>,
                    CompactMemAlignTrace,
                    CompactMemAlignAirValues
                )
            }
            (true, false) => {
                build!(
                    CompactMemAlignLargeTraceRow<F>,
                    CompactMemAlignLargeTrace,
                    CompactMemAlignLargeAirValues
                )
            }
            (true, true) => {
                build!(
                    CompactMemAlignLargeTraceRowPacked<F>,
                    CompactMemAlignLargeTrace,
                    CompactMemAlignLargeAirValues
                )
            }
        }
    }

    /// Fills the two blocks and returns the padding of the `bytes_` one, which is the only air
    /// value the fused air has.
    ///
    /// The operations are split the way the planner routed them: the byte ones to the `bytes_`
    /// block, where each takes one virtual row, and the rest to the `full_` block. The two fills
    /// write disjoint columns of the same rows, so the order between them does not matter.
    fn fill_blocks<R: MemAlignFullBlockRow<F> + MemAlignBytesBlockRow<F>>(
        &self,
        inputs: &[Vec<MemAlignInput>],
        rows: &mut [R],
    ) -> usize {
        let mut full_ops = Vec::new();
        let mut byte_ops = Vec::new();
        for chunk in inputs {
            for op in chunk {
                if is_byte_align_op(op) {
                    byte_ops.push(op);
                } else {
                    full_ops.push(op);
                }
            }
        }

        let bytes_used = self.mem_align_byte_sm.fill_bytes_block(&byte_ops, rows);
        self.fill_full_block(&full_ops, rows);

        <R as MemAlignBytesBlockRow<F>>::bytes_lanes_x_row() * rows.len() - bytes_used
    }

    /// Fills the `full_` block of a `CompactMemAlign` trace and raises the multiplicities its rows
    /// add to the program lookup and the dual-byte range.
    ///
    /// See [`fill_full_block_rows`] for what the fill does.
    fn fill_full_block<R: MemAlignFullBlockRow<F>>(
        &self,
        ops: &[&MemAlignInput],
        rows: &mut [R],
    ) -> usize {
        let lanes = R::full_lanes_x_row();
        let (used, mut dual_mults) = fill_full_block_rows(ops, rows, &|op, scratch| {
            self.mem_align_sm.prove_mem_align_op(op, scratch);
        });

        // Every padding lane assumes the same row of the program, and holds eight zero registers.
        let padding_size = (rows.len() * lanes - used) as u64;
        self.mem_align_sm.add_padding_rom_mults(padding_size);
        dual_mults[0] += (CHUNK_NUM / 2) as u64 * padding_size;
        self.mem_align_sm.add_dual_byte_mults(&dual_mults);

        used
    }
}

/// Fills the `full_` block of a `CompactMemAlign` trace: the sub-programs of the operations,
/// one after another over the block's virtual rows, then padding to the end of the block.
///
/// `rows` is the whole fused trace, and this only ever touches the `full_*` columns -- the
/// `bytes_` fill writes the others over the same rows. Returns the virtual rows the operations
/// took.
///
/// A sub-program spans 2 to 5 CONSECUTIVE virtual rows, which with more than one lane per row
/// means it runs across lanes and rolls over into the next row exactly as it used to roll over
/// into the next row before (see the LANES section of `compact_mem_align.pil`). Each operation
/// is proved into a scratch of standalone rows by the very code the `MemAlign` air is filled
/// with, and only then scattered into the lanes it occupies.
pub(crate) fn fill_full_block_rows<F: PrimeField64, R: MemAlignFullBlockRow<F>>(
    ops: &[&MemAlignInput],
    rows: &mut [R],
    prove: &(impl Fn(&MemAlignInput, &mut [MemAlignTraceRow<F>]) + Sync),
) -> (usize, Vec<u64>) {
    let lanes = R::full_lanes_x_row();
    let num_slots = rows.len() * lanes;

    // The virtual row every operation starts at.
    let mut starts = Vec::with_capacity(ops.len() + 1);
    let mut used = 0;
    for op in ops {
        starts.push(used);
        used += MemAlignSM::<F>::op_rows(op);
    }
    starts.push(used);
    assert!(
        used <= num_slots,
        "CompactMemAlign full block: {used} virtual rows do not fit in {num_slots}"
    );

    tracing::debug!(
        "··· Filling CompactMemAlign full block [{used} / {num_slots} virtual rows filled {:.2}%]",
        used as f64 / num_slots as f64 * 100.0
    );

    // The rows the operations reach into, and the ones after them, which are padding only.
    let (head, tail) = rows.split_at_mut(used.div_ceil(lanes));
    let head_slots = head.len() * lanes;

    let cuts = group_cuts(&starts, lanes, rayon::current_num_threads().max(1));

    let mut groups = Vec::with_capacity(cuts.len().saturating_sub(1));
    let mut rest = head;
    let mut done_rows = 0;
    for window in cuts.windows(2) {
        let (first, last) = (window[0], window[1]);
        let end_row = starts[last].div_ceil(lanes);
        let (chunk, next) = rest.split_at_mut(end_row - done_rows);
        // What the group has to pad: nothing, except for the last one, which owns the lanes
        // between the end of the operations and the end of its row.
        let pad_to = if last == ops.len() { head_slots } else { starts[last] };
        groups.push((first, last, pad_to, chunk));
        done_rows = end_row;
        rest = next;
    }

    // The lane the padding is made of: every column zero but `reset`, exactly the row the
    // standalone air pads with.
    let mut padding: MemAlignTraceRow<F> = Default::default();
    padding.set_reset(true);

    let dual_mults = groups
        .into_par_iter()
        .map(|(first, last, pad_to, chunk)| {
            let base = starts[first];
            let mut mults = vec![0u64; 1 << (2 * CHUNK_BITS)];
            let mut scratch = [MemAlignTraceRow::<F>::default(); MAX_OP_ROWS];

            for index in first..last {
                let op_rows = MemAlignSM::<F>::op_rows(ops[index]);
                prove(ops[index], &mut scratch[..op_rows]);
                for (row, src) in scratch[..op_rows].iter().enumerate() {
                    let slot = starts[index] + row - base;
                    chunk[slot / lanes].set_full_lane(slot % lanes, src);

                    // Range-check registers in dual-byte pairs: (reg[0],reg[1]), ...
                    let reg = src.get_all_reg();
                    for i in (0..CHUNK_NUM).step_by(2) {
                        mults[((reg[i] as usize) << CHUNK_BITS) | reg[i + 1] as usize] += 1;
                    }
                }
            }
            for slot in (starts[last] - base)..(pad_to - base) {
                chunk[slot / lanes].set_full_lane(slot % lanes, &padding);
            }
            mults
        })
        .reduce(
            || vec![0u64; 1 << (2 * CHUNK_BITS)],
            |mut acc, mults| {
                for (a, b) in acc.iter_mut().zip(mults) {
                    *a += b;
                }
                acc
            },
        );

    tail.par_iter_mut().for_each(|row| {
        for lane in 0..lanes {
            row.set_full_lane(lane, &padding);
        }
    });

    (used, dual_mults)
}

/// Where to cut the operations into groups that can be filled in parallel, as indexes into them:
/// `cuts[i]..cuts[i + 1]` is one group, and `cuts` always begins at 0 and ends at the operation
/// count.
///
/// Every cut but the last falls on a PHYSICAL ROW boundary -- `starts[cut]` is a multiple of
/// `lanes` -- so two groups never write the same row and rayon can hand each its own slice. That is
/// what makes the cuts uneven: a sub-program takes 2, 3 or 5 virtual rows, so with several lanes per
/// row a group can only end where the running total happens to fill a row, and `target` is the size
/// it aims for rather than the size it gets. When no boundary is reached, the group simply grows --
/// in the worst case to the whole trace, which is still correct, only sequential.
///
/// `starts` is the virtual row every operation starts at, plus the total as its last element.
fn group_cuts(starts: &[usize], lanes: usize, n_ranges: usize) -> Vec<usize> {
    let ops = starts.len() - 1;
    let used = starts[ops];
    let target = used.div_ceil(n_ranges).max(1);

    let mut cuts = vec![0usize];
    for index in 0..ops {
        let group_start = *cuts.last().expect("cuts begins at 0");
        if starts[index + 1] - starts[group_start] >= target && starts[index + 1] % lanes == 0 {
            cuts.push(index + 1);
        }
    }
    if *cuts.last().expect("cuts begins at 0") != ops {
        cuts.push(ops);
    }
    cuts
}

#[cfg(test)]
#[path = "compact_mem_align_fill_tests.rs"]
mod fill_tests;
