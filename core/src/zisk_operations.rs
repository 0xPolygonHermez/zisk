use crate::ZiskOperation;
use std::{collections::HashMap, num::Wrapping};

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

#[inline(always)]
pub fn opcode_execute(opcode: u8, a: u64, b: u64) -> (u64, bool) {
    match opcode {
        0x00 => op_flag(a, b),
        0x01 => op_copyb(a, b),
        0x24 => op_signextend_b(a, b),
        0x25 => op_signextend_h(a, b),
        0x26 => op_signextend_w(a, b),
        0x02 => op_add(a, b),
        0x12 => op_add_w(a, b),
        0x03 => op_sub(a, b),
        0x13 => op_sub_w(a, b),
        0x0d => op_sll(a, b),
        0x1d => op_sll_w(a, b),
        0x0f => op_sra(a, b),
        0x0e => op_srl(a, b),
        0x1f => op_sra_w(a, b),
        0x1e => op_srl_w(a, b),
        0x08 => op_eq(a, b),
        0x18 => op_eq_w(a, b),
        0x04 => op_ltu(a, b),
        0x05 => op_lt(a, b),
        0x14 => op_ltu_w(a, b),
        0x15 => op_lt_w(a, b),
        0x06 => op_leu(a, b),
        0x07 => op_le(a, b),
        0x16 => op_leu_w(a, b),
        0x17 => op_le_w(a, b),
        0x20 => op_and(a, b),
        0x21 => op_or(a, b),
        0x22 => op_xor(a, b),
        0xb0 => op_mulu(a, b),
        0xb1 => op_mul(a, b),
        0xb5 => op_mul_w(a, b),
        0xb8 => op_muluh(a, b),
        0xb9 => op_mulh(a, b),
        0xbb => op_mulsuh(a, b),
        0xc0 => op_divu(a, b),
        0xc1 => op_div(a, b),
        0xc4 => op_divu_w(a, b),
        0xc5 => op_div_w(a, b),
        0xc8 => op_remu(a, b),
        0xc9 => op_rem(a, b),
        0xcc => op_remu_w(a, b),
        0xcd => op_rem_w(a, b),
        0x09 => op_minu(a, b),
        0x0a => op_min(a, b),
        0x19 => op_minu_w(a, b),
        0x1a => op_min_w(a, b),
        0x0b => op_maxu(a, b),
        0x0c => op_max(a, b),
        0x1b => op_maxu_w(a, b),
        0x1c => op_max_w(a, b),
        _ => panic!("opcode_execute() found invalid opcode={}", opcode),
    }
}

