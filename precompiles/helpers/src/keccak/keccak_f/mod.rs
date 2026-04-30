mod constants;
mod round;
mod utils;

use constants::*;
pub use round::keccak_f_round;
pub use utils::*;

/// State representation as 5x5x64 bits.
/// The maximum value that any expression during keccakf computation can get is 144, which fits in a u8.
// Operation summary:
//  - The θ.1 step has 4 add, this gives a number in the range <= 5
//  - The θ.2 step has 1 add, this gives a number in the range <= 10
//  - The θ.3 step has 1 add, this gives a number in the range <= 11
//  - The χ.1 step has 1 add and 1 prod, this gives a number in the range <= 132
//  - The χ.2 step has 1 add, this gives a number in the range <= 143
//  - The ι step has 1 add, this gives a number in the range <= 144
pub type KeccakState = [[[u8; 64]; 5]; 5];

/// Full Keccak-f[1600] permutation
pub fn keccak_f(state: &mut KeccakState) {
    for round in 0..24 {
        keccak_f_round(state, round);

        // Reduce the state modulo 2
        reduce_state_mod2(state);
    }
}

/// Reduce the state modulo 2 by applying modulo 2 to each bit in the state
fn reduce_state_mod2(state: &mut KeccakState) {
    state.iter_mut().flatten().flatten().for_each(|bit| *bit %= 2);
}

#[cfg(test)]
mod tests {
    use super::{keccak_f, utils::keccakf_state_from_linear, KeccakState};

    /// Convert from 5x5x64 bit array to linear [u64; 25]
    #[allow(clippy::needless_range_loop)]
    pub fn keccakf_state_to_linear(state: &KeccakState) -> [u64; 25] {
        let mut linear = [0u64; 25];
        for x in 0..5 {
            for y in 0..5 {
                let mut word = 0u64;
                for z in 0..64 {
                    if state[x][y][z] == 1 {
                        word |= 1u64 << z;
                    }
                }
                linear[x + y * 5] = word;
            }
        }
        linear
    }

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
    fn test_keccak_f_full_state() {
        let state_linear = [0xFFFFFFFFFFFFFFFFu64; 25];
        let mut state = keccakf_state_from_linear(&state_linear);
        keccak_f(&mut state);
        let state_linear = keccakf_state_to_linear(&state);

        assert_eq!(
            state_linear,
            [
                0x9F00F21BBA6817C4,
                0xCDF5AA0D21AF5E78,
                0xD6539ABF24095B97,
                0x8BB6F30A010F8228,
                0xF0F711BA0547331D,
                0x4F44330558EB182F,
                0x2213B79D9055207C,
                0xEB5E5B55CA4FB490,
                0xBFAEB81A299B5D4,
                0x9E5D924F1A65ED48,
                0x4650C533B7BFB3,
                0xDDAD454B84D7AB05,
                0xF03CE56503E82921,
                0xCE442E92C6728660,
                0x1A9CE5E4B37DDCD3,
                0xF63B60E27CEA6F0E,
                0xCC4CC7FCA665BFAD,
                0x40CF4EBA54A2285D,
                0x2725F1F142304213,
                0x554D327DE6FBAD9B,
                0x19866A26CBC8BDC2,
                0xE8C3C28FAF02C7F5,
                0xC6BC1F3512A665AE,
                0xCAA831F1A5DC86CE,
                0x3F82AFE91CA4B9B0,
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
}
