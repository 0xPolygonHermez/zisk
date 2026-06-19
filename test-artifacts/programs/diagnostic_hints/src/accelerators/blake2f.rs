use zkvm_interface::{
    zkvm_blake2f, zkvm_blake2f_message, zkvm_blake2f_offset, zkvm_blake2f_state,
    zkvm_status_ZKVM_EOK as ZKVM_EOK,
};

pub fn diagnostic_zkvm_blake2f() {
    // With rounds=0, t=0, f=0, the compression collapses to h := h ⊕ h ⊕ IV = IV.
    let mut h = zkvm_blake2f_state { data: [0u8; 64] };
    let m = zkvm_blake2f_message { data: [0u8; 128] };
    let t = zkvm_blake2f_offset { data: [0u8; 16] };
    let status = unsafe { zkvm_blake2f(0, &mut h, &m, &t, 0) };
    assert_eq!(status, ZKVM_EOK);
    // BLAKE2b IV (RFC 7693), serialized as 8 little-endian u64s.
    const IV: [u64; 8] = [
        0x6a09e667f3bcc908,
        0xbb67ae8584caa73b,
        0x3c6ef372fe94f82b,
        0xa54ff53a5f1d36f1,
        0x510e527fade682d1,
        0x9b05688c2b3e6c1f,
        0x1f83d9abfb41bd6b,
        0x5be0cd19137e2179,
    ];
    let mut expected = [0u8; 64];
    for (i, iv) in IV.iter().enumerate() {
        expected[i * 8..(i + 1) * 8].copy_from_slice(&iv.to_le_bytes());
    }
    assert_eq!(h.data, expected);
}
