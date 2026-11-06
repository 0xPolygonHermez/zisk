//! Pippenger multi-scalar multiplication, shared by G1 and G2.

#[cfg(zisk_guest)]
use crate::alloc_extern::vec;

use super::super::utils::{get_scalar_window, optimal_window_size};

/// A scalar reduced mod r has at most 255 bits.
const SCALAR_BITS: usize = 255;

/// Computes k1·P1 + ... + kn·Pn with Pippenger's bucket method. `add` must be a complete
/// addition (identity and equal operands included).
///
/// # Soundness
/// Every point must be on-curve, in the subgroup and have **canonical** coordinates
/// (`x, y < p`); every scalar must be reduced mod r.
pub(crate) fn msm_pippenger<P: Copy>(
    points: &[P],
    scalars: &[[u64; 4]],
    identity: P,
    mut add: impl FnMut(&P, &P) -> P,
) -> P {
    debug_assert_eq!(points.len(), scalars.len());
    if points.is_empty() {
        return identity;
    }

    let w = optimal_window_size(points.len());
    let mut buckets = vec![identity; (1 << w) - 1];

    let mut result = identity;
    for window_idx in (0..SCALAR_BITS.div_ceil(w)).rev() {
        for _ in 0..w {
            result = add(&result, &result);
        }

        // Each point goes to the bucket of its window digit
        buckets.fill(identity);
        for (p, k) in points.iter().zip(scalars) {
            let digit = get_scalar_window(k, window_idx, w) as usize;
            if digit != 0 {
                buckets[digit - 1] = add(&buckets[digit - 1], p);
            }
        }

        // Σ j·bucket[j] as the sum of the running suffix sums
        let mut running = identity;
        let mut window_sum = identity;
        for bucket in buckets.iter().rev() {
            running = add(&running, bucket);
            window_sum = add(&window_sum, &running);
        }

        result = add(&result, &window_sum);
    }
    result
}
