use circuit::GateState;

use super::{bit_position, KECCAK_F_RC};

/// Keccak-f χ step.
/// 1. For all triples (x, y, z) such that 0 ≤ x,y < 5 and 0 ≤ z < 64 compute:  
///     A′\[x, y, z] = A\[x, y, z] ^ ((A\[(x+1) mod 5, y, z] ^ 1) ⋅ A\[(x+2) mod 5, y, z])
/// 2. Return A′
pub fn keccak_f_chi(s: &mut GateState, ir: u64) {
    // Iterate through all 5x5x64 state bits
    for x in 0..5 {
        for y in 0..5 {
            for z in 0..64 {
                // Calculate array positions
                let x1 = (x + 1) % 5;
                let x2 = (x + 2) % 5;

                // Get references to the input bits
                let a_x1_y_z = s.sin_refs[bit_position(x1, y, z)];
                let a_x2_y_z = s.sin_refs[bit_position(x2, y, z)];
                let a_x_y_z = s.sin_refs[bit_position(x, y, z)];

                // Compute (A[(x+1)%5,y,z] ⊕ 1) ⋅ A[(x+2)%5,y,z]
                let aux1 = s.get_free_ref();
                s.andp_res(a_x1_y_z, a_x2_y_z, aux1);

                // Compute final XOR
                let aux2 = s.get_free_ref();

                // Special case for last round
                if ir == 23 && !(x == 0 && y == 0) || (KECCAK_F_RC[ir as usize][z as usize] == 0) {
                    s.xor_res(aux1, a_x_y_z, aux2);
                } else {
                    s.xor_res(aux1, a_x_y_z, aux2);
                }

                // Store result in output references
                s.sout_refs[bit_position(x, y, z)] = aux2;
            }
        }
    }
}
