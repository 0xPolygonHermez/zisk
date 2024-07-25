use crate::ZiskOperation;
use std::{collections::HashMap, num::Wrapping, sync::WaitTimeoutResult};

// Constant values used in operation functions
const M64: u64 = 0xFFFFFFFFFFFFFFFF;

// Main binary operations

/// Sets flag to true (and c to 0)
#[inline(always)]
fn op_flag(_a: u64, _b: u64) -> (u64, bool) {
    (0, true)
}

/// Copies register b into c
#[inline(always)]
fn op_copyb(_a: u64, b: u64) -> (u64, bool) {
    (b, false)
}

/// Converts b from a signed 8-bits number in the range [-128, +127] into a signed 64-bit number of
/// the same value, and stores the result in c
#[inline(always)]
fn op_signextend_b(_a: u64, b: u64) -> (u64, bool) {
    ((b as i8) as u64, false)
}

/// Converts b from a signed 16-bits number in the range [-32768, 32767] into a signed 64-bit number
/// of the same value, and stores the result in c
#[inline(always)]
fn op_signextend_h(_a: u64, b: u64) -> (u64, bool) {
    ((b as i16) as u64, false)
}

/// Converts b from a signed 32-bits number in the range [-2147483648, 2147483647] into a signed
/// 64-bit number of the same value, and stores the result in c
#[inline(always)]
fn op_signextend_w(_a: u64, b: u64) -> (u64, bool) {
    ((b as i32) as u64, false)
}

/// Adds a and b, and stores the result in c
#[inline(always)]
fn op_add(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a) + Wrapping(b)).0, false)
}

/// Adds a and b as 32-bit unsigned values, and stores the result in c
#[inline(always)]
fn op_add_w(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a as i32) + Wrapping(b as i32)).0 as u64, false)
}

#[inline(always)]
fn op_sub(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a) - Wrapping(b)).0, false)
}

#[inline(always)]
fn op_sub_w(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a as i32) - Wrapping(b as i32)).0 as u64, false)
}

#[inline(always)]
fn op_sll(a: u64, b: u64) -> (u64, bool) {
    (a << (b & 0x3f), false)
}

#[inline(always)]
fn op_sll_w(a: u64, b: u64) -> (u64, bool) {
    (((Wrapping(a as u32) << (b & 0x3f) as usize).0 as i32) as u64, false)
}

#[inline(always)]
fn op_sra(a: u64, b: u64) -> (u64, bool) {
    (((a as i64) >> (b & 0x3f)) as u64, false)
}

#[inline(always)]
fn op_srl(a: u64, b: u64) -> (u64, bool) {
    (a >> (b & 0x3f), false)
}

#[inline(always)]
fn op_sra_w(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a as i32) >> (b & 0x3f) as usize).0 as u64, false)
}

#[inline(always)]
fn op_srl_w(a: u64, b: u64) -> (u64, bool) {
    (((Wrapping(a as u32) >> (b & 0x3f) as usize).0 as i32) as u64, false)
}

/// If a equals b, returns c=1, flag=true
#[inline(always)]
fn op_eq(a: u64, b: u64) -> (u64, bool) {
    if a == b {
        (1, true)
    } else {
        (0, false)
    }
}

#[inline(always)]
fn op_eq_w(a: u64, b: u64) -> (u64, bool) {
    if (a as i32) == (b as i32) {
        (1, true)
    } else {
        (0, false)
    }
}

/// If a is strictly less than b, returns c=1, flag=true
#[inline(always)]
fn op_ltu(a: u64, b: u64) -> (u64, bool) {
    if a < b {
        (1, true)
    } else {
        (0, false)
    }
}

#[inline(always)]
fn op_lt(a: u64, b: u64) -> (u64, bool) {
    if (a as i64) < (b as i64) {
        (1, true)
    } else {
        (0, false)
    }
}

