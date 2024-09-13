use tiny_keccak::keccakf;

use crate::{InstContext, ZiskOperation, SYS_ADDR};
use std::{collections::HashMap, num::Wrapping};

// Constant values used in operation functions
const M64: u64 = 0xFFFFFFFFFFFFFFFF;

// Main binary operations

/// Sets flag to true (and c to 0)
#[inline(always)]
pub const fn op_flag(_a: u64, _b: u64) -> (u64, bool) {
    (0, true)
}
#[inline(always)]
pub fn opc_flag(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_flag(ctx.a, ctx.b);
}

/// Copies register b into c
#[inline(always)]
pub const fn op_copyb(_a: u64, b: u64) -> (u64, bool) {
    (b, false)
}
#[inline(always)]
pub fn opc_copyb(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_copyb(ctx.a, ctx.b);
}

/// Converts b from a signed 8-bits number in the range [-128, +127] into a signed 64-bit number of
/// the same value, and stores the result in c
#[inline(always)]
pub const fn op_signextend_b(_a: u64, b: u64) -> (u64, bool) {
    ((b as i8) as u64, false)
}
#[inline(always)]
pub fn opc_signextend_b(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_signextend_b(ctx.a, ctx.b);
}

/// Converts b from a signed 16-bits number in the range [-32768, 32767] into a signed 64-bit number
/// of the same value, and stores the result in c
#[inline(always)]
pub const fn op_signextend_h(_a: u64, b: u64) -> (u64, bool) {
    ((b as i16) as u64, false)
}
#[inline(always)]
pub fn opc_signextend_h(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_signextend_h(ctx.a, ctx.b);
}

/// Converts b from a signed 32-bits number in the range [-2147483648, 2147483647] into a signed
/// 64-bit number of the same value, and stores the result in c
#[inline(always)]
pub const fn op_signextend_w(_a: u64, b: u64) -> (u64, bool) {
    ((b as i32) as u64, false)
}
#[inline(always)]
pub fn opc_signextend_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_signextend_w(ctx.a, ctx.b);
}

/// Adds a and b, and stores the result in c
#[inline(always)]
pub fn op_add(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a) + Wrapping(b)).0, false)
}
#[inline(always)]
pub fn opc_add(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_add(ctx.a, ctx.b);
}

/// Adds a and b as 32-bit unsigned values, and stores the result in c
#[inline(always)]
pub fn op_add_w(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a as i32) + Wrapping(b as i32)).0 as u64, false)
}
#[inline(always)]
pub fn opc_add_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_add_w(ctx.a, ctx.b);
}

/// Subs a and b, and stores the result in c
#[inline(always)]
pub fn op_sub(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a) - Wrapping(b)).0, false)
}
#[inline(always)]
pub fn opc_sub(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_sub(ctx.a, ctx.b);
}

/// Subs a and b as 32-bit unsigned values, and stores the result in c
#[inline(always)]
pub fn op_sub_w(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a as i32) - Wrapping(b as i32)).0 as u64, false)
}
#[inline(always)]
pub fn opc_sub_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_sub_w(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_sll(a: u64, b: u64) -> (u64, bool) {
    (a << (b & 0x3f), false)
}
#[inline(always)]
pub fn opc_sll(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_sll(ctx.a, ctx.b);
}

#[inline(always)]
pub fn op_sll_w(a: u64, b: u64) -> (u64, bool) {
    (((Wrapping(a as u32) << (b & 0x3f) as usize).0 as i32) as u64, false)
}
#[inline(always)]
pub fn opc_sll_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_sll_w(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_sra(a: u64, b: u64) -> (u64, bool) {
    (((a as i64) >> (b & 0x3f)) as u64, false)
}
#[inline(always)]
pub fn opc_sra(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_sra(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_srl(a: u64, b: u64) -> (u64, bool) {
    (a >> (b & 0x3f), false)
}
#[inline(always)]
pub fn opc_srl(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_srl(ctx.a, ctx.b);
}

#[inline(always)]
pub fn op_sra_w(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a as i32) >> (b & 0x3f) as usize).0 as u64, false)
}
#[inline(always)]
pub fn opc_sra_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_sra_w(ctx.a, ctx.b);
}

#[inline(always)]
pub fn op_srl_w(a: u64, b: u64) -> (u64, bool) {
    (((Wrapping(a as u32) >> (b & 0x3f) as usize).0 as i32) as u64, false)
}
#[inline(always)]
pub fn opc_srl_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_srl_w(ctx.a, ctx.b);
}

