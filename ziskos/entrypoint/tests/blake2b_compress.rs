//! Host-side check of `zisklib::blake2b_compress` against the RFC 7693 BLAKE2b-512("abc")
//! test vector. The compression unrolls its rounds over compile-time round indices; this
//! pins the round sequence (0..9, then 0, 1 for the standard 12 rounds) to a known digest.

use ziskos::zisklib::blake2b_compress;

const IV0: u64 = 0x6A09E667F3BCC908;

#[test]
fn blake2b_512_abc_matches_rfc_7693() {
    // Parameter block for an unkeyed 64-byte digest: digest_length = 64, fanout = depth = 1.
    let mut h: [u64; 8] = [
        IV0 ^ 0x0101_0040,
        0xBB67AE8584CAA73B,
        0x3C6EF372FE94F82B,
        0xA54FF53A5F1D36F1,
        0x510E527FADE682D1,
        0x9B05688C2B3E6C1F,
        0x1F83D9ABFB41BD6B,
        0x5BE0CD19137E2179,
    ];
    let mut m = [0u64; 16];
    m[0] = u64::from_le_bytes(*b"abc\0\0\0\0\0");
    let t = [3u64, 0];

    blake2b_compress(12, &mut h, &m, &t, true);

    let mut digest = [0u8; 64];
    for (i, w) in h.iter().enumerate() {
        digest[i * 8..i * 8 + 8].copy_from_slice(&w.to_le_bytes());
    }
    let expected = "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d1\
                    7d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923";
    let got: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(got, expected);
}

#[test]
fn round_counts_below_and_above_ten_follow_the_sigma_cycle() {
    // Reference: one call per round through the runtime-index entry point.
    use ziskos::syscalls::syscall_blake2b_round;
    for rounds in [0u32, 1, 7, 10, 11, 12, 23] {
        let seed = |i: u64| i.wrapping_mul(0x9E3779B97F4A7C15) ^ (rounds as u64);
        let m: [u64; 16] = core::array::from_fn(|i| seed(i as u64 + 100));
        let mut h: [u64; 8] = core::array::from_fn(|i| seed(i as u64));
        let t = [seed(50), seed(51)];
        let mut expected = h;
        {
            // Mirror the compression setup and drive the rounds one by one.
            let iv: [u64; 8] = [
                IV0,
                0xBB67AE8584CAA73B,
                0x3C6EF372FE94F82B,
                0xA54FF53A5F1D36F1,
                0x510E527FADE682D1,
                0x9B05688C2B3E6C1F,
                0x1F83D9ABFB41BD6B,
                0x5BE0CD19137E2179,
            ];
            let mut v = [0u64; 16];
            v[..8].copy_from_slice(&expected);
            v[8..12].copy_from_slice(&iv[..4]);
            v[12] = t[0] ^ iv[4];
            v[13] = t[1] ^ iv[5];
            v[14] = iv[6];
            v[15] = iv[7];
            for r in 0..rounds {
                syscall_blake2b_round((r % 10) as u64, &mut v, &m);
            }
            for i in 0..8 {
                expected[i] ^= v[i] ^ v[i + 8];
            }
        }
        blake2b_compress(rounds, &mut h, &m, &t, false);
        assert_eq!(h, expected, "rounds = {rounds}");
    }
}
