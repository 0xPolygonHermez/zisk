//! The `DmaLoop` fill against `dma_loop.pil`.
//!
//! The fill writes the committed columns and leaves the `<==` ones to the prover, so the checker
//! below derives those from their PIL expression, as the prover does, and then evaluates every
//! constraint of the air on every row, the hand-over to the neighbouring segments included. On top
//! of that it replays what the rows put on the buses -- the memory they read and write, and the
//! operation each sequence proves -- and compares it with what the operations on the bus asked
//! for. Rows that satisfy the air but copy the wrong bytes, or prove an operation the controller
//! never assumed, fail as well.

use super::*;
use crate::{
    flatten_and_reorder_inputs, loop_snapshot, DmaLoopCollector, DmaLoopInstancesBuilder,
    DMA_LOOP_CLASSES,
};
use proofman_fields::Goldilocks;
use zisk_common::{ChunkId, A, B, DMA_ENCODED, OP, OPERATION_BUS_ID, OP_TYPE, STEP};
use zisk_core::ZiskOperationType;

type Row = DmaLoopTraceRow<Goldilocks>;
type Sm = DmaLoopSM<Goldilocks>;

/// The opcodes of `pil/operations.pil` the loop proves.
const OP_DMA_MEMCPY: u64 = 0xD0;
const OP_DMA_INPUTCPY: u64 = 0xD2;
const OP_DMA_XMEMCPY: u64 = 0xD6;
const OP_DMA_XMEMSET: u64 = 0xD9;
const OP_DMA_XMEMEQ: u64 = 0xDA;

// ───────────────────────────────────────────────────────────────────────── the operations

/// The test memory: a fixed function of the address, with every word distinct, so a read one
/// byte or one word off shows.
fn mem_word(addr64: u64) -> u64 {
    let x = addr64.wrapping_add(1).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x ^ (x >> 29)
}

/// The eight bytes at `addr`, aligned or not, as a little-endian word.
fn mem_bytes_at(addr: u64) -> u64 {
    u64::from_le_bytes(std::array::from_fn(|i| {
        let byte = addr + i as u64;
        mem_word(byte >> 3).to_le_bytes()[(byte & 7) as usize]
    }))
}

/// The word `k` an input copy of the operation at `step` writes.
fn input_word(step: u64, k: usize) -> u64 {
    (step << 40) ^ (k as u64 + 1).wrapping_mul(0xD6E8_FEB8_6659_FD93)
}

/// One DMA operation with a loop, as Main puts it on the bus.
#[derive(Debug, Clone, Copy)]
struct Op {
    op: u8,
    dst: u64,
    src: u64,
    count: usize,
    fill: u8,
    step: u64,
}

impl Op {
    fn new(op: u8, dst: u64, src: u64, count: usize, fill: u8, step: u64) -> Self {
        let op = Self { op, dst, src, count, fill, step };
        assert!(op.loop_count() > 0, "{op:?} has no loop");
        op
    }

    /// A memcpy delegated by the controller.
    fn memcpy(dst: u64, src: u64, count: usize, step: u64) -> Self {
        Self::new(ZiskOp::DMA_XMEMCPY, dst, src, count, 0, step)
    }

    /// A memcpy Main sends straight to the loop, which then reads the count from memory.
    fn direct_memcpy(dst: u64, src: u64, count: usize, step: u64) -> Self {
        let op = Self::new(ZiskOp::DMA_MEMCPY, dst, src, count, 0, step);
        assert!(DmaInfo::is_direct(op.encoded()), "{op:?} is not direct");
        op
    }

    /// A memcmp of two equal ranges: the loop proves the part that is equal.
    fn memcmp(dst: u64, src: u64, count: usize, step: u64) -> Self {
        let op = Self::new(ZiskOp::DMA_XMEMCMP, dst, src, count, 0, step);
        assert!(!DmaInfo::is_direct(op.encoded()), "a memcmp is never direct");
        op
    }

    fn memset(dst: u64, count: usize, fill: u8, step: u64) -> Self {
        Self::new(ZiskOp::DMA_XMEMSET, dst, 0, count, fill, step)
    }

    fn inputcpy(dst: u64, count: usize, step: u64) -> Self {
        Self::new(ZiskOp::DMA_INPUTCPY, dst, 0, count, 0, step)
    }