#[inline(always)]
pub fn opcode_string(opcode: u8) -> &'static str {
    match opcode {
        0x00 => "flag",
        0x01 => "copyb",
        0x24 => "signextend_b",
        0x25 => "signextend_h",
        0x26 => "signextend_w",
        0x02 => "add",
        0x12 => "add_w",
        0x03 => "sub",
        0x13 => "sub_w",
        0x0d => "sll",
        0x1d => "sll_w",
        0x0f => "sra",
        0x0e => "srl",
        0x1f => "sra_w",
        0x1e => "srl_w",
        0x08 => "eq",
        0x18 => "eq_w",
        0x04 => "ltu",
        0x05 => "lt",
        0x14 => "ltu_w",
        0x15 => "lt_w",
        0x06 => "leu",
        0x07 => "le",
        0x16 => "leu_w",
        0x17 => "le_w",
        0x20 => "and",
        0x21 => "or",
        0x22 => "xor",
        0xb0 => "mulu",
        0xb1 => "mul",
        0xb5 => "mul_w",
        0xb8 => "muluh",
        0xb9 => "mulh",
        0xbb => "mulsuh",
        0xc0 => "divu",
        0xc1 => "div",
        0xc4 => "divu_w",
        0xc5 => "div_w",
        0xc8 => "remu",
        0xc9 => "rem",
        0xcc => "remu_w",
        0xcd => "rem_w",
        0x09 => "minu",
        0x0a => "min",
        0x19 => "minu_w",
        0x1a => "min_w",
        0x0b => "maxu",
        0x0c => "max",
        0x1b => "maxu_w",
        0x1c => "max_w",
        _ => panic!("opcode_string() found invalid opcode={}", opcode),
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
            ZiskOperation { n: "flag", t: "i", s: 0, c: 0x00, f: op_flag },
            ZiskOperation { n: "copyb", t: "i", s: 0, c: 0x01, f: op_copyb },
            ZiskOperation { n: "signextend_b", s: 109, t: "be", c: 0x24, f: op_signextend_b },
            ZiskOperation { n: "signextend_h", s: 109, t: "be", c: 0x25, f: op_signextend_h },
            ZiskOperation { n: "signextend_w", s: 109, t: "be", c: 0x26, f: op_signextend_w },
            ZiskOperation { n: "add", t: "b", s: 77, c: 0x02, f: op_add },
            ZiskOperation { n: "add_w", t: "b", s: 77, c: 0x12, f: op_add_w },
            ZiskOperation { n: "sub", t: "b", s: 77, c: 0x03, f: op_sub },
            ZiskOperation { n: "sub_w", t: "b", s: 77, c: 0x13, f: op_sub_w },
            ZiskOperation { n: "sll", t: "be", s: 109, c: 0x0d, f: op_sll },
            ZiskOperation { n: "sll_w", t: "be", s: 109, c: 0x1d, f: op_sll_w },
            ZiskOperation { n: "sra", t: "be", s: 109, c: 0x0f, f: op_sra },
            ZiskOperation { n: "srl", t: "be", s: 109, c: 0x0e, f: op_srl },
            ZiskOperation { n: "sra_w", t: "be", s: 109, c: 0x1f, f: op_sra_w },
            ZiskOperation { n: "srl_w", t: "be", s: 109, c: 0x1e, f: op_srl_w },
            ZiskOperation { n: "eq", t: "b", s: 77, c: 0x08, f: op_eq },
            ZiskOperation { n: "eq_w", t: "b", s: 77, c: 0x18, f: op_eq_w },
            ZiskOperation { n: "ltu", t: "b", s: 77, c: 0x04, f: op_ltu },
            ZiskOperation { n: "lt", t: "b", s: 77, c: 0x05, f: op_lt },
            ZiskOperation { n: "ltu_w", t: "b", s: 77, c: 0x14, f: op_ltu_w },
            ZiskOperation { n: "lt_w", t: "b", s: 77, c: 0x15, f: op_lt_w },
            ZiskOperation { n: "leu", t: "b", s: 77, c: 0x06, f: op_leu },
            ZiskOperation { n: "le", t: "b", s: 77, c: 0x07, f: op_le },
            ZiskOperation { n: "leu_w", t: "b", s: 77, c: 0x16, f: op_leu_w },
            ZiskOperation { n: "le_w", t: "b", s: 77, c: 0x17, f: op_le_w },
            ZiskOperation { n: "and", t: "b", s: 77, c: 0x20, f: op_and },
            ZiskOperation { n: "or", t: "b", s: 77, c: 0x21, f: op_or },
            ZiskOperation { n: "xor", t: "b", s: 77, c: 0x22, f: op_xor },
            ZiskOperation { n: "mulu", t: "a", s: 97, c: 0xb0, f: op_mulu },
            ZiskOperation { n: "mul", t: "a", s: 97, c: 0xb1, f: op_mul },
            ZiskOperation { n: "mul_w", t: "am32", s: 44, c: 0xb5, f: op_mul_w },
            ZiskOperation { n: "muluh", t: "a", s: 97, c: 0xb8, f: op_muluh },
            ZiskOperation { n: "mulh", t: "a", s: 97, c: 0xb9, f: op_mulh },
            ZiskOperation { n: "mulsuh", t: "a", s: 97, c: 0xbb, f: op_mulsuh },
            ZiskOperation { n: "divu", t: "a", s: 174, c: 0xc0, f: op_divu },
            ZiskOperation { n: "div", t: "a", s: 174, c: 0xc1, f: op_div },
            ZiskOperation { n: "divu_w", t: "a32", s: 136, c: 0xc4, f: op_divu_w },
            ZiskOperation { n: "div_w", t: "a32", s: 136, c: 0xc5, f: op_div_w },
            ZiskOperation { n: "remu", t: "a", s: 174, c: 0xc8, f: op_remu },
            ZiskOperation { n: "rem", t: "a", s: 174, c: 0xc9, f: op_rem },
            ZiskOperation { n: "remu_w", t: "a32", s: 136, c: 0xcc, f: op_remu_w },
            ZiskOperation { n: "rem_w", t: "a32", s: 136, c: 0xcd, f: op_rem_w },
            ZiskOperation { n: "minu", t: "b", s: 77, c: 0x09, f: op_minu },
            ZiskOperation { n: "min", t: "b", s: 77, c: 0x0a, f: op_min },
            ZiskOperation { n: "minu_w", t: "b", s: 77, c: 0x19, f: op_minu_w },
            ZiskOperation { n: "min_w", t: "b", s: 77, c: 0x19, f: op_min_w },
            ZiskOperation { n: "maxu", t: "b", s: 77, c: 0x0b, f: op_maxu },
            ZiskOperation { n: "max", t: "b", s: 77, c: 0x0c, f: op_max },
            ZiskOperation { n: "maxu_w", t: "b", s: 77, c: 0x1b, f: op_maxu_w },
            ZiskOperation { n: "max_w", t: "b", s: 77, c: 0x1c, f: op_max_w },
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

pub fn op_from_str(op: &str) -> ZiskOperation {
    match op {
        "flag" => ZiskOperation { n: "flag", t: "i", s: 0, c: 0x00, f: op_flag },
        "copyb" => ZiskOperation { n: "copyb", t: "i", s: 0, c: 0x01, f: op_copyb },
        "signextend_b" => {
            ZiskOperation { n: "signextend_b", t: "be", s: 109, c: 0x24, f: op_signextend_b }
        }
        "signextend_h" => {
            ZiskOperation { n: "signextend_h", t: "be", s: 109, c: 0x25, f: op_signextend_h }
        }
        "signextend_w" => {
            ZiskOperation { n: "signextend_w", t: "be", s: 109, c: 0x26, f: op_signextend_w }
        }
        "add" => ZiskOperation { n: "add", t: "b", s: 77, c: 0x02, f: op_add },
        "add_w" => ZiskOperation { n: "add_w", t: "b", s: 77, c: 0x12, f: op_add_w },
        "sub" => ZiskOperation { n: "sub", t: "b", s: 77, c: 0x03, f: op_sub },
        "sub_w" => ZiskOperation { n: "sub_w", t: "b", s: 77, c: 0x13, f: op_sub_w },
        "sll" => ZiskOperation { n: "sll", t: "be", s: 109, c: 0x0d, f: op_sll },
        "sll_w" => ZiskOperation { n: "sll_w", t: "be", s: 109, c: 0x1d, f: op_sll_w },
        "sra" => ZiskOperation { n: "sra", t: "be", s: 109, c: 0x0f, f: op_sra },
        "srl" => ZiskOperation { n: "srl", t: "be", s: 109, c: 0x0e, f: op_srl },
        "sra_w" => ZiskOperation { n: "sra_w", t: "be", s: 109, c: 0x1f, f: op_sra_w },
        "srl_w" => ZiskOperation { n: "srl_w", t: "be", s: 109, c: 0x1e, f: op_srl_w },
        "eq" => ZiskOperation { n: "eq", t: "b", s: 77, c: 0x08, f: op_eq },
        "eq_w" => ZiskOperation { n: "eq_w", t: "b", s: 77, c: 0x18, f: op_eq_w },
        "ltu" => ZiskOperation { n: "ltu", t: "b", s: 77, c: 0x04, f: op_ltu },
        "lt" => ZiskOperation { n: "lt", t: "b", s: 77, c: 0x05, f: op_lt },
        "ltu_w" => ZiskOperation { n: "ltu_w", t: "b", s: 77, c: 0x14, f: op_ltu_w },
        "lt_w" => ZiskOperation { n: "lt_w", t: "b", s: 77, c: 0x15, f: op_lt_w },
        "leu" => ZiskOperation { n: "leu", t: "b", s: 77, c: 0x06, f: op_leu },
        "le" => ZiskOperation { n: "le", t: "b", s: 77, c: 0x07, f: op_le },
        "leu_w" => ZiskOperation { n: "leu_w", t: "b", s: 77, c: 0x16, f: op_leu_w },
        "le_w" => ZiskOperation { n: "le_w", t: "b", s: 77, c: 0x17, f: op_le_w },
        "and" => ZiskOperation { n: "and", t: "b", s: 77, c: 0x20, f: op_and },
        "or" => ZiskOperation { n: "or", t: "b", s: 77, c: 0x21, f: op_or },
        "xor" => ZiskOperation { n: "xor", t: "b", s: 77, c: 0x22, f: op_xor },
        "mulu" => ZiskOperation { n: "mulu", t: "a", s: 97, c: 0xb0, f: op_mulu },
        "mul" => ZiskOperation { n: "mul", t: "a", s: 97, c: 0xb1, f: op_mul },
        "mul_w" => ZiskOperation { n: "mul_w", t: "a", s: 44, c: 0xb5, f: op_mul_w },
        "muluh" => ZiskOperation { n: "muluh", t: "a", s: 97, c: 0xb8, f: op_muluh },
        "mulh" => ZiskOperation { n: "mulh", t: "a", s: 97, c: 0xb9, f: op_mulh },
        "mulsuh" => ZiskOperation { n: "mulsuh", t: "a", s: 97, c: 0xbb, f: op_mulsuh },
        "divu" => ZiskOperation { n: "divu", t: "a", s: 174, c: 0xc0, f: op_divu },
        "div" => ZiskOperation { n: "div", t: "a", s: 174, c: 0xc1, f: op_div },
        "divu_w" => ZiskOperation { n: "divu_w", t: "a32", s: 136, c: 0xc4, f: op_divu_w },
        "div_w" => ZiskOperation { n: "div_w", t: "a32", s: 136, c: 0xc5, f: op_div_w },
        "remu" => ZiskOperation { n: "remu", t: "a", s: 174, c: 0xc8, f: op_remu },
        "rem" => ZiskOperation { n: "rem", t: "a", s: 174, c: 0xc9, f: op_rem },
        "remu_w" => ZiskOperation { n: "remu_w", t: "a32", s: 136, c: 0xcc, f: op_remu_w },
        "rem_w" => ZiskOperation { n: "rem_w", t: "a32", s: 136, c: 0xcd, f: op_rem_w },
        "minu" => ZiskOperation { n: "minu", t: "b", s: 77, c: 0x09, f: op_minu },
        "min" => ZiskOperation { n: "min", t: "b", s: 77, c: 0x0a, f: op_min },
        "minu_w" => ZiskOperation { n: "minu_w", t: "b", s: 77, c: 0x19, f: op_minu_w },
        "min_w" => ZiskOperation { n: "min_w", t: "b", s: 77, c: 0x1a, f: op_min_w },
        "maxu" => ZiskOperation { n: "maxu", t: "b", s: 77, c: 0x0b, f: op_maxu },
        "max" => ZiskOperation { n: "max", t: "b", s: 77, c: 0x0c, f: op_max },
        "maxu_w" => ZiskOperation { n: "maxu_w", t: "b", s: 77, c: 0x1b, f: op_maxu_w },
        "max_w" => ZiskOperation { n: "max_w", t: "b", s: 77, c: 0x1c, f: op_max_w },
        _ => panic!("op_from_str() found invalid opcode={}", op),
    }
}

pub fn str_to_opcode(s: &str) -> u8 {
    let op: u8 = match s {
        "flag" => 0x00,
        "copyb" => 0x01,
        "signextend_b" => 0x24,
        "signextend_h" => 0x25,
        "signextend_w" => 0x26,
        "add" => 0x02,
        "add_w" => 0x12,
        "sub" => 0x03,
        "sub_w" => 0x13,
        "sll" => 0x0d,
        "sll_w" => 0x1d,
        "sra" => 0x0f,
        "srl" => 0x0e,
        "sra_w" => 0x1f,
        "srl_w" => 0x1e,
        "eq" => 0x08,
        "eq_w" => 0x18,
        "ltu" => 0x04,
        "lt" => 0x05,
        "ltu_w" => 0x14,
        "lt_w" => 0x15,
        "leu" => 0x07,
        "le" => 0x08,
        "leu_w" => 0x17,
        "le_w" => 0x18,
        "and" => 0x20,
        "or" => 0x21,
        "xor" => 0x22,
        "mulu" => 0xb0,
        "mul" => 0xb1,
        "mul_w" => 0xb5,
        "muluh" => 0xb8,
        "mulh" => 0xb9,
        "mulsuh" => 0xbb,
        "divu" => 0xc0,
        "div" => 0xc1,
        "divu_w" => 0xc4,
        "div_w" => 0xc5,
        "remu" => 0xc8,
        "rem" => 0xc9,
        "remu_w" => 0xcc,
        "rem_w" => 0xcd,
        "minu" => 0x09,
        "min" => 0x0a,
        "minu_w" => 0x19,
        "min_w" => 0x1a,
        "maxu" => 0x0b,
        "max" => 0x0c,
        "maxu_w" => 0x1b,
        "max_w" => 0x1c,
        _ => panic!("str_to_opcode() found invalid opcode string={}", s),
    };
    op
}
