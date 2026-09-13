//! What these cover is the seam between the columns the state machine writes and the ones the air
//! derives from them.
//!
//! `pp_dst_offset`, `pp_src_offset` and `pp_count` are `<==` columns: the SM leaves them alone and
//! the prover fills them from their expression. So whenever the SM computes something *else* from
//! the sub-operation offsets — the byte rotation in `selr`, and the `DMA_PRE_POST_TABLE` row it
//! charges — it has to start from exactly the offsets that expression will produce. When the two
//! drift apart the air looks the table up with one offset while the SM charged the row of another,
//! and the lookup is left unmatched on both sides.

use super::*;
use proofman_fields::Goldilocks;

type Row = DmaWithPrePostTraceRow<Goldilocks>;
type Sm = DmaWithPrePostSM<Goldilocks>;

/// Builds the input of one operation. The memory values are arbitrary: what these tests look at is
/// the offsets, and every byte of a 64-bit word is in range whatever it holds.
fn input(op: u8, dst: u32, src: u32, encoded: u64) -> DmaWithPrePostInput {
    DmaWithPrePostInput {
        dst,
        src,
        step: 1,
        encoded,
        // memcmp subtracts `count` from this, so the two have to agree when nothing was skipped.
        count_bus: DmaInfo::get_count(encoded) as u32,
        op,
        pre_src_values: [0x0123_4567_89AB_CDEF, 0xFEDC_BA98_7654_3210],
        post_src_values: [0x1122_3344_5566_7788, 0x99AA_BBCC_DDEE_FF00],
        pre_dst_value: 0xA5A5_A5A5_A5A5_A5A5,
        post_dst_value: 0x5A5A_5A5A_5A5A_5A5A,
    }
}

/// Writes one operation into a fresh pair of rows and hands back the ones it used.
fn rows_of(input: &DmaWithPrePostInput) -> Vec<Row> {
    let mut rows = vec![Row::default(); DmaWithPrePostInput::DOUBLE_ROW];
    let used = input.rows();
    Sm::process_op(input, &mut rows[..used], &mut Mults::new());
    rows.truncate(used);
    rows
}

/// `@[pp_sel]`: the row executes a PRE or a POST.
fn has_sub_op(row: &Row) -> bool {
    row.get_is_pre_row() || row.get_use_pre() || row.get_use_post()
}

/// `@[pp_dst_offset]` and `@[pp_src_offset]`, as the air derives them from the columns the SM
/// wrote — see the `<==` definitions in `dma_with_pre_post.pil`.
fn pp_offsets(rows: &[Row], i: usize) -> (u8, u8) {
    let row = &rows[i];
    if row.get_is_pre_row() {
        // The PRE row takes them from the DMA row before it: `is_pre_row * ('X - X)`.
        let dma_row = &rows[i - 1];
        (dma_row.get_dst_offset(), dma_row.get_src_offset())
    } else if row.get_use_post() {
        // A POST starts on a 64-bit boundary, and walks the source by `src_offset_after_pre`.
        (0, row.get_src_offset_after_pre())
    } else {
        (row.get_dst_offset(), row.get_src_offset())
    }
}

/// The rotation the SM wrote: the index `selr` selects, and the direction flag beside it.
/// `selr` has seven entries, so a rotation of 7 leaves all of them clear.
fn rotation(row: &Row) -> (u8, bool) {
    let selr = row.get_all_selr();
    let value = selr.iter().position(|&selected| selected).unwrap_or(7);
    (value as u8, row.get_dst_offset_gt_src_offset())
}

