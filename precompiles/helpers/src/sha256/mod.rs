use std::cell::RefCell;

use circuit::{
    gate_u32_add, gate_u32_and, gate_u32_not, gate_u32_xor, u32_to_bits, GateConfig, GateState,
    GateU32, PinId,
};

mod sha256_constants;
mod sha256_input;

pub use sha256_constants::{SHA256_BLOCK_SIZE_BITS, SHA256_BLOCK_SIZE_BYTES};
use sha256_input::Sha256Input;

// Initial hash values (first 32 bits of fractional parts of square roots of first 8 primes)
pub const INITIAL_HASH_STATE: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

// SHA256 round constants (first 32 bits of the fractional parts of the cube roots of the first 64 primes)
pub const RC: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

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

    // output.copy_from_slice(&hash_state);
    gate_state.borrow().get_output(output);

    return None;
}

pub fn sha256f(gate_state: &RefCell<GateState>) {
    // Initialize the round constants as GateU32
    let mut k = [GateU32::new(&gate_state); 64];
    for i in 0..64 {
        k[i].from_u32(RC[i]);
    }

    // Copy the hash state into the hash state array
    let mut h32 = vec![GateU32::new(&gate_state); 8];
    for i in 0..8 {
        for j in 0..32 {
            let group = (i * 32 + j) as u64 / SHA256F_GATE_CONFIG.sin_ref_group_by;
            let group_pos = (i * 32 + j) as u64 % SHA256F_GATE_CONFIG.sin_ref_group_by;
            let ref_idx = SHA256F_GATE_CONFIG.sin_first_ref
                + group * SHA256F_GATE_CONFIG.sin_ref_distance
                + group_pos;
            h32[i].bits[j].ref_ = ref_idx;
            h32[i].bits[j].pin_id = PinId::A;
        }
    }

    // Initialize working variables with the current hash state
    let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h) = (
        h32[0].clone(),
        h32[1].clone(),
        h32[2].clone(),
        h32[3].clone(),
        h32[4].clone(),
        h32[5].clone(),
        h32[6].clone(),
        h32[7].clone(),
    );

    // Initialize the 64-entry message schedule array
    let mut w = [GateU32::new(&gate_state); 64];

    // Copy the input bits 16 words into the message schedule array
    for i in 0..16 {
        for j in 0..32 {
            let group = (256 + (i * 32 + j) as u64) / SHA256F_GATE_CONFIG.sin_ref_group_by;
            let group_pos =
                (256 + (i * 32 + j) as u64) as u64 % SHA256F_GATE_CONFIG.sin_ref_group_by;
            let ref_idx = SHA256F_GATE_CONFIG.sin_first_ref
                + group * SHA256F_GATE_CONFIG.sin_ref_distance
                + group_pos;
            w[i].bits[j].ref_ = ref_idx;
            w[i].bits[j].pin_id = PinId::A;
        }
    }

    // Extend the message schedule array
    for i in 16..64 {
        // 1] Compute sigma0(w[i-15]) = ROTR(w[i-15], 7) ^ ROTR(w[i-15], 18) ^ SHR(w[i-15], 3)
        let mut tmp1 = w[i - 15].clone();
        let mut tmp2 = w[i - 15].clone();
        let mut tmp3 = w[i - 15].clone();

        tmp1.rotate_right(7);
        tmp2.rotate_right(18);
        tmp3.shift_right(3);

        let mut tmp4 = GateU32::new(&gate_state);
        gate_u32_xor(&mut gate_state.borrow_mut(), &tmp1, &tmp2, &mut tmp4);

        let mut sigma0 = GateU32::new(&gate_state);
        gate_u32_xor(&mut gate_state.borrow_mut(), &tmp4, &tmp3, &mut sigma0);

        // 2] Compute sigma1(w[i-2]) = ROTR(w[i-2], 17) ^ ROTR(w[i-2], 19) ^ SHR(w[i-2], 10)
        tmp1 = w[i - 2].clone();
        tmp2 = w[i - 2].clone();
        tmp3 = w[i - 2].clone();

        tmp1.rotate_right(17);
        tmp2.rotate_right(19);
        tmp3.shift_right(10);

        gate_u32_xor(&mut gate_state.borrow_mut(), &tmp1, &tmp2, &mut tmp4);

        let mut sigma1 = GateU32::new(&gate_state);
        gate_u32_xor(&mut gate_state.borrow_mut(), &tmp4, &tmp3, &mut sigma1);

        // 3] Compute w[i] = w[i-16] + sigma0 + w[i-7] + sigma1
        gate_u32_add(&mut gate_state.borrow_mut(), &w[i - 16], &sigma0, &mut tmp1);
        gate_u32_add(&mut gate_state.borrow_mut(), &tmp1, &w[i - 7], &mut tmp2);
        gate_u32_add(&mut gate_state.borrow_mut(), &tmp2, &sigma1, &mut w[i]);
    }

    // Compression function main loop
    for i in 0..64 {
        // 1] Compute SIGMA1(e) = rotateRight32(e, 6) ^ rotateRight32(e, 11) ^ rotateRight32(e, 25)
        let mut tmp1 = e.clone();
        let mut tmp2 = e.clone();
        let mut tmp3 = e.clone();

        tmp1.rotate_right(6);
        tmp2.rotate_right(11);
        tmp3.rotate_right(25);

        let mut xor = GateU32::new(&gate_state);
        gate_u32_xor(&mut gate_state.borrow_mut(), &tmp1, &tmp2, &mut xor);

        let mut big_sigma1 = GateU32::new(&gate_state);
        gate_u32_xor(&mut gate_state.borrow_mut(), &xor, &tmp3, &mut big_sigma1);

        // 2] Compute ch(e,f,g) = (e & f) ^ ((¬e) & g)
        let mut ch = GateU32::new(&gate_state);
        gate_u32_and(&mut gate_state.borrow_mut(), &e, &f, &mut tmp1);
        gate_u32_not(&mut gate_state.borrow_mut(), &e, &mut tmp2);
        gate_u32_and(&mut gate_state.borrow_mut(), &tmp2, &g, &mut tmp3);
        gate_u32_xor(&mut gate_state.borrow_mut(), &tmp1, &tmp3, &mut ch);

        // 3] Compute T1 = h + SIGMA1(e) + ch(e,f,g) + k[i] + w[i]
        let mut t1 = GateU32::new(&gate_state);
        gate_u32_add(&mut gate_state.borrow_mut(), &h, &big_sigma1, &mut tmp1);
        gate_u32_add(&mut gate_state.borrow_mut(), &tmp1, &ch, &mut tmp2);
        gate_u32_add(&mut gate_state.borrow_mut(), &tmp2, &k[i], &mut tmp3);
        gate_u32_add(&mut gate_state.borrow_mut(), &tmp3, &w[i], &mut t1);

        // 4] Compute SIGMA0(a) = rotateRight32(a, 2) ^ rotateRight32(a, 13) ^ rotateRight32(a, 22)
        tmp1 = a.clone();
        tmp2 = a.clone();
        tmp3 = a.clone();

        tmp1.rotate_right(2);
        tmp2.rotate_right(13);
        tmp3.rotate_right(22);

        let mut xor = GateU32::new(&gate_state);
        gate_u32_xor(&mut gate_state.borrow_mut(), &tmp1, &tmp2, &mut xor);

        let mut big_sigma0 = GateU32::new(&gate_state);
        gate_u32_xor(&mut gate_state.borrow_mut(), &xor, &tmp3, &mut big_sigma0);

        // 5] Compute maj(a,b,c) = (a & b) ^ (a & c) ^ (b & c)
        let mut maj = GateU32::new(&gate_state);
        gate_u32_and(&mut gate_state.borrow_mut(), &a, &b, &mut tmp1);
        gate_u32_and(&mut gate_state.borrow_mut(), &a, &c, &mut tmp2);
        gate_u32_and(&mut gate_state.borrow_mut(), &b, &c, &mut tmp3);
        gate_u32_xor(&mut gate_state.borrow_mut(), &tmp1, &tmp2, &mut xor);
        gate_u32_xor(&mut gate_state.borrow_mut(), &xor, &tmp3, &mut maj);

        // 6] Compute T2 = SIGMA0(a) + maj(a,b,c)
        let mut t2 = GateU32::new(&gate_state);
        gate_u32_add(&mut gate_state.borrow_mut(), &big_sigma0, &maj, &mut t2);

        // 7] Update the working variables
        h = g.clone();
        g = f.clone();
        f = e.clone();
        // e = d + T1
        gate_u32_add(&mut gate_state.borrow_mut(), &d, &t1, &mut e);
        d = c.clone();
        c = b.clone();
        b = a.clone();
        // a = T1 + T2
        gate_u32_add(&mut gate_state.borrow_mut(), &t1, &t2, &mut a);
    }

    // Update hash values
    let mut hash_output = vec![GateU32::new(&gate_state); 8];
    gate_u32_add(&mut gate_state.borrow_mut(), &h32[0], &a, &mut hash_output[0]);
    gate_u32_add(&mut gate_state.borrow_mut(), &h32[1], &b, &mut hash_output[1]);
    gate_u32_add(&mut gate_state.borrow_mut(), &h32[2], &c, &mut hash_output[2]);
    gate_u32_add(&mut gate_state.borrow_mut(), &h32[3], &d, &mut hash_output[3]);
    gate_u32_add(&mut gate_state.borrow_mut(), &h32[4], &e, &mut hash_output[4]);
    gate_u32_add(&mut gate_state.borrow_mut(), &h32[5], &f, &mut hash_output[5]);
    gate_u32_add(&mut gate_state.borrow_mut(), &h32[6], &g, &mut hash_output[6]);
    gate_u32_add(&mut gate_state.borrow_mut(), &h32[7], &h, &mut hash_output[7]);

    // Add 256 more gates to make sure that the hash state output is located in the expected gates
    for i in 0..8 {
        for j in 0..32 {
            let group = (i * 32 + j) as u64 / SHA256F_GATE_CONFIG.sout_ref_group_by;
            let group_pos = (i * 32 + j) as u64 % SHA256F_GATE_CONFIG.sout_ref_group_by;
            let ref_idx = SHA256F_GATE_CONFIG.sout_first_ref
                + group * SHA256F_GATE_CONFIG.sout_ref_distance
                + group_pos;
            gate_state.borrow_mut().xor(
                hash_output[i].bits[j as usize].ref_,
                hash_output[i].bits[j as usize].pin_id,
                SHA256F_GATE_CONFIG.zero_ref.unwrap(),
                PinId::A,
                ref_idx,
            );
            gate_state.borrow_mut().sout_refs[(i * 32 + j) as usize] = ref_idx;
        }
    }
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
