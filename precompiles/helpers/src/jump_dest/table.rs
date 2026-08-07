//! Fixed-column generation for `JumpDestBitmapTable`.
//!
//! The table is described in `pil/jump_dest_bitmap_table.pil`; that file is the
//! specification and this is the generator, because producing 86k rows from an
//! interpreted PIL loop costs minutes of compile time. Keep the two in step: the
//! PIL declares the columns and the row count, this fills them.
//!
//! ROW ORDER is part of the contract: the witness has to name the row it looks
//! up. The order is the enumeration below, which walks
//! `(state_in, cdata0, cdata1, cdata2, cdata3, bytes_used)` but skips whatever a
//! chunk cannot follow, so a position is not a closed form of its fields.
//! [`JumpDestBitmapTableIndex`] therefore derives the mapping from this
//! generator instead of restating it.
//!
//! COLUMN PACKING. The whole input side travels in one field element:
//!
//! ```text
//! bits  0..5   state_in       every field is sized by the declared bits(n) of
//! bits  6..37  cdata4         the machine column, not by its tighter useful
//! bit  38      sel_mem_load   range, so the packing cannot alias
//! ```
//!
//! That is what lets the table hold four fixed columns instead of five, and it
//! is also what binds sel_mem_load to the state: the pairing is verified here
//! rather than left to the machine. The output side stays bare — state_out on
//! its own, with no flag, because the next op verifies its own load bit.
use std::collections::HashMap;

/// Byte states. A byte is either immediate data of a PUSH (or past the end of
/// the bytecode), a width-1 instruction that is not a JUMPDEST, a JUMPDEST, or a
/// PUSHn opcode with n in 1..=32.
pub const JD_I: u8 = 0;
pub const JD_N: u8 = 1;
pub const JD_J: u8 = 2;
pub const JD_P: u8 = 3;

/// `cdata` takes the values 0..=136 with no gaps.
pub const JD_CDATA_VALUES: u16 = 137;

/// State 0..32 counts pending PUSH bytes; 33 means the bytecode is finished.
pub const JD_STATE_FINISHED: u8 = 33;

/// Field offsets of the packed input column, in the order its name reads:
/// state_in, then cdata4, then sel_mem_load.
pub const JD_CDATA4_SHIFT: u64 = 1 << 6;
pub const JD_MEM_LOAD_SHIFT: u64 = 1 << 38;

/// Rows the generator must produce, asserted here and declared in the PIL.
pub const JUMP_DEST_BITMAP_TABLE_ROWS: usize = 138953;

/// Rows of `JumpDestCompressorTable`, four per 16-bit chunk value — one per
/// combination of "byte 0 ignored" and "byte 1 ignored". Mirrors
/// JUMP_DEST_COMPRESSOR_TABLE_ROWS in `pil/jump_dest_compressor_table.pil`.
pub const JUMP_DEST_COMPRESSOR_TABLE_ROWS: usize = 4 * 65536;

/// Packs a chunk classification into one byte. `push_len` is the length of
/// whichever byte is a push, and must be 0 when neither is.
#[inline]
pub fn jd_cdata(bs0: u8, bs1: u8, push_len: u8) -> u8 {
    if bs0 == JD_P || bs1 == JD_P {
        bs0 * 32 + push_len - 1
    } else {
        128 + bs0 * 3 + bs1
    }
}

/// Inverse of [`jd_cdata`]: the two byte states and the push length.
#[inline]
pub fn jd_decode(cdata: u8) -> (u8, u8, u8) {
    if cdata < 128 {
        let bs0 = cdata >> 5;
        let bs1 = if bs0 == JD_P { JD_I } else { JD_P };
        (bs0, bs1, (cdata & 0x1f) + 1)
    } else {
        let rest = cdata - 128;
        (rest / 3, rest % 3, 0)
    }
}

/// A word is loaded only when fewer than 8 PUSH bytes are still pending; with 8
/// or more the whole word is immediate data and no byte of it is ever examined.
#[inline]
pub fn jd_sel_mem_load(state: u8) -> u64 {
    (state < 8) as u64
}

