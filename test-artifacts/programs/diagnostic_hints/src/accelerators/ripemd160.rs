use zkvm_interface::{zkvm_ripemd160, zkvm_ripemd160_hash, zkvm_status_ZKVM_EOK as ZKVM_EOK};

pub fn diagnostic_zkvm_ripemd160() {
    // RIPEMD-160("") = 9c1185a5 c5e9fc54 61280897 7ee8f548 b2258d31, left-padded with 12 zero bytes.
    let data: [u8; 0] = [];
    let mut output = zkvm_ripemd160_hash { data: [0u8; 32] };
    let status = unsafe { zkvm_ripemd160(data.as_ptr(), data.len(), &mut output) };
    assert_eq!(status, ZKVM_EOK);
    let mut expected = [0u8; 32];
    expected[12..].copy_from_slice(&[
        0x9c, 0x11, 0x85, 0xa5, 0xc5, 0xe9, 0xfc, 0x54, 0x61, 0x28, 0x08, 0x97, 0x7e, 0xe8, 0xf5,
        0x48, 0xb2, 0x25, 0x8d, 0x31,
    ]);
    assert_eq!(output.data, expected);
}
