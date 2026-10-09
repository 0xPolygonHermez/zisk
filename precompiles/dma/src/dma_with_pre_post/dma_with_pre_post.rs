use std::marker::PhantomData;
use std::sync::Arc;

use proofman_fields::PrimeField64;
use rayon::prelude::*;

use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use zisk_core::zisk_ops::ZiskOp;
use zisk_pil::{DmaWithPrePostTrace, DmaWithPrePostTraceRow, DmaWithPrePostTraceRowPacked};

use crate::{dma_trace, DmaWithPrePostBlockRow, DmaWithPrePostInput, DmaWithPrePostModule};
use zisk_precomp_helpers::DmaInfo;

/// The `DmaWithPrePostSM` struct encapsulates the logic of the fused DmaWithPrePost State Machine.
///
/// It writes what `DmaSM` and `DmaPrePostSM` write together, but into the same air: one row per
/// operation, and a second one when the operation needs both a PRE and a POST.
///
///   case                      row i                  row i+1
///   ──────────────────────────────────────────────────────────────
///   (1) no pre, no post       DMA                    ─
///   (2) only pre              DMA + PRE              ─
///   (3) only post             DMA + POST             ─
///   (4) pre and post          DMA + POST             PRE
///
/// The POST is the sub-operation that stays on the DMA row because the memcmp result has to be
/// published there; see the header of `dma_with_pre_post.pil`.
pub struct DmaWithPrePostSM<F: PrimeField64> {
    _phantom: PhantomData<F>,
}

impl<F: PrimeField64> Default for DmaWithPrePostSM<F> {
    fn default() -> Self {
        Self { _phantom: PhantomData }
    }
}

impl<F: PrimeField64> DmaWithPrePostSM<F> {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Writes one operation into `rows`, which is exactly `input.rows()` long.
    ///
    /// The columns defined in the PIL with `<==` (`has_pre_row`, `pp_*`, `write_value`,
    /// `bus_write_value`, `last_dst_byte`, `l_memcmp_result`, `loop_b0`, `loop_extended_arg`,
    /// `static_count`, `sel_count_from_mem`) are *not* written here: they carry a `witness_calc`
    /// hint and the prover derives them from their expression.
    #[inline(always)]
    fn process_op<R: DmaWithPrePostBlockRow<F>>(input: &DmaWithPrePostInput, rows: &mut [R]) {
        debug_assert_eq!(rows.len(), input.rows());

        let use_pre = DmaInfo::get_pre_count(input.encoded) > 0;
        let use_post = DmaInfo::get_post_count(input.encoded) > 0;

        // The DMA row carries the POST when there is one, otherwise the PRE (if any).
        Self::fill_dma_row(input, &mut rows[0]);
        if use_post || use_pre {
            Self::fill_sub_op(input, &mut rows[0], use_post);
        }

        // ...and when both are needed, the PRE goes to the extra row right after it.
        if rows.len() == DmaWithPrePostInput::DOUBLE_ROW {
            debug_assert!(use_pre && use_post);
            Self::fill_pre_row(input, &mut rows[1]);
            Self::fill_sub_op(input, &mut rows[1], false);
        }
    }

