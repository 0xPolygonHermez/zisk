//! Expands one `jump_dest` operation into the per-op values the AIR rows carry.
//!
//! This is the witness side of `pil/jump_dest.pil`: given the minimal-trace
//! payload of an operation — its byte count and every source word the range
//! spans — it produces, for each op of each block, the values the trace row
//! needs. Grouping ops into rows of `op_x_row` is the caller's job, since only
//! the instance knows the AIR's shape.
//!
//! Every op it emits must correspond to a row of `JumpDestBitmapTable`, and the
//! tests check exactly that against the generated table.

use crate::jump_dest::bitmap::{bitmap_words, src_words, BYTES_PER_WORD};
use crate::jump_dest::table::{jd_cdata, JD_I, JD_J, JD_N, JD_P, JD_STATE_FINISHED};

/// The 0x5b opcode and the PUSH1..PUSH32 range, as the walk sees them.
const OPCODE_JUMPDEST: u8 = 0x5b;
const OPCODE_PUSH1: u8 = 0x60;
const OPCODE_PUSH32: u8 = 0x7f;

/// What one op contributes to its row.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct JumpDestOp {
    /// The source word in four 16-bit chunks, little-endian.
    pub data: [u16; 4],
    /// The compression of each chunk, as the compressor table admits it.
    pub cdata: [u8; 4],
    /// Per chunk, which of its two bytes the walk did not visit, as the
    /// compressor table indexes them: `ignore_b0 * 2 + ignore_b1`. This is what
    /// picks the table row, since one `data` value has four of them.
    pub ignore: [u8; 4],
    /// Whether the machine loads this word. False when a PUSH covers it whole.
    pub sel_mem_load: bool,
    /// One bit per bytecode byte of the word, set on JUMPDEST.
    pub bitmap_byte: u8,
    /// Bytes of this word inside the bytecode: what `count` consumes.
    pub bytes_used: u8,
    /// State entering the op — pending PUSH bytes, or 33 once finished.
    pub state_in: u8,
    /// State leaving the op, carried to the next one.
    pub state_out: u8,
}

/// Classifies a byte that starts an instruction: its state and, for a PUSH, the
/// number of immediate bytes that follow.
#[inline]
fn classify(byte: u8) -> (u8, u8) {
    if byte == OPCODE_JUMPDEST {
        (JD_J, 0)
    } else if (OPCODE_PUSH1..=OPCODE_PUSH32).contains(&byte) {
        (JD_P, byte - OPCODE_PUSH1 + 1)
    } else {
        (JD_N, 0)
    }
}

