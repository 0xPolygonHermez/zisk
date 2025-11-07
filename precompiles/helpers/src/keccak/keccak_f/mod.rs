mod round;

use round::keccak_f_round;

/// Full Keccak-f[1600] permutation
pub fn keccak_f(state: &mut [u64; 25]) {
    for round in 0..24 {
        keccak_f_round(state, round);
    }
}

/// Iterator that yields the state after each round of Keccak-f
pub struct KeccakRoundIterator {
    state: [u64; 25],
    round: usize,
}

impl KeccakRoundIterator {
    pub fn new(initial_state: [u64; 25]) -> Self {
        Self { state: initial_state, round: 0 }
    }
}

impl Iterator for KeccakRoundIterator {
    type Item = ([u64; 25], usize); // (state_after_round, round_number)

    fn next(&mut self) -> Option<Self::Item> {
        if self.round >= 24 {
            return None;
        }

        let current_round = self.round;

        // Perform one round of Keccak-f
        keccak_f_round(&mut self.state, current_round);

        self.round += 1;

        Some((self.state, current_round))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = 24 - self.round;
        (remaining, Some(remaining))
    }
}

/// Function-based iterator for Keccak-f rounds  
pub fn keccak_f_rounds(initial_state: [u64; 25]) -> KeccakRoundIterator {
    KeccakRoundIterator::new(initial_state)
}

/// Iterator that yields just the states (without round numbers)
pub fn keccak_f_round_states(initial_state: [u64; 25]) -> impl Iterator<Item = [u64; 25]> {
    keccak_f_rounds(initial_state).map(|(state, _)| state)
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

    #[test]
    fn test_keccak_f_round_iterator() {
        let initial_state = [0u64; 25];
        let round_iter = KeccakRoundIterator::new(initial_state);

        // Test that we get exactly 24 rounds
        let all_rounds: Vec<_> = round_iter.collect();
        assert_eq!(all_rounds.len(), 24);

        // Test that round numbers are correct
        for (i, (_, round_num)) in all_rounds.iter().enumerate() {
            assert_eq!(*round_num, i);
        }

        // Test that the final state matches the full keccak_f result
        let final_state = all_rounds.last().unwrap().0;
        let mut expected_state = [0u64; 25];
        keccak_f(&mut expected_state);
        assert_eq!(final_state, expected_state);
    }

    #[test]
    fn test_keccak_f_round_states() {
        let initial_state = [0u64; 25];
        let states: Vec<_> = keccak_f_round_states(initial_state).collect();

        assert_eq!(states.len(), 24);

        // Final state should match full keccak_f
        let mut expected_final = [0u64; 25];
        keccak_f(&mut expected_final);
        assert_eq!(states[23], expected_final);
    }
}
