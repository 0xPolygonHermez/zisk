/// Check if a pointer is 8-byte aligned.
#[inline(always)]
pub fn is_aligned_8(ptr: *const u8) -> bool {
    (ptr as usize) & 0x7 == 0
}

#[inline]
pub fn be_bytes_to_u64_4(bytes: &[u8; 32]) -> [u64; 4] {
    [
        u64::from_be_bytes(bytes[24..32].try_into().unwrap()),
        u64::from_be_bytes(bytes[16..24].try_into().unwrap()),
        u64::from_be_bytes(bytes[8..16].try_into().unwrap()),
        u64::from_be_bytes(bytes[0..8].try_into().unwrap()),
    ]
}

#[inline]
pub fn u64_4_to_be_bytes(limbs: &[u64; 4]) -> [u8; 32] {
    let b3 = limbs[3].to_be_bytes();
    let b2 = limbs[2].to_be_bytes();
    let b1 = limbs[1].to_be_bytes();
    let b0 = limbs[0].to_be_bytes();
    [
        b3[0], b3[1], b3[2], b3[3], b3[4], b3[5], b3[6], b3[7], b2[0], b2[1], b2[2], b2[3], b2[4],
        b2[5], b2[6], b2[7], b1[0], b1[1], b1[2], b1[3], b1[4], b1[5], b1[6], b1[7], b0[0], b0[1],
        b0[2], b0[3], b0[4], b0[5], b0[6], b0[7],
    ]
}

/// Given two n-bit number `x` and `y`, compares them and returns true if `x > y`; otherwise, false.
pub fn gt(x: &[u64], y: &[u64]) -> bool {
    debug_assert_eq!(x.len(), y.len(), "x and y must have the same length");

    for i in (0..x.len()).rev() {
        if x[i] > y[i] {
            return true;
        } else if x[i] < y[i] {
            return false;
        }
    }
    false
}

/// Given two n-bit number `x` and `y`, compares them and returns true if `x < y`; otherwise, false.    
pub fn lt(x: &[u64], y: &[u64]) -> bool {
    debug_assert_eq!(x.len(), y.len(), "x and y must have the same length");

    for i in (0..x.len()).rev() {
        if x[i] < y[i] {
            return true;
        } else if x[i] > y[i] {
            return false;
        }
    }
    false
}

/// Given two n-bit number `x` and `y`, compares them and returns true if `x == y`; otherwise, false.
pub fn eq(x: &[u64], y: &[u64]) -> bool {
    debug_assert_eq!(x.len(), y.len(), "x and y must have the same length");

    for i in 0..x.len() {
        if x[i] != y[i] {
            return false;
        }
    }
    true
}

/// Returns true if x == 0
pub fn is_zero(x: &[u64]) -> bool {
    for &word in x {
        if word != 0 {
            return false;
        }
    }
    true
}

/// Returns true if x == 1
pub fn is_one(x: &[u64]) -> bool {
    if x[0] != 1 {
        return false;
    }
    for &word in &x[1..] {
        if word != 0 {
            return false;
        }
    }
    true
}

/// Returns true if x is a power of two
pub fn is_power_of_two(x: &[u64]) -> bool {
    // A multiple-word number is a power of two if it has exactly one bit set across all words
    let mut found_one = false;
    for &word in x {
        if word != 0 {
            if found_one || (word & (word - 1)) != 0 {
                return false;
            }
            found_one = true;
        }
    }
    found_one
}

/// Returns true if x fits in a single 64-bit word (i.e., x < 2^64).
pub fn is_short(x: &[u64]) -> bool {
    for &word in &x[1..] {
        if word != 0 {
            return false;
        }
    }
    true
}
