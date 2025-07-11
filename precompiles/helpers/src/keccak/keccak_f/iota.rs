use circuit::{GateState, PinId};

use super::{bit_position, KECCAK_F_RC};

/// Keccak-f ι step.
/// 1. For all triples (x, y, z) such that 0 ≤ x,y < 5, and 0 ≤ z < 64, let A′\[x, y, z] = A\[x, y, z]
/// 2. Let RC = 0w.
/// 3. For j from 0 to l, let RC\[2^j – 1] = rc(j + 7ir).
/// 4. For all z such that 0 ≤ z < 64, let A′\[0, 0, z] = A′\[0, 0, z] ^ RC\[z].
/// 5. Return A′.
pub fn keccak_f_iota(s: &mut GateState, ir: u64) {
    // Step 1: Copy all state bits
    for x in 0..5 {
        for y in 0..5 {
            for z in 0..64 {
                let pos = bit_position(x, y, z);
                s.sout_refs[pos] = s.sin_refs[pos];
            }
        }
    }

    // Step 4: Apply round constants to lane (0,0)
    for z in 0..64 {
        // Since XOR(a, 0) = a, we can skip the XOR if the constant bit is zero
        if KECCAK_F_RC[ir as usize][z] == 1 {
            let pos = bit_position(0, 0, z);
            let aux = s.get_free_ref();

            // XOR with one
            s.xor3(
                s.gate_config.zero_ref.unwrap(),
                PinId::B,
                s.sout_refs[pos],
                PinId::D,
                s.gate_config.zero_ref.unwrap(),
                PinId::A,
                aux,
            );

            s.sout_refs[pos] = aux;
        }
    }
}
