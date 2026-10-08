//! EVM JUMPDEST bitmap: the reference model shared by the emulator, the
//! assembly emulator and the state machine.
//!
//! The precompile walks the bytecode one **aligned 64-bit word at a time** and
//! emits one bitmap byte per word, so every 8 source words produce one aligned
//! 64-bit bitmap word. Both the bytecode and the bitmap addresses must be
//! 8-byte aligned; the guest checks this and falls back to a software walk when
//! it does not hold, because an unaligned run cannot be arithmetized (two
//! writes to the same address in one timestamp are not provable).
//!
//! The walk carries a `state`: how many bytes of PUSH immediate data are still
//! pending at the start of the current word. When `state >= 8` the whole word
//! is push data, no jumpdest can live in it, and **the word is not read at
//! all** — that is the `sel_mem_load = 0` case in the AIR, and the reason the
//! set of memory reads depends on the bytecode content and not just its length.

/// The JUMPDEST opcode.
const JUMP_DEST: u8 = 0x5b;
/// PUSH1 .. PUSH32 opcode range.
const PUSH_FIRST: u8 = 0x60;
const PUSH_LAST: u8 = 0x7f;

/// Bytecode bytes covered by one source word (and by one bitmap byte).
pub const BYTES_PER_WORD: usize = 8;
/// Bytecode bytes covered by one 64-bit bitmap word.
pub const BYTES_PER_BITMAP_WORD: usize = 64;
/// Largest `state` the walk can carry: PUSH32 starting at the last byte of a
/// word leaves 32 immediate bytes pending. The AIR ranges `state` to [0, 32].
pub const MAX_STATE: u8 = 32;

/// Number of aligned source words spanned by `count` bytecode bytes.
#[inline(always)]
pub const fn src_words(count: usize) -> usize {
    count.div_ceil(BYTES_PER_WORD)
}

/// Number of aligned 64-bit bitmap words produced for `count` bytecode bytes.
#[inline(always)]
pub const fn bitmap_words(count: usize) -> usize {
    count.div_ceil(BYTES_PER_BITMAP_WORD)
}

/// Upper bound on the number of maximal runs of consecutive loaded words.
///
/// The walk can at worst alternate load / skip (a PUSH15 landing on byte 7 of
/// every other word does exactly that), and any two adjacent loads merge into
/// one run, so `ceil(src_words / 2)` runs is unreachable to exceed. This is the
/// figure the mops encoder must reserve buffer space for before it starts.
#[inline(always)]
pub const fn max_read_runs(count: usize) -> usize {
    src_words(count).div_ceil(2)
}

/// Appends loaded word `word` to `runs`, merging it into the previous run when
/// it is contiguous.
///
/// Runs are the unit both consumers of the read set need: the mops encoder
/// emits one aligned block per run, and the memory cost model bills one
/// contiguous range per run.
#[inline(always)]
pub fn push_read_run(runs: &mut Vec<(usize, usize)>, word: usize) {
    match runs.last_mut() {
        Some((first, len)) if *first + *len == word => *len += 1,
        _ => runs.push((word, 1)),
    }
}

/// Processes one aligned source word, the primitive the AIR lookup table
/// implements.
///
/// `word` holds the 8 bytecode bytes little-endian (byte `i` at bits `8*i`),
/// `state` is the pending PUSH-data count at the word start and must be < 8,
/// and `valid` is how many of the 8 bytes are inside the bytecode (8 for every
/// word but a trailing partial one).
///
/// Returns the bitmap byte for this word and the `state` carried to the next.
#[inline(always)]
pub fn scan_word(word: u64, state: u8, valid: usize) -> (u8, u8) {
    debug_assert!((state as usize) < BYTES_PER_WORD, "scan_word needs an in-word state");
    debug_assert!(valid <= BYTES_PER_WORD);

    let mut i = state as usize;
    let mut bitmap_byte = 0u8;

    while i < valid {
        let opcode = (word >> (8 * i)) as u8;
        if opcode == JUMP_DEST {
            bitmap_byte |= 1 << i;
            i += 1;
        } else if (PUSH_FIRST..=PUSH_LAST).contains(&opcode) {
            // 1 opcode byte + (opcode - 0x5f) immediate bytes.
            i += (opcode - (PUSH_FIRST - 2)) as usize;
        } else {
            i += 1;
        }
    }

    // Only a trailing partial word can leave the loop with i < 8, and then the
    // state is never used again.
    (bitmap_byte, i.saturating_sub(BYTES_PER_WORD) as u8)
}

