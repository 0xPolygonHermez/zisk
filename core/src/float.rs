// Constants for IEEE 754 double-precision (64-bit) format
const BIAS: i32 = 1023;
const SIGN_MASK: u64 = 0x8000_0000_0000_0000; // 1 followed by 63 zeros
const EXP_MASK: u64 = 0x7FF0_0000_0000_0000; // 11 exponent bits (0x7FF), 52 zeros
const MANT_MASK: u64 = 0x000F_FFFF_FFFF_FFFF; // 52 mantissa bits
const HIDDEN_BIT: u64 = 0x0010_0000_0000_0000; // The implied 1 at bit 52

pub fn fadd_d(a: u64, b: u64) -> u64 {
    // 1. DECOMPOSE THE DOUBLES INTO THEIR PARTS
    let a_sign = a & SIGN_MASK;
    let a_exp = ((a & EXP_MASK) >> 52) as i32; // Shift 52 bits to get the exponent value
    let mut a_mant = a & MANT_MASK;
    println!("fadd_d a: sign={:x} exp={:x} mant={:x}", a_sign, a_exp, a_mant);

    let b_sign = b & SIGN_MASK;
    let b_exp = ((b & EXP_MASK) >> 52) as i32;
    let mut b_mant = b & MANT_MASK;
    println!("fadd_d b: sign={:x} exp={:x} mant={:x}", b_sign, b_exp, b_mant);

    // Handle special cases: zeros, denormals, infinities, NaNs
    if a_exp == 0x7FF || b_exp == 0x7FF {
        return handle_special_cases(a, b); // See helper function below
    }

    // Handle denormals (exp=0): mantissa has no hidden bit
    if a_exp != 0 {
        a_mant |= HIDDEN_BIT; // Add the implied leading 1
    } else {
        // Treat denormal as normal but with exp = 1 - BIAS
        // This is a simplification and may not cover all edge cases
        return handle_underflow(a_sign, a_mant as u128);
    }

    if b_exp != 0 {
        b_mant |= HIDDEN_BIT;
    } else {
        // Treat denormal as normal but with exp = 1 - BIAS
        // This is a simplification and may not cover all edge cases
        return handle_underflow(b_sign, b_mant as u128);
    }
    // 2. ALIGN EXPONENTS (Shift the smaller number's mantissa right)
    let exp_diff = a_exp - b_exp;
    // Use a 128-bit space for the mantissa to prevent loss during shifting
    let mut a_mant_wide = (a_mant as u128) << 61; // Shift into a high-precision space
    let mut b_mant_wide = (b_mant as u128) << 61;
    let result_exp: i32;
    if exp_diff > 0 {
        b_mant_wide >>= exp_diff;
        result_exp = a_exp;
    } else if exp_diff < 0 {
        a_mant_wide >>= -exp_diff;
        result_exp = b_exp;
    } else {
        result_exp = a_exp;
    }
    // 3. ADD OR SUBTRACT THE MANTISSAS (Handle signs)
    let mut a_value = a_mant_wide;
    let mut b_value = b_mant_wide;
    if a_sign != 0 {
        a_value = (!a_value).wrapping_add(1); // Convert to negative 2's complement
    }
    if b_sign != 0 {
        b_value = (!b_value).wrapping_add(1);
    }
    let mut result_mant_wide = a_value.wrapping_add(b_value);
    // 4. NORMALIZE THE RESULT
    let result_sign: u64;
    if (result_mant_wide as i128) < 0 {
        result_sign = SIGN_MASK;
        result_mant_wide = (!result_mant_wide).wrapping_add(1); // Work with a positive magnitude
    } else {
        result_sign = 0;
    }
    // Check for zero result
    if result_mant_wide == 0 {
        return result_sign; // Positive or negative zero
    }
    // Normalize: Shift until the hidden bit (bit 61 in our wide space) is set
    let mut shift_count = 0;
    // While the top bit is not set in our 128-bit space...
    while (result_mant_wide & (1u128 << 127)) == 0 {
        result_mant_wide <<= 1;
        shift_count += 1;
        // Check for underflow after shifting
        if result_exp - shift_count <= 0 {
            return handle_underflow(result_sign, result_mant_wide);
        }
    }
    // Now check if we need to round right (if the result was >= 2.0)
    // Our hidden bit is now at position 127. We want it at 61.
    // If the bit at 127 is set, our number is in the range [2.0, 4.0)
    let mut adjusted_exp = result_exp - shift_count;
    if (result_mant_wide & (1u128 << 127)) != 0 {
        result_mant_wide >>= 1;
        adjusted_exp += 1;
        // Check for overflow after shifting
        if adjusted_exp >= 0x7FF {
            return result_sign | EXP_MASK; // Infinity
        }
    }
    // 5. ROUNDING (Simplified - using round to nearest)
    // Extract the bits beyond our 52-bit mantissa precision
    let guard_bit = (result_mant_wide >> (61 - 1)) & 1; // First bit beyond precision
    let round_bit = (result_mant_wide >> (61 - 2)) & 1; // Second bit
    let sticky_mask = (1u128 << (61 - 2)) - 1; // All bits below round bit
    let sticky_bit = if (result_mant_wide & sticky_mask) != 0 { 1 } else { 0 };
    result_mant_wide >>= 61; // Shift to get 52-bit mantissa + hidden bit
                             // Apply rounding: if guard bit is 1 and (round bit OR sticky bit is 1)
    if guard_bit == 1 && (round_bit == 1 || sticky_bit == 1) {
        result_mant_wide += 1;
        // Check if rounding caused an overflow (e.g., 1.111... + 1 = 10.000...)
        if (result_mant_wide & ((HIDDEN_BIT as u128) << 1)) != 0 {
            result_mant_wide >>= 1;
            adjusted_exp += 1;
            if adjusted_exp >= 0x7FF {
                return result_sign | EXP_MASK; // Infinity
            }
        }
    }
    // 6. CHECK FOR OVERFLOW/UNDERFLOW
    if adjusted_exp >= 0x7FF {
        return result_sign | EXP_MASK; // Infinity
    }
    if adjusted_exp <= 0 {
        // Underflow: return denormalized or zero
        return handle_underflow(result_sign, result_mant_wide >> (-adjusted_exp + 1));
    }
    // 7. REASSEMBLE THE RESULT FROM PARTS
    // Remove the hidden bit before packing
    println!(
        "fadd_d result: sign={:x} exp={} mant={:x}",
        result_sign, adjusted_exp, result_mant_wide
    );
    let result_mant = (result_mant_wide as u64) & MANT_MASK;
    let result_exp_bits = ((adjusted_exp as u64) << 52) & EXP_MASK;
    let result_bits = result_sign | result_exp_bits | result_mant;
    result_bits
}

