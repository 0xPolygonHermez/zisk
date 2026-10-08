use ziskos::syscalls::*;

pub fn diagnostic_blake2s() {
    //////////////
    // Blake2s Tests
    //////////////

    let mut state: [u64; 8] = [0; 8];
    let input: [u64; 8] = [0; 8];
    let mut params = SyscallBlake2sfParams { state: &mut state, input: &input };

    // Test #0: blake2s, the final-block compression of "abc" (RFC 7693, Appendix B) before
    // the feed-forward
    let mut state: [u64; 8] = [
        0xbb67ae856b08e647,
        0xa54ff53a3c6ef372,
        0x9b05688c510e527f,
        0x5be0cd191f83d9ab,
        0xbb67ae856a09e667,
        0xa54ff53a3c6ef372,
        0x9b05688c510e527c,
        0x5be0cd19e07c2654,
    ];
    let input: [u64; 8] = [0x636261, 0, 0, 0, 0, 0, 0, 0];
    params.state = &mut state;
    params.input = &input;
    syscall_blake2sf(&mut params);
    let expected_out: [u64; 8] = [
        0xcfec3aa6d9c994aa,
        0x2c38670e700d0ab2,
        0x1d023ef3af6a1f66,
        0x945357a51d9ec27d,
        0x969fe8113e9ffebd,
        0xa632797aef485e21,
        0xaf3d80e1deef082e,
        0x4deafd3a4e86829b,
    ];
    assert_eq!(params.state, &expected_out);
}
