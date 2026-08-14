use zisk_precomp_helpers::{
    bitmap_words, push_read_run, src_words, walk_jump_dest_bitmap, BYTES_PER_WORD,
};

use crate::{
    zisk_ops::OpStats, EmulationMode, InstContext, Mem, EXTRA_PARAMS_ADDR, JUMP_DEST_COST,
};

/// Minimal-trace layout, mirroring the DMA ops:
///
/// ```text
/// [0]   count, the value the opcode reads from EXTRA_PARAMS_ADDR (input_size = 8)
/// [1..] every source word the byte range spans, ceil(count / 8) of them
/// ```
///
/// `dst` and `src` are not in the trace — they come from the a/b operands of the
/// instruction.
///
/// The payload is the **whole** contiguous source range, including the words the
/// machine never loads because a PUSH covers them; the precompile discards those
/// while walking. Carrying them costs a few trace words on push-heavy code and
/// buys a data_ext length that follows from `count` alone, so the slice handed
/// to the collectors is a plain contiguous range like every other precompile's.
/// The mops, which are a claim about memory operations rather than a data
/// payload, do list only the words actually loaded.
///
/// Both consumers of the trace therefore need nothing but `count`.
///
/// PRECONDITIONS, all of them the guest's to respect. None is a soundness
/// matter: breaking one leaves the program unprovable rather than wrongly
/// proven, because no witness satisfies the AIR for such a call. They are
/// checked here, and not only where they would bite, so a guest bug surfaces
/// during emulation instead of hours later as a bus that does not balance.
///
/// * Both operands 8-byte aligned. The machine reads the bytecode and writes
///   the bitmap in whole aligned 64-bit words, and an unaligned run is not
///   arithmetizable — it would need two writes to one address in a single
///   timestamp. The guest checks this and falls back to a software walk.
///
/// * `count > 0`. An empty call spans no bitmap word, so it occupies no block
///   in the AIR, so nothing emits `proves_operation` for it while main still
///   emits its `assumes_operation`. Serving it inside the AIR would mean
///   spending a block on a call with nothing to compute, and its first op would
///   load the word at `src` — which for an empty call may well be a dangling
///   pointer. The guest skips the call instead.
#[inline(always)]
fn check_preconditions(count: usize, bitmap_addr: u64, bytecode_addr: u64) {
    assert_ne!(
        count, 0,
        "jump_dest called with count = 0; the guest must skip the call, the AIR cannot prove it"
    );
    // TODO: raise back to an assert once the guests are known to pass aligned
    // operands. Real bytecode still reaches here unaligned, and stopping on it
    // would block end-to-end runs that are after other problems.
    debug_assert_eq!(
        bytecode_addr & 0x07,
        0,
        "jump_dest bytecode address 0x{bytecode_addr:08x} is not 8-byte aligned"
    );
    debug_assert_eq!(
        bitmap_addr & 0x07,
        0,
        "jump_dest bitmap address 0x{bitmap_addr:08x} is not 8-byte aligned"
    );
}

/// Runs the walk over emulator memory, reporting every loaded word to
/// `on_word` as `(word_index, value)`. Returns the bitmap and how many source
/// words were loaded.
#[inline(always)]
fn compute_bitmap<F>(
    mem: &Mem,
    bytecode_addr: u64,
    count: usize,
    mut on_word: F,
) -> (Vec<u64>, usize)
where
    F: FnMut(usize, u64),
{
    let mut bitmap = vec![0u64; bitmap_words(count)];
    let reads = walk_jump_dest_bitmap(
        count,
        |word_index| {
            let word = mem.read(bytecode_addr + (word_index * BYTES_PER_WORD) as u64, 8);
            on_word(word_index, word);
            word
        },
        &mut bitmap,
    );
    (bitmap, reads)
}

#[inline(always)]
fn store_bitmap(ctx: &mut InstContext, bitmap_addr: u64, bitmap: &[u64]) {
    for (index, word) in bitmap.iter().enumerate() {
        ctx.mem.write(bitmap_addr + (index * BYTES_PER_WORD) as u64, *word, 8);
    }
}