// --- Helper Functions ---
fn handle_special_cases(a: u64, b: u64) -> u64 {
    let a_is_nan = (a & EXP_MASK == EXP_MASK) && (a & MANT_MASK != 0);
    let b_is_nan = (b & EXP_MASK == EXP_MASK) && (b & MANT_MASK != 0);
    let a_is_inf = (a & EXP_MASK == EXP_MASK) && (a & MANT_MASK == 0);
    let b_is_inf = (b & EXP_MASK == EXP_MASK) && (b & MANT_MASK == 0);
    // NaN + anything = NaN
    if a_is_nan || b_is_nan {
        return EXP_MASK | 0x0008_0000_0000_0000; // A quiet NaN
    }
    // Infinity handling
    if a_is_inf && b_is_inf {
        // Only return Inf if they have the same sign, else NaN
        if (a & SIGN_MASK) == (b & SIGN_MASK) {
            return a;
        } else {
            return EXP_MASK | 0x0008_0000_0000_0000; // NaN
        }
    }
    if a_is_inf {
        return a;
    }
    if b_is_inf {
        return b;
    }
    // If we get here, just return one of the original inputs (shouldn't happen for special cases)
    a
}
fn handle_underflow(sign: u64, mant: u128) -> u64 {
    // For simplicity, return signed zero on underflow
    sign
}

