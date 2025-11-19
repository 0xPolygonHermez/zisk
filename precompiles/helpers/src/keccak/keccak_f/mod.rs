mod round;
mod utils;

use round::keccak_f_round;
pub use utils::*;

/// State representation as 5x5x64 bits
pub type KeccakStateBits = [[[u64; 64]; 5]; 5];

/// Full Keccak-f[1600] permutation
pub fn keccak_f(state: &mut KeccakStateBits) {
    for round in 0..24 {
        keccak_f_round(state, round);

        // Reduce the state modulo 2
        reduce_state_mod2(state);
    }
}

fn reduce_state_mod2(state: &mut KeccakStateBits) {
    state.iter_mut().flatten().flatten().for_each(|bit| *bit %= 2);
}

/// Iterator that yields the state after each round of Keccak-f
pub struct KeccakRoundIterator {
    state: KeccakStateBits,
    round: usize,
    initial_returned: bool,
}

impl KeccakRoundIterator {
    pub fn new(initial_state: KeccakStateBits) -> Self {
        Self { state: initial_state, round: 0, initial_returned: false }
    }
}

impl Iterator for KeccakRoundIterator {
    type Item = (KeccakStateBits, usize); // (state, round_number)

    fn next(&mut self) -> Option<Self::Item> {
        // First return the initial state as round 0
        if !self.initial_returned {
            self.initial_returned = true;
            return Some((self.state, 0));
        }

        // Check if we've completed all 24 rounds
        if self.round >= 24 {
            return None;
        }

        // Perform one round of Keccak-f
        keccak_f_round(&mut self.state, self.round);

        // Return the unreduced state (before modulo 2)
        let unreduced_state = self.state;

        // Apply reduction for the next round
        reduce_state_mod2(&mut self.state);

        self.round += 1;

        Some((unreduced_state, self.round))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = if self.initial_returned {
            24 - self.round
        } else {
            25 - self.round // Initial state + 24 rounds
        };
        (remaining, Some(remaining))
    }
}

/// Function-based iterator for Keccak-f rounds  
pub fn keccak_f_rounds(initial_state: KeccakStateBits) -> KeccakRoundIterator {
    KeccakRoundIterator::new(initial_state)
}

/// Iterator that yields just the states (without round numbers)
pub fn keccak_f_round_states(
    initial_state: KeccakStateBits,
) -> impl Iterator<Item = KeccakStateBits> {
    keccak_f_rounds(initial_state).map(|(state, _)| state)
}

#[cfg(test)]
mod tests {
    use super::{
        keccak_f, keccak_f_round_states, reduce_state_mod2,
        utils::{keccakf_state_from_linear, keccakf_state_to_linear},
        KeccakRoundIterator,
    };

    #[test]
    fn test_keccak_f_zero_state() {
        let state_linear = [0u64; 25];
        let mut state = keccakf_state_from_linear(&state_linear);
        keccak_f(&mut state);
        let state_linear = keccakf_state_to_linear(&state);

        assert_eq!(
            state_linear,
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
        let state_linear = [
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
        let mut state = keccakf_state_from_linear(&state_linear);
        keccak_f(&mut state);
        let state_linear = keccakf_state_to_linear(&state);

        assert_eq!(
            state_linear,
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

    #[test]
    fn test_keccak_f_round_iterator() {
        let initial_state_linear = [0u64; 25];
        let initial_state = keccakf_state_from_linear(&initial_state_linear);
        let round_iter = KeccakRoundIterator::new(initial_state);

        // Test that we get exactly 25 rounds
        let all_rounds: Vec<_> = round_iter.collect();
        assert_eq!(all_rounds.len(), 25);

        // Test that round numbers are correct
        for (i, (_, round_num)) in all_rounds.iter().enumerate() {
            assert_eq!(*round_num, i);
        }

        // Test that round 0 is the initial state
        let (round_0_state, round_0_num) = &all_rounds[0];
        assert_eq!(*round_0_num, 0);
        let round_0_linear = keccakf_state_to_linear(round_0_state);
        assert_eq!(round_0_linear, initial_state_linear);

        // Test that the final state (after reduction) matches the full keccak_f result
        let final_unreduced_state = all_rounds.last().unwrap().0;

        // Apply reduction to the final unreduced state to compare with keccak_f result
        let mut final_reduced_state = final_unreduced_state;
        reduce_state_mod2(&mut final_reduced_state);
        let final_reduced_linear = keccakf_state_to_linear(&final_reduced_state);

        let mut expected_state = keccakf_state_from_linear(&initial_state_linear);
        keccak_f(&mut expected_state);
        let expected_linear = keccakf_state_to_linear(&expected_state);

        assert_eq!(final_reduced_linear, expected_linear);
    }

    #[test]
    fn test_keccak_f_round_states() {
        let initial_state_linear = [0u64; 25];
        let initial_state = keccakf_state_from_linear(&initial_state_linear);
        let states: Vec<_> = keccak_f_round_states(initial_state).collect();

        assert_eq!(states.len(), 25);

        // Final state should match full keccak_f (after reduction)
        let mut final_unreduced_state = states[24];
        reduce_state_mod2(&mut final_unreduced_state);
        let final_reduced_linear = keccakf_state_to_linear(&final_unreduced_state);

        let mut expected_final = keccakf_state_from_linear(&initial_state_linear);
        keccak_f(&mut expected_final);
        let expected_linear = keccakf_state_to_linear(&expected_final);

        assert_eq!(final_reduced_linear, expected_linear);
    }
}
