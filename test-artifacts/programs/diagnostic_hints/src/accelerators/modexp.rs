use zisk_zkvm_interface::{zkvm_modexp, zkvm_status_ZKVM_EOK as ZKVM_EOK};

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

    // An empty modulus has an empty output (EIP-198), so `output` need not be valid
    // and must not be written. Unlike the BLS12 precompiles, which reject a trivial
    // input with EFAIL, modexp accepts it and writes nothing -- the 0xff fill is a
    // sentinel that must survive the call.
    let mut untouched: [u8; 4] = [0xff; 4];
    let status = unsafe {
        zkvm_modexp(
            base.as_ptr(),
            base.len(),
            exp.as_ptr(),
            exp.len(),
            modulus.as_ptr(),
            0,
            untouched.as_mut_ptr(),
        )
    };
    assert_eq!(status, ZKVM_EOK);
    assert_eq!(untouched, [0xffu8; 4]);
}
