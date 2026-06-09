use std::num::Wrapping;

const M64: u64 = 0xFFFFFFFFFFFFFFFF;

/* Internal instructions */

/// Sets flag to true (and c to 0)
#[inline(always)]
pub const fn op_flag(_a: u64, _b: u64) -> (u64, bool) {
    (0, true)
}

/// Copies register b into c (and flag to false)
#[inline(always)]
pub const fn op_copyb(_a: u64, b: u64) -> (u64, bool) {
    (b, false)
}

/* SIGN EXTEND operations for different data widths (i8, i16 and i32) --> i64 --> u64 */

/// Sign extends an i8.
///
/// Converts b from a signed 8-bits number in the range [-128, +127] into a signed 64-bit number of
/// the same value, adding 0xFFFFFFFFFFFFFF00 if negative, and stores the result in c as a u64 (and
/// sets flag to false)
#[inline(always)]
pub const fn op_signextend_b(_a: u64, b: u64) -> (u64, bool) {
    ((b as i8) as u64, false)
}

/// Sign extends an i16.
///
/// Converts b from a signed 16-bits number in the range [-32768, 32767] into a signed 64-bit number
/// of the same value, adding 0xFFFFFFFFFFFF0000 if negative, and stores the result in c as a u64
/// (and sets flag to false)
#[inline(always)]
pub const fn op_signextend_h(_a: u64, b: u64) -> (u64, bool) {
    ((b as i16) as u64, false)
}

/// Sign extends an i32.
///
/// Converts b from a signed 32-bits number in the range [-2147483648, 2147483647] into a signed
/// 64-bit number of the same value, adding 0xFFFFFFFF00000000 if negative  and stores the result in
/// c as a u64 (and sets flag to false)
#[inline(always)]
pub const fn op_signextend_w(_a: u64, b: u64) -> (u64, bool) {
    ((b as i32) as u64, false)
}

/* ADD AND SUB operations for different data widths (i32 and u64) */

/// Adds a and b as 64-bit unsigned values, and stores the result in c (and sets flag to false)
#[inline(always)]
pub fn op_add(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a) + Wrapping(b)).0, false)
}

/// Adds a and b as 32-bit signed values, and stores the result in c (and flag to false)
#[inline(always)]
pub fn op_add_w(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a as i32) + Wrapping(b as i32)).0 as u64, false)
}

/// Subtracts a and b as 64-bit unsigned values, and stores the result in c (and sets flag to false)
#[inline(always)]
pub fn op_sub(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a) - Wrapping(b)).0, false)
}

/// Subtracts a and b as 32-bit signed values, and stores the result in c (and sets flag to false)
#[inline(always)]
pub fn op_sub_w(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a as i32) - Wrapping(b as i32)).0 as u64, false)
}

/* SHIFT operations */

/// Shifts a as a 64-bits unsigned value to the left b mod 64 bits, and stores the result in c (and
/// sets flag to false)
#[inline(always)]
pub const fn op_sll(a: u64, b: u64) -> (u64, bool) {
    (a << (b & 0x3f), false)
}

/// Shifts a as a 32-bits unsigned value to the left b mod 32 bits, and stores the result in c (and
/// sets flag to false)
#[inline(always)]
pub fn op_sll_w(a: u64, b: u64) -> (u64, bool) {
    (((Wrapping(a as u32) << (b & 0x1f) as usize).0 as i32) as u64, false)
}

/// Shifts a as a 64-bits signed value to the right b mod 64 bits, and stores the result in c (and
/// sets flag to false)
#[inline(always)]
pub const fn op_sra(a: u64, b: u64) -> (u64, bool) {
    (((a as i64) >> (b & 0x3f)) as u64, false)
}

/// Shifts a as a 64-bits unsigned value to the right b mod 64 bits, and stores the result in c (and
/// sets flag to false)
#[inline(always)]
pub const fn op_srl(a: u64, b: u64) -> (u64, bool) {
    (a >> (b & 0x3f), false)
}

/// Shifts a as a 32-bits signed value to the right b mod 32 bits, and stores the result in c (and
/// sets flag to false)
#[inline(always)]
pub fn op_sra_w(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a as i32) >> (b & 0x1f) as usize).0 as u64, false)
}

