use circuit::GateState;

use super::bit_position;

/// Keccak-f ρ step.
/// 1. For all z such that 0 ≤ z < 64, let A′\[0, 0, z] = A\[0, 0, z]
/// 2. Let (x, y) = (1, 0)
/// 3. For t from 0 to 23:  
///    a. For all z such that 0 ≤ z < 64, let A′\[x, y, z] = A\[x, y, (z – (t +1)(t + 2)/2) mod 64]  
///    b. Let (x, y) = (y, (2x + 3y) mod 5)
/// 4. Return A′
pub fn keccak_f_rho(s: &mut GateState) {
    // Step 1: Copy all z bits at (0,0) position
    for z in 0..64 {
        let pos = bit_position(0, 0, z);
        s.sout_refs[pos] = s.sin_refs[pos];
    }

    // Step 2: Initialize coordinates
    let mut x = 1;
    let mut y = 0;

    // Step 3: Process 24 rounds
    for t in 0..24 {
        // Calculate the rotation offset
        let offset = ((t + 1) * (t + 2)) / 2;

        // Process all z bits at current (x,y) position
        for z in 0..64 {
            // Calculate source z position with mod 64 arithmetic
            let src_z = (z + 64 - (offset % 64)) % 64;
            let src_pos = bit_position(x, y, src_z);
            let dst_pos = bit_position(x, y, z);

            // Copy reference with rotation
            s.sout_refs[dst_pos] = s.sin_refs[src_pos];
        }

        // let (x, y) = (y, (2x + 3y) mod 5)
        let aux = y;
        y = (2 * x + 3 * y) % 5;
        x = aux;
    }
}
