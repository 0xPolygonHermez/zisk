//! `dot_f8`: exact dot product of two vectors of int8 or 8-bit (or 4-bit) floats (experimental,
//! emulator only).
//!
//! `b` is the address of a parameter block of six `u64` (`SyscallDotF8Params` in ziskos):
//!
//! ```text
//! [0] x address   [1] y address   [2] n, the number of values
//! [3] formats: bits 0..8 the format of x, bits 8..16 the format of y, the other bits 0
//! [4] block: values per block, 0 for one block of n values
//! [5] out: address of n/block i64 block sums, or 0 for none
//! ```
//!
//! The formats are in `zisk_definitions::dot_f8`: int8, E4M3, E4M3FNUZ and E2M1 (FP4). Every
//! value decodes to an integer number of units, so the products and sums are exact integers. The
//! result `c` is the sum over all `n` values, in units `2^-(unit_shift(x) + unit_shift(y))`, and
//! the sum of each block is written to `out` when it is not 0.
//!
//! Both vectors, each block and `out` must be made of whole, aligned 64-bit words, so the planned
//! state machine can read one word of each vector per row: the addresses 8-byte aligned, and `n`
//! and `block` multiples of 8 values (16 for E2M1). `n` is at most 2^26, so the sums cannot
//! overflow. There is no state machine yet: the operation runs in the emulator and its cost is
//! estimated, but a program using it cannot be proven, and the modes of this emulator that
//! generate or consume proof inputs panic.
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

use zisk_definitions::{dot_f8_bits, dot_f8_reference, DOT_F8_FORMATS, DOT_F8_MAX_N};

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
}

impl DotF8Params {
    fn x_words(&self) -> u64 {
        self.n * dot_f8_bits(self.fx) as u64 / 64
    }

    fn y_words(&self) -> u64 {
        self.n * dot_f8_bits(self.fy) as u64 / 64
    }

    fn blocks(&self) -> u64 {
        if self.n == 0 {
            0
        } else {
            self.n / self.block
        }
    }
}

/// Reads and checks the parameter block at `addr`.
fn dot_f8_params(ctx: &InstContext, addr: u64) -> DotF8Params {
    let where_ = || format!("(PC:0x{:08X} STEP:{})", ctx.pc, ctx.step);
    assert!(addr & 0x7 == 0, "dot_f8: parameter block 0x{addr:08X} is not 8-byte aligned");
    let x = ctx.mem.read(addr, 8);
    let y = ctx.mem.read(addr + 8, 8);
    let n = ctx.mem.read(addr + 16, 8);
    let formats = ctx.mem.read(addr + 24, 8);
    let block = ctx.mem.read(addr + 32, 8);
    let out = ctx.mem.read(addr + 40, 8);
    let (fx, fy) = (formats as u8, (formats >> 8) as u8);
    assert!(
        formats >> 16 == 0 && fx < DOT_F8_FORMATS && fy < DOT_F8_FORMATS,
        "dot_f8: invalid formats 0x{formats:X} {}",
        where_()
    );
    let block = if block == 0 { n } else { block };
    // Values per 64-bit word of the vector with the fewest bits per value
    let per_word = 64 / dot_f8_bits(fx).min(dot_f8_bits(fy)) as u64;
    assert!(
        x & 0x7 == 0 && y & 0x7 == 0 && out & 0x7 == 0,
        "dot_f8: x=0x{x:08X}, y=0x{y:08X} and out=0x{out:08X} must be 8-byte aligned {}",
        where_()
    );
    assert!(
        n <= DOT_F8_MAX_N
            && n % per_word == 0
            && (n == 0 || (block % per_word == 0 && n % block == 0)),
        "dot_f8: n={n} must be at most 2^26 and a multiple of {per_word}, and block={block} a \
         multiple of {per_word} dividing n {}",
        where_()
    );
    DotF8Params { x, y, n, fx, fy, block, out }
}

/// Reads `words` aligned words at `addr` as bytes.
fn read_bytes(ctx: &InstContext, addr: u64, words: u64) -> Vec<u8> {
    let mut v = Vec::with_capacity(8 * words as usize);
    for w in 0..words {
        v.extend_from_slice(&ctx.mem.read(addr + 8 * w, 8).to_le_bytes());
    }
    v
}

pub fn opc_dot_f8(ctx: &mut InstContext) {
    match ctx.emulation_mode {
        EmulationMode::Mem => {
            let p = dot_f8_params(ctx, ctx.b);
            let x = read_bytes(ctx, p.x, p.x_words());
            let y = read_bytes(ctx, p.y, p.y_words());
            let mut sums = vec![0i64; p.blocks() as usize];
            let total = dot_f8_reference(
                p.fx,
                &x,
                p.fy,
                &y,
                p.n as usize,
                p.block.max(1) as usize,
                Some(&mut sums),
            );
            if p.out != 0 {
                for (b, s) in sums.iter().enumerate() {
                    ctx.mem.write(p.out + 8 * b as u64, *s as u64, 8);
                }
            }
            ctx.c = total as u64;
            ctx.flag = false;
        }
        EmulationMode::GenerateMemReads | EmulationMode::ConsumeMemReads => {
            panic!(
                "dot_f8 is experimental and emulator only: no state machine proves it yet \
                 (PC:0x{:08X} STEP:{})",
                ctx.pc, ctx.step
            );
        }
    }
}

/// Unimplemented. DotF8 can only be called from the system call context via InstContext.
#[inline(always)]
pub fn op_dot_f8(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_dot_f8() is not implemented");
}

/// Cost model: the parameter block and both vectors are read as aligned 64-bit words, the block
/// sums are written as words, and the planned state machine uses one row per 8 multiply-adds.
pub fn ops_dot_f8(ctx: &InstContext, stats: &mut dyn OpStats) {
    let p = dot_f8_params(ctx, ctx.b);
    stats.mem_align_read(ctx.b, 6);
    if p.n > 0 {
        stats.mem_align_read(p.x, p.x_words() as usize);
        stats.mem_align_read(p.y, p.y_words() as usize);
        if p.out != 0 {
            stats.mem_align_write(p.out, p.blocks() as usize);
        }
    }
    stats.set_variable_cost(p.n.div_ceil(DOT_F8_MACS_PER_ROW) * DOT_F8_ROW_COST);
}