/// Shifts a as a 32-bits unsigned value to the right b mod 32 bits, and stores the result in c (and
/// sets flag to false)
#[inline(always)]
pub fn op_srl_w(a: u64, b: u64) -> (u64, bool) {
    (((Wrapping(a as u32) >> (b & 0x1f) as usize).0 as i32) as u64, false)
}

/* COMPARISON operations */

/// If a and b are equal, it returns c=1, flag=true; otherwise it returns c=0, flag=false
#[inline(always)]
pub const fn op_eq(a: u64, b: u64) -> (u64, bool) {
    if a == b {
        (1, true)
    } else {
        (0, false)
    }
}

/// If a and b as 32-bit signed values are equal, as 64-bit unsigned values, it returns c=1,
/// flag=true; otherwise it returns c=0, flag=false
#[inline(always)]
pub const fn op_eq_w(a: u64, b: u64) -> (u64, bool) {
    if (a as i32) == (b as i32) {
        (1, true)
    } else {
        (0, false)
    }
}

/// If a is strictly less than b, as 64-bit unsigned values, it returns c=1, flag=true; otherwise it
/// returns c=0, flag=false
#[inline(always)]
pub const fn op_ltu(a: u64, b: u64) -> (u64, bool) {
    if a < b {
        (1, true)
    } else {
        (0, false)
    }
}

/// If a is strictly less than b, as 64-bit signed values, it returns c=1, flag=true; otherwise it
/// returns c=0, flag=false
#[inline(always)]
pub const fn op_lt(a: u64, b: u64) -> (u64, bool) {
    if (a as i64) < (b as i64) {
        (1, true)
    } else {
        (0, false)
    }
}

/// If a is strictly less than b, as 32-bit unsigned values, it returns c=1, flag=true; otherwise it
/// returns c=0, flag=false
#[inline(always)]
pub const fn op_ltu_w(a: u64, b: u64) -> (u64, bool) {
    if (a as u32) < (b as u32) {
        (1, true)
    } else {
        (0, false)
    }
}

/// If a is strictly less than b, as 32-bit signed values, it returns c=1, flag=true; otherwise it
/// returns c=0, flag=false
#[inline(always)]
pub const fn op_lt_w(a: u64, b: u64) -> (u64, bool) {
    if (a as i32) < (b as i32) {
        (1, true)
    } else {
        (0, false)
    }
}

/// If a is less than or equal to b, as 64-bit unsigned values, it returns c=1, flag=true; otherwise
/// it returns c=0, flag=false
#[inline(always)]
pub const fn op_leu(a: u64, b: u64) -> (u64, bool) {
    if a <= b {
        (1, true)
    } else {
        (0, false)
    }
}

/// If a is less than or equal to b, as 64-bit signed values, it returns c=1, flag=true; otherwise
/// it returns c=0, flag=false
#[inline(always)]
pub const fn op_le(a: u64, b: u64) -> (u64, bool) {
    if (a as i64) <= (b as i64) {
        (1, true)
    } else {
        (0, false)
    }
}

/// If a is less than or equal to b, as 32-bit unsigned values, it returns c=1, flag=true; otherwise
/// it returns c=0, flag=false
#[inline(always)]
pub const fn op_leu_w(a: u64, b: u64) -> (u64, bool) {
    if (a as u32) <= (b as u32) {
        (1, true)
    } else {
        (0, false)
    }
}

/// If a is less than or equal to b, as 32-bit signed values, it returns c=1, flag=true; otherwise
/// it returns c=0, flag=false
#[inline(always)]
pub const fn op_le_w(a: u64, b: u64) -> (u64, bool) {
    if (a as i32) <= (b as i32) {
        (1, true)
    } else {
        (0, false)
    }
}

/* LOGICAL operations */

/// Sets c to a AND b, and flag to false
#[inline(always)]
pub const fn op_and(a: u64, b: u64) -> (u64, bool) {
    (a & b, false)
}

/// Sets c to a OR b, and flag to false
#[inline(always)]
pub const fn op_or(a: u64, b: u64) -> (u64, bool) {
    (a | b, false)
}

/// Sets c to a XOR b, and flag to false
#[inline(always)]
pub const fn op_xor(a: u64, b: u64) -> (u64, bool) {
    (a ^ b, false)
}

/* ARITHMETIC operations: div / mul / rem */

