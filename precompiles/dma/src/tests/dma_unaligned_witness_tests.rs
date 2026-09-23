//! The `DmaUnaligned` fill against the constraints of `dma_unaligned.pil`, on a handful of rows.
//!
//! The fill runs in parallel groups of inputs and no longer zeroes the trace, so what these pin is
//! that the rows come out exactly as the sequential, zeroed fill wrote them: the per-row sequence
//! bookkeeping (`count`, `seq_end`, `sel_op`), the bytes each lane reads, the padding rows, the
//! dual-byte multiplicities and the hand-over the last row leaves for the continuation.

use super::*;
use proofman_fields::Goldilocks;
use zisk_pil::DmaUnalignedTraceRow;

type Row = DmaUnalignedTraceRow<Goldilocks>;
type Sm = DmaUnalignedSM<Goldilocks>;

const W: usize = DMA_UNALIGNED_OPS_BY_ROW;

/// Poison every row so a column the fill forgets shows up instead of reading as a lucky zero.
fn poisoned_rows(n: usize) -> Vec<Row> {
    let mut row = Row::default();
    row.set_count(0xDEAD_BEEF);
    row.set_seq_end(true);
    row.set_all_sel_op_from_1(&[true; W - 1]);
    row.set_all_read_bytes(&[[0xEE; 8]; W]);
    row.set_dst64(0xDEAD);
    row.set_src64(0xBEEF);
    row.set_main_step(0xDEAD_BEEF);
    row.set_offset_5(true);
    vec![row; n]
}

/// An unaligned memcpy of `words` 64-bit words from `src` (offset 3) to the aligned `dst`, as the
/// collector hands it over: `skip` rows already proved by a previous instance, `rows` rows granted
/// here, and the source words those rows read.
fn memcpy(dst: u32, src: u32, words: usize, skip: usize, rows: usize) -> DmaUnalignedInput {
    let encoded = DmaInfo::encode_memcpy(dst as u64, src as u64, words * 8);
    assert_eq!(DmaInfo::get_pre_count(encoded), 0, "the test wants an aligned dst");
    assert_eq!(DmaInfo::get_loop_count(encoded), words);
    assert_eq!(DmaInfo::get_loop_src_offset(encoded), 3);

    // `DmaUnalignedInput::from`, without the bus payload: the slots of the granted rows, plus the
    // extra source word the last write borrows from when the sequence goes on.
    let pending_slots = words + 1 - skip * W;
    let slots = pending_slots.min(rows * W);
    let src_values_count = if slots < pending_slots { slots + 1 } else { slots };
    let first_word = skip * W;
    let src_values = (first_word..first_word + src_values_count)
        .map(|i| 0x0101_0101_0101_0101u64 * (i as u64 + 1))
        .collect();

    DmaUnalignedInput {
        src,
        dst,
        is_last_instance_input: slots < pending_slots,
        is_mem_eq: false,
        trace_offset: 0,
        skip: skip as u32,
        count: rows as u32,
        step: 77,
        encoded,
        src_values,
    }
}

/// `@[ops_selected]`: lane 0 plus the selected ones above it.
fn ops_selected(row: &Row) -> usize {
    1 + row.get_all_sel_op_from_1().iter().filter(|&&s| s).count()
}

/// The two constraints of `dma_unaligned.pil` that tie the row's shape to its count, plus the
/// prefix property of the lane selectors.
fn check_row_shape(row: &Row) {
    let sel = row.get_all_sel_op_from_1();
    for i in 1..sel.len() {
        assert!(!sel[i] || sel[i - 1], "sel_op must be a prefix: {sel:?}");
    }
    let ops = ops_selected(row) as u32;
    if row.get_seq_end() {
        // seq_end * (count - (ops_selected - 1)) === 0
        assert_eq!(row.get_count(), ops - 1);
    } else {
        // (1 - seq_end) * (ops_selected - op_x_row) === 0
        assert_eq!(ops as usize, W);
    }
}

fn read_word(row: &Row, lane: usize) -> u64 {
    let mut bytes = [0u8; 8];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = row.get_read_bytes(lane, i);
    }
    u64::from_le_bytes(bytes)
}

fn total_mults(table: &[u64]) -> u64 {
    table.iter().sum()
}

/// The committed columns of a row, as one comparable value: the generated row types implement
/// neither `PartialEq` nor a column iterator.
type Committed = (u32, bool, [bool; W - 1], [[u8; 8]; W], u32, u32, u64, [bool; 6], bool);