#[inline(always)]
fn op_ltu_w(a: u64, b: u64) -> (u64, bool) {
    if (a as u32) < (b as u32) {
        (1, true)
    } else {
        (0, false)
    }
}

#[inline(always)]
fn op_lt_w(a: u64, b: u64) -> (u64, bool) {
    if (a as i32) < (b as i32) {
        (1, true)
    } else {
        (0, false)
    }
}

#[inline(always)]
fn op_leu(a: u64, b: u64) -> (u64, bool) {
    if a <= b {
        (1, true)
    } else {
        (0, false)
    }
}

#[inline(always)]
fn op_le(a: u64, b: u64) -> (u64, bool) {
    if (a as i64) <= (b as i64) {
        (1, true)
    } else {
        (0, false)
    }
}

#[inline(always)]
fn op_leu_w(a: u64, b: u64) -> (u64, bool) {
    if (a as u32) <= (b as u32) {
        (1, true)
    } else {
        (0, false)
    }
}

#[inline(always)]
fn op_le_w(a: u64, b: u64) -> (u64, bool) {
    if (a as i32) <= (b as i32) {
        (1, true)
    } else {
        (0, false)
    }
}

#[inline(always)]
fn op_and(a: u64, b: u64) -> (u64, bool) {
    (a & b, false)
}

#[inline(always)]
fn op_or(a: u64, b: u64) -> (u64, bool) {
    (a | b, false)
}

#[inline(always)]
fn op_xor(a: u64, b: u64) -> (u64, bool) {
    (a ^ b, false)
}

#[inline(always)]
fn op_mulu(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a) * Wrapping(b)).0, false)
}

#[inline(always)]
fn op_mul(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a as i64) * Wrapping(b as i64)).0 as u64, false)
}

#[inline(always)]
fn op_mul_w(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a as i32) * Wrapping(b as i32)).0 as u64, false)
}

#[inline(always)]
fn op_muluh(a: u64, b: u64) -> (u64, bool) {
    (((a as u128 * b as u128) >> 64) as u64, false)
}

#[inline(always)]
fn op_mulh(a: u64, b: u64) -> (u64, bool) {
    (((((a as i64) as i128) * ((b as i64) as i128)) >> 64) as u64, false)
}

#[inline(always)]
fn op_mulsuh(a: u64, b: u64) -> (u64, bool) {
    (((((a as i64) as i128) * (b as i128)) >> 64) as u64, false)
}

#[inline(always)]
fn op_divu(a: u64, b: u64) -> (u64, bool) {
    if b == 0 {
        return (M64, true);
    }

    (a / b, false)
}

#[inline(always)]
fn op_div(a: u64, b: u64) -> (u64, bool) {
    if b == 0 {
        return (M64, true);
    }
    ((((a as i64) as i128) / ((b as i64) as i128)) as u64, false)
}

#[inline(always)]
fn op_divu_w(a: u64, b: u64) -> (u64, bool) {
    if b as u32 == 0 {
        return (M64, true);
    }

    (((a as u32 / b as u32) as i32) as u64, false)
}

#[inline(always)]
fn op_div_w(a: u64, b: u64) -> (u64, bool) {
    if b as i32 == 0 {
        return (M64, true);
    }

    ((((a as i32) as i64) / ((b as i32) as i64)) as u64, false)
}

#[inline(always)]
fn op_remu(a: u64, b: u64) -> (u64, bool) {
    if b == 0 {
        return (a, true);
    }

    (a % b, false)
}

#[inline(always)]
fn op_rem(a: u64, b: u64) -> (u64, bool) {
    if b == 0 {
        return (a, true);
    }

    ((((a as i64) as i128) % ((b as i64) as i128)) as u64, false)
}

#[inline(always)]
fn op_remu_w(a: u64, b: u64) -> (u64, bool) {
    if (b as u32) == 0 {
        return ((a as i32) as u64, true);
    }

    ((((a as u32) % (b as u32)) as i32) as u64, false)
}

