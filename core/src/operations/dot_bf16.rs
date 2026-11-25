//! `dot_bf16`: exact dot product of two BF16 vectors, rounded once to `f32` (experimental,
//! emulator only). The semantics are in `zisk_definitions::dot_bf16` and in
//! `docs/dot-bf16-plan.md`.
//!
//! `b` is the address of a parameter block of six `u64` (`SyscallDotBf16Params` in ziskos):
//!
//! ```text
//! [0] x address   [1] y address   [2] n, the number of values
//! [3] flags: 0 = x is BF16; 1 = x is MXFP4 (E2M1 values, an E8M0 scale per 32);
//!     2 = x is NVFP4 (E2M1 values, an E4M3 scale per 16). y is always BF16
//! [4] block: values per block, 0 for one block of n values
//! [5] out: address of n/block f32 block results (u32 bits), or 0 for none
//! [6] with flags 1 and 2 only: address of x's scales, one byte per FP4 block
//! ```
//!
//! The result `c` is the `f32` bits of `Σ x[i]·y[i]`, computed exactly and rounded once to
//! nearest even; the result of each block is written to `out` when it is not 0. With FP4 weights
//! `x[i]` is `e2m1(x[i]) · scale[i / 32 or 16]`; an NVFP4 per-tensor scale is left to the caller.
//!
//! The vectors, the scales, each block and `out` are made of whole, aligned 64-bit words, so the
//! planned state machine can read one word of BF16 values per row (4 multiply-adds): the
//! addresses 8-byte aligned (unless `n` is 0), `n` and `block` multiples of 4, or of the FP4
//! block (32 or 16), and `n` at most 2^20. A last, partial word of scales is read in full and its
//! extra bytes are ignored. There is no state machine yet: the operation runs in the emulator and
//! its cost is estimated, but a program using it cannot be proven, and the modes of this emulator
//! that generate or consume proof inputs panic.
//!
//! The minimal trace of one call, as for `dot_f8`, is a header of `input_size` = 7 words (the
//! first 6 words of the parameter block and the result), followed by the data as words
//! (`data_ext_len`):
//!
//! ```text
//! flags 0:     x, y, n, flags, block, out, c, x as words, y as words
//! flags 1, 2:  x, y, n, flags, block, out, c, scales address, x as words (n/16),
//!              scales as words (n/32 or n/16 bytes, rounded up to words), y as words
//! ```

use zisk_definitions::{
    dot_bf16_fp4_block, dot_bf16_fp4_reference, dot_bf16_reference, DOT_BF16_FLAGS_BF16,
    DOT_BF16_FLAGS_MXFP4, DOT_BF16_FLAGS_NVFP4, DOT_BF16_MAX_N,
};

use crate::{
    zisk_ops::OpStats, EmulationMode, InstContext, DOT_BF16_MACS_PER_ROW, DOT_BF16_ROW_COST,
};

/// BF16 values per 64-bit word.
const PER_WORD: u64 = 4;

/// The checked parameter block of a `dot_bf16` call.
struct DotBf16Params {
    x: u64,
    y: u64,
    n: u64,
    flags: u64,
    block: u64,
    out: u64,
    /// The FP4 weights' scales (flags 1 and 2), else 0.
    scales: u64,
}

impl DotBf16Params {
    fn is_fp4(&self) -> bool {
        self.flags != DOT_BF16_FLAGS_BF16
    }

    /// Words of the BF16 vector `y` (and of `x` with flags 0).
    fn words(&self) -> u64 {
        self.n / PER_WORD
    }

    /// Words of the FP4 vector `x`: 16 values per word.
    fn fp4_words(&self) -> u64 {
        self.n / 16
    }

    /// Bytes of scales, one per FP4 block.
    fn scale_bytes(&self) -> u64 {
        self.n / dot_bf16_fp4_block(self.flags) as u64
    }

    /// Words holding the scales; a last, partial word is read in full.
    fn scale_words(&self) -> u64 {
        self.scale_bytes().div_ceil(8)
    }

    fn blocks(&self) -> u64 {
        if self.n == 0 {
            0
        } else {
            self.n / self.block
        }
    }
}

