use circuit::{ExpressionManager, GateConfig, GateState, PinId};

mod chi;
mod iota;
mod pi;
mod rho;
mod theta;
mod round_constants;
mod utils;

use chi::keccak_f_chi;
use iota::keccak_f_iota;
use pi::keccak_f_pi;
use rho::keccak_f_rho;
use theta::keccak_f_theta;
use round_constants::KECCAK_F_RC;
use utils::{bit_position, bits_to_state, state_to_bits};

const KECCAKF_INPUT_BITS_IN_PARALLEL: u64 = 1;
const KECCAKF_OUTPUT_BITS_IN_PARALLEL: u64 = 1;
const KECCAKF_CHUNKS: u64 = 1;
const KECCAKF_BITS: u64 = 1;
const KECCAKF_NUM: u64 = KECCAKF_CHUNKS * KECCAKF_BITS;
const KECCAKF_CIRCUIT_SIZE: u64 = 155286;

const KECCAKF_EXPR_RESET_THRESHOLD: u32 = 1 << 20;

// Keccak Configuration
#[rustfmt::skip]
static KECCAK_GATE_CONFIG: GateConfig = GateConfig::with_values(
    KECCAKF_CIRCUIT_SIZE,
    KECCAKF_CIRCUIT_SIZE + 1,
    Some(0),
    KECCAKF_NUM,
    KECCAKF_INPUT_BITS_IN_PARALLEL,
    1600,
    KECCAKF_NUM,
    KECCAKF_NUM + 1600 * KECCAKF_NUM / KECCAKF_INPUT_BITS_IN_PARALLEL,
    KECCAKF_OUTPUT_BITS_IN_PARALLEL,
    1600,
    KECCAKF_NUM,
);

