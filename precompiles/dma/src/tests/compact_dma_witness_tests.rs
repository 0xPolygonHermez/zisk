//! The `CompactDma` fill: two fills over the same rows.
//!
//! What makes the fused air sound is that each block is exactly the air it stands in for (see
//! `compact_dma.pil`), so what these pin is the witness side of the same claim: each block of a
//! fused row comes out exactly as the standalone air writes its own row, whatever the other block
//! holds, whichever fill runs first, and with the same multiplicities.

use super::*;
use crate::{loop_snapshot, wpp_snapshot, DmaLoopRowState, DmaLoopSegmentValues, Mults};
use proofman_fields::Goldilocks;
use zisk_common::{SegmentId, A, B, DMA_ENCODED, OP, STEP};
use zisk_core::zisk_ops::ZiskOp;
use zisk_pil::{DmaLoopTraceRow, DmaWithPrePostTraceRow};
use zisk_precomp_helpers::DmaInfo;

type F = Goldilocks;
type Row = CompactDmaTraceRow<F>;
type WppRow = DmaWithPrePostTraceRow<F>;
type LoopRow = DmaLoopTraceRow<F>;

/// Rows of the instances below: more than either block needs, so both end on padding.
const N: usize = 32;

/// The controller's view of one operation. The memory values only have to be bytes.
fn wpp_input(op: u8, dst: u32, src: u32, encoded: u64, step: u64) -> DmaWithPrePostInput {
    DmaWithPrePostInput {
        dst,
        src,
        step,
        encoded,
        count_bus: DmaInfo::get_count(encoded) as u32,
        op,
        pre_src_values: [0x0123_4567_89AB_CDEF, 0xFEDC_BA98_7654_3210],
        post_src_values: [0x1122_3344_5566_7788, 0x99AA_BBCC_DDEE_FF00],
        pre_dst_value: 0xA5A5_A5A5_A5A5_A5A5,
        post_dst_value: 0x5A5A_5A5A_5A5A_5A5A,
    }
}

fn wpp_inputs() -> Vec<DmaWithPrePostInput> {
    vec![
        // PRE and POST: two rows.
        wpp_input(
            ZiskOp::DMA_XMEMCPY,
            0x8003,
            0x9005,
            DmaInfo::encode_memcpy(0x8003, 0x9005, 30),
            1,
        ),
        wpp_input(ZiskOp::DMA_XMEMSET, 0x8101, 0, DmaInfo::encode_memset(0x8101, 13, 0x3C), 2),
        wpp_input(ZiskOp::DMA_INPUTCPY, 0x8200, 21, DmaInfo::encode_inputcpy(0x8200, 21), 3),
        wpp_input(
            ZiskOp::DMA_XMEMCMP,
            0x8306,
            0x9306,
            DmaInfo::encode_memcmp(0x8306, 0x9306, 9, 0),
            4,
        ),
        wpp_input(
            ZiskOp::DMA_XMEMCPY,
            0x8404,
            0x9401,
            DmaInfo::encode_memcpy(0x8404, 0x9401, 45),
            5,
        ),
    ]
}

/// The loop of one operation, whole, from a bus payload whose loop words are `0x11 * (k + 1)`.
fn loop_input(op: u8, dst: u64, src: u64, encoded: u64, step: u64) -> DmaLoopInput {
    let mut data = vec![0u64; DMA_ENCODED + 2];
    data[OP] = op as u64;
    data[A] = dst;
    data[B] = src;
    data[STEP] = step;
    data[DMA_ENCODED] = encoded;
    let class = DmaLoopInput::class_of(op, encoded).unwrap();
    let mut ext = vec![0; DmaInfo::get_loop_data_offset(encoded)];
    ext.extend(
        (0..DmaLoopInput::total_slots(class, encoded))
            .map(|k| 0x1111_1111_1111_1111 * (k as u64 + 1)),
    );
    DmaLoopInput::from(&data, &ext, 0, usize::MAX)
}

fn loop_inputs() -> Vec<DmaLoopInput> {
    vec![
        loop_input(
            ZiskOp::DMA_XMEMCPY,
            0x8003,
            0x9005,
            DmaInfo::encode_memcpy(0x8003, 0x9005, 60),
            1,
        ),
        loop_input(
            ZiskOp::DMA_XMEMCPY,
            0xA000,
            0xB000,
            DmaInfo::encode_memcpy(0xA000, 0xB000, 40),
            2,
        ),
        loop_input(ZiskOp::DMA_XMEMSET, 0x8101, 0, DmaInfo::encode_memset(0x8101, 50, 0x3C), 3),
        loop_input(ZiskOp::DMA_INPUTCPY, 0x8200, 0, DmaInfo::encode_inputcpy(0x8200, 64), 4),
    ]
}

/// What a fill of a whole instance produces, on the fused rows or on the standalone ones.
struct Filled<R> {
    rows: Vec<R>,
    mults: Option<Mults>,
    loop_values: Option<DmaLoopSegmentValues>,
    loop_table: Option<Vec<u64>>,
}

