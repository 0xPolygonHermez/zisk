//! Shared types and algorithms for the ZisK guest and host.

/// Returns the greatest common divisor of `a` and `b` using the Euclidean algorithm.
pub fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}
