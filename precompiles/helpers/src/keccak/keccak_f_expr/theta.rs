#![allow(clippy::needless_range_loop)]

use circuit::{ExpressionManager, ExpressionOp};

use super::bit_position;

/// Keccak-f **θ step**.
///
/// 1. For all pairs `(x, z)` such that `0 ≤ x < 5` and `0 ≤ z < 64`:
///    `C[x, z] = A[x, 0, z] ^ A[x, 1, z] ^ A[x, 2, z] ^ A[x, 3, z] ^ A[x, 4, z]`
///
/// 2. For all pairs `(x, z)` such that `0 ≤ x < 5` and `0 ≤ z < 64`:  
///    `D[x, z] = C[(x - 1) mod 5, z] ^ C[(x + 1) mod 5, (z - 1) mod 64]`
///
/// 3. For all triples `(x, y, z)` such that `0 ≤ x, y < 5` and `0 ≤ z < 64`:  
///    `A′[x, y, z] = A[x, y, z] ^ D[x, z]`
pub fn keccak_f_theta(e: &mut ExpressionManager) {
    e.set_subcontext(Some(
        "θ: C[x, z] = A[x, 0, z] ^ A[x, 1, z] ^ A[x, 2, z] ^ A[x, 3, z] ^ A[x, 4, z]",
    ));

    // Step 1: C[x, z] = A[x, 0, z] ^ A[x, 1, z] ^ A[x, 2, z] ^ A[x, 3, z] ^ A[x, 4, z]
    let mut exp_c = [[0usize; 64]; 5];
    for x in 0..5 {
        for z in 0..64 {
            // Get all y positions for this x,z
            let positions = [
                bit_position(x, 0, z),
                bit_position(x, 1, z),
                bit_position(x, 2, z),
                bit_position(x, 3, z),
                bit_position(x, 4, z),
            ];

            // First round uses pin_a directly

            // aux1 = A[x, 0, z] ^ A[x, 1, z]
            let exp_aux1 = e.create_op_expression(
                &ExpressionOp::Xor,
                e.sin_expr_ids[positions[0]],
                e.sin_expr_ids[positions[1]],
            );

            // aux2 = aux1 ^ A[x, 2, z]
            let exp_aux2 =
                e.create_op_expression(&ExpressionOp::Xor, exp_aux1, e.sin_expr_ids[positions[2]]);

            // aux3 = aux2 ^ A[x, 3, z]
            let exp_aux3 =
                e.create_op_expression(&ExpressionOp::Xor, exp_aux2, e.sin_expr_ids[positions[3]]);

            // C[x, z] = aux3 ^ A[x, 4, z]
            exp_c[x][z] =
                e.create_op_expression(&ExpressionOp::Xor, exp_aux3, e.sin_expr_ids[positions[4]]);
        }
    }

    // Step 2: Compute D[x, z] = C[(x-1) mod 5, z] ^ C[(x+1) mod 5, (z –1) mod 64]
    e.set_subcontext(Some("θ: D[x, z] = C[(x-1) mod 5, z] ^ C[(x+1) mod 5, (z –1) mod 64]"));
    let mut exp_d = [[0usize; 64]; 5];
    for x in 0..5 {
        for z in 0..64 {
            exp_d[x][z] = e.create_op_expression(
                &ExpressionOp::Xor,
                exp_c[(x + 4) % 5][z],
                exp_c[(x + 1) % 5][(z + 63) % 64],
            );
        }
    }

    // Step 3: Compute A'[x,y,z] = A[x, y, z] ^ D[x, z]
    e.set_subcontext(Some("θ: A'[x,y,z] = A[x, y, z] ^ D[x, z]"));
    for x in 0..5 {
        for y in 0..5 {
            for z in 0..64 {
                let pos = bit_position(x, y, z);

                e.sout_expr_ids[pos] =
                    e.create_op_expression(&ExpressionOp::Xor, e.sin_expr_ids[pos], exp_d[x][z]);
            }
        }
    }
}
