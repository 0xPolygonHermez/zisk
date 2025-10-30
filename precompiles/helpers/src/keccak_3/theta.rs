#![allow(clippy::needless_range_loop)]

use circuit::{ExpressionManager, ExpressionOp, GateState, PinId};

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
pub fn keccak_f_theta(s: &mut GateState, e: &mut ExpressionManager, ir: usize) {
    // Step 1: C[x, z] = A[x, 0, z] ^ A[x, 1, z] ^ A[x, 2, z] ^ A[x, 3, z] ^ A[x, 4, z]
    e.set_subcontext("θ: C[x, z] = A[x, 0, z] ^ A[x, 1, z] ^ A[x, 2, z] ^ A[x, 3, z] ^ A[x, 4, z]");
    let mut c = [[0u64; 64]; 5];
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
            let exp_aux1 = e.create_op_expression(&ExpressionOp::Xor, e.sin_expr_ids[positions[0]], e.sin_expr_ids[positions[1]]);
            let aux1 = s.get_free_ref();
            if ir == 0 {
                let group_0 = positions[0] as u64 / s.config.sin_ref_group_by;
                let group_pos_0 = positions[0] as u64 % s.config.sin_ref_group_by;
                assert_eq!(
                    s.sin_refs[positions[0]],
                    s.config.sin_first_ref + s.config.sin_ref_distance * group_0 + group_pos_0
                );
                let group_1 = positions[1] as u64 / s.config.sin_ref_group_by;
                let group_pos_1 = positions[1] as u64 % s.config.sin_ref_group_by;
                assert_eq!(
                    s.sin_refs[positions[1]],
                    s.config.sin_first_ref + s.config.sin_ref_distance * group_1 + group_pos_1
                );
                s.xor2(
                    s.sin_refs[positions[0]],
                    PinId::A,
                    s.sin_refs[positions[1]],
                    PinId::A,
                    aux1,
                );
            } else {
                s.xor2(
                    s.sin_refs[positions[0]],
                    PinId::D,
                    s.sin_refs[positions[1]],
                    PinId::D,
                    aux1,
                );
            }

            // aux2 = aux1 ^ A[x, 2, z]
            let exp_aux2 = e.create_op_expression(&ExpressionOp::Xor, exp_aux1, e.sin_expr_ids[positions[2]]);
            let aux2 = s.get_free_ref();
            if ir == 0 {
                let group_2 = positions[2] as u64 / s.config.sin_ref_group_by;
                let group_pos_2 = positions[2] as u64 % s.config.sin_ref_group_by;
                assert_eq!(
                    s.sin_refs[positions[2]],
                    s.config.sin_first_ref + s.config.sin_ref_distance * group_2 + group_pos_2
                );
                s.xor2(s.sin_refs[positions[2]], PinId::A, aux1, PinId::D, aux2);
            } else {
                s.xor2(s.sin_refs[positions[2]], PinId::D, aux1, PinId::D, aux2);
            }

            // aux3 = aux2 ^ A[x, 3, z]
            let exp_aux3 = e.create_op_expression(&ExpressionOp::Xor, exp_aux2, e.sin_expr_ids[positions[3]]);
            let aux3 = s.get_free_ref();
            if ir == 0 {
                let group_3 = positions[3] as u64 / s.config.sin_ref_group_by;
                let group_pos_3 = positions[3] as u64 % s.config.sin_ref_group_by;
                assert_eq!(
                    s.sin_refs[positions[3]],
                    s.config.sin_first_ref + s.config.sin_ref_distance * group_3 + group_pos_3
                );
                s.xor2(s.sin_refs[positions[3]], PinId::A, aux2, PinId::D, aux3);
            } else {
                s.xor2(s.sin_refs[positions[3]], PinId::D, aux2, PinId::D, aux3);
            }

            // C[x, z] = aux3 ^ A[x, 4, z]
            exp_c[x][z] = e.create_op_expression(&ExpressionOp::Xor, exp_aux3, e.sin_expr_ids[positions[4]]);
            let free_ref = s.get_free_ref();
            c[x][z] = free_ref;
            if ir == 0 {
                let group_4 = positions[4] as u64 / s.config.sin_ref_group_by;
                let group_pos_4 = positions[4] as u64 % s.config.sin_ref_group_by;
                assert_eq!(
                    s.sin_refs[positions[4]],
                    s.config.sin_first_ref + s.config.sin_ref_distance * group_4 + group_pos_4
                );
                s.xor2(s.sin_refs[positions[4]], PinId::A, aux3, PinId::D, free_ref);
            } else {
                s.xor2(s.sin_refs[positions[4]], PinId::D, aux3, PinId::D, free_ref);
            }
        }
    }

    // Step 2: Compute D[x, z] = C[(x-1) mod 5, z] ^ C[(x+1) mod 5, (z –1) mod 64]
    e.set_subcontext("θ: D[x, z] = C[(x-1) mod 5, z] ^ C[(x+1) mod 5, (z –1) mod 64]");
    let mut d = [[0u64; 64]; 5];
    let mut exp_d = [[0usize; 64]; 5];
    for x in 0..5 {
        for z in 0..64 {
            exp_d[x][z] = e.create_op_expression(&ExpressionOp::Xor, exp_c[(x + 4) % 5][z], exp_c[(x + 1) % 5][(z + 63) % 64]);
            let free_ref = s.get_free_ref();
            d[x][z] = free_ref;
            s.xor2(c[(x + 4) % 5][z], PinId::D, c[(x + 1) % 5][(z + 63) % 64], PinId::D, free_ref);
        }
    }

    // Step 3: Compute A'[x,y,z] = A[x, y, z] ^ D[x, z]
    e.set_subcontext("θ: A'[x,y,z] = A[x, y, z] ^ D[x, z]");
    for x in 0..5 {
        for y in 0..5 {
            for z in 0..64 {
                let pos = bit_position(x, y, z);

                e.sout_expr_ids[pos] = e.create_op_expression(&ExpressionOp::Xor, e.sin_expr_ids[pos], exp_d[x][z]);

                let aux = if ir == 0 {
                    // In the first round we use the first 1600 Sin bit slots to store these gates
                    let group = pos as u64 / s.config.sin_ref_group_by;
                    let group_pos = pos as u64 % s.config.sin_ref_group_by;
                    let ref_idx =
                        s.config.sin_first_ref + s.config.sin_ref_distance * group + group_pos;
                    assert_eq!(s.sin_refs[pos], ref_idx);
                    s.xor2(s.sin_refs[pos], PinId::A, d[x][z], PinId::D, ref_idx);
                    ref_idx
                } else {
                    let ref_idx = s.get_free_ref();
                    s.xor2(s.sin_refs[pos], PinId::D, d[x][z], PinId::D, ref_idx);
                    ref_idx
                };

                s.sout_refs[pos] = aux;
            }
        }
    }
}