    fn encoded(&self) -> u64 {
        match self.op {
            ZiskOp::DMA_MEMCPY | ZiskOp::DMA_XMEMCPY => {
                DmaInfo::encode_memcpy(self.dst, self.src, self.count)
            }
            ZiskOp::DMA_XMEMCMP => DmaInfo::encode_memcmp(self.dst, self.src, self.count, 0),
            ZiskOp::DMA_XMEMSET => DmaInfo::encode_memset(self.dst, self.count, self.fill),
            ZiskOp::DMA_INPUTCPY => DmaInfo::encode_inputcpy(self.dst, self.count),
            op => unreachable!("0x{op:02X}"),
        }
    }

    fn class(&self) -> usize {
        DmaLoopInput::class_of(self.op, self.encoded()).unwrap()
    }

    fn pre(&self) -> u64 {
        DmaInfo::get_pre_count(self.encoded()) as u64
    }

    fn loop_count(&self) -> usize {
        DmaInfo::get_loop_count(self.encoded())
    }

    fn rows(&self) -> usize {
        DmaLoopInput::total_rows(self.class(), self.encoded())
    }

    fn has_src(&self) -> bool {
        matches!(self.op, ZiskOp::DMA_MEMCPY | ZiskOp::DMA_XMEMCPY | ZiskOp::DMA_XMEMCMP)
    }

    /// The bus payload and the words that travel with it: whatever the controller reads before
    /// the loop (poison here, the loop must not touch it), then the words the loop reads. A
    /// memset has none.
    fn bus(&self) -> (Vec<u64>, Vec<u64>) {
        let encoded = self.encoded();
        let mut data = vec![0u64; DMA_ENCODED + 2];
        data[OP] = self.op as u64;
        data[OP_TYPE] = ZiskOperationType::Dma as u64;
        data[A] = self.dst;
        data[B] = self.src;
        data[STEP] = self.step;
        data[DMA_ENCODED] = encoded;

        if self.op == ZiskOp::DMA_XMEMSET {
            return (data, Vec::new());
        }
        let mut ext = vec![0xBAD0_BAD0_BAD0_BAD0; DmaInfo::get_loop_data_offset(encoded)];
        let slots = DmaLoopInput::total_slots(self.class(), encoded);
        let first_src64 = (self.src + self.pre()) >> 3;
        ext.extend((0..slots).map(|k| {
            if self.has_src() {
                mem_word(first_src64 + k as u64)
            } else {
                input_word(self.step, k)
            }
        }));
        (data, ext)
    }

    /// The whole loop of the operation as one input, the way a collector with room for it
    /// builds it.
    fn input(&self) -> DmaLoopInput {
        let (data, ext) = self.bus();
        DmaLoopInput::from(&data, &ext, 0, usize::MAX)
    }
}

/// Every kind of loop the air proves, at alignments that leave a PRE, a POST, both or neither,
/// and with unaligned sequences whose extra read lands on a row of its own.
fn every_kind_of_loop() -> Vec<Op> {
    vec![
        // aligned memcpy with a PRE and a POST
        Op::memcpy(0x1003, 0x2003, 5 + 8 * 9 + 3, 100),
        Op::direct_memcpy(0x1100, 0x2100, 8 * 8, 101),
        // aligned memcpy of one word
        Op::memcpy(0x1200, 0x2200, 8, 102),
        // unaligned memcpy, dst aligned
        Op::memcpy(0x1300, 0x2305, 8 * 7, 103),
        // unaligned memcpy with a PRE
        Op::memcpy(0x1401, 0x2406, 100, 104),
        // unaligned: 11 words and the extra read, three full rows
        Op::memcpy(0x1500, 0x2503, 8 * 11, 105),
        // unaligned: 12 words, so the extra read takes a row of its own
        Op::memcpy(0x1600, 0x2607, 8 * 12, 106),
        // unaligned of one word: two reads, one row
        Op::memcpy(0x1700, 0x2701, 8, 107),
        Op::memcmp(0x1803, 0x2803, 5 + 8 * 6, 108),
        Op::memcmp(0x1905, 0x2902, 90, 109),
        Op::memset(0x1A02, 77, 0x5A, 110),
        Op::memset(0x1B00, 8 * 4, 0x00, 111),
        Op::inputcpy(0x1C00, 8 * 13, 112),
        Op::inputcpy(0x1D04, 4 + 8 * 5 + 1, 113),
    ]
}

// ─────────────────────────────────────────────────────────────────────────── the checker

/// One `proves_operation` of the air: a sequence starting (or a padding row).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Prove {
    op: u64,
    a: u64,
    b: u64,
    extended_arg: u64,
    offset: u64,
    main_step: u64,
}

