use circuit::{ExpressionManager, ExpressionOp};

use super::{bit_position, KECCAK_F_RC};

/// Keccak-f **ι step**.
///
/// 1. For all triples `(x, y, z)` such that `0 ≤ x, y < 5` and `0 ≤ z < 64`, let  
///    `A′[x, y, z] = A[x, y, z]`
///
/// 2. For all `z` such that `0 ≤ z < 64`, let  
///    `A′[0, 0, z] = A′[0, 0, z] ^ RC[z]`
///
/// 3. Return `A′`
#[allow(clippy::needless_range_loop)]
pub fn keccak_f_iota(e: &mut ExpressionManager, ir: usize) {
    // Step 1: Copy all state bits
    for x in 0..5 {
        for y in 0..5 {
            for z in 0..64 {
                let pos = bit_position(x, y, z);
                e.sout_expr_ids[pos] = e.sin_expr_ids[pos];
            }
        }
    }

    // Step 2: Apply round constants to lane (0,0)
    e.set_subcontext(Some("ι: A'[0, 0, z] = A'[0, 0, z] ^ RC[z]"));
    for z in 0..64 {
        // Since XOR(a, 0) = a, we can skip the XOR if the constant bit is zero
        if KECCAK_F_RC[ir][z] == 1 {
            let pos = bit_position(0, 0, z);

            // XOR with one
            let exp_aux =
                e.create_op_expression(&ExpressionOp::Xor, e.sout_expr_ids[pos], e.one_expr_id);

            // Store result back in A'
            e.sout_expr_ids[pos] = exp_aux;
        }
    }
}