/// Sets c to a x b, as 64-bits unsigned values, and flag to false
#[inline(always)]
pub fn op_mulu(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a) * Wrapping(b)).0, false)
}

/// Sets c to a x b, as 64-bits signed values, and flag to false
#[inline(always)]
pub fn op_mul(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a as i64) * Wrapping(b as i64)).0 as u64, false)
}

/// Sets c to a x b, as 32-bits signed values, and flag to false
#[inline(always)]
pub fn op_mul_w(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a as i32) * Wrapping(b as i32)).0 as u64, false)
}

/// Sets c to the highest 64-bits of a x b, as 128-bits unsigned values, and flag to false
#[inline(always)]
pub const fn op_muluh(a: u64, b: u64) -> (u64, bool) {
    (((a as u128 * b as u128) >> 64) as u64, false)
}

/// Sets c to the highest 64-bits of a x b, as 128-bits unsigned values, and flag to false
#[inline(always)]
pub const fn op_mulh(a: u64, b: u64) -> (u64, bool) {
    (((((a as i64) as i128) * ((b as i64) as i128)) >> 64) as u64, false)
}

/// Sets c to the highest 64-bits of a x b, as 128-bits signed values, and flag to false
#[inline(always)]
pub const fn op_mulsuh(a: u64, b: u64) -> (u64, bool) {
    (((((a as i64) as i128) * (b as i128)) >> 64) as u64, false)
}

/// Sets c to a / b, as 64-bits unsigned values, and flag to false.
/// If b=0 (divide by zero) it sets c to 2^64 - 1, and sets flag to true.
#[inline(always)]
pub const fn op_divu(a: u64, b: u64) -> (u64, bool) {
    if b == 0 {
        return (M64, true);
    }

    (a / b, false)
}

/// Sets c to a / b, as 64-bits signed values, and flag to false.
///
/// If b=0 (divide by zero) it sets c to 2^64 - 1, and sets flag to true.
/// If a=0x8000000000000000 (MIN_I64) and b=0xFFFFFFFFFFFFFFFF (-1) the result should be -MIN_I64,
/// which cannot be represented with 64 bits (overflow); it returns c=a and sets flag to true.
#[inline(always)]
pub const fn op_div(a: u64, b: u64) -> (u64, bool) {
    // Handle divide by zero case
    if b == 0 {
        return (M64, true);
    }

    // Handle overflow case: -MIN_I64 cannot be represented in 64 bits, so return a.
    if a as u64 == 0x8000_0000_0000_0000 && b as i64 == -1 {
        return (0x8000_0000_0000_0000, true);
    }

    ((((a as i64) as i128) / ((b as i64) as i128)) as u64, false)
}

/// Sets c to a / b, as 32-bits unsigned values, and flag to false.
/// If b=0 (divide by zero) it sets c to 2^64 - 1, and sets flag to true.
#[inline(always)]
pub const fn op_divu_w(a: u64, b: u64) -> (u64, bool) {
    // Handle divide by zero case
    if b as u32 == 0 {
        return (M64, true);
    }

    (((a as u32 / b as u32) as i32) as u64, false)
}

/// Sets c to a / b, as 32-bits signed values, and flag to false.
/// If b=0 (divide by zero) it sets c to 2^64 - 1, and sets flag to true.
/// If a=0x80000000 (MIN_I32) and b=0xFFFFFFFF (-1) it returns 0xffffffff80000000 and sets flag to true.
#[inline(always)]
pub const fn op_div_w(a: u64, b: u64) -> (u64, bool) {
    // Handle divide by zero case
    if b as i32 == 0 {
        return (M64, true);
    }

    // Handle overflow case: DIVW semantics require MIN_I32 / -1 to return MIN_I32 (sign-extended)
    if a as u32 == 0x8000_0000 && b as i32 == -1 {
        return (0xFFFFFFFF80000000, true);
    }

    ((((a as i32) as i64) / ((b as i32) as i64)) as i32 as u64, false)
}

/// Sets c to a mod b, as 64-bits unsigned values, and flag to false.
/// If b=0 (divide by zero) it sets c to a, and sets flag to true.
#[inline(always)]
pub const fn op_remu(a: u64, b: u64) -> (u64, bool) {
    // Handle divide by zero case
    if b == 0 {
        return (a, true);
    }

    (a % b, false)
}

