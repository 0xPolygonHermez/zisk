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

use crate::BITRATE;

pub fn keccak_f(s: &mut GateState) {
    // Apply all 24 rounds of Keccak permutations
    for ir in 0..24 {
        // θ step
        if ir == 23 {
            s.print_refs(&s.sin_refs, "Before θ")
        };
        keccak_f_theta(s, ir);
        s.copy_sout_refs_to_sin_refs();
        if ir == 23 {
            s.print_refs(&s.sin_refs, "After θ")
        };

        // ρ step
        if ir == 23 {
            s.print_refs(&s.sin_refs, "Before ρ")
        };
        keccak_f_rho(s);
        s.copy_sout_refs_to_sin_refs();
        // if ir == 23 {s.print_refs(&s.sin_refs, "After ρ")};

        // π step
        // if ir == 23 {s.print_refs(&s.sin_refs, "Before π")};
        keccak_f_pi(s);
        s.copy_sout_refs_to_sin_refs();
        if ir == 23 {
            s.print_refs(&s.sin_refs, "After π")
        };

        // χ step
        if ir == 23 {
            s.print_refs(&s.sin_refs, "Before χ")
        };
        keccak_f_chi(s, ir);
        s.copy_sout_refs_to_sin_refs();
        if ir == 23 {
            s.print_refs(&s.sin_refs, "After χ")
        };

        // ι step
        if ir == 23 {
            s.print_refs(&s.sin_refs, "Before ι")
        };
        keccak_f_iota(s, ir);

        // Don't copy after last round
        if ir != 23 {
            s.copy_sout_refs_to_sin_refs();
            if ir == 23 {
                s.print_refs(&s.sin_refs, "After ι")
            };
        } else {
            s.print_refs(&s.sout_refs, "After ι")
        }
    }

    // Add BITRATE more gates to make sure that Sout is located in the expected gates,
    // both in pin a and r
    for i in 0usize..BITRATE {
        let rel_dis = i % s.gate_config.sin_ref_group_by as usize;
        let aux = if rel_dis == 0 {
            s.gate_config.sout_ref0
                + i as u64 * s.gate_config.sout_ref_distance / s.gate_config.sin_ref_group_by as u64
        } else {
            s.sout_refs[i - 1] + rel_dis as u64
        };
        s.xor(s.sout_refs[i], PinId::C, s.gate_config.zero_ref, PinId::A, aux);
        s.sout_refs[i] = aux;
    }
}
