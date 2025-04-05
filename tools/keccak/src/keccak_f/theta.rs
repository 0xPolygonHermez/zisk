use circuit::{GateState, PinId};

use super::bit_position;

/// Keccak-f θ step
/// Steps:
/// 1. For all pairs (x, z) such that 0 ≤ x < 5 and 0 ≤ z < 64:
///     C\[x, z] = A\[x, 0, z] ^ A\[x, 1, z] ^ A\[x, 2, z] ^ A\[x, 3, z] ^ A\[x, 4, z]
/// 2. For all pairs (x, z) such that 0 ≤ x < 5 and 0≤ z < 64:
///     D\[x, z] = C\[(x-1) mod 5, z] ^ C\[(x+1) mod 5, (z –1) mod 64]
/// 3. For all triples (x, y, z) such that 0 ≤ x <5, 0 ≤ y < 5, and 0 ≤ z < 64:
///     A′\[x, y, z] = A\[x, y, z] ^ D\[x, z]
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
                let rel_dist_0 = positions[0] % s.gate_config.sin_ref_group_by as usize;
                let rel_dist_1 = positions[1] % s.gate_config.sin_ref_group_by as usize;
                // First round uses pin_a directly
                assert_eq!(
                    s.sin_refs[positions[0]],
                    if rel_dist_0 == 0 {
                        s.gate_config.sin_ref0
                            + s.gate_config.sin_ref_distance * positions[0] as u64
                                / s.gate_config.sin_ref_group_by as u64
                    } else {
                        s.sin_refs[positions[0] - 1] + rel_dist_0 as u64
                    }
                );
                assert_eq!(
                    s.sin_refs[positions[1]],
                    if rel_dist_1 == 0 {
                        s.gate_config.sin_ref0
                            + s.gate_config.sin_ref_distance * positions[1] as u64
                                / s.gate_config.sin_ref_group_by as u64
                    } else {
                        s.sin_refs[positions[1] - 1] + rel_dist_1 as u64
                    }
                );
                s.xor(s.sin_refs[positions[0]], PinId::A, s.sin_refs[positions[1]], PinId::A, aux1);
            } else {
                s.xor_res(s.sin_refs[positions[0]], s.sin_refs[positions[1]], aux1);
            }

            // aux2 = aux1 ^ A[x, 2, z]
            let aux2 = s.get_free_ref();
            if ir == 0 {
                let rel_dist_2 = positions[2] % s.gate_config.sin_ref_group_by as usize;
                assert_eq!(
                    s.sin_refs[positions[2]],
                    if rel_dist_2 == 0 {
                        s.gate_config.sin_ref0
                            + s.gate_config.sin_ref_distance * positions[2] as u64
                                / s.gate_config.sin_ref_group_by as u64
                    } else {
                        s.sin_refs[positions[2] - 1] + rel_dist_2 as u64
                    }
                );
                s.xor(aux1, PinId::C, s.sin_refs[positions[2]], PinId::A, aux2);
            } else {
                s.xor_res(aux1, s.sin_refs[positions[2]], aux2);
            }

            // aux3 = aux2 ^ A[x, 3, z]
            let aux3 = s.get_free_ref();
            if ir == 0 {
                let rel_dist_3 = positions[3] % s.gate_config.sin_ref_group_by as usize;
                assert_eq!(
                    s.sin_refs[positions[3]],
                    if rel_dist_3 == 0 {
                        s.gate_config.sin_ref0
                            + s.gate_config.sin_ref_distance * positions[3] as u64
                                / s.gate_config.sin_ref_group_by as u64
                    } else {
                        s.sin_refs[positions[3] - 1] + rel_dist_3 as u64
                    }
                );
                s.xor(aux2, PinId::C, s.sin_refs[positions[3]], PinId::A, aux3);
            } else {
                s.xor_res(aux2, s.sin_refs[positions[3]], aux3);
            }

            // C[x, z] = aux3 ^ A[x, 4, z]
            let free_ref = s.get_free_ref();
            c[x][z] = free_ref as u64;
            if ir == 0 {
                let rel_dist_4 = positions[4] % s.gate_config.sin_ref_group_by as usize;
                assert_eq!(
                    s.sin_refs[positions[4]],
                    if rel_dist_4 == 0 {
                        s.gate_config.sin_ref0
                            + s.gate_config.sin_ref_distance * positions[4] as u64
                                / s.gate_config.sin_ref_group_by as u64
                    } else {
                        s.sin_refs[positions[4] - 1] + rel_dist_4 as u64
                    }
                );
                s.xor(aux3, PinId::C, s.sin_refs[positions[4]], PinId::A, free_ref);
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
            d[x][z] = free_ref as u64;
            s.xor_res(c[(x + 4) % 5][z], c[(x + 1) % 5][(z + 63) % 64], free_ref);
        }
    }

    // Step 3: Compute A'[x,y,z] = A[x, y, z] ^ D[x, z]
    for x in 0..5 {
        for y in 0..5 {
            for z in 0..64 {
                let pos = bit_position(x, y, z);
                let mut aux = 0;
                if ir == 0 {
                    // In the first round we use the first 1600 Sin bit slots to store these gates
                    let rel_dist = pos % s.gate_config.sin_ref_group_by as usize;
                    aux = if rel_dist == 0 {
                        s.gate_config.sin_ref0
                            + s.gate_config.sin_ref_distance * pos as u64
                                / s.gate_config.sin_ref_group_by as u64
                    } else {
                        s.sin_refs[pos - 1] + rel_dist as u64
                    };
                    assert_eq!(s.sin_refs[pos], aux);
                    s.xor(aux, PinId::A, d[x][z], PinId::C, aux);
                } else {
                    aux = s.get_free_ref();
                    s.xor_res(s.sin_refs[pos], d[x][z], aux);
                };

                s.sout_refs[pos] = aux;
            }
        }
    }
}
