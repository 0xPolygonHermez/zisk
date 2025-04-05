use circuit::{GateConfig, GateState, PinId};

use super::{keccak_f::keccak_f, KeccakInput, BITRATE};

// Keccak Configuration
pub static KECCAK_GATE_CONFIG: GateConfig = GateConfig {
    zero_ref: 0,
    slot_size: 155286,
    max_refs: 160000,
    first_next_ref: 1,
    sin_ref0: 61,
    sin_ref_group_by: 2,
    sin_ref_number: 1600,
    sin_ref_distance: 60,
    sout_ref0: 61 + 1600 * 30,
    sout_ref_group_by: 2,
    sout_ref_number: 1600,
    sout_ref_distance: 60,
    pol_length: 1 << 22,
};

// Main Keccak function
// Input is a buffer of any length, including 0
// Output is a 256 bits long buffer
pub fn keccak(input: &[u8], output: &mut [u8; 32]) {
    let mut input_state = KeccakInput::new(input);
    println!("Input: {:?}", input_state);
    let mut state = GateState::new(KECCAK_GATE_CONFIG.clone());
    let mut r = [0u8; BITRATE];
    while input_state.get_next_bits(&mut r) {
        println!("Input bits: {:?}", r);
        // Copy input bits to the state
        let mut ref_idx = 0;
        for (i, &bit) in r.iter().enumerate() {
            let rel_pos = i % KECCAK_GATE_CONFIG.sin_ref_group_by as usize;
            // let ref_idx = KECCAK_GATE_CONFIG.sin_ref0 + i as u64 * KECCAK_GATE_CONFIG.sin_ref_distance;
            ref_idx = if rel_pos == 0 {
                KECCAK_GATE_CONFIG.sin_ref0
                    + i as u64 * KECCAK_GATE_CONFIG.sin_ref_distance
                        / KECCAK_GATE_CONFIG.sin_ref_group_by as u64
            } else {
                ref_idx + rel_pos as u64
            };
            state.gates[ref_idx as usize].pins[PinId::A].bit ^= bit;
        }

        keccak_f(&mut state);
        state.print_counters();
        state.copy_sout_to_sin_and_reset_refs();
    }

    state.get_output(output);
}

// Unit tests
#[cfg(test)]
mod tests {
    use super::*;
    use circuit::{GateConfig, GateState};

    #[test]
    fn test_empty_input() {
        let input = b"";
        let mut output = [0u8; 32];
        keccak(input, &mut output);
        println!("Output: {:?}", output);

        // Expected Keccak-256 hash of empty input
        let expected_hash: [u8; 32] = [
            0xc5, 0xd2, 0x46, 0x01, 0x86, 0xf7, 0x23, 0x3c, 0x92, 0x7e, 0x7d, 0xb2, 0xdc, 0xc7,
            0x03, 0xc0, 0xe5, 0x00, 0xb6, 0x53, 0xca, 0x82, 0x27, 0x3b, 0x7b, 0xfa, 0xd8, 0x04,
            0x5d, 0x85, 0xa4, 0x70,
        ];
        assert_eq!(output[..], expected_hash[..]);
    }

    #[test]
    fn test_keccak_short() {
        let input = b"Hello, world!";
        let mut output = [0u8; 32];
        keccak(input, &mut output);

        // Expected Keccak-256 hash of "Hello, world!"
        let expected_hash: [u8; 32] = [
            0xb6, 0xe1, 0x6d, 0x27, 0xac, 0x5a, 0xb4, 0x27, 0xa7, 0xf6, 0x89, 0x00, 0xac, 0x55,
            0x59, 0xce, 0x27, 0x2d, 0xc6, 0xc3, 0x7c, 0x82, 0xb3, 0xe0, 0x52, 0x24, 0x6c, 0x82,
            0x24, 0x4c, 0x50, 0xe4,
        ];

        assert_eq!(output[..], expected_hash[..]);
    }

    #[test]
    fn test_keccak_long() {
        let input = b"The quick brown fox jumps over the lazy dog.The quick brown fox jumps over the lazy dog.The quick brown fox jumps over the lazy dog.The quick brown fox jumps over the lazy dog.The quick brown fox jumps over the lazy dog.The quick brown fox jumps over the lazy dog.The quick brown fox jumps over the lazy dog.The quick brown fox jumps over the lazy dog.The quick brown fox jumps over the lazy dog.The quick brown fox jumps over the lazy dog.The quick brown fox jumps over the lazy dog.The quick brown fox jumps over the lazy dog.The quick brown fox jumps over the lazy dog...";
        let mut output = [0u8; 32];
        keccak(input, &mut output);

        // Expected Keccak-256 hash of "The quick brown fox jumps over the lazy dog"
        let expected_hash: [u8; 32] = [
            0x77, 0xbd, 0x88, 0x3c, 0xce, 0xb4, 0xb6, 0x72, 0x44, 0x5d, 0x9e, 0xb7, 0x08, 0x90,
            0x08, 0xbe, 0xbe, 0xcb, 0xd3, 0x0b, 0xac, 0x62, 0xd6, 0xe9, 0xe5, 0xfe, 0xc1, 0x47,
            0xe9, 0xa0, 0x89, 0x1e,
        ];

        assert_eq!(output[..], expected_hash[..]);
    }
}
