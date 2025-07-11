use circuit::{GateConfig, GateState, PinId};

mod keccak_constants;
mod keccak_f;
mod keccak_input;

pub use keccak_constants::{BITRATE, BYTERATE};
use keccak_f::keccak_f;
use keccak_input::KeccakInput;

pub const KECCAKF_INPUT_SIZE_BITS: u64 = 1600;
pub const KECCAKF_OUTPUT_SIZE_BITS: u64 = 1600;
pub const KECCAKF_INPUT_BITS_IN_PARALLEL: u64 = 4;
pub const KECCAKF_OUTPUT_BITS_IN_PARALLEL: u64 = 4;

pub const KECCAKF_CHUNKS: u64 = 9;
pub const KECCAKF_BITS: u64 = 7;
const KECCAKF_NUM: u64 = KECCAKF_CHUNKS * KECCAKF_BITS;

const KECCAKF_CIRCUIT_SIZE: u64 = 93846;

// Keccak Configuration
#[rustfmt::skip]
pub static KECCAK_GATE_CONFIG: GateConfig = GateConfig::with_values(
    KECCAKF_CIRCUIT_SIZE,
    KECCAKF_CIRCUIT_SIZE + 1,
    Some(0),
    1 + KECCAKF_NUM,
    KECCAKF_INPUT_BITS_IN_PARALLEL,
    KECCAKF_INPUT_SIZE_BITS,
    KECCAKF_NUM,
    1 + KECCAKF_NUM + KECCAKF_INPUT_SIZE_BITS * KECCAKF_NUM / KECCAKF_INPUT_BITS_IN_PARALLEL,
    KECCAKF_OUTPUT_BITS_IN_PARALLEL,
    KECCAKF_OUTPUT_SIZE_BITS,
    KECCAKF_NUM,
);

// Main Keccak function
// Input is a buffer of any length, including 0
// Output is a 256 bits long buffer
pub fn keccak(
    input: &[u8],
    output: &mut [u8; 32],
    get_circuit_topology: bool,
) -> Option<GateState> {
    let mut input_state = KeccakInput::new(input);
    let mut state = GateState::new(KECCAK_GATE_CONFIG.clone());
    let mut r = [0u8; BITRATE];
    while input_state.get_next_bits(&mut r) {
        // Copy input bits to the state
        for (i, &bit) in r.iter().enumerate() {
            let group = i as u64 / KECCAK_GATE_CONFIG.sin_ref_group_by;
            let group_pos = i as u64 % KECCAK_GATE_CONFIG.sin_ref_group_by;
            let ref_idx = KECCAK_GATE_CONFIG.sin_first_ref
                + group * KECCAK_GATE_CONFIG.sin_ref_distance
                + group_pos;
            state.gates[ref_idx as usize].pins[PinId::A].bit ^= bit;
        }

        keccak_f(&mut state);

        if get_circuit_topology {
            #[cfg(debug_assertions)]
            state.print_circuit_topology();

            // The keccakf circuit topology is completely known after a single execution
            return Some(state);
        }

        state.copy_sout_to_sin_and_reset_refs();
    }

    state.get_output(output, false);

    None
}

// Get the circuit topology of the Keccak-f permutation
pub fn keccakf_topology() -> GateState {
    // Hash any input and stop when a single keccakf has been computed
    let input = b"";
    let mut output = [0u8; 32];
    keccak(input, &mut output, true).expect("Failed to get circuit topology")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topology() {
        let _ = keccakf_topology();
    }

    #[test]
    fn test_empty_input() {
        let input = b"";
        let mut output = [0u8; 32];
        keccak(input, &mut output, false);

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
        keccak(input, &mut output, false);

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
        keccak(input, &mut output, false);

        // Expected Keccak-256 hash of "The quick brown fox jumps over the lazy dog"
        let expected_hash: [u8; 32] = [
            0x77, 0xbd, 0x88, 0x3c, 0xce, 0xb4, 0xb6, 0x72, 0x44, 0x5d, 0x9e, 0xb7, 0x08, 0x90,
            0x08, 0xbe, 0xbe, 0xcb, 0xd3, 0x0b, 0xac, 0x62, 0xd6, 0xe9, 0xe5, 0xfe, 0xc1, 0x47,
            0xe9, 0xa0, 0x89, 0x1e,
        ];

        assert_eq!(output[..], expected_hash[..]);
    }
}