pub fn keccak_f(state: &mut [u64; 25]) {
    // Initialize the gate state and expression manager
    let mut gate_state = GateState::new(KECCAK_GATE_CONFIG.clone());
    let mut expr_manager = ExpressionManager::new(KECCAKF_EXPR_RESET_THRESHOLD, 1600, 1600);

    // Copy input bits to the state
    let state_in_bits = state_to_bits(state);
    let sin_ref_group_by = gate_state.config.sin_ref_group_by;
    let sin_first_ref = gate_state.config.sin_first_ref;
    let sin_ref_distance = gate_state.config.sin_ref_distance;
    for (i, &bit) in state_in_bits.iter().enumerate() {
        let group = i as u64 / sin_ref_group_by;
        let group_pos = i as u64 % sin_ref_group_by;
        let ref_idx = sin_first_ref + group * sin_ref_distance + group_pos;
        gate_state.gates[ref_idx as usize].pins[PinId::A].bit = bit;
    }

    // Apply all 24 rounds of Keccak permutations
    for r in 0..24 {
        // θ step
        expr_manager.set_context(r, "θ");
        keccak_f_theta(&mut gate_state, &mut expr_manager, r);
        gate_state.copy_sout_refs_to_sin_refs();
        expr_manager.copy_sout_expr_ids_to_sin_expr_ids();

        // ρ step
        expr_manager.set_context(r, "ρ");
        keccak_f_rho(&mut gate_state, &mut expr_manager);
        gate_state.copy_sout_refs_to_sin_refs();
        expr_manager.copy_sout_expr_ids_to_sin_expr_ids();

        // π step
        expr_manager.set_context(r, "π");
        keccak_f_pi(&mut gate_state, &mut expr_manager);
        gate_state.copy_sout_refs_to_sin_refs();
        expr_manager.copy_sout_expr_ids_to_sin_expr_ids();

        // χ step
        expr_manager.set_context(r, "χ");
        keccak_f_chi(&mut gate_state, &mut expr_manager);
        gate_state.copy_sout_refs_to_sin_refs();
        expr_manager.copy_sout_expr_ids_to_sin_expr_ids();

        // ι step
        expr_manager.set_context(r, "ι");
        keccak_f_iota(&mut gate_state, &mut expr_manager, r);
        if r != 23 {
            // Reset expressions after each round
            expr_manager.set_context(r, "End of round");
            for i in 0..1600 {
                expr_manager.sout_expr_ids[i] = expr_manager.create_reset_expression(expr_manager.sout_expr_ids[i], true, None);
            }

            gate_state.copy_sout_refs_to_sin_refs();
            expr_manager.copy_sout_expr_ids_to_sin_expr_ids();

            // // Create proxy expressions
            // for i in 0..1600 {
            //     expr_manager.create_proxy_expression(expr_manager.sin_expr_ids[i]);
            // }
        }

        expr_manager.print_round_events(r, Some(10));
    }

    let mut state_out_bits = [0u8; 1600];
    let sout_ref_group_by = gate_state.config.sout_ref_group_by;
    let sout_first_ref = gate_state.config.sout_first_ref;
    let sout_ref_distance = gate_state.config.sout_ref_distance;
    for i in 0..1600 {
        // Add gates to make sure that the output is located in the expected gates
        let group = i / sout_ref_group_by;
        let group_pos = i % sout_ref_group_by;
        let ref_idx = sout_first_ref + group * sout_ref_distance + group_pos;
        gate_state.xor2(
            gate_state.sout_refs[i as usize],
            PinId::D,
            gate_state.config.zero_ref.unwrap(),
            PinId::A,
            ref_idx,
        );
        gate_state.sout_refs[i as usize] = ref_idx;

        // Get the output bits
        state_out_bits[i as usize] = gate_state.gates[ref_idx as usize].pins[PinId::A].bit;
    }

    // Print final expression summary and circuit topology
    expr_manager.print_expression_summary();

    // Export expressions to file
    if let Err(e) = expr_manager.export_expressions_to_file("keccak_expressions.txt") {
        eprintln!("Failed to export expressions: {}", e);
    }

    *state = bits_to_state(&state_out_bits);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keccak_f_zero_state() {
        let mut state = [0u64; 25];
        keccak_f(&mut state);
        assert_eq!(
            state,
            [
                0xF1258F7940E1DDE7,
                0x84D5CCF933C0478A,
                0xD598261EA65AA9EE,
                0xBD1547306F80494D,
                0x8B284E056253D057,
                0xFF97A42D7F8E6FD4,
                0x90FEE5A0A44647C4,
                0x8C5BDA0CD6192E76,
                0xAD30A6F71B19059C,
                0x30935AB7D08FFC64,
                0xEB5AA93F2317D635,
                0xA9A6E6260D712103,
                0x81A57C16DBCF555F,
                0x43B831CD0347C826,
                0x01F22F1A11A5569F,
                0x05E5635A21D9AE61,
                0x64BEFEF28CC970F2,
                0x613670957BC46611,
                0xB87C5A554FD00ECB,
                0x8C3EE88A1CCF32C8,
                0x940C7922AE3A2614,
                0x1841F924A2C509E4,
                0x16F53526E70465C2,
                0x75F644E97F30A13B,
                0xEAF1FF7B5CECA249,
            ]
        );
    }

    #[test]
    fn test_keccak_f_nonzero_state() {
        let mut state = [
            0xF1258F7940E1DDE7,
            0x84D5CCF933C0478A,
            0xD598261EA65AA9EE,
            0xBD1547306F80494D,
            0x8B284E056253D057,
            0xFF97A42D7F8E6FD4,
            0x90FEE5A0A44647C4,
            0x8C5BDA0CD6192E76,
            0xAD30A6F71B19059C,
            0x30935AB7D08FFC64,
            0xEB5AA93F2317D635,
            0xA9A6E6260D712103,
            0x81A57C16DBCF555F,
            0x43B831CD0347C826,
            0x01F22F1A11A5569F,
            0x05E5635A21D9AE61,
            0x64BEFEF28CC970F2,
            0x613670957BC46611,
            0xB87C5A554FD00ECB,
            0x8C3EE88A1CCF32C8,
            0x940C7922AE3A2614,
            0x1841F924A2C509E4,
            0x16F53526E70465C2,
            0x75F644E97F30A13B,
            0xEAF1FF7B5CECA249,
        ];
        keccak_f(&mut state);
        assert_eq!(
            state,
            [
                0x2D5C954DF96ECB3C,
                0x6A332CD07057B56D,
                0x093D8D1270D76B6C,
                0x8A20D9B25569D094,
                0x4F9C4F99E5E7F156,
                0xF957B9A2DA65FB38,
                0x85773DAE1275AF0D,
                0xFAF4F247C3D810F7,
                0x1F1B9EE6F79A8759,
                0xE4FECC0FEE98B425,
                0x68CE61B6B9CE68A1,
                0xDEEA66C4BA8F974F,
                0x33C43D836EAFB1F5,
                0xE00654042719DBD9,
                0x7CF8A9F009831265,
                0xFD5449A6BF174743,
                0x97DDAD33D8994B40,
                0x48EAD5FC5D0BE774,
                0xE3B8C8EE55B7B03C,
                0x91A0226E649E42E9,
                0x900E3129E7BADD7B,
                0x202A9EC5FAA3CCE8,
                0x5B3402464E1C3DB6,
                0x609F4E62A44C1059,
                0x20D06CD26A8FBF5C,
            ]
        );
    }
}
