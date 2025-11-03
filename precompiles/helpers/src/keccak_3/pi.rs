use circuit::ExpressionManager;

use super::bit_position;

/// Keccak-f **π step**.
///
/// 1. For all triples `(x, y, z)` such that `0 ≤ x, y < 5` and `0 ≤ z < 64`, let  
///    `A′[x, y, z] = A[(x + 3y) mod 5, x, z]`
///
/// 2. Return `A′`
pub fn keccak_f_pi(e: &mut ExpressionManager) {
    for x in 0..5 {
        for y in 0..5 {
            for z in 0..64 {
                // Calculate source position with mod 5 arithmetic
                let src_x = (x + 3 * y) % 5;
                let src_pos = bit_position(src_x, x, z);
                let dst_pos = bit_position(x, y, z);

                // Copy reference
                e.sout_expr_ids[dst_pos] = e.sin_expr_ids[src_pos];
            }
        }
    }
}
