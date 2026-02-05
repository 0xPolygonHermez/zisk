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
