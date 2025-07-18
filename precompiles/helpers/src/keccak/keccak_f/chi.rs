use circuit::{GateState, PinId};

use super::bit_position;

/// Keccak-f χ step.
/// 1. For all triples (x, y, z) such that 0 ≤ x,y < 5 and 0 ≤ z < 64 compute:
///    A′\[x, y, z] = A\[x, y, z] ^ ((¬A\[x+1 (mod 5), y, z]) & A\[x+2 (mod 5), y, z])
/// 2. Return A′
pub fn keccak_f_chi(s: &mut GateState) {
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

                // Compute A[x, y, z] ^ ((¬A[(x+1) mod 5, y, z]) & A[(x+2) mod 5, y, z])
                let result = s.get_free_ref();
                s.xor_andp(a_x_y_z, PinId::D, a_x1_y_z, PinId::D, a_x2_y_z, PinId::D, result);

                // Store result in output references
                s.sout_refs[bit_position(x, y, z)] = result;
            }
        }
    }
}
