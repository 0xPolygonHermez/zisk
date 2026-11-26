//! `dot_f8`: exact dot product of two vectors of int8 or 8-bit (or 4-bit) floats.
//!
//! `b` is the address of a parameter block of six `u64` (`SyscallDotF8Params` in ziskos):
//!
//! ```text
//! [0] x address   [1] y address   [2] n, the number of values
//! [3] formats: bits 0..8 the format of x, bits 8..16 the format of y, bit 16 DOT_F8_REPEAT_Y,
//!     the other bits 0
//! [4] block: values per block, 0 for one block of n values
//! [5] out: address of n/block i64 block sums, or 0 for none
//! ```
//!
//! With `DOT_F8_REPEAT_Y`, `y` holds only `block` values (`block` must not be 0) and is read
//! again from its start for every block: one call computes a matrix-vector product, `x` being
//! the `n / block` rows of `block` values.
//!
//! The formats are in `zisk_definitions::dot_f8`: int8, E4M3, E4M3FNUZ and E2M1 (FP4). Every
//! value decodes to an integer number of units, so the products and sums are exact integers. The
//! result `c` is the sum over all `n` values, in units `2^-(unit_shift(x) + unit_shift(y))`, and
//! the sum of each block is written to `out` when it is not 0.
//!
//! Both vectors, each block and `out` must be made of whole, aligned 64-bit words, so the
//! state machine can read one word of each vector per row: the addresses 8-byte aligned, and `n`
//! and `block` multiples of 8 values (16 for E2M1). `n` is at most 2^24, so a call fits in one
//! instance and the sums cannot overflow. It is proven by the `DotF8` air (precompiles/dot_f8).
//!
//! The assembly emulator also runs it (`emulator-asm/src/dot_f8/`), in the fast, ROM histogram
//! and minimal trace modes. Its minimal trace for one call is a header of `input_size` = 7 words,
//! the parameter block and then the result (as `dma_memcmp` puts its result in its header, so a
//! replay does not need the data to get `c`), followed by both vectors as words, as for the
//! variable-length DMA operations (`data_ext_len`):
//!
//! ```text
//! x, y, n, formats, block, out, c, x as words, y as words
//! ```
//!
//! With `DOT_F8_REPEAT_Y`, y appears once (its `block` values); the state machine reads it again
//! for every block.

use zisk_definitions::{
    dot_f8_bits, dot_f8_reference_with, DOT_F8_FORMATS, DOT_F8_MAX_N, DOT_F8_REPEAT_Y,
};

use crate::{zisk_ops::OpStats, EmulationMode, InstContext, DOT_F8_MACS_PER_ROW, DOT_F8_ROW_COST};

/// The checked parameter block of a `dot_f8` call.
struct DotF8Params {
    x: u64,
    y: u64,
    n: u64,
    fx: u8,
    fy: u8,
    block: u64,
    out: u64,
    repeat_y: bool,
}

impl DotF8Params {
    fn x_words(&self) -> u64 {
        self.n * dot_f8_bits(self.fx) as u64 / 64
    }

    fn y_words(&self) -> u64 {
        dot_f8_y_values(self.n, self.block, self.repeat_y) * dot_f8_bits(self.fy) as u64 / 64
    }

    fn blocks(&self) -> u64 {
        if self.n == 0 {
            0
        } else {
            self.n / self.block
        }
    }
}

/// Values of y: `n`, or with `DOT_F8_REPEAT_Y` one block (none when `n` is 0).
fn dot_f8_y_values(n: u64, block: u64, repeat_y: bool) -> u64 {
    if repeat_y {
        block.min(n)
    } else {
        n
    }
}

fn dot_f8_header(ctx: &InstContext, addr: u64) -> [u64; 6] {
    assert!(addr & 7 == 0, "dot_f8: parameter block 0x{addr:08X} is not 8-byte aligned");
    std::array::from_fn(|i| ctx.mem.read(addr + 8 * i as u64, 8))
}

fn dot_f8_params([x, y, n, formats, block, out]: [u64; 6]) -> DotF8Params {
    let (fx, fy, repeat_y) = (formats as u8, (formats >> 8) as u8, formats & DOT_F8_REPEAT_Y != 0);
    assert!(
        formats >> 17 == 0 && fx < DOT_F8_FORMATS && fy < DOT_F8_FORMATS,
        "dot_f8: invalid formats 0x{formats:X}"
    );
    assert!(!repeat_y || block != 0, "dot_f8: DOT_F8_REPEAT_Y needs block != 0");
    let block = if block == 0 { n } else { block };
    // Values per 64-bit word of the vector with the fewest bits per value
    let per_word = 64 / dot_f8_bits(fx).min(dot_f8_bits(fy)) as u64;
    assert!(
        x & 0x7 == 0 && y & 0x7 == 0 && out & 0x7 == 0,
        "dot_f8: x=0x{x:08X}, y=0x{y:08X} and out=0x{out:08X} must be 8-byte aligned"
    );
    assert!(
        n <= DOT_F8_MAX_N
            && n % per_word == 0
            && block % per_word == 0
            && (n == 0 || n % block == 0),
        "dot_f8: n={n} must be at most 2^24 and a multiple of {per_word}, and block={block} a \
         multiple of {per_word} dividing n"
    );
    DotF8Params { x, y, n, fx, fy, block, out, repeat_y }
}

