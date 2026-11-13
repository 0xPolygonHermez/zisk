//! `dot_i8`: dot product of two int8 vectors (experimental, emulator only).
//!
//! `b` is the address of a parameter block of three `u64`: the address of vector `x`, the address
//! of vector `y` and their length `n` in bytes. The result `c` is `Σ x[i]·y[i]` over signed bytes,
//! wrapped to `i32` and sign-extended to 64 bits, as Rust's wrapping `i32` sum.
//!
//! Both addresses must be 8-byte aligned and `n` a multiple of 8, so the planned state machine can
//! read one 64-bit word of each vector per row (8 multiply-adds). There is no state machine yet:
//! the operation runs in the emulator and its cost is estimated, but a program using it cannot be
//! proven, and the modes that generate proof inputs panic.

use crate::{zisk_ops::OpStats, EmulationMode, InstContext, DOT_I8_MACS_PER_ROW, DOT_I8_ROW_COST};

/// Reads and checks the parameter block at `addr`: (x address, y address, length in bytes).
fn dot_i8_params(ctx: &InstContext, addr: u64) -> (u64, u64, u64) {
    assert!(addr & 0x7 == 0, "dot_i8: parameter block 0x{addr:08X} is not 8-byte aligned");
    let x = ctx.mem.read(addr, 8);
    let y = ctx.mem.read(addr + 8, 8);
    let n = ctx.mem.read(addr + 16, 8);
    assert!(
        x & 0x7 == 0 && y & 0x7 == 0 && n & 0x7 == 0,
        "dot_i8: x=0x{x:08X} and y=0x{y:08X} must be 8-byte aligned and n={n} a multiple of 8 \
         (PC:0x{:08X} STEP:{})",
        ctx.pc,
        ctx.step
    );
    (x, y, n)
}

/// Dot product of the signed bytes of two 64-bit words.
#[inline(always)]
fn dot_word(x: u64, y: u64) -> i32 {
    let (x, y) = (x.to_le_bytes(), y.to_le_bytes());
    let mut acc = 0i32;
    for k in 0..8 {
        acc += x[k] as i8 as i32 * y[k] as i8 as i32;
    }
    acc
}

pub fn opc_dot_i8(ctx: &mut InstContext) {
    match ctx.emulation_mode {
        EmulationMode::Mem => {
            let (x, y, n) = dot_i8_params(ctx, ctx.b);
            let mut acc = 0i32;
            for w in 0..n / 8 {
                let xw = ctx.mem.read(x + 8 * w, 8);
                let yw = ctx.mem.read(y + 8 * w, 8);
                acc = acc.wrapping_add(dot_word(xw, yw));
            }
            ctx.c = acc as i64 as u64;
            ctx.flag = false;
        }
        EmulationMode::GenerateMemReads | EmulationMode::ConsumeMemReads => {
            panic!(
                "dot_i8 is experimental and emulator only: no state machine proves it yet \
                 (PC:0x{:08X} STEP:{})",
                ctx.pc, ctx.step
            );
        }
    }
}

/// Unimplemented. DotI8 can only be called from the system call context via InstContext.
#[inline(always)]
pub fn op_dot_i8(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_dot_i8() is not implemented");
}

/// Cost model: the parameter block and both vectors are read as aligned 64-bit words, and the
/// planned state machine uses one row per 8 multiply-adds.
pub fn ops_dot_i8(ctx: &InstContext, stats: &mut dyn OpStats) {
    let (x, y, n) = dot_i8_params(ctx, ctx.b);
    let words = (n / 8) as usize;
    stats.mem_align_read(ctx.b, 3);
    if words > 0 {
        stats.mem_align_read(x, words);
        stats.mem_align_read(y, words);
    }
    stats.set_variable_cost(n.div_ceil(DOT_I8_MACS_PER_ROW) * DOT_I8_ROW_COST);
}

#[cfg(test)]
mod tests {
    use super::dot_word;

    #[test]
    fn dot_word_uses_signed_bytes() {
        let x = u64::from_le_bytes([1, 2, 3, 4, 0x80, 0x7f, 0xff, 0]);
        let y = u64::from_le_bytes([5, 6, 7, 8, 0x80, 0x7f, 0xff, 9]);
        // 5 + 12 + 21 + 32 + (-128)² + 127² + (-1)² + 0
        assert_eq!(dot_word(x, y), 70 + 16384 + 16129 + 1);
    }
}