/// What a segment's rows put on the buses.
#[derive(Debug, Default)]
struct Effects {
    proves: Vec<Prove>,
    padding_proves: usize,
    /// Source reads: (address, value, step).
    loads: Vec<(u64, u64, u64)>,
    /// Destination writes: (address, value, step).
    writes: Vec<(u64, u64, u64)>,
    /// Destination reads of a memcmp, which asserts memory holds the value: (address, value,
    /// step).
    compares: Vec<(u64, u64, u64)>,
    /// Loads of the count a direct memcpy reads from memory: (step, value).
    count_loads: Vec<(u64, u64)>,
}

impl Effects {
    fn extend(&mut self, other: Effects) {
        self.proves.extend(other.proves);
        self.padding_proves += other.padding_proves;
        self.loads.extend(other.loads);
        self.writes.extend(other.writes);
        self.compares.extend(other.compares);
        self.count_loads.extend(other.count_loads);
    }
}

fn word(bytes: &[u8]) -> u64 {
    u64::from_le_bytes(bytes.try_into().unwrap())
}

/// Evaluates every constraint of `dma_loop.pil` on `rows`, one segment whose air values are
/// `values` and whose dual-byte multiplicities are `table`, and returns what the rows put on the
/// buses.
fn check_segment(rows: &[Row], values: &DmaLoopSegmentValues, table: &[u64]) -> Effects {
    let n = rows.len();
    let previous = &values.previous;
    let last = &values.last;

    // The air values on their own.
    if values.is_last_segment {
        assert!(last.seq_end, "the last segment must end on a finished sequence");
    }
    // A finished hand-over carries nothing, the bytes included.
    if previous.seq_end {
        assert_eq!(*previous, DmaLoopRowState::ENDED, "the previous hand-over is not empty");
    }
    if last.seq_end {
        assert_eq!(*last, DmaLoopRowState::ENDED, "the last hand-over is not empty");
    }
    let [low, high] = values.last_count_chunks();
    assert_eq!(low as u32 + ((high as u32) << 16), last.count, "last_count_chunk");

    let mut effects = Effects::default();
    let mut expected_table = vec![0u64; 1 << 16];

    for (i, row) in rows.iter().enumerate() {
        let l1 = i == 0;
        let is_last = i + 1 == n;
        let seq_end = row.get_seq_end();
        let (memcpy, memeq, memset, inputcpy) = (
            row.get_sel_memcpy(),
            row.get_sel_memeq(),
            row.get_sel_memset(),
            row.get_sel_inputcpy(),
        );
        let src64 = row.get_src64() as u64;
        let dst64 = row.get_dst64() as u64;
        let count = row.get_count() as u64;
        let main_step = row.get_main_step();
        let fill_byte = row.get_fill_byte();

        let sel = memcpy as u8 + memeq as u8 + memset as u8 + inputcpy as u8;
        assert!(sel <= 1, "row {i}: more than one operation selected");
        let sel = sel == 1;

        // previous_seq_end <== L1 * (segment_previous_seq_end - 'seq_end) + 'seq_end
        let previous_seq_end = if l1 { previous.seq_end } else { rows[i - 1].get_seq_end() };

        let count_load = row.get_sel_memcpy_count_load();
        if count_load {
            assert!(memcpy, "row {i}: a count load outside a memcpy");
            assert!(previous_seq_end, "row {i}: a count load past the first row of a sequence");
        }

        let mut sel_op = [false; W + 1];
        sel_op[0] = sel;
        sel_op[1..W].copy_from_slice(&row.get_all_sel_op_from_1());
        for lane in 1..W {
            assert!(!sel_op[lane] || sel_op[lane - 1], "row {i}: sel_op is not a prefix");
        }
        let ops_selected = sel_op.iter().filter(|&&s| s).count() as u64;

        if !memset {
            assert_eq!(fill_byte, 0, "row {i}: a fill byte outside a memset");
        }

        let offsets = [
            row.get_offset_1(),
            row.get_offset_2(),
            row.get_offset_3(),
            row.get_offset_4(),
            row.get_offset_5(),
            row.get_offset_6(),
            row.get_offset_7(),
        ];
        assert!(offsets.iter().filter(|&&o| o).count() <= 1, "row {i}: two offsets selected");
        let offset = offsets.iter().position(|&o| o).map_or(0, |p| p + 1);
        if memset || inputcpy {
            assert_eq!(offset, 0, "row {i}: an offset on an operation without a source");
        }
        let flags = memcpy as u64
            + 2 * memeq as u64
            + 4 * inputcpy as u64
            + 8 * memset as u64
            + 16 * offset as u64;

        let read_bytes = row.get_all_read_bytes();
        if memset {
            assert!(read_bytes.iter().all(|&b| b == fill_byte), "row {i}: memset reads");
        }
        if l1 && !previous.seq_end {
            assert_eq!(
                read_bytes[..8],
                previous.bytes,
                "the first read is not the one handed over"
            );
        }

        // next_bytes = LAST * segment_next_bytes + no_last_no_seq_end * 'read_bytes
        let next_bytes: [u8; 8] = if is_last {
            last.bytes
        } else if !seq_end {
            rows[i + 1].get_all_read_bytes()[..8].try_into().unwrap()
        } else {
            [0; 8]
        };
        let stream: Vec<u8> = read_bytes.iter().chain(next_bytes.iter()).copied().collect();

        // The memory the row touches.
        let tail_no_write = seq_end && offset != 0;
        for lane in 0..W {
            let sel_op_last = sel_op[lane] && !sel_op[lane + 1];
            let sel_read = sel_op[lane] && (memcpy || memeq);
            let sel_write = sel_op[lane] && !(tail_no_write && sel_op_last);
            let read_value = word(&read_bytes[8 * lane..8 * lane + 8]);
            let write_value = word(&stream[8 * lane + offset..8 * lane + offset + 8]);
            if sel_read {
                effects.loads.push((8 * (src64 + lane as u64), read_value, main_step));
            }
            if sel_write {
                let access = (8 * (dst64 + lane as u64), write_value, main_step);
                if memeq {
                    effects.compares.push(access);
                } else {
                    effects.writes.push(access);
                }
            }
            if sel_op[lane] {
                for chunk in read_bytes[8 * lane..8 * lane + 8].chunks(2) {
                    expected_table[chunk[0] as usize + 256 * chunk[1] as usize] += 1;
                }
            }
        }
        if count_load {
            effects.count_loads.push((main_step, count * 8));
        }

        // proves_operation(op:, a: [dst64 * 8], b: [b0], ..., mul: previous_seq_end)
        if previous_seq_end {
            let op = OP_DMA_XMEMCPY * memcpy as u64
                + OP_DMA_XMEMEQ * memeq as u64
                + OP_DMA_XMEMSET * memset as u64
                + OP_DMA_INPUTCPY * inputcpy as u64
                - (OP_DMA_XMEMCPY - OP_DMA_MEMCPY) * count_load as u64;
            let b0 = src64 * 8 * (memcpy || memeq) as u64 + count * 8 * (inputcpy || memset) as u64;
            let extended_arg =
                count * 8 * ((memcpy && !count_load) || memeq) as u64 + fill_byte as u64;
            let prove =
                Prove { op, a: dst64 * 8, b: b0, extended_arg, offset: offset as u64, main_step };
            if prove == Prove::default() {
                effects.padding_proves += 1;
            } else {
                effects.proves.push(prove);
            }
        }

        // The hand-over to the next segment.
        if is_last {
            assert_eq!(seq_end, last.seq_end, "the last row's seq_end is not the one handed over");
            if !seq_end {
                assert_eq!(
                    (src64, dst64, main_step, count, flags, fill_byte),
                    (
                        last.src64 as u64,
                        last.dst64 as u64,
                        last.main_step,
                        last.count as u64,
                        last.flags,
                        last.fill_byte
                    ),
                    "the last row is not the one handed over"
                );
            }
        }

        // A sequence goes on: from the previous segment on the first row, from the row above on
        // every other one.
        let before = if l1 {
            (!previous.seq_end).then_some((
                previous.count as u64,
                previous.src64 as u64,
                previous.dst64 as u64,
                previous.flags,
                previous.fill_byte,
                previous.main_step,
            ))
        } else {
            let above = &rows[i - 1];
            (!above.get_seq_end()).then(|| {
                (
                    above.get_count() as u64,
                    above.get_src64() as u64,
                    above.get_dst64() as u64,
                    flags_of_row(above),
                    above.get_fill_byte(),
                    above.get_main_step(),
                )
            })
        };
        if let Some((b_count, b_src64, b_dst64, b_flags, b_fill, b_step)) = before {
            let w = W as u64;
            assert_eq!(count + w, b_count, "row {i}: count does not go down by a row");
            assert_eq!(src64, b_src64 + w, "row {i}: src64 does not go up by a row");
            assert_eq!(dst64, b_dst64 + w, "row {i}: dst64 does not go up by a row");
            assert_eq!(flags, b_flags, "row {i}: the flags changed inside a sequence");
            assert_eq!(fill_byte, b_fill, "row {i}: the fill byte changed inside a sequence");
            assert_eq!(main_step, b_step, "row {i}: the step changed inside a sequence");
        }

        // The shape of the row against its count.
        if seq_end {
            assert_eq!(count + (offset != 0) as u64, ops_selected, "row {i}: last row lanes");
        } else {
            assert_eq!(ops_selected, W as u64, "row {i}: a row inside a sequence is not full");
        }
    }

    assert_eq!(effects.padding_proves, values.padding_size, "padding_size");
    assert!(table == expected_table.as_slice(), "the dual-byte multiplicities are not the lanes'");
    effects
}