/// Reads `words` aligned words at `addr` as bytes.
fn read_bytes(ctx: &InstContext, addr: u64, words: u64) -> Vec<u8> {
    let mut v = Vec::with_capacity(8 * words as usize);
    for w in 0..words {
        v.extend_from_slice(&ctx.mem.read(addr + 8 * w, 8).to_le_bytes());
    }
    v
}

/// Runs the call: writes the block sums and returns the minimal trace words of the call (the
/// parameter block, the result and both vectors).
fn dot_f8_run(ctx: &mut InstContext) -> Vec<u64> {
    let header = dot_f8_header(ctx, ctx.b);
    let p = dot_f8_params(header);
    let x = read_bytes(ctx, p.x, p.x_words());
    let y = read_bytes(ctx, p.y, p.y_words());
    let mut sums = vec![0i64; p.blocks() as usize];
    let total = dot_f8_reference_with(
        p.fx,
        &x,
        p.fy,
        &y,
        p.n as usize,
        p.block.max(1) as usize,
        p.repeat_y,
        Some(&mut sums),
    );
    if p.out != 0 {
        for (b, s) in sums.iter().enumerate() {
            ctx.mem.write(p.out + 8 * b as u64, *s as u64, 8);
        }
    }
    let mut trace = Vec::from(header);
    trace.push(total as u64);
    for bytes in [x, y] {
        trace.extend(bytes.chunks(8).map(|w| u64::from_le_bytes(w.try_into().unwrap())));
    }
    trace
}

/// Vector words of the call whose minimal trace header is `h` (parameter block and result).
pub fn dot_f8_data_ext_len(h: &[u64]) -> usize {
    let p = dot_f8_params(h[..6].try_into().expect("dot_f8 descriptor"));
    (p.x_words() + p.y_words()) as usize
}

pub fn opc_dot_f8(ctx: &mut InstContext) {
    match ctx.emulation_mode {
        EmulationMode::Mem => ctx.c = dot_f8_run(ctx)[6],
        EmulationMode::GenerateMemReads => {
            let trace = dot_f8_run(ctx);
            ctx.c = trace[6];
            ctx.precompiled.input_data = trace;
            ctx.precompiled.step = ctx.step;
        }
        EmulationMode::ConsumeMemReads => {
            let h = &ctx.precompiled.input_data;
            assert_eq!(h.len(), 7, "dot_f8: the minimal trace header has 7 words");
            ctx.c = h[6];
            ctx.data_ext_len = dot_f8_data_ext_len(h);
            ctx.precompiled.step = ctx.step;
        }
    }
    ctx.flag = false;
}

/// Unimplemented. DotF8 can only be called from the system call context via InstContext.
#[inline(always)]
pub fn op_dot_f8(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_dot_f8() is not implemented");
}

/// Cost model: the parameter block and both vectors are read as aligned 64-bit words (y once per
/// block with `DOT_F8_REPEAT_Y`), the block sums are written as words, and the state machine
/// uses one row per 8 multiply-adds.
pub fn ops_dot_f8(ctx: &InstContext, stats: &mut dyn OpStats) {
    let p = dot_f8_params(dot_f8_header(ctx, ctx.b));
    stats.mem_align_read(ctx.b, 6);
    if p.n > 0 {
        stats.mem_align_read(p.x, p.x_words() as usize);
        let y_reads = if p.repeat_y { p.blocks() } else { 1 };
        for _ in 0..y_reads {
            stats.mem_align_read(p.y, p.y_words() as usize);
        }
        if p.out != 0 {
            stats.mem_align_write(p.out, p.blocks() as usize);
        }
    }
    stats.set_variable_cost(p.n.div_ceil(DOT_F8_MACS_PER_ROW).max(1) * DOT_F8_ROW_COST);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "multiple of 16")]
    fn empty_fp4_call_still_requires_a_whole_block_word() {
        let formats = zisk_definitions::DOT_F8_E2M1 as u64;
        dot_f8_params([0, 0, 0, formats, (1 << 32) + 8, 0]);
    }

    #[test]
    fn repeat_y_trace_round_trips_through_replay() {
        let mut ctx = InstContext::new();
        let base = 0xa000_0000;
        ctx.mem.add_write_section(base, 128);
        ctx.b = base;
        let descriptor = [base + 48, base + 64, 16, DOT_F8_REPEAT_Y, 8, base + 72];
        for (i, &v) in descriptor.iter().enumerate() {
            ctx.mem.write(base + 8 * i as u64, v, 8);
        }
        let words = [[1; 8], [2; 8], [3; 8]].map(u64::from_le_bytes);
        for (i, &v) in words.iter().enumerate() {
            ctx.mem.write(base + 48 + 8 * i as u64, v, 8);
        }
        ctx.emulation_mode = EmulationMode::GenerateMemReads;
        opc_dot_f8(&mut ctx);
        assert_eq!(ctx.c, 72);
        assert_eq!(ctx.mem.read(base + 72, 8), 24);
        assert_eq!(ctx.mem.read(base + 80, 8), 48);
        assert_eq!(&ctx.precompiled.input_data[..6], &descriptor);
        assert_eq!(&ctx.precompiled.input_data[7..], &words);

        ctx.precompiled.input_data.truncate(7);
        ctx.c = 0;
        ctx.emulation_mode = EmulationMode::ConsumeMemReads;
        opc_dot_f8(&mut ctx);
        assert_eq!(ctx.c, 72);
        assert_eq!(ctx.data_ext_len, words.len());
    }
}