/// Walks `count` bytecode bytes and fills `bitmap`, reading the source through
/// `read_word`.
///
/// `read_word(w)` must return source word `w` (bytecode bytes `8*w..8*w+8`,
/// little-endian) and **is called only for the words the machine actually
/// loads**, in strictly increasing order. That call sequence is the set of
/// memory reads the minimal trace and the mops must record.
///
/// `bitmap` must be exactly `bitmap_words(count)` long; every word is written.
/// Returns the number of `read_word` calls made.
#[inline]
pub fn walk_jump_dest_bitmap<R>(count: usize, mut read_word: R, bitmap: &mut [u64]) -> usize
where
    R: FnMut(usize) -> u64,
{
    assert_eq!(
        bitmap.len(),
        bitmap_words(count),
        "jumpdest bitmap must hold exactly ceil(count / 64) words"
    );

    for word in bitmap.iter_mut() {
        *word = 0;
    }

    let mut state = 0u8;
    let mut reads = 0usize;

    for w in 0..src_words(count) {
        if state >= BYTES_PER_WORD as u8 {
            // Whole word is PUSH immediate data: skip the load entirely.
            state -= BYTES_PER_WORD as u8;
            continue;
        }

        let valid = core::cmp::min(BYTES_PER_WORD, count - w * BYTES_PER_WORD);
        let (bitmap_byte, next_state) = scan_word(read_word(w), state, valid);
        reads += 1;
        state = next_state;

        if bitmap_byte != 0 {
            bitmap[w / 8] |= (bitmap_byte as u64) << (8 * (w % 8));
        }
    }

    reads
}

/// Builds an EVM jumpdest bitmap where each bit marks a valid JUMPDEST (0x5b)
/// byte offset in the input bytecode. PUSH-data bytes are skipped.
///
/// `bitmap` must be exactly `bytecode.len().div_ceil(64)` words long, the same
/// size the C++ guest allocates for `build_jumpdest_bitset`. The length is
/// asserted rather than clamped: a short buffer would drop the jumpdests past
/// its end, and a truncated bitmap surfaces far from here as a valid `JUMP`
/// reverting with an invalid-destination error.
///
/// Returns the number of source words read, i.e. how many the machine loads.
#[inline]
pub fn build_jump_dest_bitmap(bytecode: &[u8], bitmap: &mut [u64]) -> usize {
    walk_jump_dest_bitmap(bytecode.len(), |w| slice_word(bytecode, w), bitmap)
}

