use circuit::{ExpressionManager, ExpressionOp};

use super::bit_position;

/// Keccak-f **χ step**.
///
/// 1. For all triples `(x, y, z)` such that `0 ≤ x, y < 5` and `0 ≤ z < 64`, compute:  
///    `A′[x, y, z] = A[x, y, z] ^ (¬A[(x + 1) mod 5, y, z] & A[(x + 2) mod 5, y, z])`
///
/// 2. Return `A′`
pub fn keccak_f_chi(e: &mut ExpressionManager) {
    e.set_subcontext(Some(
        "χ: A'[x,y,z] = A[x, y, z] ^ (¬A[(x + 1) mod 5, y, z] & A[(x + 2) mod 5, y, z])",
    ));
    for x in 0..5 {
        let x1 = (x + 1) % 5;
        let x2 = (x + 2) % 5;
        for y in 0..5 {
            for z in 0..64 {
                // Calculate array positions
                let positions =
                    [bit_position(x1, y, z), bit_position(x2, y, z), bit_position(x, y, z)];

                // Compute (¬A[x+1 (mod 5),y,z]) & A[x+2 (mod 5),y,z]
                let exp_aux1 = e.create_op_expression(
                    &ExpressionOp::Nand,
                    e.sin_expr_ids[positions[0]],
                    e.sin_expr_ids[positions[1]],
                );

                // Compute final XOR
                let exp_aux2 = e.create_op_expression(
                    &ExpressionOp::Xor,
                    e.sin_expr_ids[positions[2]],
                    exp_aux1,
                );

                // Store result back in A'
                e.sout_expr_ids[positions[2]] = exp_aux2;
            }
        }
    }
}
