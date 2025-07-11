mod chi;
mod iota;
mod pi;
mod rho;
mod round_constants;
mod theta;
mod utils;

pub use round_constants::KECCAK_F_RC;
pub use utils::bit_position;

use chi::keccak_f_chi;
use iota::keccak_f_iota;
use pi::keccak_f_pi;
use rho::keccak_f_rho;
use theta::keccak_f_theta;

use circuit::{GateState, PinId};

pub fn keccak_f(s: &mut GateState) {
    // Instead of adding 1600 dummy gates to introduce the input bits,
    // we exploit the Keccak-f θ step structure to introduce them
    // In particular, since we have to perform:
    //      A′[x, y, z] = A[x, y, z] ^ D[x, z]
    // We use this XOR to introduce the input bits

    // Apply all 24 rounds of Keccak permutations
    for ir in 0..24 {
        // θ step
        keccak_f_theta(s, ir);
        s.copy_sout_refs_to_sin_refs();

        // ρ step
        keccak_f_rho(s);
        s.copy_sout_refs_to_sin_refs();

        // π step
        keccak_f_pi(s);
        s.copy_sout_refs_to_sin_refs();

        // χ step
        keccak_f_chi(s);
        s.copy_sout_refs_to_sin_refs();

        // ι step
        keccak_f_iota(s, ir);

        // Don't copy after last round
        if ir != 23 {
            s.copy_sout_refs_to_sin_refs();
        }
    }

    // Add BITRATE more gates to make sure that the output is located in the expected gates
    for i in 0..s.gate_config.sout_ref_number {
        let group = i / s.gate_config.sout_ref_group_by;
        let group_pos = i % s.gate_config.sout_ref_group_by;
        let ref_idx =
            s.gate_config.sout_first_ref + group * s.gate_config.sout_ref_distance + group_pos;
        s.xor3(
            s.sout_refs[i as usize],
            PinId::D,
            s.gate_config.zero_ref.unwrap(),
            PinId::A,
            s.gate_config.zero_ref.unwrap(),
            PinId::A,
            ref_idx,
        );
        s.sout_refs[i as usize] = ref_idx;
    }
}
