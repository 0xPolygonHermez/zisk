use std::cell::RefCell;

use circuit::{gate_u32_add, gate_u32_and, gate_u32_not, gate_u32_xor, GateState, GateU32, PinId};

// SHA256 round constants (first 32 bits of the fractional parts of the cube roots of the first 64 primes)
const RC: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

pub fn sha256f_internal(gate_state: &RefCell<GateState>) {
    #[cfg(debug_assertions)]
    gate_state.borrow().print_refs(&gate_state.borrow().sin_refs, "Before permutation");

    // Initialize the round constants as GateU32
    let mut k = [GateU32::new(gate_state); 64];
    for i in 0..64 {
        k[i].from_u32(RC[i]);
    }

    // Copy the hash state into the hash state array
    let mut h32 = vec![GateU32::new(gate_state); 8];
    for i in 0..8 {
        for j in 0..32 {
            let group = (i * 32 + j) as u64 / gate_state.borrow().gate_config.sin_ref_group_by;
            let group_pos = (i * 32 + j) as u64 % gate_state.borrow().gate_config.sin_ref_group_by;
            let ref_idx = gate_state.borrow().gate_config.sin_first_ref
                + group * gate_state.borrow().gate_config.sin_ref_distance
                + group_pos;
            h32[i].bits[j].ref_ = ref_idx;
            h32[i].bits[j].pin_id = PinId::A;
        }
    }

    // Initialize working variables with the current hash state
    let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h) =
        (h32[0], h32[1], h32[2], h32[3], h32[4], h32[5], h32[6], h32[7]);

    // Initialize the 64-entry message schedule array
    let mut w = [GateU32::new(gate_state); 64];

    // Copy the input bits 16 words into the message schedule array
    for i in 0..16 {
        for j in 0..32 {
            let group =
                (256 + (i * 32 + j) as u64) / gate_state.borrow().gate_config.sin_ref_group_by;
            let group_pos =
                (256 + (i * 32 + j) as u64) % gate_state.borrow().gate_config.sin_ref_group_by;
            let ref_idx = gate_state.borrow().gate_config.sin_first_ref
                + group * gate_state.borrow().gate_config.sin_ref_distance
                + group_pos;
            w[i].bits[j].ref_ = ref_idx;
            w[i].bits[j].pin_id = PinId::A;
        }
    }

    // Extend the message schedule array
    for i in 16..64 {
        // 1] Compute sigma0(w[i-15]) = ROTR(w[i-15], 7) ^ ROTR(w[i-15], 18) ^ SHR(w[i-15], 3)
        let mut tmp1 = w[i - 15];
        let mut tmp2 = w[i - 15];
        let mut tmp3 = w[i - 15];

        tmp1.rotate_right(7);
        tmp2.rotate_right(18);
        tmp3.shift_right(3);

        let mut tmp4 = GateU32::new(gate_state);
        gate_u32_xor(&mut gate_state.borrow_mut(), &tmp1, &tmp2, &mut tmp4);

        let mut sigma0 = GateU32::new(gate_state);
        gate_u32_xor(&mut gate_state.borrow_mut(), &tmp4, &tmp3, &mut sigma0);

        // 2] Compute sigma1(w[i-2]) = ROTR(w[i-2], 17) ^ ROTR(w[i-2], 19) ^ SHR(w[i-2], 10)
        tmp1 = w[i - 2];
        tmp2 = w[i - 2];
        tmp3 = w[i - 2];

        tmp1.rotate_right(17);
        tmp2.rotate_right(19);
        tmp3.shift_right(10);

        gate_u32_xor(&mut gate_state.borrow_mut(), &tmp1, &tmp2, &mut tmp4);

        let mut sigma1 = GateU32::new(gate_state);
        gate_u32_xor(&mut gate_state.borrow_mut(), &tmp4, &tmp3, &mut sigma1);

        // 3] Compute w[i] = w[i-16] + sigma0 + w[i-7] + sigma1
        gate_u32_add(&mut gate_state.borrow_mut(), &w[i - 16], &sigma0, &mut tmp1);
        gate_u32_add(&mut gate_state.borrow_mut(), &tmp1, &w[i - 7], &mut tmp2);
        gate_u32_add(&mut gate_state.borrow_mut(), &tmp2, &sigma1, &mut w[i]);
    }

    // Compression function main loop
    for i in 0..64 {
        // 1] Compute SIGMA1(e) = rotateRight32(e, 6) ^ rotateRight32(e, 11) ^ rotateRight32(e, 25)
        let mut tmp1 = e;
        let mut tmp2 = e;
        let mut tmp3 = e;

        tmp1.rotate_right(6);
        tmp2.rotate_right(11);
        tmp3.rotate_right(25);

        let mut xor = GateU32::new(gate_state);
        gate_u32_xor(&mut gate_state.borrow_mut(), &tmp1, &tmp2, &mut xor);

        let mut big_sigma1 = GateU32::new(gate_state);
        gate_u32_xor(&mut gate_state.borrow_mut(), &xor, &tmp3, &mut big_sigma1);

        // 2] Compute ch(e,f,g) = (e & f) ^ ((¬e) & g)
        let mut ch = GateU32::new(gate_state);
        gate_u32_and(&mut gate_state.borrow_mut(), &e, &f, &mut tmp1);
        gate_u32_not(&mut gate_state.borrow_mut(), &e, &mut tmp2);
        gate_u32_and(&mut gate_state.borrow_mut(), &tmp2, &g, &mut tmp3);
        gate_u32_xor(&mut gate_state.borrow_mut(), &tmp1, &tmp3, &mut ch);

        // 3] Compute T1 = h + SIGMA1(e) + ch(e,f,g) + k[i] + w[i]
        let mut t1 = GateU32::new(gate_state);
        gate_u32_add(&mut gate_state.borrow_mut(), &h, &big_sigma1, &mut tmp1);
        gate_u32_add(&mut gate_state.borrow_mut(), &tmp1, &ch, &mut tmp2);
        gate_u32_add(&mut gate_state.borrow_mut(), &tmp2, &k[i], &mut tmp3);
        gate_u32_add(&mut gate_state.borrow_mut(), &tmp3, &w[i], &mut t1);

        // 4] Compute SIGMA0(a) = rotateRight32(a, 2) ^ rotateRight32(a, 13) ^ rotateRight32(a, 22)
        tmp1 = a;
        tmp2 = a;
        tmp3 = a;

        tmp1.rotate_right(2);
        tmp2.rotate_right(13);
        tmp3.rotate_right(22);

        let mut xor = GateU32::new(gate_state);
        gate_u32_xor(&mut gate_state.borrow_mut(), &tmp1, &tmp2, &mut xor);

        let mut big_sigma0 = GateU32::new(gate_state);
        gate_u32_xor(&mut gate_state.borrow_mut(), &xor, &tmp3, &mut big_sigma0);

        // 5] Compute maj(a,b,c) = (a & b) ^ (a & c) ^ (b & c)
        let mut maj = GateU32::new(gate_state);
        gate_u32_and(&mut gate_state.borrow_mut(), &a, &b, &mut tmp1);
        gate_u32_and(&mut gate_state.borrow_mut(), &a, &c, &mut tmp2);
        gate_u32_and(&mut gate_state.borrow_mut(), &b, &c, &mut tmp3);
        gate_u32_xor(&mut gate_state.borrow_mut(), &tmp1, &tmp2, &mut xor);
        gate_u32_xor(&mut gate_state.borrow_mut(), &xor, &tmp3, &mut maj);

        // 6] Compute T2 = SIGMA0(a) + maj(a,b,c)
        let mut t2 = GateU32::new(gate_state);
        gate_u32_add(&mut gate_state.borrow_mut(), &big_sigma0, &maj, &mut t2);

        // 7] Update the working variables
        h = g;
        g = f;
        f = e;
        // e = d + T1
        gate_u32_add(&mut gate_state.borrow_mut(), &d, &t1, &mut e);
        d = c;
        c = b;
        b = a;
        // a = T1 + T2
        gate_u32_add(&mut gate_state.borrow_mut(), &t1, &t2, &mut a);
    }

    // Update hash values
    let mut state_output = vec![GateU32::new(gate_state); 8];
    gate_u32_add(&mut gate_state.borrow_mut(), &h32[0], &a, &mut state_output[0]);
    gate_u32_add(&mut gate_state.borrow_mut(), &h32[1], &b, &mut state_output[1]);
    gate_u32_add(&mut gate_state.borrow_mut(), &h32[2], &c, &mut state_output[2]);
    gate_u32_add(&mut gate_state.borrow_mut(), &h32[3], &d, &mut state_output[3]);
    gate_u32_add(&mut gate_state.borrow_mut(), &h32[4], &e, &mut state_output[4]);
    gate_u32_add(&mut gate_state.borrow_mut(), &h32[5], &f, &mut state_output[5]);
    gate_u32_add(&mut gate_state.borrow_mut(), &h32[6], &g, &mut state_output[6]);
    gate_u32_add(&mut gate_state.borrow_mut(), &h32[7], &h, &mut state_output[7]);

    // Add 256 more gates to make sure that the hash state output is located in the expected gates
    let zero_ref = gate_state.borrow().gate_config.zero_ref.unwrap();
    for i in 0..8 {
        for j in 0..32 {
            let group = (i * 32 + j) as u64 / gate_state.borrow().gate_config.sout_ref_group_by;
            let group_pos = (i * 32 + j) as u64 % gate_state.borrow().gate_config.sout_ref_group_by;
            let ref_idx = gate_state.borrow().gate_config.sout_first_ref
                + group * gate_state.borrow().gate_config.sout_ref_distance
                + group_pos;
            gate_state.borrow_mut().xor(
                state_output[i].bits[j].ref_,
                state_output[i].bits[j].pin_id,
                zero_ref,
                PinId::A,
                ref_idx,
            );
            gate_state.borrow_mut().sout_refs[i * 32 + j] = ref_idx;
        }
    }

    #[cfg(debug_assertions)]
    gate_state.borrow().print_refs(&gate_state.borrow().sout_refs, "After permutation");
}