    /// Fills the DMA controller columns of the row that drives the operation.
    fn fill_dma_row<R: DmaWithPrePostBlockRow<F>>(input: &DmaWithPrePostInput, row: &mut R) {
        let encoded = input.encoded;

        // count: h_count | l_count, with the shift that lets the ROM tell a zero count from a
        // multiple of 256 (see @[count_lt_256] in the PIL).
        let count = DmaInfo::get_count(encoded);
        let count_lt_256 = count < 256;
        let count_ge_256 = 1 - count_lt_256 as usize;
        let h_count = ((count >> 8) - count_ge_256) as u32;
        let l_count = (count & 0xFF) as u16 + 256 * count_ge_256 as u16;
        row.set_count_lt_256(count_lt_256);
        row.set_h_count(h_count);
        row.set_l_count(l_count);

        let use_src = input.op != ZiskOp::DMA_INPUTCPY && input.op != ZiskOp::DMA_XMEMSET;

        // Without a source the src columns are zeroed rather than filled with whatever `b` carried
        // (the count, for inputcpy). `DmaSM` cannot do that: it has to keep them equal to what it
        // publishes on DMA_BUS_ID. Here there is no bus, `src` is multiplied by
        // `sel_memcpy + sel_memcmp` everywhere it is used, and zeroing them keeps
        // @[pp_src_offset] at 0 — which is what the PRE/POST byte rotation of an inputcpy needs.
        let src = if use_src { input.src } else { 0 };

        // The low 32 bits are the address the air works with; the high word travels apart and is
        // only allowed to be non-zero when the count is (see `src_hi` / `dst_hi` in the PIL).
        let src32 = src as u32;
        let dst32 = input.dst as u32;
        let h_src64 = src32 >> 10;
        let h_dst64 = dst32 >> 10;
        let l_src64 = (src32 >> 3) as u8 & 0x7F;
        let l_dst64 = (dst32 >> 3) as u8 & 0x7F;
        let src_offset = src32 as u8 & 0x07;

        row.set_src_hi((src >> 32) as u32);
        row.set_h_src64(h_src64);
        row.set_l_src64(l_src64);
        row.set_src_offset(src_offset);
        row.set_dst_hi((input.dst >> 32) as u32);
        row.set_h_dst64(h_dst64);
        row.set_l_dst64(l_dst64);
        row.set_dst_offset(dst32 as u8 & 0x07);
        row.set_main_step(input.step);

        let pre_count = DmaInfo::get_pre_count(encoded) as u8;
        let loop_count = DmaInfo::get_loop_count(encoded);
        let post_count = DmaInfo::get_post_count(encoded);
        row.set_use_pre(pre_count > 0);
        row.set_use_loop(loop_count > 0);
        row.set_use_post(post_count > 0);
        row.set_src64_inc_by_pre(DmaInfo::get_src64_inc_by_pre(encoded) > 0);
        row.set_pre_count(pre_count);
        row.set_l_count64((l_count - pre_count as u16 - post_count as u16) >> 3);
        if use_src {
            row.set_src_offset_after_pre((src_offset + pre_count) % 8);
        }

        match input.op {
            ZiskOp::DMA_MEMCPY => row.set_sel_memcpy(true),
            ZiskOp::DMA_XMEMCPY => {
                row.set_sel_memcpy(true);
                row.set_sel_extended(true);
            }
            ZiskOp::DMA_MEMCMP | ZiskOp::DMA_XMEMCMP => {
                row.set_sel_memcmp(true);
                row.set_sel_extended(input.op == ZiskOp::DMA_XMEMCMP);
                let count_diff = input.count_bus - count as u32;
                let count_diff_chunks = [count_diff as u16, (count_diff >> 16) as u16];
                row.set_all_count_diff_chunks(&count_diff_chunks);
            }
            ZiskOp::DMA_INPUTCPY => row.set_sel_inputcpy(true),
            ZiskOp::DMA_XMEMSET => {
                row.set_sel_memset(true);
                row.set_sel_extended(true);
                row.set_fill_byte(DmaInfo::get_fill_byte(encoded));
            }
            op => panic!("Invalid DMA operation {op}"),
        }
    }

    /// Fills the extra PRE row: no DMA operation at all, only the columns it shares with the DMA
    /// row of the operation (@[latch] in the PIL) plus the PRE/POST part filled by `fill_sub_op`.
    fn fill_pre_row<R: DmaWithPrePostBlockRow<F>>(input: &DmaWithPrePostInput, row: &mut R) {
        row.set_is_pre_row(true);
        row.set_main_step(input.step);
        match input.op {
            ZiskOp::DMA_MEMCPY | ZiskOp::DMA_XMEMCPY => row.set_sel_memcpy(true),
            ZiskOp::DMA_MEMCMP | ZiskOp::DMA_XMEMCMP => row.set_sel_memcmp(true),
            ZiskOp::DMA_INPUTCPY => row.set_sel_inputcpy(true),
            ZiskOp::DMA_XMEMSET => {
                row.set_sel_memset(true);
                row.set_fill_byte(DmaInfo::get_fill_byte(input.encoded));
            }
            op => panic!("Invalid DMA operation {op}"),
        }
    }

