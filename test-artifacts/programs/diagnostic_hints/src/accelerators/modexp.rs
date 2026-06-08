use zkvm_interface::{zkvm_modexp, zkvm_status_ZKVM_EOK as ZKVM_EOK};

pub fn diagnostic_zkvm_modexp() {
    // 2^3 mod 5 = 3
    let base: [u8; 1] = [0x02];
    let exp: [u8; 1] = [0x03];
    let modulus: [u8; 1] = [0x05];
    let mut output: [u8; 1] = [0x00];
    let status = unsafe {
        zkvm_modexp(
            base.as_ptr(),
            base.len(),
            exp.as_ptr(),
            exp.len(),
            modulus.as_ptr(),
            modulus.len(),
            output.as_mut_ptr(),
        )
    };
    assert_eq!(status, ZKVM_EOK);
    assert_eq!(output, [0x03]);
}