/// Packs the input side of a row: `state_in + 2^6 * cdata4 + 2^38 * sel_mem_load`.
#[inline]
pub fn jd_pack_input(state: u8, cdata4: u64) -> u64 {
    state as u64 + JD_CDATA4_SHIFT * cdata4 + JD_MEM_LOAD_SHIFT * jd_sel_mem_load(state)
}

/// What one chunk does to the walk.
#[derive(Clone, Copy)]
struct Step {
    /// PUSH bytes still pending once the chunk is consumed.
    pending: u8,
    /// The bytecode ended at or before this chunk.
    ended: bool,
    /// The chunk's two bitmap bits.
    bits: u8,
    /// Byte states, needed to find the last byte of the word in use.
    bs0: u8,
    bs1: u8,
}

/// Applies one chunk. `None` when the chunk cannot follow the incoming state,
/// which is what forces the four chunks of a word to be coherent.
fn jd_step(cdata: u8, pending: u8, ended: bool) -> Option<Step> {
    let (bs0, bs1, n) = jd_decode(cdata);
    let mut out = Step { pending: 0, ended: false, bits: 0, bs0, bs1 };

    if ended {
        // Past the end of the bytecode every byte must be ignored.
        if bs0 != JD_I || bs1 != JD_I {
            return None;
        }
        out.ended = true;
    } else if pending >= 2 {
        // The chunk lies entirely inside the data of an earlier push.
        if bs0 != JD_I || bs1 != JD_I {
            return None;
        }
        out.pending = pending - 2;
    } else if pending == 1 {
        // Byte 0 is the last data byte of an earlier push; byte 1 starts an
        // instruction, or the bytecode ends there.
        if bs0 != JD_I {
            return None;
        }
        match bs1 {
            JD_I => out.ended = true,
            JD_J => out.bits = 2,
            JD_N => {}
            _ => out.pending = n,
        }
    } else if bs0 == JD_I {
        // Nothing pending and byte 0 is not an instruction: the bytecode ended.
        if bs1 != JD_I {
            return None;
        }
        out.ended = true;
    } else if bs0 == JD_P {
        // Byte 1 is the first data byte, so n - 1 bytes spill past the chunk.
        if bs1 != JD_I {
            return None;
        }
        out.pending = n - 1;
    } else {
        if bs0 == JD_J {
            out.bits = 1;
        }
        match bs1 {
            JD_I => out.ended = true,
            JD_J => out.bits += 2,
            JD_N => {}
            _ => out.pending = n,
        }
    }

    Some(out)
}

/// One row of the table.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct JumpDestBitmapTableRow {
    /// state_in, cdata4 and sel_mem_load packed into one field element.
    pub state_cdata4_mem_load: u64,
    pub bytes_used: u64,
    pub bitmap_byte: u64,
    /// Carried to the next op bare: its load bit is verified on its own input.
    pub state_out: u64,
}

