#![allow(clippy::needless_range_loop)]

use std::cell::RefCell;

use circuit::{byte_to_bits_msb, bytes_to_u32_msb, u32_to_bits_msb, GateConfig, GateState, PinId};

mod sha256f;
use sha256f::sha256f_internal;

// Sha256 Configuration
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

fn sha256f(
    state: &mut [u32; 8],
    input: &[u8; 64],
    get_circuit_topology: bool,
) -> Option<GateState> {
    // Initialize the gate state
    let gate_state = RefCell::new(GateState::new(SHA256F_GATE_CONFIG.clone()));

    // Copy the hash state bits into the state
    for i in 0..8 {
        let bits = u32_to_bits_msb(state[i]);
        for j in 0..32 {
            let group = (i * 32 + j) as u64 / SHA256F_GATE_CONFIG.sin_ref_group_by;
            let group_pos = (i * 32 + j) as u64 % SHA256F_GATE_CONFIG.sin_ref_group_by;
            let ref_idx = SHA256F_GATE_CONFIG.sin_first_ref
                + group * SHA256F_GATE_CONFIG.sin_ref_distance
                + group_pos;
            gate_state.borrow_mut().gates[ref_idx as usize].pins[PinId::A].bit = bits[j];
        }
    }

    // Copy the input bits into the state
    let state_offset = 256;
    for i in 0..64 {
        let bits = byte_to_bits_msb(input[i]);
        for j in 0..8 {
            let group = (state_offset + i * 8 + j) as u64 / SHA256F_GATE_CONFIG.sin_ref_group_by;
            let group_pos =
                (state_offset + i * 8 + j) as u64 % SHA256F_GATE_CONFIG.sin_ref_group_by;
            let ref_idx = SHA256F_GATE_CONFIG.sin_first_ref
                + group * SHA256F_GATE_CONFIG.sin_ref_distance
                + group_pos;
            gate_state.borrow_mut().gates[ref_idx as usize].pins[PinId::A].bit = bits[j];
        }
    }

    // Execute the sha256f function
    sha256f_internal(&gate_state);

    if get_circuit_topology {
        // The sha256f circuit topology is completely known after a single execution
        return Some(gate_state.into_inner());
    } else {
        #[cfg(debug_assertions)]
        gate_state.borrow().print_circuit_topology();
    }

    gate_state.borrow_mut().copy_sout_to_sin_and_reset_refs();

    let mut output = [0u8; 32];
    gate_state.borrow().get_output(&mut output, true);

    // Convert bytes to u32
    for i in 0..8 {
        state[i] = bytes_to_u32_msb(&output[i * 4..(i + 1) * 4].try_into().unwrap());
    }

    None
}

// Get the circuit topology of the Sha256-f permutation
pub fn sha256f_topology() -> GateState {
    // Hashf any input and get the circuit topology
    let mut state = [0u32; 8];
    let input = [0u8; 64];
    sha256f(&mut state, &input, true).expect("Failed to get circuit topology")
}

#[cfg(test)]
mod tests {
    use super::sha256f;

    // Initial hash values (first 32 bits of fractional parts of square roots of first 8 primes)
    const SHA256_INITIAL_HASH_STATE: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    #[test]
    fn test_empty_string_f() {
        let mut state = SHA256_INITIAL_HASH_STATE;

        let mut input = [0u8; 64];
        input[0] = 0x80; // 1 << 7
        sha256f(&mut state, &input, false);

        // Expected Sha256f
        let expected_hash: [u32; 8] = [
            0xE3B0C442, 0x98FC1C14, 0x9AFBF4C8, 0x996FB924, 0x27AE41E4, 0x649B934C, 0xA495991B,
            0x7852B855,
        ];
        assert_eq!(state[..], expected_hash[..]);
    }

    #[test]
    fn test_one_block_message_f() {
        let mut state = SHA256_INITIAL_HASH_STATE;

        let mut input = [0u8; 64];
        input[0] = 0x61; // 'a'
        input[1] = 0x62; // 'b'
        input[2] = 0x63; // 'c'
        input[3] = 0x80; // 1 << 7
        input[63] = 0x18; // Length of the message in bits (3 bytes + 1 bit padding)
        sha256f(&mut state, &input, false);

        // Expected Sha256f
        let expected_hash: [u32; 8] = [
            0xBA7816BF, 0x8F01CFEA, 0x414140DE, 0x5DAE2223, 0xB00361A3, 0x96177A9C, 0xB410FF61,
            0xF20015AD,
        ];
        assert_eq!(state[..], expected_hash[..]);
    }
}