/// Reads and checks the parameter block at `addr`: 6 words, or 7 for FP4 weights.
fn dot_bf16_params(ctx: &InstContext, addr: u64) -> DotBf16Params {
    let where_ = || format!("(PC:0x{:08X} STEP:{})", ctx.pc, ctx.step);
    assert!(addr & 0x7 == 0, "dot_bf16: parameter block 0x{addr:08X} is not 8-byte aligned");
    let x = ctx.mem.read(addr, 8);
    let y = ctx.mem.read(addr + 8, 8);
    let n = ctx.mem.read(addr + 16, 8);
    let flags = ctx.mem.read(addr + 24, 8);
    let block = ctx.mem.read(addr + 32, 8);
    let out = ctx.mem.read(addr + 40, 8);
    assert!(
        flags == DOT_BF16_FLAGS_BF16
            || flags == DOT_BF16_FLAGS_MXFP4
            || flags == DOT_BF16_FLAGS_NVFP4,
        "dot_bf16: flags={flags} must be 0 (BF16), 1 (MXFP4) or 2 (NVFP4) {}",
        where_()
    );
    let scales = if flags == DOT_BF16_FLAGS_BF16 { 0 } else { ctx.mem.read(addr + 48, 8) };
    let block = if block == 0 { n } else { block };
    // Values per unit: a word of BF16 (4), or a whole FP4 block (32 or 16)
    let unit =
        if flags == DOT_BF16_FLAGS_BF16 { PER_WORD } else { dot_bf16_fp4_block(flags) as u64 };
    // With n = 0 nothing is read or written, so any addresses are accepted (Rust's empty slices
    // have dangling, unaligned pointers)
    assert!(
        n == 0 || ((x | y | out | scales) & 0x7 == 0),
        "dot_bf16: x=0x{x:08X}, y=0x{y:08X}, out=0x{out:08X} and scales=0x{scales:08X} must be \
         8-byte aligned {}",
        where_()
    );
    assert!(
        n <= DOT_BF16_MAX_N && n % unit == 0 && (n == 0 || (block % unit == 0 && n % block == 0)),
        "dot_bf16: n={n} must be at most 2^20 and a multiple of {unit}, and block={block} a \
         multiple of {unit} dividing n {}",
        where_()
    );
    DotBf16Params { x, y, n, flags, block, out, scales }
}

/// Reads `words` aligned words at `addr` as bytes.
fn read_bytes(ctx: &InstContext, addr: u64, words: u64) -> Vec<u8> {
    let mut v = Vec::with_capacity(8 * words as usize);
    for w in 0..words {
        v.extend_from_slice(&ctx.mem.read(addr + 8 * w, 8).to_le_bytes());
    }
    v
}

/// Reads `words` aligned words at `addr` as BF16 values.
fn read_bf16(ctx: &InstContext, addr: u64, words: u64) -> Vec<u16> {
    read_bytes(ctx, addr, words).chunks_exact(2).map(|b| u16::from_le_bytes([b[0], b[1]])).collect()
}

pub fn opc_dot_bf16(ctx: &mut InstContext) {
    match ctx.emulation_mode {
        EmulationMode::Mem => {
            let p = dot_bf16_params(ctx, ctx.b);
            let y = read_bf16(ctx, p.y, p.words());
            let mut results = vec![0u32; p.blocks() as usize];
            let (n, block) = (p.n as usize, p.block as usize);
            let total = if p.is_fp4() {
                let x = read_bytes(ctx, p.x, p.fp4_words());
                let scales = read_bytes(ctx, p.scales, p.scale_words());
                dot_bf16_fp4_reference(p.flags, &x, &scales, &y, n, block, Some(&mut results))
            } else {
                let x = read_bf16(ctx, p.x, p.words());
                dot_bf16_reference(&x, &y, n, block, Some(&mut results))
            };
            if p.out != 0 {
                // Two f32 results per word, the first one in the low half
                for (w, pair) in results.chunks(2).enumerate() {
                    let lo = pair[0] as u64;
                    let hi = pair.get(1).map_or(0, |&r| r as u64);
                    if pair.len() == 2 {
                        ctx.mem.write(p.out + 8 * w as u64, lo | hi << 32, 8);
                    } else {
                        ctx.mem.write(p.out + 8 * w as u64, lo, 4);
                    }
                }
            }
            ctx.c = total as u64;
            ctx.flag = false;
        }
        EmulationMode::GenerateMemReads | EmulationMode::ConsumeMemReads => {
            panic!(
                "dot_bf16 is experimental and emulator only: no state machine proves it yet \
                 (PC:0x{:08X} STEP:{})",
                ctx.pc, ctx.step
            );
        }
    }
}

/// Unimplemented. DotBf16 can only be called from the system call context via InstContext.
#[inline(always)]
pub fn op_dot_bf16(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_dot_bf16() is not implemented");
}

/// Cost model: the parameter block and the vectors (and FP4 scales) are read as aligned 64-bit
/// words, the block results are written as words, and the planned state machine uses one row per
/// 4 multiply-adds, that is per word of BF16 values, whatever the weights' format.
pub fn ops_dot_bf16(ctx: &InstContext, stats: &mut dyn OpStats) {
    let p = dot_bf16_params(ctx, ctx.b);
    stats.mem_align_read(ctx.b, if p.is_fp4() { 7 } else { 6 });
    if p.n > 0 {
        if p.is_fp4() {
            stats.mem_align_read(p.x, p.fp4_words() as usize);
            stats.mem_align_read(p.scales, p.scale_words() as usize);
        } else {
            stats.mem_align_read(p.x, p.words() as usize);
        }
        stats.mem_align_read(p.y, p.words() as usize);
        if p.out != 0 {
            stats.mem_align_write(p.out, p.blocks().div_ceil(2) as usize);
        }
    }
    stats.set_variable_cost(p.n.div_ceil(DOT_BF16_MACS_PER_ROW) * DOT_BF16_ROW_COST);
}