/// Reads source word `w` from a byte slice, zero-padding a trailing partial
/// word. In memory the precompile always loads the whole aligned word; the
/// bytes past the end are masked off by `valid` in [`scan_word`].
#[inline(always)]
fn slice_word(bytecode: &[u8], w: usize) -> u64 {
    let offset = w * BYTES_PER_WORD;
    let available = core::cmp::min(BYTES_PER_WORD, bytecode.len() - offset);
    let mut bytes = [0u8; BYTES_PER_WORD];
    bytes[..available].copy_from_slice(&bytecode[offset..offset + available]);
    u64::from_le_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds the bitmap with a correctly sized buffer, on a scratch buffer
    /// pre-filled with a non-zero pattern so any word left unwritten shows up.
    fn build(bytecode: &[u8]) -> Vec<u64> {
        let mut bitmap = vec![0xAAAA_AAAA_AAAA_AAAAu64; bitmap_words(bytecode.len())];
        build_jump_dest_bitmap(bytecode, &mut bitmap);
        bitmap
    }

    /// The set of source words the walk loads, in order.
    fn reads_of(bytecode: &[u8]) -> Vec<usize> {
        let mut bitmap = vec![0u64; bitmap_words(bytecode.len())];
        let mut reads = Vec::new();
        walk_jump_dest_bitmap(
            bytecode.len(),
            |w| {
                reads.push(w);
                super::slice_word(bytecode, w)
            },
            &mut bitmap,
        );
        reads
    }

    /// Byte-at-a-time reference walk, the shape of the C++ guest builder.
    fn reference(bytecode: &[u8]) -> Vec<u64> {
        let mut bitmap = vec![0u64; bitmap_words(bytecode.len())];
        let mut pc = 0usize;
        while pc < bytecode.len() {
            let opcode = bytecode[pc];
            if opcode == 0x5b {
                bitmap[pc / 64] |= 1u64 << (pc % 64);
                pc += 1;
            } else if (0x60..=0x7f).contains(&opcode) {
                pc += (opcode - 0x5e) as usize;
            } else {
                pc += 1;
            }
        }
        bitmap
    }

    #[test]
    fn marks_jumpdest_and_skips_push_data() {
        // 0x5b (jumpdest), 0x60 0x5b (push1 with push-data 0x5b), 0x5b (jumpdest)
        let bytecode = [0x5b, 0x60, 0x5b, 0x5b];
        let mut bitmap = [0u64; 1];

        build_jump_dest_bitmap(&bytecode, &mut bitmap);

        // Valid jumpdests at pc 0 and pc 3.
        assert_eq!(bitmap[0], (1u64 << 0) | (1u64 << 3));
    }

    #[test]
    fn empty_bytecode_needs_no_words() {
        assert!(build(&[]).is_empty());
        assert!(reads_of(&[]).is_empty());
    }

    #[test]
    fn marks_every_bit_position_across_two_words() {
        for pos in 0..128usize {
            let mut bytecode = vec![0x00u8; 130];
            bytecode[pos] = 0x5b;

            let bitmap = build(&bytecode);

            assert_eq!(bitmap[pos / 64], 1u64 << (pos % 64), "pc {pos}");
            assert_eq!(bitmap[1 - pos / 64], 0, "pc {pos}");
            assert_eq!(bitmap[2], 0, "pc {pos}");
        }
    }

    #[test]
    fn push_data_spanning_a_word_boundary_is_skipped() {
        // PUSH32 at pc 63: its 32 immediate bytes cover pc 64..=95, so the only
        // jumpdest in the second word is at pc 96.
        let mut bytecode = vec![0x5bu8; 130];
        bytecode[63] = 0x7f;

        let bitmap = build(&bytecode);

        assert_eq!(bitmap[0], !(1u64 << 63));
        assert_eq!(bitmap[1], !0u64 << 32);
    }

    #[test]
    fn truncated_push_data_ends_the_walk() {
        // PUSH32 at pc 68 of 70 bytes: its immediate data runs past the end, so
        // the last byte is push data and no further jumpdest is marked.
        let mut bytecode = vec![0x5bu8; 70];
        bytecode[68] = 0x7f;

        let bitmap = build(&bytecode);

        assert_eq!(bitmap[0], !0u64);
        assert_eq!(bitmap[1], 0b1111);
    }

    #[test]
    fn trailing_bits_past_the_code_stay_clear() {
        // 65 jumpdests: word 1 holds a single bit, and bits 1..=63 stay clear.
        let bitmap = build(&[0x5bu8; 65]);

        assert_eq!(bitmap[0], !0u64);
        assert_eq!(bitmap[1], 1u64);
    }

    #[test]
    #[should_panic(expected = "jumpdest bitmap must hold exactly")]
    fn rejects_a_bitmap_that_is_too_short() {
        // One word short: the jumpdest at pc 64 would be dropped in silence.
        let mut bitmap = [0u64; 1];
        build_jump_dest_bitmap(&[0x5bu8; 65], &mut bitmap);
    }

    #[test]
    #[should_panic(expected = "jumpdest bitmap must hold exactly")]
    fn rejects_an_oversized_bitmap() {
        // The C++ builder leaves the extra words untouched while this one would
        // clear them; requiring the exact size keeps the two in agreement.
        let mut bitmap = [0u64; 2];
        build_jump_dest_bitmap(&[0x5bu8; 4], &mut bitmap);
    }

    #[test]
    fn dense_code_reads_every_word() {
        // No push data at all: every source word holds a boundary.
        let bytecode = vec![0x5bu8; 200];
        assert_eq!(reads_of(&bytecode), (0..src_words(200)).collect::<Vec<_>>());
    }

    #[test]
    fn a_push32_chain_skips_the_words_it_covers() {
        // PUSH32 every 33 bytes: boundaries at 0, 33, 66, 99, so words 0, 4, 8
        // and 12 are read and the ones fully inside push data are not. Word 16
        // is the trailing partial word and is read under the uniform rule (see
        // `a_trailing_word_is_read_whenever_state_fits`).
        let mut bytecode = vec![0x00u8; 132];
        for pc in (0..132).step_by(33) {
            bytecode[pc] = 0x7f;
        }
        assert_eq!(reads_of(&bytecode), vec![0, 4, 8, 12, 16]);
    }

    #[test]
    fn a_trailing_word_is_read_whenever_state_fits() {
        // 132 bytes: the last word holds 4 valid bytes and is entered with
        // state = 4, so the next boundary is exactly at the end of the code and
        // no byte is examined. The load still happens: the rule is `state < 8`
        // for every word, with no special case for the tail, which is what
        // keeps `sel_mem_load` a function of `state` alone in the AIR.
        let mut bytecode = vec![0x00u8; 132];
        bytecode[99] = 0x7f;

        let reads = reads_of(&bytecode);

        assert!(reads.contains(&16), "trailing word must still be loaded");
        assert_eq!(build(&bytecode)[2], 0, "and it contributes no jumpdest");
    }

    #[test]
    fn worst_case_alternates_read_and_skip() {
        // PUSH15 (width 16) starting at byte 7 of a word lands the next
        // boundary on byte 7 of the word after next, so every other word is
        // skipped. This is the case that bounds the mops entry count.
        let mut bytecode = vec![0x00u8; 128];
        let mut pc = 7usize;
        while pc < 128 {
            bytecode[pc] = 0x6e; // PUSH15
            pc += 16;
        }
        let reads = reads_of(&bytecode);
        assert_eq!(reads, (0..16).step_by(2).collect::<Vec<_>>());
        // The bound the assembly must reserve space for.
        assert_eq!(reads.len(), src_words(128).div_ceil(2));
    }

    #[test]
    fn a_skipped_word_never_holds_a_jumpdest() {
        // 0x5b bytes buried inside PUSH data must not be marked even though
        // their word is never loaded.
        let mut bytecode = vec![0x5bu8; 96];
        bytecode[0] = 0x7f; // PUSH32 covers pc 1..=32
        let bitmap = build(&bytecode);
        assert_eq!(bitmap[0], !0u64 << 33);
        assert_eq!(reads_of(&bytecode)[0..2], [0, 4]);
    }

    /// The runs of consecutive loaded words, as the mops encoder groups them.
    fn runs_of(bytecode: &[u8]) -> Vec<(usize, usize)> {
        let mut runs = Vec::new();
        for w in reads_of(bytecode) {
            push_read_run(&mut runs, w);
        }
        runs
    }

    #[test]
    fn adjacent_reads_merge_into_one_run() {
        // Dense code: a single run covering every word.
        assert_eq!(runs_of(&[0x5bu8; 200]), vec![(0, src_words(200))]);
    }

    #[test]
    fn the_alternating_worst_case_hits_the_run_bound() {
        let mut bytecode = vec![0x00u8; 128];
        let mut pc = 7usize;
        while pc < 128 {
            bytecode[pc] = 0x6e; // PUSH15
            pc += 16;
        }
        let runs = runs_of(&bytecode);

        // Every run is a single word and none can be merged: this is exactly
        // the bound the mops buffer check has to reserve for.
        assert!(runs.iter().all(|&(_, len)| len == 1));
        assert_eq!(runs.len(), max_read_runs(128));
    }

    #[test]
    fn read_runs_stay_within_the_bound() {
        let mut seed = 0x5b60_7f00_u64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        // Push-heavy pool: the shape that produces the sparsest read patterns.
        let pool: Vec<u8> = (0x60..0x80).chain(0x60..0x80).chain([0x5b, 0x00]).collect();

        for len in 0..800usize {
            let bytecode: Vec<u8> =
                (0..len).map(|_| pool[(next() % pool.len() as u64) as usize]).collect();
            let runs = runs_of(&bytecode);
            assert!(
                runs.len() <= max_read_runs(len),
                "len {len}: {} runs exceeds bound {}",
                runs.len(),
                max_read_runs(len)
            );
            // Runs must be disjoint, ordered and contiguous internally.
            let mut previous_end = 0usize;
            for (first, run_len) in runs {
                assert!(first >= previous_end, "len {len}: runs out of order");
                previous_end = first + run_len;
            }
            assert!(previous_end <= src_words(len));
        }
    }

    #[test]
    fn word_walk_matches_the_byte_walk() {
        // The word-granular walk and the byte-at-a-time reference must agree on
        // every input: same bitmap, whatever the read pattern.
        let mut seed = 0x2026_0806_u64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let pool: Vec<u8> = (0..30)
            .map(|_| 0x5b)
            .chain((0x60..0x80).flat_map(|op| [op, op, op]))
            .chain([0x00, 0x5a, 0x5f, 0x80, 0xff].iter().flat_map(|&b| [b; 4]))
            .collect();

        for len in 0..600usize {
            let bytecode: Vec<u8> =
                (0..len).map(|_| pool[(next() % pool.len() as u64) as usize]).collect();
            assert_eq!(build(&bytecode), reference(&bytecode), "len {len}");
        }
    }
}
