use std::sync::Arc;

use proofman_fields::PrimeField64;
use rayon::prelude::*;

use pil2_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use zisk_core::zisk_ops::ZiskOp;
use zisk_pil::{
    DmaWithPrePostTrace, DmaWithPrePostTraceRow, DmaWithPrePostTraceRowOps,
    DmaWithPrePostTraceRowPacked, DMA_BYTE_CMP_TABLE_ID, DMA_PRE_POST_TABLE_ID,
    DMA_PRE_POST_TABLE_SIZE, DMA_ROM_ID, DUAL_RANGE_7_BITS_ID, DUAL_RANGE_BYTE_ID,
};

use crate::{
    dma_trace, DmaPrePostRom, DmaRom, DmaWithPrePostInput, DmaWithPrePostModule,
    DMA_ROM_WITH_MEMCMP_SIZE,
};
use zisk_precomp_helpers::DmaInfo;

/// Multiplicities of every table the air looks up, accumulated per worker and merged at the end.
struct Mults {
    /// `DUAL_RANGE_7_BITS`: the (l_src64, l_dst64) pair of a DMA row.
    dual_7_bits: Vec<u64>,
    /// 22-bit range check of h_src64 / h_dst64.
    values_22_bits: Vec<u32>,
    /// 24-bit range check of h_count. Values below 256 (virtually all of them) are accumulated in
    /// `low_24_bits`, the rest are pushed one by one.
    values_24_bits: Vec<u32>,
    low_24_bits: Vec<u32>,
    /// 16-bit range check of the two count_diff chunks of a memcmp.
    range_16_bits: Vec<u32>,
    /// `DMA_ROM`: the phase decomposition of a DMA row.
    rom: Vec<u64>,
    /// `DMA_PRE_POST_TABLE`: the byte mask and rotation of a PRE/POST sub-operation.
    pre_post: Vec<u64>,
    /// `DMA_BYTE_CMP_TABLE`: the first differing byte of a memcmp.
    byte_cmp: Vec<u64>,
    /// `DUAL_RANGE_BYTE`: the read bytes (`rb`) and the pre-write bytes (`pb`).
    dual_byte: Vec<u64>,
}

impl Mults {
    fn new() -> Self {
        Self {
            dual_7_bits: vec![0u64; 1 << 14],
            values_22_bits: Vec::new(),
            values_24_bits: Vec::new(),
            low_24_bits: vec![0u32; 256],
            range_16_bits: vec![0u32; 1 << 16],
            rom: vec![0u64; DMA_ROM_WITH_MEMCMP_SIZE],
            pre_post: vec![0u64; DMA_PRE_POST_TABLE_SIZE],
            byte_cmp: vec![0u64; 256 * 255],
            dual_byte: vec![0u64; 1 << 16],
        }
    }

    fn merge(mut self, other: Self) -> Self {
        fn add<T: std::ops::AddAssign + Copy>(acc: &mut [T], other: &[T]) {
            for (a, &b) in acc.iter_mut().zip(other.iter()) {
                *a += b;
            }
        }
        add(&mut self.dual_7_bits, &other.dual_7_bits);
        add(&mut self.low_24_bits, &other.low_24_bits);
        add(&mut self.range_16_bits, &other.range_16_bits);
        add(&mut self.rom, &other.rom);
        add(&mut self.pre_post, &other.pre_post);
        add(&mut self.byte_cmp, &other.byte_cmp);
        add(&mut self.dual_byte, &other.dual_byte);
        self.values_22_bits.extend(other.values_22_bits);
        self.values_24_bits.extend(other.values_24_bits);
        self
    }

    /// Charges the four `DUAL_RANGE_BYTE` lookups of one 64-bit value. The pairs are indexed the
    /// same way the lookup builds them, two bytes per row of the table.
    #[inline(always)]
    fn add_dual_bytes(&mut self, value: u64) {
        self.dual_byte[(value & 0xFFFF) as usize] += 1;
        self.dual_byte[((value >> 16) & 0xFFFF) as usize] += 1;
        self.dual_byte[((value >> 32) & 0xFFFF) as usize] += 1;
        self.dual_byte[((value >> 48) & 0xFFFF) as usize] += 1;
    }
}

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
    /// Reference to the PIL2 standard library.
    pub std: Arc<Std<F>>,

    rom_table_id: usize,
    pre_post_table_id: usize,
    byte_cmp_table_id: usize,
    dual_range_7_bits_id: usize,
    dual_range_byte_id: usize,
    range_22_bits_id: usize,
    range_24_bits_id: usize,
    range_16_bits_id: usize,
}