#[inline(always)]
fn op_rem_w(a: u64, b: u64) -> (u64, bool) {
    if (b as i32) == 0 {
        return ((a as i32) as u64, true);
    }

    (((a as i32) % (b as i32)) as u64, false)
}

#[inline(always)]
fn op_minu(a: u64, b: u64) -> (u64, bool) {
    //if op_s64(a) < op_s64(b)
    if a < b {
        (a, false)
    } else {
        (b, false)
    }
}

#[inline(always)]
fn op_min(a: u64, b: u64) -> (u64, bool) {
    if (a as i64) < (b as i64) {
        (a, false)
    } else {
        (b, false)
    }
}

#[inline(always)]
fn op_minu_w(a: u64, b: u64) -> (u64, bool) {
    if (a as u32) < (b as u32) {
        (a, false)
    } else {
        (b, false)
    }
}

#[inline(always)]
fn op_min_w(a: u64, b: u64) -> (u64, bool) {
    if (a as i32) < (b as i32) {
        (a, false)
    } else {
        (b, false)
    }
}

#[inline(always)]
fn op_maxu(a: u64, b: u64) -> (u64, bool) {
    //if op_s64(a) > op_s64(b)
    if a > b {
        (a, false)
    } else {
        (b, false)
    }
}

#[inline(always)]
fn op_max(a: u64, b: u64) -> (u64, bool) {
    if (a as i64) > (b as i64) {
        (a, false)
    } else {
        (b, false)
    }
}

#[inline(always)]
fn op_maxu_w(a: u64, b: u64) -> (u64, bool) {
    if (a as u32) > (b as u32) {
        (a, false)
    } else {
        (b, false)
    }
}

#[inline(always)]
fn op_max_w(a: u64, b: u64) -> (u64, bool) {
    if (a as i32) > (b as i32) {
        (a, false)
    } else {
        (b, false)
    }
}

/// ZisK operations list, mapped by operation code and by operation string, for convenience
pub struct ZiskOperations {
    pub ops: Vec<ZiskOperation>,
    pub op_from_str: HashMap<&'static str, ZiskOperation>,
    pub op_from_code: HashMap<u8, ZiskOperation>,
}

/// Default constructor for ZiskOperations structure
impl Default for ZiskOperations {
    fn default() -> Self {
        Self::new()
    }
}

