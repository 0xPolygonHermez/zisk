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

            // aux1 = A[x, 0, z] ^ A[x, 1, z]
            let aux1 = s.get_free_ref();
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
                s.xor(s.sin_refs[positions[0]], PinId::A, s.sin_refs[positions[1]], PinId::A, aux1);
            } else {
                s.xor_res(s.sin_refs[positions[0]], s.sin_refs[positions[1]], aux1);
            }

            // aux2 = aux1 ^ A[x, 2, z]
            let aux2 = s.get_free_ref();
            if ir == 0 {
                let group_2 = positions[2] as u64 / s.gate_config.sin_ref_group_by;
                let group_pos_2 = positions[2] as u64 % s.gate_config.sin_ref_group_by;
                assert_eq!(
                    s.sin_refs[positions[2]],
                    s.gate_config.sin_first_ref
                        + s.gate_config.sin_ref_distance * group_2
                        + group_pos_2
                );
                s.xor(s.sin_refs[positions[2]], PinId::A, aux1, PinId::D, aux2);
            } else {
                s.xor_res(aux1, s.sin_refs[positions[2]], aux2);
            }

            // aux3 = aux2 ^ A[x, 3, z]
            let aux3 = s.get_free_ref();
            if ir == 0 {
                let group_3 = positions[3] as u64 / s.gate_config.sin_ref_group_by;
                let group_pos_3 = positions[3] as u64 % s.gate_config.sin_ref_group_by;
                assert_eq!(
                    s.sin_refs[positions[3]],
                    s.gate_config.sin_first_ref
                        + s.gate_config.sin_ref_distance * group_3
                        + group_pos_3
                );
                s.xor(s.sin_refs[positions[3]], PinId::A, aux2, PinId::D, aux3);
            } else {
                s.xor_res(aux2, s.sin_refs[positions[3]], aux3);
            }

            // C[x, z] = aux3 ^ A[x, 4, z]
            let free_ref = s.get_free_ref();
            c[x][z] = free_ref;
            if ir == 0 {
                let group_4 = positions[4] as u64 / s.gate_config.sin_ref_group_by;
                let group_pos_4 = positions[4] as u64 % s.gate_config.sin_ref_group_by;
                assert_eq!(
                    s.sin_refs[positions[4]],
                    s.gate_config.sin_first_ref
                        + s.gate_config.sin_ref_distance * group_4
                        + group_pos_4
                );
                s.xor(s.sin_refs[positions[4]], PinId::A, aux3, PinId::D, free_ref);
            } else {
                s.xor_res(aux3, s.sin_refs[positions[4]], free_ref);
            }
        }
    }

    // Step 2: Compute D[x, z] = C[(x-1) mod 5, z] ^ C[(x+1) mod 5, (z –1) mod 64]
    let mut d = [[0u64; 64]; 5];
    for x in 0..5 {
        for z in 0..64 {
            let free_ref = s.get_free_ref();
            d[x][z] = free_ref;
            s.xor_res(c[(x + 4) % 5][z], c[(x + 1) % 5][(z + 63) % 64], free_ref);
        }
    }

    // Step 3: Compute A'[x,y,z] = A[x, y, z] ^ D[x, z]
    for x in 0..5 {
        for y in 0..5 {
            for z in 0..64 {
                let pos = bit_position(x, y, z);
                let aux = if ir == 0 {
                    // In the first round we use the first 1600 Sin bit slots to store these gates
                    let group = pos as u64 / s.gate_config.sin_ref_group_by;
                    let group_pos = pos as u64 % s.gate_config.sin_ref_group_by;
                    let ref_idx = s.gate_config.sin_first_ref
                        + s.gate_config.sin_ref_distance * group
                        + group_pos;
                    assert_eq!(s.sin_refs[pos], ref_idx);
                    s.xor(ref_idx, PinId::A, d[x][z], PinId::D, ref_idx);
                    ref_idx
                } else {
                    let ref_idx = s.get_free_ref();
                    s.xor_res(s.sin_refs[pos], d[x][z], ref_idx);
                    ref_idx
                };

                s.sout_refs[pos] = aux;
            }
        }
    }
}