fn flags_of_row(row: &Row) -> u64 {
    let offsets = [
        row.get_offset_1(),
        row.get_offset_2(),
        row.get_offset_3(),
        row.get_offset_4(),
        row.get_offset_5(),
        row.get_offset_6(),
        row.get_offset_7(),
    ];
    let offset = offsets.iter().position(|&o| o).map_or(0, |p| p + 1) as u64;
    row.get_sel_memcpy() as u64
        + 2 * row.get_sel_memeq() as u64
        + 4 * row.get_sel_inputcpy() as u64
        + 8 * row.get_sel_memset() as u64
        + 16 * offset
}

/// The operation the loop of `op` must prove: what the controller assumes of it (the
/// `assumes_operation` of `dma_with_pre_post.pil`), or, for a direct memcpy, what Main does.
fn expected_prove(op: &Op) -> Prove {
    let encoded = op.encoded();
    let loop_bytes = op.loop_count() as u64 * 8;
    if op.op == ZiskOp::DMA_MEMCPY && DmaInfo::is_direct(encoded) {
        return Prove {
            op: OP_DMA_MEMCPY,
            a: op.dst,
            b: op.src,
            extended_arg: 0,
            offset: 0,
            main_step: op.step,
        };
    }
    // loop_dst = dst64 + use_pre * 8, loop_src = src64 + src64_inc_by_pre * 8
    let loop_dst = (op.dst & !7) + 8 * (op.pre() > 0) as u64;
    let loop_src = (op.src & !7) + 8 * DmaInfo::get_src64_inc_by_pre(encoded) as u64;
    let src_offset_after_pre = (op.src + op.pre()) % 8;
    let (code, b, extended_arg, offset) = match op.op {
        ZiskOp::DMA_MEMCPY | ZiskOp::DMA_XMEMCPY => {
            (OP_DMA_XMEMCPY, loop_src, loop_bytes, src_offset_after_pre)
        }
        ZiskOp::DMA_XMEMCMP => (OP_DMA_XMEMEQ, loop_src, loop_bytes, src_offset_after_pre),
        ZiskOp::DMA_XMEMSET => (OP_DMA_XMEMSET, loop_bytes, op.fill as u64, 0),
        ZiskOp::DMA_INPUTCPY => (OP_DMA_INPUTCPY, loop_bytes, 0, 0),
        code => unreachable!("0x{code:02X}"),
    };
    Prove { op: code, a: loop_dst, b, extended_arg, offset, main_step: op.step }
}

