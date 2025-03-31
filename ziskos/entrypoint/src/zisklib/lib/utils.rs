/// Given two 256-bit number `x` and `y`, compares them and returns true if `x > y`; otherwise, false.
pub(super) fn gt(x: &[u64; 4], y: &[u64; 4]) -> bool {
    for i in (0..4).rev() {
        if x[i] > y[i] {
            return true;
        } else if x[i] < y[i] {
            return false;
        }
    }
    false
}

/// Given two 256-bit unsigned integers `x` and `y`, computes the subtraction `x - y`.
pub(super) fn sub(x: &[u64; 4], y: &[u64; 4]) -> [u64; 4] {
    let mut result = [0u64; 4];
    let mut borrow = 0u64;
    for i in 0..4 {
        let xi = x[i];
        let yi = y[i] + borrow;
        if xi >= yi {
            result[i] = xi - yi;
            borrow = 0;
        } else {
            let r = (1u128 << 64) + xi as u128 - yi as u128;
            result[i] = r as u64;
            borrow = 1;
        }
    }

    result
}
