mod chi;
mod iota;
mod pi;
mod rho;
mod round_constants;
mod theta;
mod utils;

pub(self) use round_constants::KECCAK_F_RC;
pub(self) use utils::bit_position;

use chi::keccak_f_chi;
use iota::keccak_f_iota;
use pi::keccak_f_pi;
use rho::keccak_f_rho;
use theta::keccak_f_theta;

use circuit::{GateState, PinId};

pub fn keccak_f(s: &mut GateState) {
    // Apply all 24 rounds of Keccak permutations
    for ir in 0..24 {
        // θ step
        #[cfg(debug_assertions)]
        if ir == 0 {
            s.print_refs(&s.sin_refs, "Before θ")
        };

        keccak_f_theta(s, ir);
        s.copy_sout_refs_to_sin_refs();

        #[cfg(debug_assertions)]
        if ir == 0 {
            s.print_refs(&s.sin_refs, "After θ")
        };

        // ρ step
        #[cfg(debug_assertions)]
        if ir == 0 {
            s.print_refs(&s.sin_refs, "Before ρ")
        };

        keccak_f_rho(s);
        s.copy_sout_refs_to_sin_refs();

        #[cfg(debug_assertions)]
        if ir == 0 {
            s.print_refs(&s.sin_refs, "After ρ")
        };

        // π step
        #[cfg(debug_assertions)]
        if ir == 0 {
            s.print_refs(&s.sin_refs, "Before π")
        };

        keccak_f_pi(s);
        s.copy_sout_refs_to_sin_refs();

        #[cfg(debug_assertions)]
        if ir == 0 {
            s.print_refs(&s.sin_refs, "After π")
        };

        // χ step
        #[cfg(debug_assertions)]
        if ir == 0 {
            s.print_refs(&s.sin_refs, "Before χ")
        };

        keccak_f_chi(s, ir);
        s.copy_sout_refs_to_sin_refs();

        #[cfg(debug_assertions)]
        if ir == 0 {
            s.print_refs(&s.sin_refs, "After χ")
        };

        // ι step
        #[cfg(debug_assertions)]
        if ir == 0 {
            s.print_refs(&s.sin_refs, "Before ι")
        };

        keccak_f_iota(s, ir);

        // Don't copy after last round
        if ir != 23 {
            s.copy_sout_refs_to_sin_refs();

            #[cfg(debug_assertions)]
            if ir == 0 {
                s.print_refs(&s.sin_refs, "After ι")
            };
        }
    }

    // Add BITRATE more gates to make sure that the output is located in the expected gates
    for i in 0..s.gate_config.sout_ref_number {
        let group = i / s.gate_config.sout_ref_group_by;
        let group_pos = i % s.gate_config.sout_ref_group_by;
        let ref_idx =
            s.gate_config.sout_first_ref + group * s.gate_config.sout_ref_distance + group_pos;
        s.xor(s.sout_refs[i as usize], PinId::C, s.gate_config.zero_ref, PinId::A, ref_idx);
        s.sout_refs[i as usize] = ref_idx;
    }
}