/// Builds every row of `JumpDestBitmapTable`, in the order the PIL specifies.
pub fn build_jump_dest_bitmap_table() -> Vec<JumpDestBitmapTableRow> {
    let mut rows = Vec::with_capacity(JUMP_DEST_BITMAP_TABLE_ROWS);
    let mut push = |cdata4: u64, state_in: u8, bytes_used: u8, bitmap_byte: u8, state_out: u8| {
        rows.push(JumpDestBitmapTableRow {
            state_cdata4_mem_load: jd_pack_input(state_in, cdata4),
            bytes_used: bytes_used as u64,
            bitmap_byte: bitmap_byte as u64,
            state_out: state_out as u64,
        });
    };

    // Loaded words: fewer than 8 PUSH bytes pending, so the walk reads them.
    for state in 0..8u8 {
        for c0 in 0..JD_CDATA_VALUES {
            let Some(s0) = jd_step(c0 as u8, state, false) else { continue };
            for c1 in 0..JD_CDATA_VALUES {
                let Some(s1) = jd_step(c1 as u8, s0.pending, s0.ended) else { continue };
                for c2 in 0..JD_CDATA_VALUES {
                    let Some(s2) = jd_step(c2 as u8, s1.pending, s1.ended) else { continue };
                    for c3 in 0..JD_CDATA_VALUES {
                        let Some(s3) = jd_step(c3 as u8, s2.pending, s2.ended) else { continue };

                        let cdata4 =
                            c0 as u64 | (c1 as u64) << 8 | (c2 as u64) << 16 | (c3 as u64) << 24;
                        let bits = s0.bits | s1.bits << 2 | s2.bits << 4 | s3.bits << 6;

                        // Highest byte of the word that is in use; the bytecode
                        // can only end after it.
                        let mut last: i32 = -1;
                        for (index, step) in [s0, s1, s2, s3].iter().enumerate() {
                            if step.bs0 != JD_I {
                                last = 2 * index as i32;
                            }
                            if step.bs1 != JD_I {
                                last = 2 * index as i32 + 1;
                            }
                        }

                        // The whole word lies inside the bytecode. Two ways out:
                        // the code carries on, or it ends exactly at the word
                        // boundary. The byte states cannot tell them apart —
                        // that is what count is for — so both are legal and the
                        // walk picks. Without the second one the next op would
                        // start with a state under 8 and, by the rule that ties
                        // sel_mem_load to the state, load a word past the end.
                        if !s3.ended {
                            push(cdata4, state, 8, bits, s3.pending);
                            push(cdata4, state, 8, bits, JD_STATE_FINISHED);
                        }
                        // Or it ends anywhere after the last byte in use. The
                        // byte states cannot tell a PUSH with its data present
                        // from one truncated at the end of the bytecode, so all
                        // of these are legal here and `count` picks one.
                        for k in (last + 1)..8 {
                            push(cdata4, state, k as u8, bits, JD_STATE_FINISHED);
                        }
                    }
                }
            }
        }
    }

    // Skipped words: the state says the whole word is PUSH data, so nothing is
    // loaded and no bit can be set, but the 8 bytes still belong to the bytecode
    // and `count` has to consume them — unless a PUSH truncated at the end of
    // the bytecode makes it end inside the word.
    let all_ignored = {
        let cdata = jd_cdata(JD_I, JD_I, 0) as u64;
        cdata | cdata << 8 | cdata << 16 | cdata << 24
    };
    for state in 8..=32u8 {
        push(all_ignored, state, 8, 0, state - 8);
        // As above, a full word can also be the one that carries the last byte.
        push(all_ignored, state, 8, 0, JD_STATE_FINISHED);
        for k in 0..8u8 {
            push(all_ignored, state, k, 0, JD_STATE_FINISHED);
        }
    }

    // Already past the end: nothing read, nothing consumed.
    push(all_ignored, JD_STATE_FINISHED, 0, 0, JD_STATE_FINISHED);

    rows
}

/// Maps a lookup to the row of `JumpDestBitmapTable` that proves it.
///
/// The enumeration in [`build_jump_dest_bitmap_table`] skips the combinations a
/// chunk cannot follow, so a row's position is not a closed form of its fields —
/// it depends on how many valid ones precede it. Rather than mirror that logic
/// and risk it drifting from the generator, the index is built from the
/// generator itself, once, and queried by hash.
pub struct JumpDestBitmapTableIndex {
    rows: HashMap<(u64, u64, u64), u32>,
}

impl Default for JumpDestBitmapTableIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl JumpDestBitmapTableIndex {
    pub fn new() -> Self {
        let table = build_jump_dest_bitmap_table();
        let mut rows = HashMap::with_capacity(table.len());
        for (index, row) in table.iter().enumerate() {
            // A full word can either carry on or finish the bytecode, so the
            // input alone does not name a row — state_out tells the two apart.
            rows.insert((row.state_cdata4_mem_load, row.bytes_used, row.state_out), index as u32);
        }
        debug_assert_eq!(rows.len(), table.len(), "the lookup key must be unique per row");
        Self { rows }
    }