    /// Fills the PRE/POST part of a row: the byte selectors and rotation, the bytes read and
    /// pre-written, and the memcmp result of the sub-operation.
    fn fill_sub_op<R: DmaWithPrePostBlockRow<F>>(
        input: &DmaWithPrePostInput,
        row: &mut R,
        is_post: bool,
    ) {
        let encoded = input.encoded;
        let is_memcmp = input.op == ZiskOp::DMA_MEMCMP || input.op == ZiskOp::DMA_XMEMCMP;
        let is_memcpy = input.op == ZiskOp::DMA_MEMCPY || input.op == ZiskOp::DMA_XMEMCPY;
        let is_memset = input.op == ZiskOp::DMA_XMEMSET;
        let load_src = is_memcpy || is_memcmp;

        let pre_count = DmaInfo::get_pre_count(encoded);
        let post_count = DmaInfo::get_post_count(encoded);
        let dma_src_offset = if load_src { (input.src & 0x07) as usize } else { 0 };

        // The sub-operation parameters, exactly as @[pp_dst_offset], @[pp_src_offset] and
        // @[pp_count] derive them from the DMA columns.
        let (dst_offset, src_offset, count, src_values, dst_pre_value) = if is_post {
            (
                0usize,
                // @[pp_src_offset] of a POST is @[src_offset_after_pre], and the PRE only walks
                // the source when there is one: `fill_dma_row` leaves that column at 0 for an
                // inputcpy or a memset, and the DMA ROM row of a no-src operation carries 0 there
                // too. Adding `pre_count` regardless would make the byte rotation this function
                // derives disagree with the offset the air looks the table up with.
                if load_src { (dma_src_offset + pre_count) & 0x07 } else { 0 },
                post_count,
                input.post_src_values,
                input.post_dst_value,
            )
        } else {
            (
                (input.dst & 0x07) as usize,
                dma_src_offset,
                pre_count,
                input.pre_src_values,
                input.pre_dst_value,
            )
        };
        debug_assert!((1..=8).contains(&count));

        let second_read = (src_offset + count) > 8;
        row.set_enabled_second_read(second_read);

        // Read bytes: the src 64-bit word(s), or the fill byte everywhere for a memset — which is
        // what `pp_sel_memset * (rb[i] - fill_byte) === 0` asks for.
        let mut rb = [0u8; 16];
        if is_memset {
            let fill_byte = DmaInfo::get_fill_byte(encoded);
            rb.fill(fill_byte);
        } else {
            rb[..8].copy_from_slice(&src_values[0].to_le_bytes());
            if second_read {
                rb[8..].copy_from_slice(&src_values[1].to_le_bytes());
            }
        }
        row.set_all_rb(&rb);

        // Pre-write bytes: the dst word as it was before this row writes it.
        let pb = dst_pre_value.to_le_bytes();
        row.set_all_pb(&pb);

        // Byte rotation: sr_value = |dst_offset - src_offset|.
        let selr_value = if dst_offset > src_offset {
            row.set_dst_offset_gt_src_offset(true);
            dst_offset - src_offset
        } else {
            row.set_dst_offset_gt_src_offset(false);
            src_offset - dst_offset
        };
        let selr: [bool; 7] = std::array::from_fn(|i| selr_value == i);
        row.set_all_selr(&selr);

        // Byte selectors: which bytes of the dst word this sub-operation touches.
        //
        // NOTE: special case of count = 8 for memcmp, the mask must be all 1s; applying the shift
        // for count = 8 would overflow, so it is handled apart.
        let mask = if count == 8 {
            debug_assert_eq!(dst_offset, 0);
            u64::MAX
        } else {
            let base = u64::MAX << (dst_offset * 8);
            base ^ (base << (count * 8))
        };
        let sb: [bool; 8] = std::array::from_fn(|i| (mask >> (i * 8)) & 0xFF != 0);
        row.set_all_sb(&sb);

        if is_memcmp {
            // The result belongs to the POST, or to the PRE when the operation has no POST. That
            // is the invariant the DMA ROM keeps, and what lets the fused air publish the result
            // straight from its DMA row (`is_pre_row * memcmp_result_nz === 0`).
            let result = if is_post || post_count == 0 {
                DmaInfo::get_memcmp_res_as_u64(encoded)
            } else {
                0
            };
            let is_nz = result != 0;
            let is_negative = is_nz && DmaInfo::is_memcmp_negative(encoded);
            row.set_memcmp_result_nz(is_nz);
            row.set_memcmp_result_is_negative(is_negative);

            let abs_diff_dst_src = if is_negative { (!result).wrapping_add(1) } else { result };
            debug_assert!(abs_diff_dst_src <= 0xFF);
            let abs_diff_dst_src = abs_diff_dst_src as u8;
            row.set_abs_diff_dst_src(abs_diff_dst_src);

            if is_nz {
                // the index of the differing byte determines the factor
                let dst_index = dst_offset + count - 1;
                let factor = 1u64 << (8 * (dst_index % 4));
                let factor = if is_negative { F::ORDER_U64 - factor } else { factor };
                if dst_index < 4 {
                    row.set_all_diff_factor(&[factor, 0]);
                } else {
                    row.set_all_diff_factor(&[0, factor]);
                }

                let last_dst_byte = pb[dst_index];
                if is_negative {
                    debug_assert!(
                        abs_diff_dst_src <= (255 - last_dst_byte) && abs_diff_dst_src > 0,
                        "abs_diff_dst_src: {abs_diff_dst_src} last_dst_byte: 0x{last_dst_byte:02X} \
                         result: 0x{result:016X} S:{} index:{dst_index} dst_offset:{dst_offset} \
                         src_offset:{src_offset} count:{count} is_post:{is_post}",
                        input.step,
                    );
                } else {
                    debug_assert!(
                        abs_diff_dst_src <= last_dst_byte && abs_diff_dst_src > 0,
                        "abs_diff_dst_src: {abs_diff_dst_src} last_dst_byte: 0x{last_dst_byte:02X} \
                         result: 0x{result:016X} S:{} index:{dst_index} dst_offset:{dst_offset} \
                         src_offset:{src_offset} count:{count} is_post:{is_post}",
                        input.step,
                    );
                }
            }
        }
    }