/// Sets c to a mod b, as 64-bits signed values, and flag to false.
/// If b=0 (divide by zero) it sets c to a, and sets flag to true.
#[inline(always)]
pub const fn op_rem(a: u64, b: u64) -> (u64, bool) {
    // Handle divide by zero case
    if b == 0 {
        return (a, true);
    }

    // Handle overflow case: -MIN_I64 cannot be represented in 64 bits, so return 0.
    if a as u64 == 0x8000_0000_0000_0000 && b as i64 == -1 {
        return (0, true);
    }

    ((((a as i64) as i128) % ((b as i64) as i128)) as u64, false)
}

/// Sets c to a mod b, as 32-bits unsigned values, and flag to false.
/// If b=0 (divide by zero) it sets c to a, and sets flag to true.
#[inline(always)]
pub const fn op_remu_w(a: u64, b: u64) -> (u64, bool) {
    // Handle divide by zero case
    if (b as u32) == 0 {
        return ((a as i32) as u64, true);
    }

    ((((a as u32) % (b as u32)) as i32) as u64, false)
}

/// Sets c to a mod b, as 32-bits signed values, and flag to false.
/// If b=0 (divide by zero) it sets c to a, and sets flag to true.
#[inline(always)]
pub const fn op_rem_w(a: u64, b: u64) -> (u64, bool) {
    // Handle divide by zero case
    if (b as i32) == 0 {
        return ((a as i32) as u64, true);
    }

    // Handle overflow case: -MIN_I32 cannot be represented in 32 bits, so return 0.
    if a as u32 == 0x8000_0000 && b as i32 == -1 {
        return (0, true);
    }

    ((((a as i32) as i64) % ((b as i32) as i64)) as u64, false)
}

/* MIN / MAX operations */

/// Sets c to the minimum of a and b as 64-bits unsigned values (and flag to false)
#[inline(always)]
pub const fn op_minu(a: u64, b: u64) -> (u64, bool) {
    if a < b {
        (a, false)
    } else {
        (b, false)
    }
}

/// Sets c to the minimum of a and b as 64-bits signed values (and flag to false)
#[inline(always)]
pub const fn op_min(a: u64, b: u64) -> (u64, bool) {
    if (a as i64) < (b as i64) {
        (a, false)
    } else {
        (b, false)
    }
}

/// Sets c to the minimum of a and b as 32-bits unsigned values (and flag to false)
#[inline(always)]
pub const fn op_minu_w(a: u64, b: u64) -> (u64, bool) {
    if (a as u32) < (b as u32) {
        (a as i32 as i64 as u64, false)
    } else {
        (b as i32 as i64 as u64, false)
    }
}

/// Sets c to the minimum of a and b as 32-bits signed values (and flag to false)
#[inline(always)]
pub const fn op_min_w(a: u64, b: u64) -> (u64, bool) {
    if (a as i32) < (b as i32) {
        (a as i32 as i64 as u64, false)
    } else {
        (b as i32 as i64 as u64, false)
    }
}

/// Sets c to the maximum of a and b as 64-bits unsigned values (and flag to false)
#[inline(always)]
pub const fn op_maxu(a: u64, b: u64) -> (u64, bool) {
    if a > b {
        (a, false)
    } else {
        (b, false)
    }
}

/// Sets c to the maximum of a and b as 64-bits signed values (and flag to false)
#[inline(always)]
pub const fn op_max(a: u64, b: u64) -> (u64, bool) {
    if (a as i64) > (b as i64) {
        (a, false)
    } else {
        (b, false)
    }
}

/// Sets c to the maximum of a and b as 32-bits unsigned values (and flag to false)
#[inline(always)]
pub const fn op_maxu_w(a: u64, b: u64) -> (u64, bool) {
    if (a as u32) > (b as u32) {
        (a as i32 as i64 as u64, false)
    } else {
        (b as i32 as i64 as u64, false)
    }
}

/// Sets c to the maximum of a and b as 32-bits signed values (and flag to false)
#[inline(always)]
pub const fn op_max_w(a: u64, b: u64) -> (u64, bool) {
    if (a as i32) > (b as i32) {
        (a as i32 as i64 as u64, false)
    } else {
        (b as i32 as i64 as u64, false)
    }
}
