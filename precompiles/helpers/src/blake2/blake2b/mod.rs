mod round;

pub use round::blake2b_round;

/// BLAKE2b initialization vectors
const IV: [u64; 8] = [
    0x6A09E667F3BCC908,
    0xBB67AE8584CAA73B,
    0x3C6EF372FE94F82B,
    0xA54FF53A5F1D36F1,
    0x510E527FADE682D1,
    0x9B05688C2B3E6C1F,
    0x1F83D9ABFB41BD6B,
    0x5BE0CD19137E2179,
];

/// BLAKE2b compression function
///
/// # Arguments
/// * `rounds` - Number of rounds (typically 12 for BLAKE2b)
/// * `state` - The internal state h (8 x 64-bit words as bits)
/// * `message` - The message block m (16 x 64-bit words as bits)
/// * `t` - Offset counters (2 x 64-bit words)
/// * `f` - Final block flag
pub fn blake2b_compress(rounds: u32, h: &mut [u64; 8], m: &[u64; 16], t: &[u64; 2], f: bool) {
    let mut v = [0u64; 16];

    v[..8].copy_from_slice(h);
    v[8..16].copy_from_slice(&IV);

    v[12] ^= t[0];
    v[13] ^= t[1];

    if f {
        v[14] = !v[14];
    }

    for r in 0..rounds {
        blake2b_round(&mut v, m, r);
    }

    for i in 0..8 {
        h[i] ^= v[i] ^ v[i + 8];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blake2b_eip152_vector1() {
        // Test vector from EIP-152
        // Input:
        // rounds = 0
        // h = 48c9bdf267e6096a 3ba7ca8485ae67bb 2bf894fe72f36e3c f1361d5f3af54fa5
        //     d182e6ad7f520e51 1f6c3e2b8c68059b 6bbd41fbabd9831f 79217e1319cde05b
        // m = 6162630000000000 0000000000000000 0000000000000000 0000000000000000
        //     0000000000000000 0000000000000000 0000000000000000 0000000000000000
        //     0000000000000000 0000000000000000 0000000000000000 0000000000000000
        //     0000000000000000 0000000000000000 0000000000000000 0000000000000000
        // t = 03 00 00 00 00 00 00 00, 00 00 00 00 00 00 00 00
        // f = true
        //
        // Expected output:
        // 08c9bcf367e6096a 3ba7ca8485ae67bb 2bf894fe72f36e3c f1361d5f3af54fa5
        // d282e6ad7f520e51 1f6c3e2b8c68059b 9442be0454267ce0 79217e1319cde05b

        let rounds = 0u32;

        let mut h: [u64; 8] = [
            0x48c9bdf267e6096au64.swap_bytes(),
            0x3ba7ca8485ae67bbu64.swap_bytes(),
            0x2bf894fe72f36e3cu64.swap_bytes(),
            0xf1361d5f3af54fa5u64.swap_bytes(),
            0xd182e6ad7f520e51u64.swap_bytes(),
            0x1f6c3e2b8c68059bu64.swap_bytes(),
            0x6bbd41fbabd9831fu64.swap_bytes(),
            0x79217e1319cde05bu64.swap_bytes(),
        ];

        let m: [u64; 16] = [
            0x6162630000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
        ];

        let t: [u64; 2] = [0x0300000000000000u64.swap_bytes(), 0x0000000000000000u64.swap_bytes()];

        let f = true;

        blake2b_compress(rounds, &mut h, &m, &t, f);

        // Expected output (8 × u64, little-endian)
        let expected = [
            0x08c9bcf367e6096au64.swap_bytes(),
            0x3ba7ca8485ae67bbu64.swap_bytes(),
            0x2bf894fe72f36e3cu64.swap_bytes(),
            0xf1361d5f3af54fa5u64.swap_bytes(),
            0xd282e6ad7f520e51u64.swap_bytes(),
            0x1f6c3e2b8c68059bu64.swap_bytes(),
            0x9442be0454267ce0u64.swap_bytes(),
            0x79217e1319cde05bu64.swap_bytes(),
        ];

        assert_eq!(
            h, expected,
            "Blake2b does not match:\n   exp: {:02x?},\n   got: {:02x?}",
            expected, h
        );
    }

    #[test]
    fn test_blake2b_eip152_vector2() {
        // Test vector from EIP-152
        // Input:
        // rounds = 12
        // h = 48c9bdf267e6096a 3ba7ca8485ae67bb 2bf894fe72f36e3c f1361d5f3af54fa5
        //     d182e6ad7f520e51 1f6c3e2b8c68059b 6bbd41fbabd9831f 79217e1319cde05b
        // m = 6162630000000000 0000000000000000 0000000000000000 0000000000000000
        //     0000000000000000 0000000000000000 0000000000000000 0000000000000000
        //     0000000000000000 0000000000000000 0000000000000000 0000000000000000
        //     0000000000000000 0000000000000000 0000000000000000 0000000000000000
        // t = 03 00 00 00 00 00 00 00, 00 00 00 00 00 00 00 00
        // f = true
        //
        // Expected output:
        // ba80a53f981c4d0d 6a2797b69f12f6e9 4c212f14685ac4b7 4b12bb6fdbffa2d1
        // 7d87c5392aab792d c252d5de4533cc95 18d38aa8dbf1925a b92386edd4009923

        let rounds = 12;

        let mut h: [u64; 8] = [
            0x48c9bdf267e6096au64.swap_bytes(),
            0x3ba7ca8485ae67bbu64.swap_bytes(),
            0x2bf894fe72f36e3cu64.swap_bytes(),
            0xf1361d5f3af54fa5u64.swap_bytes(),
            0xd182e6ad7f520e51u64.swap_bytes(),
            0x1f6c3e2b8c68059bu64.swap_bytes(),
            0x6bbd41fbabd9831fu64.swap_bytes(),
            0x79217e1319cde05bu64.swap_bytes(),
        ];

        let m: [u64; 16] = [
            0x6162630000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
        ];

        let t: [u64; 2] = [0x0300000000000000u64.swap_bytes(), 0x0000000000000000u64.swap_bytes()];

        let f = true;

        blake2b_compress(rounds, &mut h, &m, &t, f);

        let expected: [u64; 8] = [
            0xba80a53f981c4d0du64.swap_bytes(),
            0x6a2797b69f12f6e9u64.swap_bytes(),
            0x4c212f14685ac4b7u64.swap_bytes(),
            0x4b12bb6fdbffa2d1u64.swap_bytes(),
            0x7d87c5392aab792du64.swap_bytes(),
            0xc252d5de4533cc95u64.swap_bytes(),
            0x18d38aa8dbf1925au64.swap_bytes(),
            0xb92386edd4009923u64.swap_bytes(),
        ];

        assert_eq!(
            h, expected,
            "Blake2b does not match:\n   exp: {:016x?},\n   got: {:016x?}",
            expected, h
        );
    }

    #[test]
    fn test_blake2b_eip152_vector3() {
        // Test vector from EIP-152
        // Input:
        // rounds = 12
        // h = 48c9bdf267e6096a 3ba7ca8485ae67bb 2bf894fe72f36e3c f1361d5f3af54fa5
        //     d182e6ad7f520e51 1f6c3e2b8c68059b 6bbd41fbabd9831f 79217e1319cde05b
        // m = 6162630000000000 0000000000000000 0000000000000000 0000000000000000
        //     0000000000000000 0000000000000000 0000000000000000 0000000000000000
        //     0000000000000000 0000000000000000 0000000000000000 0000000000000000
        //     0000000000000000 0000000000000000 0000000000000000 0000000000000000
        // t = 03 00 00 00 00 00 00 00, 00 00 00 00 00 00 00 00
        // f = false
        //
        // Expected output:
        // 75ab69d3190a562c 51aef8d88f1c2775 876944407270c42c 9844252c26d28752
        // 98743e7f6d5ea2f2 d3e8d226039cd31b 4e426ac4f2d3d666 a610c2116fde4735

        let rounds = 12;

        let mut h: [u64; 8] = [
            0x48c9bdf267e6096au64.swap_bytes(),
            0x3ba7ca8485ae67bbu64.swap_bytes(),
            0x2bf894fe72f36e3cu64.swap_bytes(),
            0xf1361d5f3af54fa5u64.swap_bytes(),
            0xd182e6ad7f520e51u64.swap_bytes(),
            0x1f6c3e2b8c68059bu64.swap_bytes(),
            0x6bbd41fbabd9831fu64.swap_bytes(),
            0x79217e1319cde05bu64.swap_bytes(),
        ];

        let m: [u64; 16] = [
            0x6162630000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
        ];

        let t: [u64; 2] = [0x0300000000000000u64.swap_bytes(), 0x0000000000000000u64.swap_bytes()];

        let f = false;

        blake2b_compress(rounds, &mut h, &m, &t, f);

        let expected: [u64; 8] = [
            0x75ab69d3190a562cu64.swap_bytes(),
            0x51aef8d88f1c2775u64.swap_bytes(),
            0x876944407270c42cu64.swap_bytes(),
            0x9844252c26d28752u64.swap_bytes(),
            0x98743e7f6d5ea2f2u64.swap_bytes(),
            0xd3e8d226039cd31bu64.swap_bytes(),
            0x4e426ac4f2d3d666u64.swap_bytes(),
            0xa610c2116fde4735u64.swap_bytes(),
        ];

        assert_eq!(
            h, expected,
            "Blake2b does not match:\n   exp: {:016x?},\n   got: {:016x?}",
            expected, h
        );
    }

    #[test]
    fn test_blake2b_eip152_vector4() {
        // Test vector from EIP-152
        // Input:
        // rounds = 1
        // h = 48c9bdf267e6096a 3ba7ca8485ae67bb 2bf894fe72f36e3c f1361d5f3af54fa5
        //     d182e6ad7f520e51 1f6c3e2b8c68059b 6bbd41fbabd9831f 79217e1319cde05b
        // m = 6162630000000000 0000000000000000 0000000000000000 0000000000000000
        //     0000000000000000 0000000000000000 0000000000000000 0000000000000000
        //     0000000000000000 0000000000000000 0000000000000000 0000000000000000
        //     0000000000000000 0000000000000000 0000000000000000 0000000000000000
        // t = 03 00 00 00 00 00 00 00, 00 00 00 00 00 00 00 00
        // f = true
        //
        // Expected output:
        // b63a380cb2897d52 1994a85234ee2c18 1b5f844d2c624c00 2677e9703449d2fb
        // a551b3a8333bcdf5 f2f7e08993d53923 de3d64fcc68c034e 717b9293fed7a421

        let rounds = 1;

        let mut h: [u64; 8] = [
            0x48c9bdf267e6096au64.swap_bytes(),
            0x3ba7ca8485ae67bbu64.swap_bytes(),
            0x2bf894fe72f36e3cu64.swap_bytes(),
            0xf1361d5f3af54fa5u64.swap_bytes(),
            0xd182e6ad7f520e51u64.swap_bytes(),
            0x1f6c3e2b8c68059bu64.swap_bytes(),
            0x6bbd41fbabd9831fu64.swap_bytes(),
            0x79217e1319cde05bu64.swap_bytes(),
        ];

        let m: [u64; 16] = [
            0x6162630000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
            0x0000000000000000u64.swap_bytes(),
        ];

        let t: [u64; 2] = [0x0300000000000000u64.swap_bytes(), 0x0000000000000000u64.swap_bytes()];

        let f = true;

        blake2b_compress(rounds, &mut h, &m, &t, f);

        let expected: [u64; 8] = [
            0xb63a380cb2897d52u64.swap_bytes(),
            0x1994a85234ee2c18u64.swap_bytes(),
            0x1b5f844d2c624c00u64.swap_bytes(),
            0x2677e9703449d2fbu64.swap_bytes(),
            0xa551b3a8333bcdf5u64.swap_bytes(),
            0xf2f7e08993d53923u64.swap_bytes(),
            0xde3d64fcc68c034eu64.swap_bytes(),
            0x717b9293fed7a421u64.swap_bytes(),
        ];

        assert_eq!(
            h, expected,
            "Blake2b does not match:\n   exp: {:016x?},\n   got: {:016x?}",
            expected, h
        );
    }
}