/// Expands one operation into its ops, in execution order.
///
/// `words` is the minimal-trace payload — every source word the byte range
/// spans, `src_words(count)` of them. The result is padded to a whole number of
/// blocks, because the machine always writes a full 64-bit bitmap word: the
/// trailing ops sit past the end of the bytecode, consume nothing and keep the
/// finished state.
pub fn expand_jump_dest_ops(count: usize, words: &[u64]) -> Vec<JumpDestOp> {
    assert_eq!(
        words.len(),
        src_words(count),
        "the payload must hold every source word the range spans"
    );

    let total_ops = bitmap_words(count) * 8;
    let mut ops = Vec::with_capacity(total_ops);
    let mut state = 0u8;

    for index in 0..total_ops {
        let mut op = JumpDestOp { state_in: state, ..Default::default() };

        // How many of this word's bytes are inside the bytecode. Zero once the
        // walk is past its end, whether because the code ended earlier or
        // because this op only exists to complete the block.
        let valid = if state == JD_STATE_FINISHED || index >= words.len() {
            0
        } else {
            core::cmp::min(BYTES_PER_WORD, count.saturating_sub(index * BYTES_PER_WORD))
        };

        let word = if index < words.len() { words[index] } else { 0 };
        op.data = [word as u16, (word >> 16) as u16, (word >> 32) as u16, (word >> 48) as u16];

        // Byte states of the word. Anything the walk does not visit stays
        // "ignored", which covers both PUSH data and bytes past the end.
        let mut byte_state = [JD_I; BYTES_PER_WORD];
        let mut push_len = [0u8; BYTES_PER_WORD];

        // The bytecode is finished once this word carries its last byte, whether
        // it ends inside the word or exactly at its boundary. Both close it: the
        // next op must not load, and the only state that says so is 33, since
        // sel_mem_load is a function of the state alone.
        let finished = count <= (index + 1) * BYTES_PER_WORD;

        if state >= 8 {
            // The word lies entirely inside the data of an earlier PUSH: not
            // loaded, no byte examined, but its 8 bytes still belong to the
            // bytecode unless the code ends here.
            op.state_out = if finished { JD_STATE_FINISHED } else { state - 8 };
        } else if state == JD_STATE_FINISHED {
            op.state_out = JD_STATE_FINISHED;
        } else {
            op.sel_mem_load = true;
            let mut offset = state as usize;
            while offset < valid {
                let byte = (word >> (8 * offset)) as u8;
                let (kind, immediates) = classify(byte);
                byte_state[offset] = kind;
                match kind {
                    JD_J => {
                        op.bitmap_byte |= 1 << offset;
                        offset += 1;
                    }
                    JD_P => {
                        push_len[offset] = immediates;
                        offset += 1 + immediates as usize;
                    }
                    _ => offset += 1,
                }
            }
            op.state_out =
                if finished { JD_STATE_FINISHED } else { (offset - BYTES_PER_WORD) as u8 };
        }

        op.bytes_used = valid as u8;
        for chunk in 0..4 {
            let (first, second) = (2 * chunk, 2 * chunk + 1);
            let immediates =
                if byte_state[first] == JD_P { push_len[first] } else { push_len[second] };
            op.cdata[chunk] = jd_cdata(byte_state[first], byte_state[second], immediates);
            // A byte is "ignored" exactly when the walk never classified it:
            // PUSH data, or past the end of the bytecode.
            op.ignore[chunk] =
                ((byte_state[first] == JD_I) as u8) * 2 + (byte_state[second] == JD_I) as u8;
        }

        state = op.state_out;
        ops.push(op);
    }

    ops
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jump_dest::bitmap::build_jump_dest_bitmap;
    use crate::jump_dest::table::{build_jump_dest_bitmap_table, jd_pack_input};
    use std::collections::HashSet;

    /// The AIR reads a word exactly when the op's sel_mem_load is set. The mem
    /// input generator must claim the same set, or the memory bus does not
    /// balance.
    ///
    /// The case this guards is a bytecode ending on a word boundary but not on a
    /// block boundary (count % 8 == 0, count % 64 != 0). The op that carries the
    /// last byte has to close the walk even though it used all 8, or the next one
    /// would start with a state under 8 and load a word past the end — because
    /// sel_mem_load is a function of the state alone.
    #[test]
    fn the_air_and_the_mem_inputs_agree_on_which_words_are_read() {
        for count in 1..=200usize {
            let bytecode = vec![0x00u8; count];
            let words = payload(&bytecode);

            let from_air: Vec<usize> = expand_jump_dest_ops(count, &words)
                .iter()
                .enumerate()
                .filter(|(_, op)| op.sel_mem_load)
                .map(|(index, _)| index)
                .collect();

            let mut from_walk = Vec::new();
            let mut bitmap = vec![0u64; crate::jump_dest::bitmap::bitmap_words(count)];
            crate::jump_dest::bitmap::walk_jump_dest_bitmap(
                count,
                |w| {
                    from_walk.push(w);
                    words[w]
                },
                &mut bitmap,
            );

            assert_eq!(from_air, from_walk, "count {count}");
        }
    }

    /// The compressor row the witness names must carry the cdata the op claims,
    /// or the lookup is counted against the wrong row and the bus does not
    /// balance. The table is generated by the PIL, so this checks the witness
    /// against a mirror of that generation.
    #[test]
    fn every_chunk_lands_on_a_compressor_row_with_its_cdata() {
        use crate::jump_dest::table::{jd_compressor_cdata, jd_compressor_row};

        for len in 1..=140usize {
            for stride in [1usize, 2, 3, 8, 31, 32, 33] {
                let mut bytecode: Vec<u8> = (0..len).map(|i| (i * 7 % 256) as u8).collect();
                for pc in (0..len).step_by(stride) {
                    bytecode[pc] = 0x7f;
                }
                for (index, op) in expand_jump_dest_ops(len, &payload(&bytecode)).iter().enumerate()
                {
                    for chunk in 0..4 {
                        let row = jd_compressor_row(op.data[chunk], op.ignore[chunk]);
                        assert_eq!(row / 4, op.data[chunk] as u32);
                        assert_eq!(
                            jd_compressor_cdata(op.data[chunk], op.ignore[chunk]),
                            op.cdata[chunk],
                            "len {len} stride {stride} op {index} chunk {chunk}"
                        );
                    }
                }
            }
        }
    }

    /// The payload the minimal trace carries for a bytecode.
    fn payload(bytecode: &[u8]) -> Vec<u64> {
        (0..src_words(bytecode.len()))
            .map(|w| {
                let mut bytes = [0u8; 8];
                let offset = w * 8;
                let available = core::cmp::min(8, bytecode.len() - offset);
                bytes[..available].copy_from_slice(&bytecode[offset..offset + available]);
                u64::from_le_bytes(bytes)
            })
            .collect()
    }

    /// Deterministic push-heavy corpus, the shape that exercises the walk.
    fn corpus() -> Vec<Vec<u8>> {
        let mut seed = 0x1234_5678_u64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let pool: Vec<u8> = (0..20)
            .map(|_| 0x5b)
            .chain((0x60..0x80).flat_map(|op| [op, op]))
            .chain([0x00, 0x5f, 0xff])
            .collect();

        let mut out = vec![vec![], vec![0x5b], vec![0x7f], vec![0x5b; 64], vec![0x5b; 65]];
        for len in [1, 7, 8, 9, 63, 64, 65, 70, 128, 132, 200, 511] {
            out.push((0..len).map(|_| pool[(next() % pool.len() as u64) as usize]).collect());
            // a PUSH32 truncated at the very end
            let mut truncated = vec![0x5bu8; len];
            truncated[len - 1] = 0x7f;
            out.push(truncated);
        }
        out
    }

    #[test]
    fn bitmap_bytes_rebuild_the_reference_bitmap() {
        for bytecode in corpus() {
            let ops = expand_jump_dest_ops(bytecode.len(), &payload(&bytecode));

            let mut rebuilt = vec![0u64; bitmap_words(bytecode.len())];
            for (index, op) in ops.iter().enumerate() {
                rebuilt[index / 8] |= (op.bitmap_byte as u64) << (8 * (index % 8));
            }

            let mut expected = vec![0u64; bitmap_words(bytecode.len())];
            build_jump_dest_bitmap(&bytecode, &mut expected);
            assert_eq!(rebuilt, expected, "len {}", bytecode.len());
        }
    }

    #[test]
    fn bytes_used_sums_to_the_byte_count() {
        for bytecode in corpus() {
            let ops = expand_jump_dest_ops(bytecode.len(), &payload(&bytecode));
            let total: usize = ops.iter().map(|op| op.bytes_used as usize).sum();
            assert_eq!(total, bytecode.len(), "count must be consumed exactly");
        }
    }

    #[test]
    fn the_state_chain_is_continuous() {
        for bytecode in corpus() {
            let ops = expand_jump_dest_ops(bytecode.len(), &payload(&bytecode));
            let mut state = 0u8;
            for op in &ops {
                assert_eq!(op.state_in, state, "each op enters with the previous one's state");
                state = op.state_out;
            }

            // Whatever the length, the op carrying the last byte closes the
            // walk: inside a word or exactly at its boundary, both leave 33. Any
            // other ending would let the next op load a word past the bytecode,
            // since sel_mem_load follows the state.
            if bytecode.is_empty() {
                continue;
            }
            assert_eq!(state, JD_STATE_FINISHED, "len {}", bytecode.len());
        }
    }

    #[test]
    fn a_word_is_loaded_exactly_when_its_state_is_below_eight() {
        for bytecode in corpus() {
            for op in expand_jump_dest_ops(bytecode.len(), &payload(&bytecode)) {
                assert_eq!(op.sel_mem_load, op.state_in < 8, "state {}", op.state_in);
                if !op.sel_mem_load {
                    assert_eq!(op.bitmap_byte, 0, "a word that is not read sets no bit");
                }
            }
        }
    }

    /// The strong one: every op the witness emits has to exist as a row of the
    /// fixed table, or its lookup fails at proving time.
    #[test]
    fn every_op_matches_a_row_of_the_fixed_table() {
        let table: HashSet<(u64, u64, u64, u64)> = build_jump_dest_bitmap_table()
            .iter()
            .map(|r| (r.state_cdata4_mem_load, r.bytes_used, r.bitmap_byte, r.state_out))
            .collect();

        for bytecode in corpus() {
            for (index, op) in
                expand_jump_dest_ops(bytecode.len(), &payload(&bytecode)).iter().enumerate()
            {
                let cdata4 = op
                    .cdata
                    .iter()
                    .enumerate()
                    .fold(0u64, |acc, (j, &c)| acc | (c as u64) << (8 * j));
                let key = (
                    jd_pack_input(op.state_in, cdata4),
                    op.bytes_used as u64,
                    op.bitmap_byte as u64,
                    op.state_out as u64,
                );
                assert!(
                    table.contains(&key),
                    "op {index} of a {}-byte bytecode is not in the table: {op:?}",
                    bytecode.len()
                );
            }
        }
    }
}