    /// Row proving `(state_in, cdata4, bytes_used) -> state_out`.
    ///
    /// # Panics
    /// If the combination is not in the table, which would mean the walk
    /// produced an op the AIR cannot verify.
    #[inline]
    pub fn row(&self, state_in: u8, cdata4: u64, bytes_used: u8, state_out: u8) -> u32 {
        let key = (jd_pack_input(state_in, cdata4), bytes_used as u64, state_out as u64);
        *self.rows.get(&key).unwrap_or_else(|| {
            panic!(
                "jump_dest: no bitmap table row for state {state_in}, cdata4 {cdata4:#x}, \
                 bytes_used {bytes_used}, state_out {state_out}"
            )
        })
    }
}

/// The `CDATA` that `jump_dest_compressor_table.pil` puts at
/// `jd_compressor_row(data, ignore)`. Mirrors the generation loop of that file,
/// which is the specification; this exists so the witness can be checked against
/// it, since the table itself is built by the PIL and not by this crate.
pub fn jd_compressor_cdata(data: u16, ignore: u8) -> u8 {
    let (b0, b1) = (data as u8, (data >> 8) as u8);
    let (s0, n0) = jd_classify(b0);
    let (s1, n1) = jd_classify(b1);
    let (ignore_b0, ignore_b1) = (ignore / 2 == 1, ignore % 2 == 1);

    if ignore_b0 {
        // Byte 0 is data of an earlier push, or past the end.
        if ignore_b1 {
            jd_cdata(JD_I, JD_I, 0)
        } else {
            jd_cdata(JD_I, s1, n1)
        }
    } else if s0 == JD_P {
        // A push at byte 0 always claims byte 1 as its data.
        jd_cdata(JD_P, JD_I, n0)
    } else if ignore_b1 {
        jd_cdata(s0, JD_I, 0)
    } else {
        jd_cdata(s0, s1, n1)
    }
}

/// State of a byte that starts an instruction, and its immediate length.
#[inline]
fn jd_classify(byte: u8) -> (u8, u8) {
    const JUMPDEST: u8 = 0x5b;
    const PUSH1: u8 = 0x60;
    const PUSH32: u8 = 0x7f;
    if byte == JUMPDEST {
        (JD_J, 0)
    } else if (PUSH1..=PUSH32).contains(&byte) {
        (JD_P, byte - PUSH1 + 1)
    } else {
        (JD_N, 0)
    }
}

/// Row of `JumpDestCompressorTable` proving one chunk. Unlike the bitmap table
/// this one is dense — four rows per `data`, one per ignore combination — so the
/// index is a multiplication.
#[inline]
pub fn jd_compressor_row(data: u16, ignore: u8) -> u32 {
    debug_assert!(ignore < 4);
    data as u32 * 4 + ignore as u32
}