fn sorted<T: Ord + Clone>(values: &[T]) -> Vec<T> {
    let mut values = values.to_vec();
    values.sort();
    values
}

/// Compares what the rows put on the buses with what `ops` do: each operation proved exactly
/// once, every word of its loop written (or compared) once with the right value, every source
/// word read once, and the count of a direct memcpy loaded from memory.
fn check_effects(ops: &[Op], effects: &Effects) {
    let mut proves = Vec::new();
    let mut loads = Vec::new();
    let mut writes = Vec::new();
    let mut compares = Vec::new();
    let mut count_loads = Vec::new();
    for op in ops {
        proves.push(expected_prove(op));
        let pre = op.pre();
        for k in 0..op.loop_count() as u64 {
            let addr = op.dst + pre + 8 * k;
            let value = match op.op {
                ZiskOp::DMA_XMEMSET => u64::from_le_bytes([op.fill; 8]),
                ZiskOp::DMA_INPUTCPY => input_word(op.step, k as usize),
                _ => mem_bytes_at(op.src + pre + 8 * k),
            };
            if op.op == ZiskOp::DMA_XMEMCMP {
                compares.push((addr, value, op.step));
            } else {
                writes.push((addr, value, op.step));
            }
        }
        if op.has_src() {
            let first = (op.src + pre) & !7;
            for k in 0..DmaLoopInput::total_slots(op.class(), op.encoded()) as u64 {
                let addr = first + 8 * k;
                loads.push((addr, mem_word(addr >> 3), op.step));
            }
        }
        if op.op == ZiskOp::DMA_MEMCPY && DmaInfo::is_direct(op.encoded()) {
            count_loads.push((op.step, op.loop_count() as u64 * 8));
        }
    }
    assert_eq!(sorted(&effects.proves), sorted(&proves), "the operations proved");
    assert_eq!(sorted(&effects.loads), sorted(&loads), "the source reads");
    assert_eq!(sorted(&effects.writes), sorted(&writes), "the destination writes");
    assert_eq!(sorted(&effects.compares), sorted(&compares), "the memcmp reads");
    assert_eq!(sorted(&effects.count_loads), sorted(&count_loads), "the count loads");
}