/// Every operation kind, at every alignment, with counts that leave a PRE, a POST, both or
/// neither.
fn every_operation() -> Vec<DmaWithPrePostInput> {
    const BASE: u64 = 0x8000;
    const COUNTS: [usize; 8] = [1, 3, 4, 7, 8, 12, 19, 26];
    let mut inputs = Vec::new();
    for dst_offset in 0..8u64 {
        let dst = BASE + dst_offset;
        for count in COUNTS {
            inputs.push(input(
                ZiskOp::DMA_INPUTCPY,
                dst as u32,
                count as u32,
                DmaInfo::encode_inputcpy(dst, count),
            ));
            inputs.push(input(
                ZiskOp::DMA_XMEMSET,
                dst as u32,
                0,
                DmaInfo::encode_memset(dst, count, 0x5A),
            ));
            for src_offset in 0..8u64 {
                let src = BASE + 0x1000 + src_offset;
                for op in [ZiskOp::DMA_MEMCPY, ZiskOp::DMA_XMEMCPY] {
                    inputs.push(input(
                        op,
                        dst as u32,
                        src as u32,
                        DmaInfo::encode_memcpy(dst, src, count),
                    ));
                }
                inputs.push(input(
                    ZiskOp::DMA_MEMCMP,
                    dst as u32,
                    src as u32,
                    DmaInfo::encode_memcmp(dst, src, count, 0),
                ));
            }
        }
    }
    inputs
}

/// The regression this file exists for: an inputcpy or a memset has no source, so the PRE never
/// walks one and the POST starts at offset 0 — which is what `src_offset_after_pre` holds and
/// therefore what `pp_src_offset` derives. Adding `pre_count` to it anyway left the rotation
/// claiming a source offset the air never looks the table up with.
#[test]
fn a_post_without_a_source_starts_at_offset_zero() {
    // dst_offset 4 needs 4 PRE bytes, and 4 more bytes past the loop need a POST: the case that
    // charged the table row of src_offset 4 while the air asked for src_offset 0.
    for op in [ZiskOp::DMA_INPUTCPY, ZiskOp::DMA_XMEMSET] {
        let dst = 0x8004u64;
        let count = 4 + 8 + 4;
        let encoded = match op {
            ZiskOp::DMA_INPUTCPY => DmaInfo::encode_inputcpy(dst, count),
            _ => DmaInfo::encode_memset(dst, count, 0x5A),
        };
        assert_eq!(DmaInfo::get_pre_count(encoded), 4, "op 0x{op:02X}: pre count");
        assert_eq!(DmaInfo::get_post_count(encoded), 4, "op 0x{op:02X}: post count");

        let rows = rows_of(&input(op, dst as u32, 0, encoded));
        // The DMA row carries the POST, the extra row the PRE.
        assert_eq!(rows.len(), DmaWithPrePostInput::DOUBLE_ROW);
        assert_eq!(rows[0].get_src_offset_after_pre(), 0, "op 0x{op:02X}: no source to walk");
        assert_eq!(rotation(&rows[0]), (0, false), "op 0x{op:02X}: the POST must not rotate");
    }
}

/// And the invariant behind it, over every kind and alignment: the rotation the SM writes is the
/// one the offsets the air derives call for.
#[test]
fn the_rotation_matches_the_offsets_the_air_derives() {
    for input in every_operation() {
        let rows = rows_of(&input);
        for i in 0..rows.len() {
            if !has_sub_op(&rows[i]) {
                continue;
            }
            let (dst_offset, src_offset) = pp_offsets(&rows, i);
            let case = format!(
                "op 0x{:02X} dst 0x{:X} src 0x{:X} encoded 0x{:016X} row {i}",
                input.op, input.dst, input.src, input.encoded
            );
            assert_eq!(
                rotation(&rows[i]),
                (dst_offset.abs_diff(src_offset), dst_offset > src_offset),
                "{case}: rotation vs pp offsets ({dst_offset}, {src_offset})"
            );
        }
    }
}

/// The second read is decided by the same offsets, so it has to follow them too: a sub-operation
/// needs the next source word exactly when its bytes run past the one it started in.
#[test]
fn the_second_read_follows_the_same_offsets() {
    for input in every_operation() {
        let rows = rows_of(&input);
        for i in 0..rows.len() {
            if !has_sub_op(&rows[i]) {
                continue;
            }
            let (_, src_offset) = pp_offsets(&rows, i);
            let count = if rows[i].get_is_pre_row() || !rows[i].get_use_post() {
                DmaInfo::get_pre_count(input.encoded)
            } else {
                DmaInfo::get_post_count(input.encoded)
            };
            assert_eq!(
                rows[i].get_enabled_second_read(),
                src_offset as usize + count > 8,
                "op 0x{:02X} encoded 0x{:016X} row {i}: second read vs src offset {src_offset} \
                 and count {count}",
                input.op,
                input.encoded,
            );
        }
    }
}
