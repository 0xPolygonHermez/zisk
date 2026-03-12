/// Check if a pointer is 8-byte aligned.
#[inline(always)]
pub fn is_aligned_8(ptr: *const u8) -> bool {
    (ptr as usize) & 0x7 == 0
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