/// Poisons every column, so a column the fill forgets shows up instead of reading as a lucky
/// zero: the fill takes the buffer as it comes.
fn poisoned_rows(n: usize) -> Vec<Row> {
    let mut row = Row::default();
    row.set_main_step(0xDEAD_BEEF);
    row.set_src64(0xDEAD);
    row.set_dst64(0xBEEF);
    row.set_count(0xDEAD_BEEF);
    row.set_seq_end(false);
    row.set_sel_memeq(true);
    row.set_sel_memcpy_count_load(true);
    row.set_fill_byte(0x77);
    row.set_offset_3(true);
    row.set_all_sel_op_from_1(&[true; W - 1]);
    row.set_all_read_bytes(&[0xEE; 8 * W]);
    vec![row; n]
}

/// Fills one segment with `inputs` and checks it, returning the fill and the bus effects.
fn fill_and_check(
    inputs: &[&DmaLoopInput],
    num_rows: usize,
    segment_id: usize,
    is_last_segment: bool,
) -> (Vec<Row>, DmaLoopFill, Effects) {
    let mut rows = poisoned_rows(num_rows);
    let fill = Sm::fill_rows(inputs, &mut rows, SegmentId(segment_id), is_last_segment);
    assert_eq!(fill.values.segment_id, segment_id);
    assert_eq!(fill.values.is_last_segment, is_last_segment);
    let effects = check_segment(&rows, &fill.values, &fill.dual_byte_table);
    (rows, fill, effects)
}

// ───────────────────────────────────────────────────────────────────────────── the tests

/// Every kind of loop, in one segment that ends on padding.
#[test]
fn every_kind_of_loop_satisfies_the_air() {
    let ops = every_kind_of_loop();
    let inputs: Vec<DmaLoopInput> = ops.iter().map(Op::input).collect();
    let inputs: Vec<&DmaLoopInput> = inputs.iter().collect();
    let total: usize = ops.iter().map(Op::rows).sum();
    assert_eq!(total, inputs.iter().map(|input| input.rows as usize).sum::<usize>());

    let (_, fill, effects) = fill_and_check(&inputs, total + 5, 0, true);
    assert_eq!(fill.values.padding_size, 5);
    assert_eq!(fill.values.previous, DmaLoopRowState::ENDED);
    assert_eq!(fill.values.last, DmaLoopRowState::ENDED);
    check_effects(&ops, &effects);
}

/// A segment that ends on the last row of a sequence has no padding and hands nothing over.
#[test]
fn a_segment_filled_to_the_last_row_hands_nothing_over() {
    let ops = [Op::memcpy(0x4000, 0x5003, 8 * 9, 7), Op::memset(0x6000, 8 * 5, 0x11, 8)];
    let inputs: Vec<DmaLoopInput> = ops.iter().map(Op::input).collect();
    let inputs: Vec<&DmaLoopInput> = inputs.iter().collect();
    let total: usize = ops.iter().map(Op::rows).sum();

    let (_, fill, effects) = fill_and_check(&inputs, total, 0, true);
    assert_eq!(fill.values.padding_size, 0);
    assert_eq!(fill.values.last, DmaLoopRowState::ENDED);
    check_effects(&ops, &effects);
}

/// A segment with nothing to prove -- the loop block of a `CompactDma` instance whose controller
/// block got all the work, say -- is all padding, and is still a valid link of the chain.
#[test]
fn a_segment_with_no_input_is_all_padding() {
    let (rows, fill, effects) = fill_and_check(&[], 6, 3, true);
    assert_eq!(fill.values.padding_size, 6);
    assert_eq!(fill.values.previous, DmaLoopRowState::ENDED);
    assert_eq!(fill.values.last, DmaLoopRowState::ENDED);
    assert!(fill.dual_byte_table.iter().all(|&m| m == 0));
    assert!(effects.proves.is_empty() && effects.writes.is_empty() && effects.loads.is_empty());

    let mut padding = Row::default();
    set_dma_loop_padding::<Goldilocks, _>(&mut padding);
    for row in &rows {
        assert_eq!(loop_snapshot::<Goldilocks, _>(row), loop_snapshot::<Goldilocks, _>(&padding));
    }
}