fn committed(row: &Row) -> Committed {
    let mut read_bytes = [[0u8; 8]; W];
    for (lane, bytes) in read_bytes.iter_mut().enumerate() {
        *bytes = read_word(row, lane).to_le_bytes();
    }
    (
        row.get_count(),
        row.get_seq_end(),
        row.get_all_sel_op_from_1(),
        read_bytes,
        row.get_dst64(),
        row.get_src64(),
        row.get_main_step(),
        [
            row.get_offset_2(),
            row.get_offset_3(),
            row.get_offset_4(),
            row.get_offset_5(),
            row.get_offset_6(),
            row.get_offset_7(),
        ],
        row.get_is_memeq(),
    )
}

/// One sequence that ends inside the instance, followed by padding.
#[test]
fn a_sequence_walks_its_words_and_the_rest_is_padding() {
    // 7 words: 8 slots, so 2 rows -- the second one ends the sequence with all four lanes busy,
    // the last of them being the extra read that writes nothing.
    let input = memcpy(0x8000, 0x9003, 7, 0, 2);
    let mut rows = poisoned_rows(5);
    let fill = Sm::fill_rows(&[&input], &mut rows);

    assert_eq!(fill.padding_size, 3);
    assert!(fill.last.seq_end);

    // Row 0: the four first words, the sequence goes on.
    assert_eq!(rows[0].get_count(), 7);
    assert!(!rows[0].get_seq_end());
    assert_eq!(rows[0].get_dst64(), 0x8000 >> 3);
    assert_eq!(rows[0].get_src64(), 0x9003 >> 3);
    assert_eq!(rows[0].get_main_step(), 77);
    assert!(!rows[0].get_is_memeq());
    // src offset 3: offset_3 is the only offset flag set.
    assert!(rows[0].get_offset_3());
    assert!(!rows[0].get_offset_2() && !rows[0].get_offset_5() && !rows[0].get_offset_7());
    for lane in 0..W {
        assert_eq!(read_word(&rows[0], lane), input.src_values[lane]);
    }

    // Row 1: count 3 => seq_end with ops_selected 4; lanes read words 4..7.
    assert_eq!(rows[1].get_count(), 3);
    assert!(rows[1].get_seq_end());
    assert_eq!(rows[1].get_dst64(), (0x8000 >> 3) + W as u32);
    assert_eq!(rows[1].get_src64(), (0x9003 >> 3) + W as u32);
    for lane in 0..W {
        assert_eq!(read_word(&rows[1], lane), input.src_values[W + lane]);
    }

    for row in &rows[..2] {
        check_row_shape(row);
    }

    // Padding: every column zero but seq_end, whatever the buffer held before.
    let padding = Sm::padding_row::<Row>();
    for row in &rows[2..] {
        assert_eq!(committed(row), committed(&padding));
        assert!(row.get_seq_end());
        assert_eq!(row.get_count(), 0);
        assert_eq!(ops_selected(row), 1);
        check_row_shape(row);
    }

    // Every lane of every row is range-checked in 4 dual-byte pairs, padding included.
    assert_eq!(total_mults(&fill.dual_byte_table), (rows.len() * W * 4) as u64);
    // The 8 words read: 32 pairs off the zero bucket, the rest (unused lanes) on it.
    let nonzero: u64 = fill.dual_byte_table[1..].iter().sum();
    assert_eq!(nonzero, 8 * 4);
}

