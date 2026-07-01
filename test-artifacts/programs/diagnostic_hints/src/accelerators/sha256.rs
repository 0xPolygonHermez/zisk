use zkvm_interface::{zkvm_sha256, zkvm_sha256_hash, zkvm_status_ZKVM_EOK as ZKVM_EOK};

pub fn diagnostic_zkvm_sha256() {
    // SHA-256("") = e3b0c442 98fc1c14 9afbf4c8 996fb924 27ae41e4 649b934c a495991b 7852b855
    let data: [u8; 0] = [];
    let mut output = zkvm_sha256_hash { data: [0u8; 32] };
    let status = unsafe { zkvm_sha256(data.as_ptr(), data.len(), &mut output) };
    assert_eq!(status, ZKVM_EOK);
    let expected: [u8; 32] = [
        0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9,
        0x24, 0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52,
        0xb8, 0x55,
    ];
    assert_eq!(output.data, expected);
}
