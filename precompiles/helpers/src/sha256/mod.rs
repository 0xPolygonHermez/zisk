use std::cell::RefCell;

use circuit::{u32_to_bits, GateConfig, GateState, PinId};

mod sha256_constants;
mod sha256_input;
mod sha256f;

pub use sha256_constants::{SHA256_BLOCK_SIZE_BITS, SHA256_BLOCK_SIZE_BYTES};
use sha256_input::Sha256Input;
use sha256f::sha256f;

// Keccak Configuration
#[rustfmt::skip]
pub static SHA256F_GATE_CONFIG: GateConfig = GateConfig::with_values(
    160480,
    170000,
    Some(0),
    64,
    2,
    768, // 256 (hash state bits) + 512 (input bits)
    63,
    64 + 768 * 63 / 2,
    2,
    256, // 256 (output bits)
    63,
);

// Main Keccak function
// Input is a buffer of any length, including 0
// Output is a 256 bits long buffer
pub fn sha256(
    input: &[u8],
    output: &mut [u8; 32],
    get_circuit_topology: bool,
) -> Option<GateState> {
    // Initialize the gate state
    let gate_state = RefCell::new(GateState::new(SHA256F_GATE_CONFIG.clone()));

    // Initialize the input and perform the padding
    let mut input = Sha256Input::new(input);

    // Initial hash values (first 32 bits of fractional parts of square roots of first 8 primes)
    const INITIAL_HASH_STATE: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    // Copy the initial hash state bits into the state
    for i in 0..8 {
        let bits = u32_to_bits(INITIAL_HASH_STATE[i]);
        for j in 0..32 {
            let group = (i * 32 + j) as u64 / SHA256F_GATE_CONFIG.sin_ref_group_by;
            let group_pos = (i * 32 + j) as u64 % SHA256F_GATE_CONFIG.sin_ref_group_by;
            let ref_idx = SHA256F_GATE_CONFIG.sin_first_ref
                + group * SHA256F_GATE_CONFIG.sin_ref_distance
                + group_pos;
            gate_state.borrow_mut().gates[ref_idx as usize].pins[PinId::A].bit = bits[j];
        }
    }

    // Process each block
    let mut block = [0u8; SHA256_BLOCK_SIZE_BITS];
    while input.get_next_bits(&mut block) {
        // TODO: Idea: Add a parameter to differenciate between input state and hash state
        // so I don't need to hardcode the 256 below
        // Copy input bits into the state
        for i in 0..16 {
            let bits: [u8; 32] = block[i * 32..(i + 1) * 32].try_into().unwrap(); // MSB
            for j in 0..32 {
                let group = (256 + (i * 32 + j) as u64) / SHA256F_GATE_CONFIG.sin_ref_group_by;
                let group_pos =
                    (256 + (i * 32 + j) as u64) as u64 % SHA256F_GATE_CONFIG.sin_ref_group_by;
                let ref_idx = SHA256F_GATE_CONFIG.sin_first_ref
                    + group * SHA256F_GATE_CONFIG.sin_ref_distance
                    + group_pos;
                gate_state.borrow_mut().gates[ref_idx as usize].pins[PinId::A].bit = bits[31 - j];
            }
        }

        sha256f(&gate_state);

        #[cfg(debug_assertions)]
        gate_state.borrow().print_circuit_topology();

        if get_circuit_topology {
            // The sha256f circuit topology is completely known after a single execution
            return Some(gate_state.into_inner());
        }

        gate_state.borrow_mut().copy_sout_to_sin_and_reset_refs();
    }

    gate_state.borrow().get_output(output);

    return None;
}

// Get the circuit topology of the Keccak-f permutation
pub fn sha256f_topology() -> GateState {
    // Hash any input and stop when a single sha256f has been computed
    let input = b"";
    let mut output = [0u8; 32];
    sha256(input, &mut output, true).expect("Failed to get circuit topology")
}

#[cfg(test)]
mod tests {
    use super::sha256;