impl<F: PrimeField64> DmaWithPrePostSM<F> {
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        Arc::new(Self {
            std: std.clone(),
            rom_table_id: std.get_virtual_table_id(DMA_ROM_ID).expect("Failed to get dma rom ID"),
            pre_post_table_id: std
                .get_virtual_table_id(DMA_PRE_POST_TABLE_ID)
                .expect("Failed to get table DMA_PRE_POST_TABLE identifier"),
            byte_cmp_table_id: std
                .get_virtual_table_id(DMA_BYTE_CMP_TABLE_ID)
                .expect("Failed to get table DMA_BYTE_CMP_TABLE identifier"),
            dual_range_7_bits_id: std
                .get_virtual_table_id(DUAL_RANGE_7_BITS_ID)
                .expect("Failed to get dual 7-bits table ID"),
            dual_range_byte_id: std
                .get_virtual_table_id(DUAL_RANGE_BYTE_ID)
                .expect("Failed to get table DUAL_RANGE_BYTE identifier"),
            range_22_bits_id: std
                .get_range_id(0, 0x3F_FFFF, None)
                .expect("Failed to get 22b table ID"),
            range_24_bits_id: std
                .get_range_id(0, 0xFF_FFFF, None)
                .expect("Failed to get 24b table ID"),
            range_16_bits_id: std
                .get_range_id(0, 0xFFFF, None)
                .expect("Failed to get 16b table ID"),
        })
    }

    /// Writes one operation into `rows`, which is exactly `input.rows()` long.
    ///
    /// The columns defined in the PIL with `<==` (`has_pre_row`, `pp_*`, `write_value`,
    /// `bus_write_value`, `last_dst_byte`, `l_memcmp_result`, `loop_b0`, `loop_extended_arg`,
    /// `static_count`, `sel_count_from_mem`) are *not* written here: they carry a `witness_calc`
    /// hint and the prover derives them from their expression.
    #[inline(always)]
    fn process_op<R: DmaWithPrePostTraceRowOps<F>>(
        &self,
        input: &DmaWithPrePostInput,
        rows: &mut [R],
        mults: &mut Mults,
    ) {
        debug_assert_eq!(rows.len(), input.rows());

        let use_pre = DmaInfo::get_pre_count(input.encoded) > 0;
        let use_post = DmaInfo::get_post_count(input.encoded) > 0;

        // The DMA row carries the POST when there is one, otherwise the PRE (if any).
        self.fill_dma_row(input, &mut rows[0], mults);
        if use_post || use_pre {
            self.fill_sub_op(input, &mut rows[0], use_post, mults);
        }

        // ...and when both are needed, the PRE goes to the extra row right after it.
        if rows.len() == DmaWithPrePostInput::DOUBLE_ROW {
            debug_assert!(use_pre && use_post);
            self.fill_pre_row(input, &mut rows[1], mults);
            self.fill_sub_op(input, &mut rows[1], false, mults);
        }
    }

    /// Fills the DMA controller columns of the row that drives the operation.
    fn fill_dma_row<R: DmaWithPrePostTraceRowOps<F>>(
        &self,
        input: &DmaWithPrePostInput,
        row: &mut R,
        mults: &mut Mults,
    ) {
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

        // to increase performance because the 99.99% of count is < 64K => h_count < 256
        if h_count < 256 {
            mults.low_24_bits[h_count as usize] += 1;
        } else {
            mults.values_24_bits.push(h_count);
        }

        let use_src = input.op != ZiskOp::DMA_INPUTCPY && input.op != ZiskOp::DMA_XMEMSET;

        // Without a source the src columns are zeroed rather than filled with whatever `b` carried
        // (the count, for inputcpy). `DmaSM` cannot do that: it has to keep them equal to what it
        // publishes on DMA_BUS_ID. Here there is no bus, `src` is multiplied by
        // `sel_memcpy + sel_memcmp` everywhere it is used, and zeroing them keeps
        // @[pp_src_offset] at 0 — which is what the PRE/POST byte rotation of an inputcpy needs.
        let src = if use_src { input.src } else { 0 };

        let h_src64 = src >> 10;
        let h_dst64 = input.dst >> 10;
        let l_src64 = (src >> 3) as u8 & 0x7F;
        let l_dst64 = (input.dst >> 3) as u8 & 0x7F;
        let src_offset = src as u8 & 0x07;

        row.set_h_src64(h_src64);
        row.set_l_src64(l_src64);
        row.set_src_offset(src_offset);
        row.set_h_dst64(h_dst64);
        row.set_l_dst64(l_dst64);
        row.set_dst_offset(input.dst as u8 & 0x07);
        row.set_main_step(input.step);

        mults.values_22_bits.push(h_src64);
        mults.values_22_bits.push(h_dst64);
        mults.dual_7_bits[((l_src64 as usize) << 7) | l_dst64 as usize] += 1;

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

        let mut result_nz = false;
        match input.op {
            ZiskOp::DMA_MEMCPY => row.set_sel_memcpy(true),
            ZiskOp::DMA_XMEMCPY => {
                row.set_sel_memcpy(true);
                row.set_sel_extended(true);
            }
            ZiskOp::DMA_MEMCMP | ZiskOp::DMA_XMEMCMP => {
                row.set_sel_memcmp(true);
                row.set_sel_extended(input.op == ZiskOp::DMA_XMEMCMP);
                // The DMA row publishes the result of its own sub-operation, so `result_nz` is
                // only set when that sub-operation exists — which is what the DMA ROM expects
                // from @[result_nz] and what `fill_sub_op` writes in @[memcmp_result_nz].
                result_nz = DmaInfo::get_memcmp_pre_result_nz(encoded)
                    || DmaInfo::get_memcmp_post_result_nz(encoded);

                let count_diff = input.count_bus - count as u32;
                let count_diff_chunks = [count_diff as u16, (count_diff >> 16) as u16];
                row.set_all_count_diff_chunks(&count_diff_chunks);
                mults.range_16_bits[count_diff_chunks[0] as usize] += 1;
                mults.range_16_bits[count_diff_chunks[1] as usize] += 1;
            }
            ZiskOp::DMA_INPUTCPY => row.set_sel_inputcpy(true),
            ZiskOp::DMA_XMEMSET => {
                row.set_sel_memset(true);
                row.set_sel_extended(true);
                row.set_fill_byte(DmaInfo::get_fill_byte(encoded));
            }
            op => panic!("Invalid DMA operation {op}"),
        }

        let rom_index =
            DmaRom::get_row(input.dst & 0x07, src_offset as u32, count, result_nz, use_src);
        mults.rom[rom_index] += 1;
    }

    /// Fills the extra PRE row: no DMA operation at all, only the columns it shares with the DMA
    /// row of the operation (@[latch] in the PIL) plus the PRE/POST part filled by `fill_sub_op`.
    fn fill_pre_row<R: DmaWithPrePostTraceRowOps<F>>(
        &self,
        input: &DmaWithPrePostInput,
        row: &mut R,
        mults: &mut Mults,
    ) {
        row.set_is_pre_row(true);
        row.set_main_step(input.step);
        match input.op {
            ZiskOp::DMA_MEMCPY | ZiskOp::DMA_XMEMCPY => row.set_sel_memcpy(true),
            ZiskOp::DMA_MEMCMP | ZiskOp::DMA_XMEMCMP => {
                row.set_sel_memcmp(true);
                // The two count_diff range checks are selected by @[sel_memcmp], which is latched
                // here, so they fire on this row too — on the zero this row leaves in the chunks.
                mults.range_16_bits[0] += 2;
            }
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
    fn fill_sub_op<R: DmaWithPrePostTraceRowOps<F>>(
        &self,
        input: &DmaWithPrePostInput,
        row: &mut R,
        is_post: bool,
        mults: &mut Mults,
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
                (dma_src_offset + pre_count) & 0x07,
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
            mults.add_dual_bytes(src_values[0]);
            if second_read {
                rb[8..].copy_from_slice(&src_values[1].to_le_bytes());
                mults.add_dual_bytes(src_values[1]);
            } else {
                // rb[8..16] stay zero, but the range check does not depend on `second_read`.
                mults.dual_byte[0] += 4;
            }
        }
        row.set_all_rb(&rb);

        // Pre-write bytes: the dst word as it was before this row writes it.
        let pb = dst_pre_value.to_le_bytes();
        row.set_all_pb(&pb);
        mults.add_dual_bytes(dst_pre_value);

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

        let table_row = if is_memcmp {
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
                let row_byte_cmp_table = if is_negative {
                    debug_assert!(
                        abs_diff_dst_src <= (255 - last_dst_byte) && abs_diff_dst_src > 0,
                        "abs_diff_dst_src: {abs_diff_dst_src} last_dst_byte: 0x{last_dst_byte:02X} \
                         result: 0x{result:016X} S:{} index:{dst_index} dst_offset:{dst_offset} \
                         src_offset:{src_offset} count:{count} is_post:{is_post}",
                        input.step,
                    );
                    last_dst_byte as usize * 255 + (abs_diff_dst_src + last_dst_byte) as usize - 1
                } else {
                    debug_assert!(
                        abs_diff_dst_src <= last_dst_byte && abs_diff_dst_src > 0,
                        "abs_diff_dst_src: {abs_diff_dst_src} last_dst_byte: 0x{last_dst_byte:02X} \
                         result: 0x{result:016X} S:{} index:{dst_index} dst_offset:{dst_offset} \
                         src_offset:{src_offset} count:{count} is_post:{is_post}",
                        input.step,
                    );
                    last_dst_byte as usize * 255 + (last_dst_byte - abs_diff_dst_src) as usize
                };
                mults.byte_cmp[row_byte_cmp_table] += 1;
            }
            DmaPrePostRom::get_row(dst_offset, src_offset, count, is_nz, is_negative, true)
        } else {
            DmaPrePostRom::get_row(dst_offset, src_offset, count, false, false, load_src)
        };
        mults.pre_post[table_row] += 1;
    }

    fn compute_witness_inner<R: DmaWithPrePostTraceRowOps<F> + Copy + Send>(
        &self,
        inputs: &[Vec<DmaWithPrePostInput>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = DmaWithPrePostTrace::<R>::new_from_vec_zeroes(trace_buffer)?;
        let num_rows = trace.num_rows();

        let flat_inputs: Vec<&DmaWithPrePostInput> = inputs.iter().flatten().collect();
        let total_rows: usize = flat_inputs.iter().map(|input| input.rows()).sum();

        // The planner reserves the rows of every operation as a block (see
        // `DmaWithPrePostInstancesBuilder`), so this can only fire if the plan and the collected
        // inputs disagree.
        assert!(
            total_rows <= num_rows,
            "DmaWithPrePost: {} operations need {total_rows} rows, only {num_rows} available",
            flat_inputs.len()
        );

        dma_trace("DmaWithPrePost", total_rows, num_rows);

        timer_start_trace!(DMA_WITH_PRE_POST_TRACE);

        // Split the inputs into groups and cut the trace at the row each group starts on. An
        // operation is never split, so a group always owns whole operations and its rows are
        // contiguous — which is what keeps a PRE row next to its DMA row.
        let num_threads = rayon::current_num_threads();
        let group_len = flat_inputs.len().div_ceil(num_threads).max(1);

        let mut groups: Vec<(&[&DmaWithPrePostInput], &mut [R])> = Vec::new();
        let mut pending_inputs: &[&DmaWithPrePostInput] = &flat_inputs;
        let mut pending_rows: &mut [R] = trace.buffer.as_mut_slice();
        while !pending_inputs.is_empty() {
            let take = group_len.min(pending_inputs.len());
            let rows: usize = pending_inputs[..take].iter().map(|input| input.rows()).sum();
            let (group_inputs, rest_inputs) = pending_inputs.split_at(take);
            let (group_rows, rest_rows) = pending_rows.split_at_mut(rows);
            groups.push((group_inputs, group_rows));
            pending_inputs = rest_inputs;
            pending_rows = rest_rows;
        }

        let mults = groups
            .into_par_iter()
            .map(|(group_inputs, group_rows)| {
                let mut mults = Mults::new();
                let mut cursor = 0usize;
                for input in group_inputs {
                    let rows = input.rows();
                    self.process_op(input, &mut group_rows[cursor..cursor + rows], &mut mults);
                    cursor += rows;
                }
                mults
            })
            .reduce(Mults::new, Mults::merge);

        // The padding rows are left as `new_from_vec_zeroes` made them: with every selector at
        // zero the air asks nothing of them, and no lookup is charged for them.

        self.std.inc_virtual_rows_ranged(self.dual_range_7_bits_id, None, &mults.dual_7_bits);
        self.std.inc_virtual_rows_ranged(self.rom_table_id, None, &mults.rom);
        self.std.inc_virtual_rows_ranged(self.pre_post_table_id, None, &mults.pre_post);
        self.std.inc_virtual_rows_ranged(self.byte_cmp_table_id, None, &mults.byte_cmp);
        self.std.inc_virtual_rows_ranged(self.dual_range_byte_id, None, &mults.dual_byte);
        self.std.range_check_ranged(self.range_24_bits_id, None, &mults.low_24_bits);
        self.std.range_check_ranged(self.range_16_bits_id, None, &mults.range_16_bits);
        for value in mults.values_22_bits {
            self.std.range_check_one(self.range_22_bits_id, value);
        }
        for value in mults.values_24_bits {
            self.std.range_check_one(self.range_24_bits_id, value);
        }

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