/// The fill writes every committed column of every row it owns, so it does not matter what the
/// buffer held before -- which is why `compute_witness` does not zero it.
#[test]
fn the_fill_does_not_depend_on_what_the_buffer_held() {
    let ops = every_kind_of_loop();
    let inputs: Vec<DmaLoopInput> = ops.iter().map(Op::input).collect();
    let inputs: Vec<&DmaLoopInput> = inputs.iter().collect();
    let num_rows = ops.iter().map(Op::rows).sum::<usize>() + 3;

    let mut clean = vec![Row::default(); num_rows];
    let mut poisoned = poisoned_rows(num_rows);
    let clean_fill = Sm::fill_rows(&inputs, &mut clean, SegmentId(0), true);
    let poisoned_fill = Sm::fill_rows(&inputs, &mut poisoned, SegmentId(0), true);

    for (i, (a, b)) in clean.iter().zip(&poisoned).enumerate() {
        assert_eq!(
            loop_snapshot::<Goldilocks, _>(a),
            loop_snapshot::<Goldilocks, _>(b),
            "row {i} kept part of what the buffer held"
        );
    }
    assert_eq!(clean_fill.values, poisoned_fill.values);
    assert!(clean_fill.dual_byte_table == poisoned_fill.dual_byte_table);
}

/// The fill cuts the inputs into groups filled in parallel; the rows and the multiplicities must
/// come out exactly as a single worker writes them.
#[test]
fn the_parallel_fill_matches_a_single_worker() {
    // Enough inputs to give every worker several, of every kind.
    let ops: Vec<Op> = (0..8)
        .flat_map(|round| {
            every_kind_of_loop().into_iter().map(move |mut op| {
                op.step += 1000 * round;
                op.dst += 0x10_0000 * round;
                op
            })
        })
        .collect();
    let inputs: Vec<DmaLoopInput> = ops.iter().map(Op::input).collect();
    let inputs: Vec<&DmaLoopInput> = inputs.iter().collect();
    let num_rows = ops.iter().map(Op::rows).sum::<usize>() + 11;

    let run = |threads: usize| {
        let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build().unwrap();
        pool.install(|| {
            let mut rows = poisoned_rows(num_rows);
            let fill = Sm::fill_rows(&inputs, &mut rows, SegmentId(0), true);
            (rows, fill)
        })
    };
    let (single_rows, single) = run(1);
    let (parallel_rows, parallel) = run(7);

    for (i, (a, b)) in single_rows.iter().zip(&parallel_rows).enumerate() {
        assert_eq!(
            loop_snapshot::<Goldilocks, _>(a),
            loop_snapshot::<Goldilocks, _>(b),
            "row {i} differs"
        );
    }
    assert_eq!(single.values, parallel.values);
    assert!(single.dual_byte_table == parallel.dual_byte_table);

    let effects = check_segment(&parallel_rows, &parallel.values, &parallel.dual_byte_table);
    check_effects(&ops, &effects);
}

/// Where the loop starts is decided twice: by `DmaLoopInput::from`, from the bus, and by the
/// controller's `assumes_operation`, from its own columns. The two have to agree at every
/// alignment, or the operation the loop proves is not the one the controller assumed.
#[test]
fn the_loop_starts_where_the_controller_hands_it_over() {
    for dst_offset in 0..8u64 {
        for src_offset in 0..8u64 {
            for count in [8, 9, 15, 16, 17, 31, 40, 63, 64, 100] {
                let dst = 0x8000 + dst_offset;
                let src = 0x9000 + src_offset;
                let op = Op { op: ZiskOp::DMA_XMEMCPY, dst, src, count, fill: 0, step: 5 };
                if op.loop_count() == 0 {
                    continue;
                }
                let input = op.input();
                let expected = expected_prove(&op);
                let case = format!("dst+{dst_offset} src+{src_offset} count {count}");
                assert_eq!(input.dst64 as u64 * 8, expected.a, "{case}: dst");
                assert_eq!(input.src64 as u64 * 8, expected.b, "{case}: src");
                assert_eq!(input.offset as u64, expected.offset, "{case}: offset");
                assert_eq!(
                    DmaLoopInput::is_unaligned_class(input.class()),
                    expected.offset != 0,
                    "{case}: an aligned class with an offset, or the other way round"
                );
            }
        }
    }
}

