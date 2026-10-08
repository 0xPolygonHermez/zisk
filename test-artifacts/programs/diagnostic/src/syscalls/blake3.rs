use ziskos::syscalls::*;

pub fn diagnostic_blake3() {
    //////////////
    // Blake3 Tests
    //////////////

    let mut state: [u64; 8] = [0; 8];
    let input: [u64; 8] = [0; 8];
    let mut params = SyscallBlake3fParams { state: &mut state, input: &input };

    // Test #0: blake3
    let mut state: [u64; 8] = [
        0xbb67ae856a09e667,
        0xa54ff53a3c6ef372,
        0x9b05688c510e527f,
        0x5be0cd191f83d9ab,
        0xbb67ae856a09e667,
        0xa54ff53a3c6ef372,
        0x0000000000000000,
        0x0000000b00000003,
    ];
    let input: [u64; 8] = [0x636261, 0, 0, 0, 0, 0, 0, 0];
    params.state = &mut state;
    params.input = &input;
    syscall_blake3f(&mut params);
    let expected_out: [u64; 8] = [
        0x58c37bce68ea631c,
        0x59cfd54f14e356a5,
        0xd4a1df268bf60c1a,
        0xd5c204ff811f35a9,
        0x6b923df6c4595478,
        0xec42ef6861d8e05a,
        0xd77aa67bcdaec952,
        0x505fb92aed830054,
    ];
    assert_eq!(params.state, &expected_out);
}