    #[test]
    fn test_empty_string() {
        let input = b"";
        let mut output = [0u8; 32];
        sha256(input, &mut output, false);

        // Expected Keccak-256
        let expected_hash: [u8; 32] = [
            0x42, 0xC4, 0xB0, 0xE3, // 0xE3B0C442
            0x14, 0x1C, 0xFC, 0x98, // 0x98FC1C14
            0xC8, 0xF4, 0xFB, 0x9A, // 0x9AFBF4C8
            0x24, 0xB9, 0x6F, 0x99, // 0x996FB924
            0xE4, 0x41, 0xAE, 0x27, // 0x27AE41E4
            0x4C, 0x93, 0x9B, 0x64, // 0x649B934C
            0x1B, 0x99, 0x95, 0xA4, // 0xA495991B
            0x55, 0xB8, 0x52, 0x78, // 0x7852B855
        ];
        assert_eq!(output[..], expected_hash[..]);
    }

    #[test]
    fn test_one_block_message() {
        let input = b"abc";
        let mut output = [0u8; 32];
        sha256(input, &mut output, false);

        // Expected Keccak-256
        let expected_hash: [u8; 32] = [
            0xBF, 0x16, 0x78, 0xBA, // 0xBA7816BF
            0xEA, 0xCF, 0x01, 0x8F, // 0x8F01CFEA
            0xDE, 0x40, 0x41, 0x41, // 0x414140DE
            0x23, 0x22, 0xAE, 0x5D, // 0x5DAE2223
            0xA3, 0x61, 0x03, 0xB0, // 0xB00361A3
            0x9C, 0x7A, 0x17, 0x96, // 0x96177A9C
            0x61, 0xFF, 0x10, 0xB4, // 0xB410FF61
            0xAD, 0x15, 0x00, 0xF2, // 0xF20015AD
        ];
        assert_eq!(output[..], expected_hash[..]);
    }

    #[test]
    fn test_two_block_message() {
        let input = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
        let mut output = [0u8; 32];
        sha256(input, &mut output, false);

        // Expected Keccak-256 hash
        let expected_hash: [u8; 32] = [
            0x61, 0x6A, 0x8D, 0x24, // 0x248D6A61
            0xB8, 0x38, 0x06, 0xD2, // 0xD20638B8
            0x93, 0x26, 0xC0, 0xE5, // 0xE5C02693
            0x39, 0x60, 0x3E, 0x0C, // 0x0C3E6039
            0x59, 0xE4, 0x3C, 0xA3, // 0xA33CE459
            0x67, 0x21, 0xFF, 0x64, // 0x64FF2167
            0xD4, 0xED, 0xEC, 0xF6, // 0xF6ECEDD4
            0xC1, 0x06, 0xDB, 0x19, // 0x19DB06C1
        ];

        assert_eq!(output[..], expected_hash[..]);
    }

    #[test]
    fn test_sha256_long() {
        let input = b"The quick brown fox jumps over the lazy dog.The quick brown fox jumps over the lazy dog.The quick brown fox jumps over the lazy dog.The quick brown fox jumps over the lazy dog.The quick brown fox jumps over the lazy dog.The quick brown fox jumps over the lazy dog.The quick brown fox jumps over the lazy dog.The quick brown fox jumps over the lazy dog.The quick brown fox jumps over the lazy dog.The quick brown fox jumps over the lazy dog.The quick brown fox jumps over the lazy dog.The quick brown fox jumps over the lazy dog.The quick brown fox jumps over the lazy dog...";
        let mut output = [0u8; 32];
        sha256(input, &mut output, false);

        // Expected Keccak-256 hash
        let expected_hash: [u8; 32] = [
            0x46, 0xC7, 0x80, 0x8C, // 0x8C80C746
            0xA8, 0xB4, 0x52, 0x73, // 0x7352B4A8
            0xAF, 0x81, 0xC6, 0xF4, // 0xF4C681AF
            0x43, 0x83, 0x8B, 0x38, // 0x388B8343
            0x88, 0x17, 0x54, 0x79, // 0x79541788
            0xAC, 0x5B, 0xDE, 0x6E, // 0x6EDE5BAC
            0x40, 0xE8, 0x90, 0xB1, // 0xB190E840
            0xEA, 0x79, 0xBE, 0x35, // 0x35BE79EA
        ];

        assert_eq!(output[..], expected_hash[..]);
    }
}