/// Fills a zeroed `CompactDma` instance, the way `compute_witness` does, running the loop fill
/// first when `loop_first`.
fn fill_compact(
    wpp: &[&DmaWithPrePostInput],
    lp: &[&DmaLoopInput],
    loop_first: bool,
) -> Filled<Row> {
    let mut rows = vec![Row::default(); N];
    let (mults, loop_fill) = if loop_first {
        let loop_fill = DmaLoopSM::<F>::fill_rows(lp, &mut rows, SegmentId(2), true);
        (DmaWithPrePostSM::<F>::fill_rows(wpp, &mut rows), loop_fill)
    } else {
        let mults = DmaWithPrePostSM::<F>::fill_rows(wpp, &mut rows);
        (mults, DmaLoopSM::<F>::fill_rows(lp, &mut rows, SegmentId(2), true))
    };
    Filled {
        rows,
        mults: Some(mults),
        loop_values: Some(loop_fill.values),
        loop_table: Some(loop_fill.dual_byte_table),
    }
}

fn fill_wpp(wpp: &[&DmaWithPrePostInput]) -> Filled<WppRow> {
    let mut rows = vec![WppRow::default(); N];
    let mults = DmaWithPrePostSM::<F>::fill_rows(wpp, &mut rows);
    Filled { rows, mults: Some(mults), loop_values: None, loop_table: None }
}

fn fill_loop(lp: &[&DmaLoopInput]) -> Filled<LoopRow> {
    let mut rows = vec![LoopRow::default(); N];
    let fill = DmaLoopSM::<F>::fill_rows(lp, &mut rows, SegmentId(2), true);
    Filled {
        rows,
        mults: None,
        loop_values: Some(fill.values),
        loop_table: Some(fill.dual_byte_table),
    }
}

/// Each block of `compact` is what the standalone airs wrote, row by row, with the same
/// multiplicities and air values.
fn assert_blocks_match(compact: &Filled<Row>, wpp: &Filled<WppRow>, lp: &Filled<LoopRow>) {
    for i in 0..N {
        assert_eq!(
            wpp_snapshot::<F, _>(&compact.rows[i]),
            wpp_snapshot::<F, _>(&wpp.rows[i]),
            "row {i}: the wpp_ block is not the DmaWithPrePost row"
        );
        assert_eq!(
            loop_snapshot::<F, _>(&compact.rows[i]),
            loop_snapshot::<F, _>(&lp.rows[i]),
            "row {i}: the loop_ block is not the DmaLoop row"
        );
    }
    assert!(compact.mults == wpp.mults, "the wpp_ multiplicities differ");
    assert_eq!(compact.loop_values, lp.loop_values, "the loop_ air values differ");
    assert!(compact.loop_table == lp.loop_table, "the loop_ multiplicities differ");
}

fn refs<T>(values: &[T]) -> Vec<&T> {
    values.iter().collect()
}

/// Both blocks with work, of different heights, so each ends on its own padding.
#[test]
fn each_block_is_its_standalone_air() {
    let (wpp, lp) = (wpp_inputs(), loop_inputs());
    let (wpp, lp) = (refs(&wpp), refs(&lp));
    let wpp_rows: usize = wpp.iter().map(|input| input.rows()).sum();
    let loop_rows: usize = lp.iter().map(|input| input.rows as usize).sum();
    assert_ne!(wpp_rows, loop_rows, "the blocks should end on different rows");
    assert!(wpp_rows.max(loop_rows) < N);

    let compact = fill_compact(&wpp, &lp, false);
    assert_blocks_match(&compact, &fill_wpp(&wpp), &fill_loop(&lp));
}

/// The two fills write disjoint columns, so the order they run in does not show.
#[test]
fn the_order_of_the_fills_does_not_matter() {
    let (wpp, lp) = (wpp_inputs(), loop_inputs());
    let (wpp, lp) = (refs(&wpp), refs(&lp));
    let wpp_first = fill_compact(&wpp, &lp, false);
    let loop_first = fill_compact(&wpp, &lp, true);
    for i in 0..N {
        assert_eq!(
            wpp_snapshot::<F, _>(&wpp_first.rows[i]),
            wpp_snapshot::<F, _>(&loop_first.rows[i])
        );
        assert_eq!(
            loop_snapshot::<F, _>(&wpp_first.rows[i]),
            loop_snapshot::<F, _>(&loop_first.rows[i])
        );
    }
    assert!(wpp_first.mults == loop_first.mults);
    assert_eq!(wpp_first.loop_values, loop_first.loop_values);
}

/// A block with nothing to prove is its air's padding: an all-zero controller block, and a loop
/// block of padding rows that receives and hands over nothing -- still a valid segment of the
/// loop chain, which runs through every instance of the air.
#[test]
fn a_block_with_no_work_is_padding() {
    let (wpp, lp) = (wpp_inputs(), loop_inputs());
    let (wpp, lp) = (refs(&wpp), refs(&lp));

    let only_wpp = fill_compact(&wpp, &[], false);
    assert_blocks_match(&only_wpp, &fill_wpp(&wpp), &fill_loop(&[]));
    let values = only_wpp.loop_values.unwrap();
    assert_eq!(values.padding_size, N);
    assert_eq!((values.previous, values.last), (DmaLoopRowState::ENDED, DmaLoopRowState::ENDED));

    let only_loop = fill_compact(&[], &lp, false);
    assert_blocks_match(&only_loop, &fill_wpp(&[]), &fill_loop(&lp));
    let zero = wpp_snapshot::<F, _>(&WppRow::default());
    assert!(only_loop.rows.iter().all(|row| wpp_snapshot::<F, _>(row) == zero));
}