    /// Fills `rows` with `inputs`, in order.
    ///
    /// `rows` is the whole instance, and the rows past the operations are left as they are: the
    /// buffer is zeroed, and a zero row is the air's padding. The operations are split into groups
    /// and the rows cut at the row each group starts on. An operation is never split, so a group
    /// always owns whole operations and its rows are contiguous -- which is what keeps a PRE row
    /// next to its DMA row.
    ///
    /// Takes `rows` rather than a `DmaWithPrePostTrace` because the fused `CompactDma` air carries
    /// this block inside a wider row (see [`DmaWithPrePostBlockRow`]).
    pub(crate) fn fill_rows<R: DmaWithPrePostBlockRow<F>>(
        inputs: &[&DmaWithPrePostInput],
        rows: &mut [R],
    ) {
        let total_rows: usize = inputs.iter().map(|input| input.rows()).sum();
        // The planner reserves the rows of every operation as a block (see
        // `DmaWithPrePostInstancesBuilder`), so this can only fire if the plan and the collected
        // inputs disagree.
        assert!(
            total_rows <= rows.len(),
            "DmaWithPrePost: {} operations need {total_rows} rows, only {} available",
            inputs.len(),
            rows.len()
        );

        let num_threads = rayon::current_num_threads();
        let group_len = inputs.len().div_ceil(num_threads).max(1);

        let mut groups: Vec<(&[&DmaWithPrePostInput], &mut [R])> = Vec::new();
        let mut pending_inputs: &[&DmaWithPrePostInput] = inputs;
        let mut pending_rows: &mut [R] = rows;
        while !pending_inputs.is_empty() {
            let take = group_len.min(pending_inputs.len());
            let group_rows: usize = pending_inputs[..take].iter().map(|input| input.rows()).sum();
            let (group_inputs, rest_inputs) = pending_inputs.split_at(take);
            let (group_rows, rest_rows) = pending_rows.split_at_mut(group_rows);
            groups.push((group_inputs, group_rows));
            pending_inputs = rest_inputs;
            pending_rows = rest_rows;
        }

        groups.into_par_iter().for_each(|(group_inputs, group_rows)| {
            let mut cursor = 0usize;
            for input in group_inputs {
                let rows = input.rows();
                Self::process_op(input, &mut group_rows[cursor..cursor + rows]);
                cursor += rows;
            }
        });
    }

    fn compute_witness_inner<R: DmaWithPrePostBlockRow<F>>(
        &self,
        inputs: &[Vec<DmaWithPrePostInput>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = DmaWithPrePostTrace::<R>::new_from_vec_zeroes(trace_buffer)?;
        let num_rows = trace.num_rows();

        let flat_inputs: Vec<&DmaWithPrePostInput> = inputs.iter().flatten().collect();
        let total_rows: usize = flat_inputs.iter().map(|input| input.rows()).sum();
        dma_trace("DmaWithPrePost", total_rows, num_rows);

        timer_start_trace!(DMA_WITH_PRE_POST_TRACE);
        Self::fill_rows(&flat_inputs, trace.buffer.as_mut_slice());
        timer_stop_and_log_trace!(DMA_WITH_PRE_POST_TRACE);

        let from_trace = FromTrace::new(&mut trace);
        Ok(AirInstance::new_from_trace(from_trace))
    }
}

impl<F: PrimeField64> DmaWithPrePostModule<F> for DmaWithPrePostSM<F> {
    fn get_name(&self) -> &'static str {
        "dma_with_pre_post"
    }
    fn compute_witness(
        &self,
        inputs: &[Vec<DmaWithPrePostInput>],
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<AirInstance<F>> {
        if packed {
            self.compute_witness_inner::<DmaWithPrePostTraceRowPacked<F>>(inputs, trace_buffer)
        } else {
            self.compute_witness_inner::<DmaWithPrePostTraceRow<F>>(inputs, trace_buffer)
        }
    }
}

#[cfg(test)]
#[path = "../tests/dma_with_pre_post_witness_tests.rs"]
mod tests;