/*
// Constants for IEEE 754 double-precision (64-bit) format
BIAS ← 1023
SIGN_MASK ← 0x8000000000000000   // 1 followed by 63 zeros
EXP_MASK  ← 0x7FF0000000000000    // 11 exponent bits (0x7FF), 52 zeros
MANT_MASK ← 0x000FFFFFFFFFFFFF    // 52 mantissa bits
HIDDEN_BIT ← 0x0010000000000000   // The implied 1 at bit 52

FUNCTION integerDoubleAdd(a_bits: UINT64, b_bits: UINT64) RETURNS UINT64

    // 1. DECOMPOSE THE DOUBLES INTO THEIR PARTS
    a_sign ← (a_bits & SIGN_MASK)
    a_exp ← (a_bits & EXP_MASK) >> 52 // Shift 52 bits to get the exponent value
    a_mant ← (a_bits & MANT_MASK)

    b_sign ← (b_bits & SIGN_MASK)
    b_exp ← (b_bits & EXP_MASK) >> 52
    b_mant ← (b_bits & MANT_MASK)

    // Handle special cases: zeros, denormals, infinities, NaNs
    IF (a_exp == 0x7FF) OR (b_exp == 0x7FF) THEN
        RETURN handleSpecialCases(a_bits, b_bits) // See helper function below

    // Handle denormals (exp=0): mantissa has no hidden bit
    IF a_exp != 0 THEN
        a_mant ← a_mant | HIDDEN_BIT // Add the implied leading 1
    ELSE
        a_exp ← 1 // Treat denormal as normal but with exp = 1 - BIAS

    IF b_exp != 0 THEN
        b_mant ← b_mant | HIDDEN_BIT
    ELSE
        b_exp ← 1

    // 2. ALIGN EXPONENTS (Shift the smaller number's mantissa right)
    exp_diff ← a_exp - b_exp

    // Use a 128-bit space for the mantissa to prevent loss during shifting
    a_mant_wide ← (UINT128)(a_mant) << 61 // Shift into a high-precision space
    b_mant_wide ← (UINT128)(b_mant) << 61

    IF exp_diff > 0 THEN
        b_mant_wide ← b_mant_wide >> exp_diff
        result_exp ← a_exp
    ELSE IF exp_diff < 0 THEN
        a_mant_wide ← a_mant_wide >> (-exp_diff)
        result_exp ← b_exp
    ELSE
        result_exp ← a_exp

    // 3. ADD OR SUBTRACT THE MANTISSAS (Handle signs)
    a_value ← a_mant_wide
    b_value ← b_mant_wide

    IF a_sign != 0 THEN
        a_value ← -a_value // Convert to negative 2's complement

    IF b_sign != 0 THEN
        b_value ← -b_value

    result_mant_wide ← a_value + b_value

    // 4. NORMALIZE THE RESULT
    IF result_mant_wide < 0 THEN
        result_sign ← SIGN_MASK
        result_mant_wide ← -result_mant_wide // Work with a positive magnitude
    ELSE
        result_sign ← 0

    // Check for zero result
    IF result_mant_wide == 0 THEN
        RETURN result_sign // Positive or negative zero

    // Normalize: Shift until the hidden bit (bit 61 in our wide space) is set
    shift_count ← 0
    // While the top bit is not set in our 128-bit space...
    WHILE (result_mant_wide & (UINT128(1) << 127)) == 0
        result_mant_wide ← result_mant_wide << 1
        shift_count ← shift_count + 1
        result_exp ← result_exp - 1
        // Check for underflow after shifting
        IF result_exp <= 0 THEN
            return handleUnderflow(result_sign, result_mant_wide)
    END WHILE

    // Now check if we need to round right (if the result was >= 2.0)
    // Our hidden bit is now at position 127. We want it at 61.
    // If the bit at 127 is set, our number is in the range [2.0, 4.0)
    IF (result_mant_wide & (UINT128(1) << 127)) != 0 THEN
        result_mant_wide ← result_mant_wide >> 1
        result_exp ← result_exp + 1
        // Check for overflow after shifting
        IF result_exp >= 0x7FF THEN
            RETURN result_sign | EXP_MASK // Infinity

    // 5. ROUNDING (Simplified - using round to nearest)
    // Extract the bits beyond our 52-bit mantissa precision
    guard_bit ← (result_mant_wide >> (61 - 1)) & 1   // First bit beyond precision
    round_bit ← (result_mant_wide >> (61 - 2)) & 1    // Second bit
    sticky_bit ← 0
    sticky_mask ← (UINT128(1) << (61 - 2)) - 1        // All bits below round bit
    IF (result_mant_wide & sticky_mask) != 0 THEN
        sticky_bit ← 1

    result_mant_wide ← result_mant_wide >> (61) // Shift to get 52-bit mantissa + hidden bit

    // Apply rounding: if guard bit is 1 and (round bit OR sticky bit is 1)
    IF (guard_bit == 1) AND ((round_bit == 1) OR (sticky_bit == 1)) THEN
        result_mant_wide ← result_mant_wide + 1
        // Check if rounding caused an overflow (e.g., 1.111... + 1 = 10.000...)
        IF (result_mant_wide & (HIDDEN_BIT << 1)) != 0 THEN
            result_mant_wide ← result_mant_wide >> 1
            result_exp ← result_exp + 1
            IF result_exp >= 0x7FF THEN
                RETURN result_sign | EXP_MASK // Infinity

    // 6. CHECK FOR OVERFLOW/UNDERFLOW
    IF result_exp >= 0x7FF THEN
        RETURN result_sign | EXP_MASK // Infinity

    IF result_exp <= 0 THEN
        // Underflow: return denormalized or zero
        RETURN handleUnderflow(result_sign, result_mant_wide >> (-result_exp + 1))

    // 7. REASSEMBLE THE RESULT FROM PARTS
    // Remove the hidden bit before packing
    result_mant ← (UINT64)(result_mant_wide) & MANT_MASK
    result_exp_bits ← ((UINT64)(result_exp) << 52) & EXP_MASK

    result_bits ← result_sign | result_exp_bits | result_mant
    RETURN result_bits

END FUNCTION

// --- Helper Functions ---
FUNCTION handleSpecialCases(a: UINT64, b: UINT64) RETURNS UINT64
    a_is_nan ← (a & EXP_MASK == EXP_MASK) AND (a & MANT_MASK != 0)
    b_is_nan ← (b & EXP_MASK == EXP_MASK) AND (b & MANT_MASK != 0)
    a_is_inf ← (a & EXP_MASK == EXP_MASK) AND (a & MANT_MASK == 0)
    b_is_inf ← (b & EXP_MASK == EXP_MASK) AND (b & MANT_MASK == 0)

    // NaN + anything = NaN
    IF a_is_nan OR b_is_nan THEN
        RETURN EXP_MASK | 0x0008000000000000 // A quiet NaN

    // Infinity handling
    IF a_is_inf AND b_is_inf THEN
        // Only return Inf if they have the same sign, else NaN
        IF (a & SIGN_MASK) == (b & SIGN_MASK) THEN
            RETURN a
        ELSE
            RETURN EXP_MASK | 0x0008000000000000 // NaN
    ELSE IF a_is_inf THEN
        RETURN a
    ELSE IF b_is_inf THEN
        RETURN b

    // If we get here, just return one of the original inputs (shouldn't happen for special cases)
    RETURN a
END FUNCTION

FUNCTION handleUnderflow(sign: UINT64, mant: UINT128) RETURNS UINT64
    // For simplicity, return signed zero on underflow
    RETURN sign
END FUNCTION

// --- Example Usage ---
FLOAT64 a = 5.5
FLOAT64 b = 2.25

// Crucial Step: Reinterpret the double's memory as a 64-bit integer
a_bits = * (UINT64*) &a
b_bits = * (UINT64*) &b

sum_bits = integerDoubleAdd(a_bits, b_bits)

// Reinterpret the resulting integer bits back as a double
result = * (FLOAT64*) &sum_bits

PRINT "The sum is: " + result*/
