use crate::syscalls::{syscall_blake3f, SyscallBlake3fParams};

/// BLAKE3 initialization vectors
const IV: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

pub fn blake3_compress(
    h: &mut [u32; 8],
    m: &[u32; 16],
    t: &[u32; 2],
    len: u32,
    flags: u32,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    // Initilize local state
    let mut state = [
        h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7], IV[0], IV[1], IV[2], IV[3], t[0], t[1],
        len, flags,
    ];

    // Perform the cryptographic mixing
    let state_u64: &mut [u64; 8] = unsafe { &mut *(state.as_mut_ptr() as *mut [u64; 8]) };
    let input: &[u64; 8] = unsafe { &*(m.as_ptr() as *const [u64; 8]) };

    let mut params = SyscallBlake3fParams { state: state_u64, input };
    syscall_blake3f(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    // Compute the output state
    h[0] = state[0] ^ state[8];
    h[1] = state[1] ^ state[9];
    h[2] = state[2] ^ state[10];
    h[3] = state[3] ^ state[11];
    h[4] = state[4] ^ state[12];
    h[5] = state[5] ^ state[13];
    h[6] = state[6] ^ state[14];
    h[7] = state[7] ^ state[15];
}

#[cfg(test)]
mod tests {
    // Test vectors from https://www.ietf.org/archive/id/draft-aumasson-blake3-00.html#appendix-B

    use super::*;

    #[test]
    fn test_blake3_vector1() {
        // Input:
        //     h = 6a09e667 bb67ae85 3c6ef372 a54ff53a 510e527f 9b05688c 1f83d9ab 5be0cd19
        //     m = 46544549 00000000 00000000 00000000 00000000 00000000 00000000 00000000
        //         00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000
        //     t = 00000000 00000000
        //   len = 4
        // flags = 0b
        //
        // Expected output:
        // 1edea283 abe6f4e6 24896868 cfc04e8f 9470c54c ff82a646 d6b4cbd1 e2815116

        let mut h: [u32; 8] = [
            0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
            0x5be0cd19,
        ];
        let m: [u32; 16] = [
            0x46544549, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
            0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
            0x00000000, 0x00000000,
        ];
        let t: [u32; 2] = [0x00000000, 0x00000000];
        let len: u32 = 4;
        let flags: u32 = 0x0b;

        blake3_compress(
            &mut h,
            &m,
            &t,
            len,
            flags,
            #[cfg(feature = "hints")]
            &mut Vec::new(),
        );

        // Expected output (8 × u64, little-endian)
        let expected = [
            0x1edea283, 0xabe6f4e6, 0x24896868, 0xcfc04e8f, 0x9470c54c, 0xff82a646, 0xd6b4cbd1,
            0xe2815116,
        ];

        assert_eq!(
            h, expected,
            "blake3 does not match:\n   exp: {:02x?},\n   got: {:02x?}",
            expected, h
        );
    }

    #[test]
    fn test_blake3_vector2() {
        // Input:
        //     h = 6a09e667 bb67ae85 3c6ef372 a54ff53a 510e527f 9b05688c 1f83d9ab 5be0cd19
        //     m = aaaaaaaa aaaaaaaa aaaaaaaa aaaaaaaa aaaaaaaa aaaaaaaa aaaaaaaa aaaaaaaa
        //         aaaaaaaa aaaaaaaa aaaaaaaa aaaaaaaa aaaaaaaa aaaaaaaa aaaaaaaa aaaaaaaa
        //     t = 00000000 00000000
        //   len = 64
        // flags = 01
        //
        // Expected output:
        // db668896 8e557d4d 684294f4 ae36d8ae eaec1efd 5f5fc3ec d8d1abc5 10094488

        let mut h: [u32; 8] = [
            0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
            0x5be0cd19,
        ];
        let m: [u32; 16] = [
            0xaaaaaaaa, 0xaaaaaaaa, 0xaaaaaaaa, 0xaaaaaaaa, 0xaaaaaaaa, 0xaaaaaaaa, 0xaaaaaaaa,
            0xaaaaaaaa, 0xaaaaaaaa, 0xaaaaaaaa, 0xaaaaaaaa, 0xaaaaaaaa, 0xaaaaaaaa, 0xaaaaaaaa,
            0xaaaaaaaa, 0xaaaaaaaa,
        ];
        let t: [u32; 2] = [0x00000000, 0x00000000];
        let len: u32 = 64;
        let flags: u32 = 0x01;

        blake3_compress(
            &mut h,
            &m,
            &t,
            len,
            flags,
            #[cfg(feature = "hints")]
            &mut Vec::new(),
        );

        // Expected output (8 × u64, little-endian)
        let expected = [
            0xdb668896, 0x8e557d4d, 0x684294f4, 0xae36d8ae, 0xeaec1efd, 0x5f5fc3ec, 0xd8d1abc5,
            0x10094488,
        ];

        assert_eq!(
            h, expected,
            "blake3 does not match:\n   exp: {:02x?},\n   got: {:02x?}",
            expected, h
        );

        // Input:
        //     h = db668896 8e557d4d 684294f4 ae36d8ae eaec1efd 5f5fc3ec d8d1abc5 10094488
        //     m = aaaaaaaa aaaaaaaa aaaaaaaa aaaaaaaa aaaaaaaa aaaaaaaa aaaaaaaa aaaaaaaa
        //         aaaaaaaa aaaaaaaa aaaaaaaa aaaaaaaa aaaaaaaa aaaaaaaa aaaaaaaa aaaaaaaa
        //     t = 00000000 00000000
        //   len = 64
        // flags = 00
        //
        // Expected output:
        // 68f7c3a8 8aaed76b f0decee2 d1b5993d 9564cba3 85b6c1ee baffea5b 0be671fb

        let mut h: [u32; 8] = [
            0xdb668896, 0x8e557d4d, 0x684294f4, 0xae36d8ae, 0xeaec1efd, 0x5f5fc3ec, 0xd8d1abc5,
            0x10094488,
        ];
        let m: [u32; 16] = [
            0xaaaaaaaa, 0xaaaaaaaa, 0xaaaaaaaa, 0xaaaaaaaa, 0xaaaaaaaa, 0xaaaaaaaa, 0xaaaaaaaa,
            0xaaaaaaaa, 0xaaaaaaaa, 0xaaaaaaaa, 0xaaaaaaaa, 0xaaaaaaaa, 0xaaaaaaaa, 0xaaaaaaaa,
            0xaaaaaaaa, 0xaaaaaaaa,
        ];
        let t: [u32; 2] = [0x00000000, 0x00000000];
        let len: u32 = 64;
        let flags: u32 = 0x00;

        blake3_compress(
            &mut h,
            &m,
            &t,
            len,
            flags,
            #[cfg(feature = "hints")]
            &mut Vec::new(),
        );

        // Expected output (8 × u64, little-endian)
        let expected = [
            0x68f7c3a8, 0x8aaed76b, 0xf0decee2, 0xd1b5993d, 0x9564cba3, 0x85b6c1ee, 0xbaffea5b,
            0x0be671fb,
        ];

        assert_eq!(
            h, expected,
            "blake3 does not match:\n   exp: {:02x?},\n   got: {:02x?}",
            expected, h
        );
    }
}