/// ZisK operations implementation
impl ZiskOperations {
    /// ZisK operations constructor
    pub fn new() -> ZiskOperations {
        // Create a vector of ZisK operations and fill it with the corresponding operation data
        let ops: Vec<ZiskOperation> = vec![
            ZiskOperation { n: "flag", t: "i", c: 0x00, f: op_flag },
            ZiskOperation { n: "copyb", t: "i", c: 0x01, f: op_copyb },
            ZiskOperation { n: "signextend_b", t: "e", c: 0x02, f: op_signextend_b },
            ZiskOperation { n: "signextend_h", t: "e", c: 0x03, f: op_signextend_h },
            ZiskOperation { n: "signextend_w", t: "e", c: 0x04, f: op_signextend_w },
            ZiskOperation { n: "add", t: "e", c: 0x10, f: op_add },
            ZiskOperation { n: "add_w", t: "e", c: 0x14, f: op_add_w },
            ZiskOperation { n: "sub", t: "e", c: 0x20, f: op_sub },
            ZiskOperation { n: "sub_w", t: "e", c: 0x24, f: op_sub_w },
            ZiskOperation { n: "sll", t: "e", c: 0x30, f: op_sll },
            ZiskOperation { n: "sll_w", t: "e", c: 0x34, f: op_sll_w },
            ZiskOperation { n: "sra", t: "e", c: 0x40, f: op_sra },
            ZiskOperation { n: "srl", t: "e", c: 0x41, f: op_srl },
            ZiskOperation { n: "sra_w", t: "e", c: 0x44, f: op_sra_w },
            ZiskOperation { n: "srl_w", t: "e", c: 0x45, f: op_srl_w },
            ZiskOperation { n: "eq", t: "e", c: 0x50, f: op_eq },
            ZiskOperation { n: "eq_w", t: "e", c: 0x54, f: op_eq_w },
            ZiskOperation { n: "ltu", t: "e", c: 0x60, f: op_ltu },
            ZiskOperation { n: "lt", t: "e", c: 0x61, f: op_lt },
            ZiskOperation { n: "ltu_w", t: "e", c: 0x64, f: op_ltu_w },
            ZiskOperation { n: "lt_w", t: "e", c: 0x65, f: op_lt_w },
            ZiskOperation { n: "leu", t: "e", c: 0x70, f: op_leu },
            ZiskOperation { n: "le", t: "e", c: 0x71, f: op_le },
            ZiskOperation { n: "leu_w", t: "e", c: 0x74, f: op_leu_w },
            ZiskOperation { n: "le_w", t: "e", c: 0x75, f: op_le_w },
            ZiskOperation { n: "and", t: "e", c: 0x80, f: op_and },
            ZiskOperation { n: "or", t: "e", c: 0x90, f: op_or },
            ZiskOperation { n: "xor", t: "e", c: 0xa0, f: op_xor },
            ZiskOperation { n: "mulu", t: "e", c: 0xb0, f: op_mulu },
            ZiskOperation { n: "mul", t: "e", c: 0xb1, f: op_mul },
            ZiskOperation { n: "mul_w", t: "e", c: 0xb5, f: op_mul_w },
            ZiskOperation { n: "muluh", t: "e", c: 0xb8, f: op_muluh },
            ZiskOperation { n: "mulh", t: "e", c: 0xb9, f: op_mulh },
            ZiskOperation { n: "mulsuh", t: "e", c: 0xbb, f: op_mulsuh },
            ZiskOperation { n: "divu", t: "e", c: 0xc0, f: op_divu },
            ZiskOperation { n: "div", t: "e", c: 0xc1, f: op_div },
            ZiskOperation { n: "divu_w", t: "e", c: 0xc4, f: op_divu_w },
            ZiskOperation { n: "div_w", t: "e", c: 0xc5, f: op_div_w },
            ZiskOperation { n: "remu", t: "e", c: 0xc8, f: op_remu },
            ZiskOperation { n: "rem", t: "e", c: 0xc9, f: op_rem },
            ZiskOperation { n: "remu_w", t: "e", c: 0xcc, f: op_remu_w },
            ZiskOperation { n: "rem_w", t: "e", c: 0xcd, f: op_rem_w },
            ZiskOperation { n: "minu", t: "e", c: 0xd0, f: op_minu },
            ZiskOperation { n: "min", t: "e", c: 0xd1, f: op_min },
            ZiskOperation { n: "minu_w", t: "e", c: 0xd4, f: op_minu_w },
            ZiskOperation { n: "min_w", t: "e", c: 0xd5, f: op_min_w },
            ZiskOperation { n: "maxu", t: "e", c: 0xe0, f: op_maxu },
            ZiskOperation { n: "max", t: "e", c: 0xe1, f: op_max },
            ZiskOperation { n: "maxu_w", t: "e", c: 0xe4, f: op_maxu_w },
            ZiskOperation { n: "max_w", t: "e", c: 0xe5, f: op_max_w },
        ];

        // Create two empty maps
        let mut op_from_str: HashMap<&'static str, ZiskOperation> = HashMap::new();
        let mut op_from_code: HashMap<u8, ZiskOperation> = HashMap::new();

        // Insert operations in both maps, using the proper keys
        for op in &ops {
            op_from_str.insert(op.n, op.clone());
            op_from_code.insert(op.c, op.clone());
        }

        // Return an instance with the constructed data
        ZiskOperations { ops, op_from_str, op_from_code }
    }
}
