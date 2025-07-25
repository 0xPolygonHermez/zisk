#![allow(clippy::needless_range_loop)]

use circuit::{GateState, PinId};

use super::bit_position;

/// Keccak-f θ step.
/// 1. For all pairs (x, z) such that 0 ≤ x < 5 and 0 ≤ z < 64:  
///    C\[x, z] = A\[x, 0, z] ^ A\[x, 1, z] ^ A\[x, 2, z] ^ A\[x, 3, z] ^ A\[x, 4, z]
/// 2. For all pairs (x, z) such that 0 ≤ x < 5 and 0 ≤ z < 64:  
///    D\[x, z] = C\[(x-1) mod 5, z] ^ C\[(x+1) mod 5, (z –1) mod 64]
/// 3. For all triples (x, y, z) such that 0 ≤ x,y < 5, and 0 ≤ z < 64:   
///    A′\[x, y, z] = A\[x, y, z] ^ D\[x, z]
pub fn keccak_f_theta(s: &mut GateState, ir: u64) {
    // Step 1: C[x, z] = A[x, 0, z] ^ A[x, 1, z] ^ A[x, 2, z] ^ A[x, 3, z] ^ A[x, 4, z]
    let mut c = [[0u64; 64]; 5];
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

            // aux = A[x, 0, z] ^ A[x, 1, z] ^ A[x, 2, z]
            let aux = s.get_free_ref();
            if ir == 0 {
                let group_0 = positions[0] as u64 / s.gate_config.sin_ref_group_by;
                let group_pos_0 = positions[0] as u64 % s.gate_config.sin_ref_group_by;
                // First round uses pin_a directly
                assert_eq!(
                    s.sin_refs[positions[0]],
                    s.gate_config.sin_first_ref
                        + s.gate_config.sin_ref_distance * group_0
                        + group_pos_0
                );
                let group_1 = positions[1] as u64 / s.gate_config.sin_ref_group_by;
                let group_pos_1 = positions[1] as u64 % s.gate_config.sin_ref_group_by;
                assert_eq!(
                    s.sin_refs[positions[1]],
                    s.gate_config.sin_first_ref
                        + s.gate_config.sin_ref_distance * group_1
                        + group_pos_1
                );

                let group_2 = positions[2] as u64 / s.gate_config.sin_ref_group_by;
                let group_pos_2 = positions[2] as u64 % s.gate_config.sin_ref_group_by;
                assert_eq!(
                    s.sin_refs[positions[2]],
                    s.gate_config.sin_first_ref
                        + s.gate_config.sin_ref_distance * group_2
                        + group_pos_2
                );

                s.xor3(
                    s.sin_refs[positions[0]],
                    PinId::A,
                    s.sin_refs[positions[1]],
                    PinId::A,
                    s.sin_refs[positions[2]],
                    PinId::A,
                    aux,
                );
            } else {
                s.xor3(
                    s.sin_refs[positions[0]],
                    PinId::D,
                    s.sin_refs[positions[1]],
                    PinId::D,
                    s.sin_refs[positions[2]],
                    PinId::D,
                    aux,
                );
            }

            // C[x, z] = aux ^ A[x, 3, z] ^ A[x, 4, z]
            let cxy = s.get_free_ref();
            c[x][z] = cxy;
            if ir == 0 {
                let group_3 = positions[3] as u64 / s.gate_config.sin_ref_group_by;
                let group_pos_3 = positions[3] as u64 % s.gate_config.sin_ref_group_by;
                assert_eq!(
                    s.sin_refs[positions[3]],
                    s.gate_config.sin_first_ref
                        + s.gate_config.sin_ref_distance * group_3
                        + group_pos_3
                );
                s.xor3(
                    aux,
                    PinId::D,
                    s.sin_refs[positions[3]],
                    PinId::A,
                    s.sin_refs[positions[4]],
                    PinId::A,
                    cxy,
                );
            } else {
                s.xor3(
                    aux,
                    PinId::D,
                    s.sin_refs[positions[3]],
                    PinId::D,
                    s.sin_refs[positions[4]],
                    PinId::D,
                    cxy,
                );
            }
        }
    }

    // Step 2: Compute D[x, z] = C[(x-1) mod 5, z] ^ C[(x+1) mod 5, (z–1) mod 64]
    // Step 3: Compute A'[x,y,z] = A[x, y, z] ^ D[x, z]
    for z in 0..64 {
        for x in 0..5 {
            for y in 0..5 {
                let pos = bit_position(x, y, z);
                let aux = if ir == 0 {
                    // In the first round we use the first 1600 Sin bit slots to store these gates
                    let group = pos as u64 / s.gate_config.sin_ref_group_by;
                    let group_pos = pos as u64 % s.gate_config.sin_ref_group_by;
                    let ref_idx = s.gate_config.sin_first_ref
                        + s.gate_config.sin_ref_distance * group
                        + group_pos;
                    assert_eq!(s.sin_refs[pos], ref_idx);
                    s.xor3(
                        ref_idx,
                        PinId::A,
                        c[(x + 4) % 5][z],
                        PinId::D,
                        c[(x + 1) % 5][(z + 63) % 64],
                        PinId::D,
                        ref_idx,
                    );
                    ref_idx
                } else {
                    let ref_idx = s.get_free_ref();
                    s.xor3(
                        s.sin_refs[pos],
                        PinId::D,
                        c[(x + 4) % 5][z],
                        PinId::D,
                        c[(x + 1) % 5][(z + 63) % 64],
                        PinId::D,
                        ref_idx,
                    );
                    ref_idx
                };

                s.sout_refs[pos] = aux;
            }
        }
    }
}