#[inline(always)]
pub fn opc_jump_dest(ctx: &mut InstContext) {
    let bitmap_addr = ctx.a;
    let bytecode_addr = ctx.b;

    match ctx.emulation_mode {
        EmulationMode::Mem => {
            let count = ctx.mem.read(EXTRA_PARAMS_ADDR, 8) as usize;
            // Traces every call with how the byte count divides into blocks and
            // whether the operands are aligned. Kept for profiling real bytecode.
            check_preconditions(count, bitmap_addr, bytecode_addr);
            let (bitmap, _) = compute_bitmap(&ctx.mem, bytecode_addr, count, |_, _| {});
            store_bitmap(ctx, bitmap_addr, &bitmap);
        }
        EmulationMode::GenerateMemReads => {
            let count = ctx.mem.read(EXTRA_PARAMS_ADDR, 8) as usize;

            ctx.precompiled.input_data.clear();
            ctx.precompiled.output_data.clear();
            ctx.precompiled.step = ctx.step;
            // The value the opcode reads from EXTRA_PARAMS_ADDR, which is also
            // the length of the payload that follows.
            ctx.precompiled.input_data.push(count as u64);

            check_preconditions(count, bitmap_addr, bytecode_addr);

            // The whole source range, one contiguous run of words.
            {
                let mem = &ctx.mem;
                let input_data = &mut ctx.precompiled.input_data;
                for word_index in 0..src_words(count) {
                    input_data
                        .push(mem.read(bytecode_addr + (word_index * BYTES_PER_WORD) as u64, 8));
                }
            }

            // Walk over the words just captured rather than memory again;
            // the ones the walk skips are the ones the machine never loads.
            let bitmap = {
                let words = &ctx.precompiled.input_data[1..];
                let mut bitmap = vec![0u64; bitmap_words(count)];
                walk_jump_dest_bitmap(count, |word_index| words[word_index], &mut bitmap);
                bitmap
            };

            store_bitmap(ctx, bitmap_addr, &bitmap);
        }
        EmulationMode::ConsumeMemReads => {
            // Only the count is preloaded (input_size = 8); the source words
            // that follow it reach the operation bus through data_ext_len, the
            // same shape the DMA ops use for their payload.
            assert_eq!(
                ctx.precompiled.input_data.len(),
                1,
                "opc_jump_dest() expects a single header word, found {}",
                ctx.precompiled.input_data.len()
            );
            ctx.data_ext_len = src_words(ctx.precompiled.input_data[0] as usize);
        }
    }

    ctx.c = 0;
    ctx.flag = false;
}

#[inline(always)]
pub fn op_jump_dest(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_jump_dest() is not implemented");
}

#[inline(always)]
pub fn ops_jump_dest(ctx: &InstContext, stats: &mut dyn OpStats) {
    let bitmap_addr = ctx.a;
    let bytecode_addr = ctx.b;
    let count = ctx.mem.read(EXTRA_PARAMS_ADDR, 8) as usize;
    stats.mem_align_read(EXTRA_PARAMS_ADDR, 1);

    check_preconditions(count, bitmap_addr, bytecode_addr);

    // Which words are loaded depends on the bytecode, so the read set has to be
    // walked rather than computed from the length.
    let mut runs: Vec<(usize, usize)> = Vec::new();
    compute_bitmap(&ctx.mem, bytecode_addr, count, |word_index, _| {
        push_read_run(&mut runs, word_index)
    });

    // mem_align_read bills addr, addr+8, ... so a sparse read set is reported
    // one contiguous run at a time — the same grouping the mops encoder uses.
    for (first, len) in runs {
        stats.mem_align_read(bytecode_addr + (first * BYTES_PER_WORD) as u64, len);
    }
    stats.mem_align_write(bitmap_addr, bitmap_words(count));

    // Area is one row per source word whether or not its load happens: a
    // skipped word still needs its row to carry the state and emit a zero
    // bitmap byte.
    // src_words(count) - 1 because JUMP_DEST_COST is the fixed cost
    stats.set_variable_cost((src_words(count) - 1) as u64 * JUMP_DEST_COST);
}