/// Two sequences in one instance: the first ends mid-row, the second is cut by the end of the
/// instance and hands its state over to the next one.
#[test]
fn a_cut_sequence_hands_its_last_row_over() {
    // 5 words: 6 slots, 2 rows; the second row has count 1 => ops_selected 2.
    let first = memcpy(0x8000, 0x9003, 5, 0, 2);
    // 13 words: 14 slots, 4 rows needed, only 2 granted -- the instance ends without seq_end.
    let second = memcpy(0xA000, 0xB003, 13, 0, 2);
    assert!(second.is_last_instance_input);
    assert_eq!(second.src_values.len(), 2 * W + 1, "the extra word the last write borrows");

    let mut rows = poisoned_rows(4);
    let fill = Sm::fill_rows(&[&first, &second], &mut rows);

    assert_eq!(fill.padding_size, 0);
    for row in &rows {
        check_row_shape(row);
    }

    // First sequence.
    assert_eq!(rows[0].get_count(), 5);
    assert!(!rows[0].get_seq_end());
    assert_eq!(rows[1].get_count(), 1);
    assert!(rows[1].get_seq_end());
    assert_eq!(ops_selected(&rows[1]), 2);
    assert_eq!(read_word(&rows[1], 0), first.src_values[4]);
    assert_eq!(read_word(&rows[1], 1), first.src_values[5]);
    // Unselected lanes read zeros.
    assert_eq!(read_word(&rows[1], 2), 0);
    assert_eq!(read_word(&rows[1], 3), 0);

    // Second sequence: two full rows, none of them ending it.
    assert_eq!(rows[2].get_count(), 13);
    assert_eq!(rows[3].get_count(), 9);
    assert!(!rows[2].get_seq_end() && !rows[3].get_seq_end());
    assert_eq!(rows[2].get_dst64(), 0xA000 >> 3);
    assert_eq!(rows[3].get_dst64(), (0xA000 >> 3) + W as u32);
    for lane in 0..W {
        assert_eq!(read_word(&rows[2], lane), second.src_values[lane]);
        assert_eq!(read_word(&rows[3], lane), second.src_values[W + lane]);
    }

    // The hand-over is the last row plus the word its last write borrows from the next instance.
    let last = &fill.last;
    assert!(!last.seq_end);
    assert_eq!(last.count, 9);
    assert_eq!(last.dst64, rows[3].get_dst64());
    assert_eq!(last.src64, rows[3].get_src64());
    assert_eq!(last.main_step, 77);
    assert_eq!(last.offset, 3);
    assert!(!last.is_memeq);
    assert_eq!(last.next_value, second.src_values[2 * W]);

    assert_eq!(total_mults(&fill.dual_byte_table), (rows.len() * W * 4) as u64);
}

/// A sequence that started in a previous instance: the rows resume where that one stopped.
#[test]
fn a_continued_sequence_resumes_a_row_behind() {
    // 13 words with one row already proved: 10 slots pending, 3 rows, ending with count 1.
    let input = memcpy(0xA000, 0xB003, 13, 1, 3);
    assert!(!input.is_last_instance_input);
    assert_eq!(input.src_values.len(), 10);

    let mut rows = poisoned_rows(3);
    let fill = Sm::fill_rows(&[&input], &mut rows);

    assert_eq!(fill.padding_size, 0);
    assert!(fill.last.seq_end);
    for row in &rows {
        check_row_shape(row);
    }
    assert_eq!(rows[0].get_count(), 9);
    assert_eq!(rows[0].get_dst64(), (0xA000 >> 3) + W as u32);
    assert_eq!(rows[0].get_src64(), (0xB003 >> 3) + W as u32);
    assert_eq!(rows[1].get_count(), 5);
    assert_eq!(rows[2].get_count(), 1);
    assert!(rows[2].get_seq_end());
    assert_eq!(ops_selected(&rows[2]), 2);
    // Words 4..13 of the source, in lane order.
    for (slot, &value) in input.src_values.iter().enumerate() {
        assert_eq!(read_word(&rows[slot / W], slot % W), value);
    }
}

/// The grouping is only a work split: however many groups the pool opens, the rows are the same.
#[test]
fn the_groups_of_the_parallel_fill_do_not_show_in_the_rows() {
    let inputs: Vec<DmaUnalignedInput> = (0..9u32)
        .map(|i| memcpy(0x8000 + i * 0x100, 0x9003 + i * 0x100, 3 + i as usize, 0, 0))
        .collect();
    // `rows` was left at 0 above so the constructor could compute the slots; grant every input
    // exactly what it needs.
    let inputs: Vec<DmaUnalignedInput> = inputs
        .into_iter()
        .map(|input| {
            let words = DmaInfo::get_loop_count(input.encoded);
            memcpy(input.dst, input.src, words, 0, (words + 1).div_ceil(W))
        })
        .collect();
    let refs: Vec<&DmaUnalignedInput> = inputs.iter().collect();
    let total_rows: usize = inputs.iter().map(|i| i.count as usize).sum();

    let mut sequential = poisoned_rows(total_rows + 2);
    let seq_fill = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap()
        .install(|| Sm::fill_rows(&refs, &mut sequential));

    let mut parallel = poisoned_rows(total_rows + 2);
    let par_fill = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| Sm::fill_rows(&refs, &mut parallel));

    let sequential_columns: Vec<Committed> = sequential.iter().map(committed).collect();
    let parallel_columns: Vec<Committed> = parallel.iter().map(committed).collect();
    assert_eq!(sequential_columns, parallel_columns);
    assert_eq!(seq_fill.dual_byte_table, par_fill.dual_byte_table);
    assert_eq!(seq_fill.padding_size, par_fill.padding_size);
    for row in &sequential[..total_rows] {
        check_row_shape(row);
    }
}
