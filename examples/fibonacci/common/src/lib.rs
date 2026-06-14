//! Shared types and algorithms for the ZisK guest and host.

/// 256-bit unsigned integer re-exported from [`ruint`].
pub use ruint::aliases::U256;

/// Returns the zero-indexed *n*-th Fibonacci number as a [`U256`].
pub fn fibonacci(n: u8) -> U256 {
    if n == 0 {
        return U256::ZERO;
    }

    let mut a = U256::ZERO;
    let mut b = U256::ONE;

    for _ in 1..n {
        let c = a + b;
        a = b;
        b = c;
    }

    b
}