/// Plans `chunks` over instances of `height` rows the way the strategy hands a loop air its
/// work, collects each instance from the bus with its own collectors, fills it and checks it.
/// Returns the air values of every segment, in order, and what they all put on the buses.
fn prove_in_instances(chunks: &[Vec<Op>], height: usize) -> (Vec<DmaLoopSegmentValues>, Effects) {
    let total: usize = chunks.iter().flatten().map(Op::rows).sum();
    let mut builder = DmaLoopInstancesBuilder::new("test", total.div_ceil(height), height);
    for (chunk, ops) in chunks.iter().enumerate() {
        for class in 0..DMA_LOOP_CLASSES {
            let of_class: Vec<&Op> = ops.iter().filter(|op| op.class() == class).collect();
            let rows = of_class.iter().map(|op| op.rows()).sum();
            builder.add_class_rows(ChunkId(chunk), class, rows, of_class.len());
        }
    }
    let plan = builder.get_plan();
    assert_eq!(plan.len(), total.div_ceil(height), "the plan does not tile the rows");

    let mut values = Vec::new();
    let mut effects = Effects::default();
    for (segment, (_, cp)) in plan.iter().enumerate() {
        assert_eq!(cp.is_last_segment, segment + 1 == plan.len());
        let mut chunk_ids: Vec<ChunkId> = cp.chunks.keys().copied().collect();
        chunk_ids.sort();
        let per_chunk: Vec<Vec<DmaLoopInput>> = chunk_ids
            .iter()
            .map(|&chunk_id| {
                let (num_inputs, counters) = cp.chunks[&chunk_id];
                let mut collector = DmaLoopCollector::new(chunk_id, num_inputs, counters);
                for op in &chunks[chunk_id.0] {
                    let (data, ext) = op.bus();
                    collector.process_data(&OPERATION_BUS_ID, &data, &ext);
                }
                collector.take_inputs()
            })
            .collect();
        let inputs = flatten_and_reorder_inputs(&per_chunk);
        let (_, fill, segment_effects) =
            fill_and_check(&inputs, height, segment, cp.is_last_segment);
        values.push(fill.values);
        effects.extend(segment_effects);
    }
    (values, effects)
}

/// Sequences cut between instances go on in the next one from the row they stopped at: what a
/// segment hands over is exactly what the next one receives, the chain starts and ends on a
/// finished sequence, and over the whole chain every operation is proved once and every word of
/// it written once.
#[test]
fn sequences_cut_between_instances_continue_where_they_stopped() {
    let chunks = vec![
        vec![
            Op::memcpy(0x1_0003, 0x2_0003, 5 + 8 * 23 + 5, 1),
            Op::memset(0x3001, 8 * 9, 0xC3, 2),
            Op::memcpy(0x4000, 0x5006, 8 * 17, 3),
        ],
        vec![
            Op::inputcpy(0x6000, 8 * 21, 4),
            Op::memcmp(0x7004, 0x8001, 8 * 19, 5),
            Op::direct_memcpy(0x9000, 0xA000, 8 * 14, 6),
        ],
        vec![Op::memcmp(0xB003, 0xC003, 5 + 8 * 6, 7)],
        vec![],
        vec![Op::memcpy(0xD000, 0xE001, 8, 8), Op::memcpy(0xD100, 0xE107, 8 * 7, 9)],
    ];
    let ops: Vec<Op> = chunks.iter().flatten().copied().collect();
    let total: usize = ops.iter().map(Op::rows).sum();

    // One row per instance cuts every sequence at every row; the odd heights cut them at every
    // lane position a sequence can be at; the last one holds everything.
    for height in [1, 2, 3, 5, 7, total] {
        let (values, effects) = prove_in_instances(&chunks, height);
        assert_eq!(values[0].previous, DmaLoopRowState::ENDED, "height {height}: the chain start");
        assert!(values.last().unwrap().last.seq_end, "height {height}: the chain end");
        for (segment, pair) in values.windows(2).enumerate() {
            assert_eq!(
                pair[0].last,
                pair[1].previous,
                "height {height}: segment {segment} hands over what segment {} does not receive",
                segment + 1
            );
        }
        if height < total {
            assert!(
                values.iter().any(|v| !v.last.seq_end),
                "height {height}: no sequence was cut, the test lost its point"
            );
        }
        check_effects(&ops, &effects);
    }
}