#[cfg(test)]
#[test]
fn the_compressor_row_count_covers_every_index() {
    assert_eq!(jd_compressor_row(u16::MAX, 3) as usize, JUMP_DEST_COMPRESSOR_TABLE_ROWS - 1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// The index must name, for every row, that exact row.
    #[test]
    fn the_index_finds_every_row_of_the_table() {
        let table = build_jump_dest_bitmap_table();
        let index = JumpDestBitmapTableIndex::new();

        for (position, row) in table.iter().enumerate() {
            let state_in = (row.state_cdata4_mem_load % JD_CDATA4_SHIFT) as u8;
            let cdata4 = (row.state_cdata4_mem_load / JD_CDATA4_SHIFT) % (1 << 32);
            let found = index.row(state_in, cdata4, row.bytes_used as u8, row.state_out as u8);
            assert_eq!(found as usize, position, "row {position}");
        }
    }

    /// The compressor table holds four rows per data value, and the one the
    /// index names must carry the cdata the walk claims.
    #[test]
    fn the_compressor_index_lands_on_the_claimed_cdata() {
        for data in [0u16, 0x5b5b, 0x7f00, 0x0060, 0xffff, 0x5b7f] {
            for ignore in 0..4u8 {
                let row = jd_compressor_row(data, ignore);
                assert_eq!(row / 4, data as u32, "the row must belong to its data");
                assert_eq!(row % 4, ignore as u32, "and to its ignore combination");
            }
        }
    }

    #[test]
    fn cdata_is_injective_and_gapless() {
        let mut seen = HashSet::new();
        for bs0 in [JD_I, JD_N, JD_J, JD_P] {
            for bs1 in [JD_I, JD_N, JD_J, JD_P] {
                // After a PUSHn byte 1 is always its data.
                if bs0 == JD_P && bs1 != JD_I {
                    continue;
                }
                if bs0 == JD_P && bs1 == JD_P {
                    continue;
                }
                let lens: Vec<u8> =
                    if bs0 == JD_P || bs1 == JD_P { (1..=32).collect() } else { vec![0] };
                for n in lens {
                    let cdata = jd_cdata(bs0, bs1, n);
                    assert!(seen.insert(cdata), "cdata collision at {cdata}");
                    assert_eq!(jd_decode(cdata), (bs0, bs1, n), "decode mismatch at {cdata}");
                }
            }
        }
        assert_eq!(seen.len(), 137);
        assert_eq!(*seen.iter().max().unwrap(), 136, "values must be 0..=136");
        assert!((0..137u8).all(|c| seen.contains(&c)), "the range must have no gaps");
    }

    #[test]
    fn table_has_the_declared_number_of_rows() {
        assert_eq!(build_jump_dest_bitmap_table().len(), JUMP_DEST_BITMAP_TABLE_ROWS);
    }

    #[test]
    fn table_has_no_duplicate_rows() {
        let rows = build_jump_dest_bitmap_table();
        let unique: HashSet<_> = rows
            .iter()
            .map(|r| (r.state_cdata4_mem_load, r.bytes_used, r.bitmap_byte, r.state_out))
            .collect();
        assert_eq!(unique.len(), rows.len());
    }

    #[test]
    fn every_state_in_value_is_covered() {
        let rows = build_jump_dest_bitmap_table();
        let seen: HashSet<u64> =
            rows.iter().map(|r| r.state_cdata4_mem_load % JD_CDATA4_SHIFT).collect();
        // 34 states: 0..=32 pending plus "finished".
        assert_eq!(seen.len(), 34);
    }

    #[test]
    fn the_packed_input_is_injective_and_binds_the_load_bit() {
        for state in 0..=JD_STATE_FINISHED {
            for cdata4 in [0u64, 1, 0xFFFF_FFFF] {
                let packed = jd_pack_input(state, cdata4);
                assert_eq!(packed % JD_CDATA4_SHIFT, state as u64);
                assert_eq!((packed / JD_CDATA4_SHIFT) % (1 << 32), cdata4);
                // the load bit is not free: it follows from the state
                assert_eq!(packed / JD_MEM_LOAD_SHIFT, (state < 8) as u64);
            }
        }
    }

    #[test]
    fn a_truncated_push_at_the_end_is_representable() {
        // A word opening with PUSH32 whose data is absent: the same cdata4 as one
        // whose data is present, told apart only by bytes_used.
        let cdata4 = jd_cdata(JD_P, JD_I, 32) as u64
            | (jd_cdata(JD_I, JD_I, 0) as u64) << 8
            | (jd_cdata(JD_I, JD_I, 0) as u64) << 16
            | (jd_cdata(JD_I, JD_I, 0) as u64) << 24;
        let rows = build_jump_dest_bitmap_table();
        let matching: Vec<_> =
            rows.iter().filter(|r| r.state_cdata4_mem_load == jd_pack_input(0, cdata4)).collect();

        // bytes_used 1..=7 truncated, plus the two readings of a full word:
        // the push data carries on, or the bytecode ends at the word boundary.
        assert_eq!(matching.len(), 9);
        let full: Vec<_> = matching.iter().filter(|r| r.bytes_used == 8).collect();
        assert_eq!(full.len(), 2);
        assert!(full.iter().any(|r| r.state_out == 25), "33 - 8 bytes still pending");
        assert!(full.iter().any(|r| r.state_out == JD_STATE_FINISHED as u64));
        for row in matching.iter().filter(|r| r.bytes_used < 8) {
            assert_eq!(row.state_out, JD_STATE_FINISHED as u64);
            assert_eq!(row.bitmap_byte, 0);
        }
    }
}
