use ziskos::zisklib::hash_to_curve_g2_bls12_381;

// DST used by the IETF test vectors (RFC 9380 §J.10.1)
const DST: &[u8] = b"QUUX-V01-CS02-with-BLS12381G2_XMD:SHA-256_SSWU_RO_";

pub fn hash_to_curve_tests() {
    // RFC 9380 §J.10.1 test vectors

    // msg = ""
    let msg = b"";
    let expected = g2_from_hex(
        "0141ebfbdca40eb85b87142e130ab689c673cf60f1a3e98d69335266f30d9b8d4ac44c1038e9dcdd5393faf5c41fb78a",
        "05cb8437535e20ecffaef7752baddf98034139c38452458baeefab379ba13dff5bf5dd71b72418717047f5b0f37da03d",
        "0503921d7f6a12805e72940b963c0cf3471c7b2a524950ca195d11062ee75ec076daf2d4bc358c4b190c0c98064fdd92",
        "12424ac32561493f3fe3c260708a12b7c620e7be00099a974e259ddc7d1f6395c3c811cdd19f1e8dbf3e9ecfdcbab8d6",
    );
    let p = hash_to_curve_g2_bls12_381(msg, DST);
    assert_eq!(p, expected, "hash_to_curve_g2_bls12_381 test vector 1 mismatch");

    // msg = "abc"
    let msg = b"abc";
    let expected = g2_from_hex(
        "02c2d18e033b960562aae3cab37a27ce00d80ccd5ba4b7fe0e7a210245129dbec7780ccc7954725f4168aff2787776e6",
        "139cddbccdc5e91b9623efd38c49f81a6f83f175e80b06fc374de9eb4b41dfe4ca3a230ed250fbe3a2acf73a41177fd8",
        "1787327b68159716a37440985269cf584bcb1e621d3a7202be6ea05c4cfe244aeb197642555a0645fb87bf7466b2ba48",
        "00aa65dae3c8d732d10ecd2c50f8a1baf3001578f71c694e03866e9f3d49ac1e1ce70dd94a733534f106d4cec0eddd16",
    );
    let p = hash_to_curve_g2_bls12_381(msg, DST);
    assert_eq!(p, expected, "hash_to_curve_g2_bls12_381 test vector 2 mismatch");

    // msg = "abcdef0123456789"
    let msg = b"abcdef0123456789";
    let expected = g2_from_hex(
        "121982811d2491fde9ba7ed31ef9ca474f0e1501297f68c298e9f4c0028add35aea8bb83d53c08cfc007c1e005723cd0",
        "190d119345b94fbd15497bcba94ecf7db2cbfd1e1fe7da034d26cbba169fb3968288b3fafb265f9ebd380512a71c3f2c",
        "05571a0f8d3c08d094576981f4a3b8eda0a8e771fcdcc8ecceaf1356a6acf17574518acb506e435b639353c2e14827c8",
        "0bb5e7572275c567462d91807de765611490205a941a5a6af3b1691bfe596c31225d3aabdf15faff860cb4ef17c7c3be",
    );
    let p = hash_to_curve_g2_bls12_381(msg, DST);
    assert_eq!(p, expected, "hash_to_curve_g2_bls12_381 test vector 3 mismatch");

    // msg = "q128_" + 'q' * 128  (133 bytes)
    let mut msg = [b'q'; 133];
    msg[..5].copy_from_slice(b"q128_");
    let expected = g2_from_hex(
        "19a84dd7248a1066f737cc34502ee5555bd3c19f2ecdb3c7d9e24dc65d4e25e50d83f0f77105e955d78f4762d33c17da",
        "0934aba516a52d8ae479939a91998299c76d39cc0c035cd18813bec433f587e2d7a4fef038260eef0cef4d02aae3eb91",
        "14f81cd421617428bc3b9fe25afbb751d934a00493524bc4e065635b0555084dd54679df1536101b2c979c0152d09192",
        "09bcccfa036b4847c9950780733633f13619994394c23ff0b32fa6b795844f4a0673e20282d07bc69641cee04f5e5662",
    );
    let p = hash_to_curve_g2_bls12_381(&msg, DST);
    assert_eq!(p, expected, "hash_to_curve_g2_bls12_381 test vector 4 mismatch");

    // msg = "a512_" + 'a' * 512  (517 bytes)
    let mut msg = [b'a'; 517];
    msg[..5].copy_from_slice(b"a512_");
    let expected = g2_from_hex(
        "01a6ba2f9a11fa5598b2d8ace0fbe0a0eacb65deceb476fbbcb64fd24557c2f4b18ecfc5663e54ae16a84f5ab7f62534",
        "11fca2ff525572795a801eed17eb12785887c7b63fb77a42be46ce4a34131d71f7a73e95fee3f812aea3de78b4d01569",
        "0b6798718c8aed24bc19cb27f866f1c9effcdbf92397ad6448b5c9db90d2b9da6cbabf48adc1adf59a1a28344e79d57e",
        "03a47f8e6d1763ba0cad63d6114c0accbef65707825a511b251a660a9b3994249ae4e63fac38b23da0c398689ee2ab52",
    );
    let p = hash_to_curve_g2_bls12_381(&msg, DST);
    assert_eq!(p, expected, "hash_to_curve_g2_bls12_381 test vector 5 mismatch");
}

/// Parse a 96-char big-endian hex string into 6 little-endian u64 limbs.
const fn fp_from_be_hex(hex: &str) -> [u64; 6] {
    let bytes = hex.as_bytes();
    assert!(bytes.len() == 96, "Fp hex must be exactly 96 chars (no 0x prefix)");

    let mut limbs = [0u64; 6];
    let mut i = 0;
    while i < 6 {
        let start = i * 16;
        let mut limb = 0u64;
        let mut j = 0;
        while j < 16 {
            let c = bytes[start + j];
            let nibble = match c {
                b'0'..=b'9' => c - b'0',
                b'a'..=b'f' => c - b'a' + 10,
                b'A'..=b'F' => c - b'A' + 10,
                _ => panic!("invalid hex digit"),
            };
            limb = (limb << 4) | nibble as u64;
            j += 1;
        }
        // BE hex's most-significant limb is at hex[0..16], stored at the LE index 5.
        limbs[5 - i] = limb;
        i += 1;
    }
    limbs
}

/// Build a G2 point `[u64; 24]` from four BE hex Fp values: x.c0, x.c1, y.c0, y.c1.
const fn g2_from_hex(x_c0: &str, x_c1: &str, y_c0: &str, y_c1: &str) -> [u64; 24] {
    let xc0 = fp_from_be_hex(x_c0);
    let xc1 = fp_from_be_hex(x_c1);
    let yc0 = fp_from_be_hex(y_c0);
    let yc1 = fp_from_be_hex(y_c1);

    let mut p = [0u64; 24];
    let mut i = 0;
    while i < 6 {
        p[i] = xc0[i];
        p[6 + i] = xc1[i];
        p[12 + i] = yc0[i];
        p[18 + i] = yc1[i];
        i += 1;
    }
    p
}