/// If a equals b, returns c=1, flag=true
#[inline(always)]
pub const fn op_eq(a: u64, b: u64) -> (u64, bool) {
    if a == b {
        (1, true)
    } else {
        (0, false)
    }
}
#[inline(always)]
pub fn opc_eq(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_eq(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_eq_w(a: u64, b: u64) -> (u64, bool) {
    if (a as i32) == (b as i32) {
        (1, true)
    } else {
        (0, false)
    }
}
#[inline(always)]
pub fn opc_eq_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_eq_w(ctx.a, ctx.b);
}

/// If a is strictly less than b, returns c=1, flag=true
#[inline(always)]
pub const fn op_ltu(a: u64, b: u64) -> (u64, bool) {
    if a < b {
        (1, true)
    } else {
        (0, false)
    }
}
#[inline(always)]
pub fn opc_ltu(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_ltu(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_lt(a: u64, b: u64) -> (u64, bool) {
    if (a as i64) < (b as i64) {
        (1, true)
    } else {
        (0, false)
    }
}
#[inline(always)]
pub fn opc_lt(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_lt(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_ltu_w(a: u64, b: u64) -> (u64, bool) {
    if (a as u32) < (b as u32) {
        (1, true)
    } else {
        (0, false)
    }
}
#[inline(always)]
pub fn opc_ltu_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_ltu_w(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_lt_w(a: u64, b: u64) -> (u64, bool) {
    if (a as i32) < (b as i32) {
        (1, true)
    } else {
        (0, false)
    }
}
#[inline(always)]
pub fn opc_lt_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_lt_w(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_leu(a: u64, b: u64) -> (u64, bool) {
    if a <= b {
        (1, true)
    } else {
        (0, false)
    }
}
#[inline(always)]
pub fn opc_leu(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_leu(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_le(a: u64, b: u64) -> (u64, bool) {
    if (a as i64) <= (b as i64) {
        (1, true)
    } else {
        (0, false)
    }
}
#[inline(always)]
pub fn opc_le(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_le(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_leu_w(a: u64, b: u64) -> (u64, bool) {
    if (a as u32) <= (b as u32) {
        (1, true)
    } else {
        (0, false)
    }
}
#[inline(always)]
pub fn opc_leu_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_leu_w(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_le_w(a: u64, b: u64) -> (u64, bool) {
    if (a as i32) <= (b as i32) {
        (1, true)
    } else {
        (0, false)
    }
}
#[inline(always)]
pub fn opc_le_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_le_w(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_and(a: u64, b: u64) -> (u64, bool) {
    (a & b, false)
}
#[inline(always)]
pub fn opc_and(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_and(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_or(a: u64, b: u64) -> (u64, bool) {
    (a | b, false)
}
#[inline(always)]
pub fn opc_or(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_or(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_xor(a: u64, b: u64) -> (u64, bool) {
    (a ^ b, false)
}
#[inline(always)]
pub fn opc_xor(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_xor(ctx.a, ctx.b);
}

#[inline(always)]
pub fn op_mulu(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a) * Wrapping(b)).0, false)
}
#[inline(always)]
pub fn opc_mulu(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_mulu(ctx.a, ctx.b);
}

#[inline(always)]
pub fn op_mul(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a as i64) * Wrapping(b as i64)).0 as u64, false)
}
#[inline(always)]
pub fn opc_mul(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_mul(ctx.a, ctx.b);
}

#[inline(always)]
pub fn op_mul_w(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a as i32) * Wrapping(b as i32)).0 as u64, false)
}
#[inline(always)]
pub fn opc_mul_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_mul_w(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_muluh(a: u64, b: u64) -> (u64, bool) {
    (((a as u128 * b as u128) >> 64) as u64, false)
}
#[inline(always)]
pub fn opc_muluh(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_muluh(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_mulh(a: u64, b: u64) -> (u64, bool) {
    (((((a as i64) as i128) * ((b as i64) as i128)) >> 64) as u64, false)
}
#[inline(always)]
pub fn opc_mulh(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_mulh(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_mulsuh(a: u64, b: u64) -> (u64, bool) {
    (((((a as i64) as i128) * (b as i128)) >> 64) as u64, false)
}
#[inline(always)]
pub fn opc_mulsuh(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_mulsuh(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_divu(a: u64, b: u64) -> (u64, bool) {
    if b == 0 {
        return (M64, true);
    }

    (a / b, false)
}
#[inline(always)]
pub fn opc_divu(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_divu(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_div(a: u64, b: u64) -> (u64, bool) {
    if b == 0 {
        return (M64, true);
    }
    ((((a as i64) as i128) / ((b as i64) as i128)) as u64, false)
}
#[inline(always)]
pub fn opc_div(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_div(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_divu_w(a: u64, b: u64) -> (u64, bool) {
    if b as u32 == 0 {
        return (M64, true);
    }

    (((a as u32 / b as u32) as i32) as u64, false)
}
#[inline(always)]
pub fn opc_divu_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_divu_w(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_div_w(a: u64, b: u64) -> (u64, bool) {
    if b as i32 == 0 {
        return (M64, true);
    }

    ((((a as i32) as i64) / ((b as i32) as i64)) as u64, false)
}
#[inline(always)]
pub fn opc_div_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_div_w(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_remu(a: u64, b: u64) -> (u64, bool) {
    if b == 0 {
        return (a, true);
    }

    (a % b, false)
}
#[inline(always)]
pub fn opc_remu(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_remu(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_rem(a: u64, b: u64) -> (u64, bool) {
    if b == 0 {
        return (a, true);
    }

    ((((a as i64) as i128) % ((b as i64) as i128)) as u64, false)
}
#[inline(always)]
pub fn opc_rem(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_rem(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_remu_w(a: u64, b: u64) -> (u64, bool) {
    if (b as u32) == 0 {
        return ((a as i32) as u64, true);
    }

    ((((a as u32) % (b as u32)) as i32) as u64, false)
}
#[inline(always)]
pub fn opc_remu_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_remu_w(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_rem_w(a: u64, b: u64) -> (u64, bool) {
    if (b as i32) == 0 {
        return ((a as i32) as u64, true);
    }

    (((a as i32) % (b as i32)) as u64, false)
}
#[inline(always)]
pub fn opc_rem_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_rem_w(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_minu(a: u64, b: u64) -> (u64, bool) {
    //if op_s64(a) < op_s64(b)
    if a < b {
        (a, false)
    } else {
        (b, false)
    }
}
#[inline(always)]
pub fn opc_minu(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_minu(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_min(a: u64, b: u64) -> (u64, bool) {
    if (a as i64) < (b as i64) {
        (a, false)
    } else {
        (b, false)
    }
}
#[inline(always)]
pub fn opc_min(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_min(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_minu_w(a: u64, b: u64) -> (u64, bool) {
    if (a as u32) < (b as u32) {
        (a, false)
    } else {
        (b, false)
    }
}
#[inline(always)]
pub fn opc_minu_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_minu_w(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_min_w(a: u64, b: u64) -> (u64, bool) {
    if (a as i32) < (b as i32) {
        (a, false)
    } else {
        (b, false)
    }
}
#[inline(always)]
pub fn opc_min_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_min_w(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_maxu(a: u64, b: u64) -> (u64, bool) {
    //if op_s64(a) > op_s64(b)
    if a > b {
        (a, false)
    } else {
        (b, false)
    }
}
#[inline(always)]
pub fn opc_maxu(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_maxu(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_max(a: u64, b: u64) -> (u64, bool) {
    if (a as i64) > (b as i64) {
        (a, false)
    } else {
        (b, false)
    }
}
#[inline(always)]
pub fn opc_max(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_max(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_maxu_w(a: u64, b: u64) -> (u64, bool) {
    if (a as u32) > (b as u32) {
        (a, false)
    } else {
        (b, false)
    }
}
#[inline(always)]
pub fn opc_maxu_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_maxu_w(ctx.a, ctx.b);
}

#[inline(always)]
pub const fn op_max_w(a: u64, b: u64) -> (u64, bool) {
    if (a as i32) > (b as i32) {
        (a, false)
    } else {
        (b, false)
    }
}
#[inline(always)]
pub fn opc_max_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_max_w(ctx.a, ctx.b);
}

#[inline(always)]
pub fn opc_keccak(ctx: &mut InstContext) {
    // Get address from register a1 = x11
    let address = ctx.mem.read(SYS_ADDR + 10_u64 * 8, 8);

    // Allocate room for 25 u64 = 128 bytes = 1600 bits
    const WORDS: usize = 25;
    let mut data = [0u64; WORDS];

    // Read them from the address
    for (i, d) in data.iter_mut().enumerate() {
        *d = ctx.mem.read(address + (8 * i as u64), 8);
    }

    // Call keccakf
    keccakf(&mut data);

    // Write them from the address
    for (i, d) in data.iter().enumerate() {
        ctx.mem.write(address + (8 * i as u64), *d, 8);
    }

    ctx.c = 0;
    ctx.flag = false;
}
#[inline(always)]
pub fn op_keccak(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_keccak() is not implemented");
}
