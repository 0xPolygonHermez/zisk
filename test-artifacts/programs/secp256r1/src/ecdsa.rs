use ziskos::zisklib::ecdsa_verify_secp256r1;

use crate::constants::{G_X, G_Y};

// Tests from https://github.com/ethereum/go-ethereum/blob/master/core/vm/testdata/precompiles/p256Verify.json
pub fn ecdsa_tests() {
    // Verify
    let pk = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    let z = [0x7a419feca605023, 0x36e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r = [0xb8cc6af9bd5c2e18, 0xffe50d85a1eee859, 0x80a6d9d1190a436e, 0x2ba3a8be6b94d5ec];
    let s = [0x77a67f79e6fadd76, 0x525fe710fab9aa7c, 0x3c7b11eb6c4e0ae7, 0x4cd60b855d442f5b];
    assert!(ecdsa_verify_secp256r1(&pk, &z, &r, &s));

    // zero hash tests: take any public key (x,y) and set the signature (r,y) = (x,x)
    let z2 = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let r2 = G_X;
    let s2 = G_X;
    let pk2 = [G_X[0], G_X[1], G_X[2], G_X[3], G_Y[0], G_Y[1], G_Y[2], G_Y[3]];
    assert!(ecdsa_verify_secp256r1(&pk2, &z2, &r2, &s2));

    let z3 = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let r3 = [0x69c8c4df6c732838, 0x2903269919f70860, 0xdcfe467828128bad, 0x2927b10512bae3ed];
    let s3 = [0x69c8c4df6c732838, 0x2903269919f70860, 0xdcfe467828128bad, 0x2927b10512bae3ed];
    let pk3 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk3, &z3, &r3, &s3));

    // 1] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #1: signature malleability
    let z4 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r4 = [0xb8cc6af9bd5c2e18, 0xffe50d85a1eee859, 0x80a6d9d1190a436e, 0x2ba3a8be6b94d5ec];
    let s4 = [0x77a67f79e6fadd76, 0x525fe710fab9aa7c, 0x3c7b11eb6c4e0ae7, 0x4cd60b855d442f5b];
    let pk4 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk4, &z4, &r4, &s4));

    // same test but with the s = n - s
    let z5 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r5 = [0xb8cc6af9bd5c2e18, 0xffe50d85a1eee859, 0x80a6d9d1190a436e, 0x2ba3a8be6b94d5ec];
    let s5 = [0x7c134b49156847db, 0x6a87139cac5df408, 0xc384ee1493b1f518, 0xb329f479a2bbd0a5];
    let pk5 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk5, &z5, &r5, &s5));

    // 2] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #3: Modified r or s, e.g. by adding or subtracting the order of the group
    let z6 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r6 = [0x3aed5fc93f06f739, 0xbd01ed280528b62b, 0x7f59262ee6f5bc90, 0xd45c5740946b2a14];
    let s6 = [0x7c134b49156847db, 0x6a87139cac5df408, 0xc384ee1493b1f518, 0xb329f479a2bbd0a5];
    let pk6 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk6, &z6, &r6, &s6));

    // 3] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #5: Modified r or s, e.g. by adding or subtracting the order of the group
    let z7 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r7 = [0x4733950642a3d1e8, 0x001af27a5e1117a6, 0x7f59262ee6f5bc91, 0xd45c5741946b2a13];
    let s7 = [0x7c134b49156847db, 0x6a87139cac5df408, 0xc384ee1493b1f518, 0xb329f479a2bbd0a5];
    let pk7 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk7, &z7, &r7, &s7));

    // 4] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #8: Modified r or s, e.g. by adding or subtracting the order of the group
    let z8 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r8 = [0xb8cc6af9bd5c2e18, 0xffe50d85a1eee859, 0x80a6d9d1190a436e, 0x2ba3a8be6b94d5ec];
    let s8 = [0x83ecb4b6ea97b825, 0x9578ec6353a20bf7, 0x3c7b11eb6c4e0ae7, 0x4cd60b865d442f5a];
    let pk8 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk8, &z8, &r8, &s8));

    // 5] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #9: Signature with special case values for r and s
    let z9 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r9 = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s9 = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk9 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk9, &z9, &r9, &s9));

    // 6] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #10: Signature with special case values for r and s
    let z10 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r10 = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s10 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk10 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk10, &z10, &r10, &s10));

    // 7] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #11: Signature with special case values for r and s
    let z11 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r11 = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s11 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk11 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk11, &z11, &r11, &s11));

    // 8] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #12: Signature with special case values for r and s
    let z12 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r12 = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s12 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk12 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk12, &z12, &r12, &s12));

    // 9] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #13: Signature with special case values for r and s
    let z13 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r13 = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s13 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk13 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk13, &z13, &r13, &s13));

    // 10] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #14: Signature with special case values for r and s
    let z14 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r14 = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s14 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let pk14 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk14, &z14, &r14, &s14));

    // 11] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #15: Signature with special case values for r and s
    let z15 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r15 = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s15 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let pk15 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk15, &z15, &r15, &s15));

    // 12] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #16: Signature with special case values for r and s
    let z16 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r16 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s16 = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk16 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk16, &z16, &r16, &s16));

    // 13] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #17: Signature with special case values for r and s
    let z17 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r17 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s17 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk17 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk17, &z17, &r17, &s17));

    // 14] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #18: Signature with special case values for r and s
    let z18 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r18 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s18 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk18 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk18, &z18, &r18, &s18));

    // 15] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #19: Signature with special case values for r and s
    let z19 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r19 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s19 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk19 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk19, &z19, &r19, &s19));

    // 16] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #20: Signature with special case values for r and s
    let z20 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r20 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s20 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk20 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk20, &z20, &r20, &s20));

    // 17] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #21: Signature with special case values for r and s
    let z21 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r21 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s21 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let pk21 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk21, &z21, &r21, &s21));

    // 18] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #22: Signature with special case values for r and s
    let z22 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r22 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s22 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let pk22 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk22, &z22, &r22, &s22));

    // 19] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #23: Signature with special case values for r and s
    let z23 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r23 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s23 = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk23 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk23, &z23, &r23, &s23));

    // 20] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #24: Signature with special case values for r and s
    let z24 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r24 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s24 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk24 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk24, &z24, &r24, &s24));

    // 21] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #25: Signature with special case values for r and s
    let z25 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r25 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s25 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk25 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk25, &z25, &r25, &s25));

    // 22] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #26: Signature with special case values for r and s
    let z26 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r26 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s26 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk26 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk26, &z26, &r26, &s26));

    // 23] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #27: Signature with special case values for r and s
    let z27 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r27 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s27 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk27 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk27, &z27, &r27, &s27));

    // 24] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #28: Signature with special case values for r and s
    let z28 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r28 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s28 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let pk28 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk28, &z28, &r28, &s28));

    // 25] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #29: Signature with special case values for r and s
    let z29 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r29 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s29 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let pk29 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk29, &z29, &r29, &s29));

    // 26] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #30: Signature with special case values for r and s
    let z30 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r30 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s30 = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk30 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk30, &z30, &r30, &s30));

    // 27] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #31: Signature with special case values for r and s
    let z31 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r31 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s31 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk31 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk31, &z31, &r31, &s31));

    // 28] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #32: Signature with special case values for r and s
    let z32 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r32 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s32 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk32 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk32, &z32, &r32, &s32));

    // 29] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #33: Signature with special case values for r and s
    let z33 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r33 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s33 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk33 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk33, &z33, &r33, &s33));

    // 30] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #34: Signature with special case values for r and s
    let z34 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r34 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s34 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk34 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk34, &z34, &r34, &s34));

    // 31] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #35: Signature with special case values for r and s
    let z35 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r35 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s35 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let pk35 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk35, &z35, &r35, &s35));

    // 32] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #36: Signature with special case values for r and s
    let z36 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r36 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s36 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let pk36 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk36, &z36, &r36, &s36));

    // 33] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #37: Signature with special case values for r and s
    let z37 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r37 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s37 = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk37 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk37, &z37, &r37, &s37));

    // 34] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #38: Signature with special case values for r and s
    let z38 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r38 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s38 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk38 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk38, &z38, &r38, &s38));

    // 35] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #39: Signature with special case values for r and s
    let z39 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r39 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s39 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk39 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk39, &z39, &r39, &s39));

    // 36] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #40: Signature with special case values for r and s
    let z40 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r40 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s40 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk40 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk40, &z40, &r40, &s40));

    // 37] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #41: Signature with special case values for r and s
    let z41 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r41 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s41 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk41 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk41, &z41, &r41, &s41));

    // 38] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #42: Signature with special case values for r and s
    let z42 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r42 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s42 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let pk42 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk42, &z42, &r42, &s42));

    // 39] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #43: Signature with special case values for r and s
    let z43 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r43 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s43 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let pk43 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk43, &z43, &r43, &s43));

    // 40] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #44: Signature with special case values for r and s
    let z44 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r44 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let s44 = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk44 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk44, &z44, &r44, &s44));

    // 41] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #45: Signature with special case values for r and s
    let z45 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r45 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let s45 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk45 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk45, &z45, &r45, &s45));

    // 42] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #46: Signature with special case values for r and s
    let z46 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r46 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let s46 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk46 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk46, &z46, &r46, &s46));

    // 43] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #47: Signature with special case values for r and s
    let z47 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r47 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let s47 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk47 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk47, &z47, &r47, &s47));

    // 44] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #48: Signature with special case values for r and s
    let z48 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r48 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let s48 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk48 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk48, &z48, &r48, &s48));

    // 45] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #49: Signature with special case values for r and s
    let z49 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r49 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let s49 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let pk49 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk49, &z49, &r49, &s49));

    // 46] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #50: Signature with special case values for r and s
    let z50 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r50 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let s50 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let pk50 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk50, &z50, &r50, &s50));

    // 47] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #51: Signature with special case values for r and s
    let z51 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r51 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let s51 = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk51 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk51, &z51, &r51, &s51));

    // 48] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #52: Signature with special case values for r and s
    let z52 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r52 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let s52 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk52 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk52, &z52, &r52, &s52));

    // 49] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #53: Signature with special case values for r and s
    let z53 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r53 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let s53 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk53 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk53, &z53, &r53, &s53));

    // 50] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #54: Signature with special case values for r and s
    let z54 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r54 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let s54 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk54 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk54, &z54, &r54, &s54));

    // 51] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #55: Signature with special case values for r and s
    let z55 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r55 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let s55 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk55 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk55, &z55, &r55, &s55));

    // 52] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #56: Signature with special case values for r and s
    let z56 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r56 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let s56 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let pk56 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk56, &z56, &r56, &s56));

    // 53] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #57: Signature with special case values for r and s
    let z57 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r57 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let s57 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let pk57 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk57, &z57, &r57, &s57));

    // 54] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #58: Edge case for Shamir multiplication
    let z58 = [0x2fa50c772ed6f807, 0x2f2627416faf2f07, 0xc422f44dea4ed1a5, 0x70239dd877f7c944];
    let r58 = [0x11547c97711c898e, 0x8ff312334e2ba16d, 0x4f3e2fc02bdee9be, 0x64a1aab5000d0e80];
    let s58 = [0xfd683b9bb2cf4f1b, 0x7772a2f91d73286f, 0xd1a206d4e013e099, 0x6af015971cc30be6];
    let pk58 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk58, &z58, &r58, &s58));

    // 55] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #59: special case hash
    let z59 = [0x7ead3645f356e7a9, 0x84bcd58a1bb5e747, 0xccf17803ebe2bd08, 0x00000000690ed426];
    let r59 = [0x1e19a0ec580bf266, 0xded7d397738448de, 0x6f78c81c91fc7e8b, 0x16aea964a2f6506d];
    let s59 = [0x38c3ff033be928e9, 0x391e8e80c578d1cd, 0xcfe8b7bc47d27d78, 0x252cd762130c6667];
    let pk59 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk59, &z59, &r59, &s59));

    // 56] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #60: special case hash
    let z60 = [0x140697ad25770d91, 0xf696ad3ebb5ee47f, 0x525c6035725235c2, 0x7300000000213f2a];
    let r60 = [0x7c665baccb23c882, 0xf2d26d6ef524af91, 0xf476dfc26b9b733d, 0x9cc98be2347d469b];
    let s60 = [0xa631dacb16b56c32, 0x0ec1b7847929d10e, 0xd70727b82462f61d, 0x093496459effe2d8];
    let pk60 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk60, &z60, &r60, &s60));

    // 57] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #61: special case hash
    let z61 = [0x4a0161c27fe06045, 0x8afd25daadeb3edb, 0xe0635b245f0b9797, 0xddf2000000005e0b];
    let r61 = [0x093999f07ab8aa43, 0x03dce3dea0d53fa8, 0x058164524dde8927, 0x73b3c90ecd390028];
    let s61 = [0x188c0c4075c88634, 0x2ed25a395387b5f4, 0x5bb7d8bf0a651c80, 0x2f67b0b8e2063669];
    let pk61 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk61, &z61, &r61, &s61));

    // 58] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #62: special case hash
    let z62 = [0x5be1ec355d0841a0, 0x642b8499588b8985, 0x4769c4ecb9e164d6, 0x67ab190000000078];
    let r62 = [0xf37e90119d5ba3dd, 0x1a7f0eb390763378, 0x28fadf2f89b95c85, 0xbfab3098252847b3];
    let s62 = [0x1e2da9b8b4987e3b, 0x8195ccebb65c2aaf, 0x67c2d058ccb44d97, 0xbdd64e234e832b10];
    let pk62 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk62, &z62, &r62, &s62));

    // 59] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #63: special case hash
    let z63 = [0xe296b6350fc311cf, 0x02095dff252ee905, 0x76d7dbeffe125eaf, 0xa2bf094600000000];
    let r63 = [0xd17093c5cd21d2cd, 0xf1c9aaab168b1596, 0x8bf8bf04a4ceb1c1, 0x204a9784074b246d];
    let s63 = [0x582fe648d1d88b52, 0xa406c2506fe17975, 0xdc06a759c8847868, 0x51cce41670636783];
    let pk63 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk63, &z63, &r63, &s63));

    // 60] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #64: special case hash
    let z64 = [0x29e15c544e4f0e65, 0xa0a3531711608581, 0x00e1e75e624a06b3, 0x3554e827c7000000];
    let r64 = [0x027bca0f1ceeaa03, 0x0031a91d1314f835, 0xf63d4aa4f81fe2cb, 0xed66dc34f551ac82];
    let s64 = [0xbb8953d67c0c48c7, 0x67623c3f6e5d4d6a, 0x194a422e18d5fda1, 0x99ca123aa09b13cd];
    let pk64 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk64, &z64, &r64, &s64));

    // 61] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #65: special case hash
    let z65 = [0x26e3a54b9fc6965c, 0x3255ea4c9fd0cb34, 0x000026941a0f0bb5, 0x9b6cd3b812610000];
    let r65 = [0x56bf0f60a237012b, 0x126b062023ccc3c0, 0x899d44f2356a578d, 0x060b700bef665c68];
    let s65 = [0x6be5d581c11d3610, 0xedbb410cbef3f26d, 0x4fcc78a3366ca95d, 0x8d186c027832965f];
    let pk65 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk65, &z65, &r65, &s65));

    // 62] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #66: special case hash
    let z66 = [0x77162f93c4ae0186, 0x82a52baa51c71ca8, 0x000000e7561c26fc, 0x883ae39f50bf0100];
    let r66 = [0x2bb0c8e38c96831d, 0xc93ea76cd313c913, 0x24d7aa7934b6cf29, 0x9f6adfe8d5eb5b2c];
    let s66 = [0x051593883b5e9902, 0x906a33e66b5bd15e, 0x890c944cf271756c, 0xb26a9c9e40e55ee0];
    let pk66 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk66, &z66, &r66, &s66));

    // 63] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #67: special case hash
    let z67 = [0x01fe9fce011d0ba6, 0x10540f420fb4ff74, 0x0000000000fa7cd0, 0xa1ce5d6e5ecaf28b];
    let r67 = [0x8868f4ba273f16b7, 0xa1abf6da168cebfa, 0x3ad2f33615e56174, 0xa1af03ca91677b67];
    let s67 = [0x5caf24c8c5e06b1c, 0x77d69022e7d098d7, 0x35cd258b173d0c23, 0x20aa73ffe48afa64];
    let pk67 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk67, &z67, &r67, &s67));

    // 64] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #68: special case hash
    let z68 = [0x5494cdffd5ee8054, 0x97330012a8ee836c, 0x9300000000383453, 0x8ea5f645f373f580];
    let r68 = [0xe327a28c11893db9, 0x659355507b843da6, 0x11a6c99a71c973d5, 0xfdc70602766f8eed];
    let s68 = [0xa7f83f2b10d21350, 0x0f6d15ec0078ca60, 0x37b1eacf456a9e9e, 0x3df5349688a085b1];
    let pk68 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk68, &z68, &r68, &s68));

    // 65] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #69: special case hash
    let z69 = [0x8d9c1bbdcb5ef305, 0xd65ce93eabb7d60d, 0xa734000000008792, 0x660570d323e9f75f];
    let r69 = [0x0dc738f7b876e675, 0x23456f63c643cf8e, 0xd6537f6a6c49966c, 0xb516a314f2fce530];
    let s69 = [0xa66b0120cd16fff2, 0x967c4bd80954479b, 0x17dd536fbc5efdf1, 0xd39ffd033c92b6d7];
    let pk69 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk69, &z69, &r69, &s69));

    // 66] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #70: special case hash
    let z70 = [0x46ada2de4c568c34, 0x8d35f1f45cf9c3bf, 0x7dde8800000000e9, 0xd0462673154cce58];
    let r70 = [0xa485c101e29ff0a8, 0x82717bebb6492fd0, 0x2ecb7984d4758315, 0x3b2cbf046eac4584];
    let s70 = [0xc8595fc1c1d99258, 0x701099cac5f76e68, 0xde512bc9313aaf51, 0x4c9b7b47a98b0f82];
    let pk70 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk70, &z70, &r70, &s70));

    // 67] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #71: special case hash
    let z71 = [0xb83e7b4418d7278f, 0x0caef15a6171059a, 0x80cedfef00000000, 0xbd90640269a78226];
    let r71 = [0x6c3fb15bfde48dcf, 0xd79d0312cfa1ab65, 0x841f14af54e2f9ed, 0x30c87d35e636f540];
    let s71 = [0x0db9abf6340677ed, 0x71409ede23efd08e, 0xc85a692bd6ecafeb, 0x47c15a5a82d24b75];
    let pk71 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk71, &z71, &r71, &s71));

    // 68] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #72: special case hash
    let z72 = [0x4beae8e284788a73, 0x00d2dcceb301c54b, 0x512e41222a000000, 0x33239a52d72f1311];
    let r72 = [0x68ff262113760f52, 0xe2e8176d168dec3c, 0xbc43b58cfe6647b9, 0x38686ff0fda2cef6];
    let s72 = [0xc2ddabb3fde9d67d, 0xe976e2db5e6a4cf7, 0x9601662167fa8717, 0x067ec3b651f42266];
    let pk72 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk72, &z72, &r72, &s72));

    // 69] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #73: special case hash
    let z73 = [0x1dc84c2d941ffaf1, 0x00007ee4a21a1cbe, 0x1365d4e6d95c0000, 0xb8d64fbcd4a1c10f];
    let r73 = [0x225985ab6e2775cf, 0xf3e17d27f5ee844b, 0x44fc25c7f2de8b6a, 0x44a3e23bf314f2b3];
    let s73 = [0x93c9cc3f4dd15e86, 0x84f0411f57295004, 0x1ddc87be532abed5, 0x2d48e223205e9804];
    let pk73 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk73, &z73, &r73, &s73));

    // 70] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #74: special case hash
    let z74 = [0x4088b20fe0e9d84a, 0x0000003a227420db, 0xa3fef3183ed09200, 0x01603d3982bf77d7];
    let r74 = [0x0eb9d638781688e9, 0x41b99db3b5aa8d33, 0xf11f967a3d95110c, 0x2ded5b7ec8e90e7b];
    let s74 = [0xec69238a009808f9, 0x8de049c328ae1f44, 0x1bfc46fb1a67e308, 0x7d5792c53628155e];
    let pk74 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk74, &z74, &r74, &s74));

    // 71] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #75: special case hash
    let z75 = [0xb7e9eb0cfbff7363, 0x000000004d89ef50, 0x599aa02e6cf66d9c, 0x9ea6994f1e0384c8];
    let r75 = [0x05976f15137d8b8f, 0x3eaccafcd40ec2f6, 0xefd3bc3d31870f92, 0xbdae7bcb580bf335];
    let s75 = [0x24838122ce7ec3c7, 0x9f373a4fb318994f, 0x0b0106eecfe25749, 0xf6dfa12f19e52527];
    let pk75 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk75, &z75, &r75, &s75));

    // 72] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #76: special case hash
    let z76 = [0xf692bc670905b18c, 0x4700000000e2fa5b, 0x693979371a01068a, 0xd03215a8401bcf16];
    let r76 = [0x1ece251c2401f1c6, 0x99209b78596956d2, 0x62720957ffff5137, 0x50f9c4f0cd6940e1];
    let s76 = [0xaa5167dfab244726, 0x5a4355e411a59c32, 0x889defaaabb106b9, 0xd7033a0a787d338e];
    let pk76 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk76, &z76, &r76, &s76));

    // 73] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #77: special case hash
    let z77 = [0xfd5f64b582e3bb14, 0xc87e000000008408, 0x9c84bf83f0300e5d, 0x307bfaaffb650c88];
    let r77 = [0xbe90924ead5c860d, 0x0982e29575d019aa, 0x1906066a378d6754, 0xf612820687604fa0];
    let s77 = [0x328230ce294b0fef, 0x1a99f4857b316525, 0x75ea98afd20e328a, 0x3f9367702dd7dd4f];
    let pk77 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk77, &z77, &r77, &s77));

    // 74] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #78: special case hash
    let z78 = [0xaf574bb4d54ea6b8, 0x51527c00000000e4, 0x33324d36bb0c1575, 0xbab5c4f4df540d7b];
    let r78 = [0x0f2f507da5782a7a, 0x1f61980c1949f56b, 0xc93db5da7aa6f508, 0x9505e407657d6e8b];
    let s78 = [0x5e7f71784f9c5021, 0x08e0ed5cb92b3cfa, 0x8ffbeccab6c3656c, 0xc60d31904e366973];
    let pk78 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk78, &z78, &r78, &s78));

    // 75] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #79: special case hash
    let z79 = [0xc3b869197ef5e15e, 0xc2456f5b00000000, 0xe4f58d8036f9c36e, 0xd4ba47f6ae28f274];
    let r79 = [0x3e1c68a40404517d, 0x08735aed37173272, 0xd83e6a7787cd691b, 0xbbd16fbbb656b6d0];
    let s79 = [0x560e3e7fd25c0f00, 0x7d2d097be5e8ee34, 0x787d91315be67587, 0x9d8e35dba96028b7];
    let pk79 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk79, &z79, &r79, &s79));

    // 76] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #80: special case hash
    let z80 = [0x00801e47f8c184e1, 0xfe0f10aafd000000, 0xf29f1fa00984342a, 0x79fd19c7235ea212];
    let r80 = [0xcf57c61e92df327e, 0x442d2ceef7559a30, 0x06ea76848d35a6da, 0x2ec9760122db98fd];
    let s80 = [0xc4963625c0a19878, 0x393fb6814c27b760, 0x701fccf86e462ee3, 0x7ab271da90859479];
    let pk80 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk80, &z80, &r80, &s80));

    // 77] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #81: special case hash
    let z81 = [0x0000a37ea6700cda, 0x79cbeb7ac9730000, 0xaf9aba5c0583462d, 0x8c291e8eeaa45adb];
    let r81 = [0x4f1005a89fe00c59, 0xd9ba9dd463221f7a, 0xaa6a7fc49b1c51ee, 0x54e76b7683b6650b];
    let s81 = [0x52f2f7806a31c8fd, 0xcfd11b1c1ae11661, 0x37ec1cc8374b7915, 0x2ea076886c773eb9];
    let pk81 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk81, &z81, &r81, &s81));

    // 78] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #82: special case hash
    let z82 = [0x0000003c278a6b21, 0xf4cdcf66c3f78a00, 0x9803efbfb8140732, 0x0eaae8641084fa97];
    let r82 = [0x10419c0c496c9466, 0x7a74abdbb69be4fb, 0xbce6e3c26f602109, 0x5291deaf24659ffb];
    let s82 = [0xbf83469270a03dc3, 0x827f84742f29f10a, 0xcdb982bb4e4ecef5, 0x65d6fcf336d27cc7];
    let pk82 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk82, &z82, &r82, &s82));

    // 79] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #83: special case hash
    let z83 = [0x00000000afc0f89d, 0xef17c6d96e13846c, 0x0068399bf01bab42, 0xe02716d01fb23a5a];
    let r83 = [0xd15166a88479f107, 0x003b33fc17eb50f9, 0x47419dc58efb05e8, 0x207a3241812d75d9];
    let s83 = [0x82d5caadf7592767, 0xf1c5d70793cf55e3, 0x3ce80b32d0574f62, 0xcdee749f2e492b21];
    let pk83 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk83, &z83, &r83, &s83));

    // 80] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #84: special case hash
    let z84 = [0x9a00000000fc7de1, 0x9061768af89d0065, 0x194e9a16bc7dab2a, 0x9eb0bf583a1a6b9a];
    let r84 = [0xc0dee3cf81aa7728, 0xbe84437a355a0a37, 0x4328ac94913bf01b, 0x6554e49f82a85520];
    let s84 = [0x86effe7f22b4f929, 0x16250a2eaebc8be4, 0xc94e1e126980d3df, 0xaea00de2507ddaf5];
    let pk84 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk84, &z84, &r84, &s84));

    // 81] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #85: special case hash
    let z85 = [0x690e00000000cd15, 0x6e1030cb53d9a82b, 0x2c214f0d5e72ef28, 0x62aac98818b3b84a];
    let r85 = [0x2990ac82707efdfc, 0x6c6e19b4d80a8c60, 0xbff06f71c88216c2, 0xa54c5062648339d2];
    let s85 = [0xff09be73c9731b0d, 0x1056317f467ad09a, 0x69fd016777517aa0, 0xe99bbe7fcfafae3e];
    let pk85 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk85, &z85, &r85, &s85));

    // 82] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #86: special case hash
    let z86 = [0x464b9300000000c8, 0xd2b6f552ea4b6895, 0xf29ae43732e513ef, 0x3760a7f37cf96218];
    let r86 = [0x4ca8b059cff37eaf, 0xd23096593133e71b, 0x309f1f444012b1a1, 0x975bd7157a8d363b];
    let s86 = [0xacc46786bf919622, 0xd4c69840fe090f2a, 0xa241793f2abc930b, 0x7faa7a28b1c822ba];
    let pk86 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk86, &z86, &r86, &s86));

    // 83] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #87: special case hash
    let z87 = [0xbb6ff6c800000000, 0x6b4320bea836cd9c, 0x3834f2098c088009, 0x0da0a1d2851d3302];
    let r87 = [0x7b95b3e0da43885e, 0xde9ec90305afb135, 0x276afd2ebcfe4d61, 0x5694a6f84b8f875c];
    let s87 = [0x3b6ccc7c679cbaa4, 0x8ee2dc5c7870c082, 0x8051dec02ebdf70d, 0x0dffad9ffd0b757d];
    let pk87 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk87, &z87, &r87, &s87));

    // 84] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #88: special case hash
    let z88 = [0xa764a231e82d289a, 0x0fe975f735887194, 0x086fd567aafd598f, 0xffffffff293886d3];
    let r88 = [0xd7454ba9790f1ba6, 0xf7098f1a98d21620, 0xb4968a27d16a6d08, 0xa0c30e8026fdb2b4];
    let s88 = [0x8bd2760c65424339, 0xacc5ca6445914968, 0x5baf463f9deceb53, 0x5e470453a8a399f1];
    let pk88 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk88, &z88, &r88, &s88));

    // 85] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #89: special case hash
    let z89 = [0x0e8d9ca99527e7b7, 0x26acdc4ce127ec2e, 0xe3c03445a072e243, 0x7bffffffff2376d1];
    let r89 = [0x2aa0228cf7b99a88, 0x1dfebebd5ad8aca5, 0xdd73602cd4bb4eea, 0x614ea84acf736527];
    let s89 = [0x2a4dd193195c902f, 0xde14368e96a9482c, 0xd1b8183f3ed490e4, 0x737cc85f5f2d2f60];
    let pk89 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk89, &z89, &r89, &s89));

    // 86] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #90: special case hash
    let z90 = [0xfd016807e97fa395, 0xbc80872602a6e467, 0x51b085377605a224, 0xa2b5ffffffffebb2];
    let r90 = [0xa8d74dfbd0f942fa, 0x45377338febfd439, 0x0d3fb2ea00b17329, 0xbead6734ebe44b81];
    let s90 = [0x36a46b103ef56e2a, 0xf4bbe7a10f73b3e0, 0x3cad35919fd21a8a, 0x6bb18eae36616a7d];
    let pk90 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk90, &z90, &r90, &s90));

    // 87] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #91: special case hash
    let z91 = [0x7b83d0967d4b20c0, 0xc1a3c256870d45a6, 0x1b96fa5f097fcf3c, 0x641227ffffffff6f];
    let r91 = [0x654fae182df9bad2, 0x8d922cbf212703e9, 0xd4db9d9ce64854c9, 0x499625479e161dac];
    let s91 = [0x95b64fca76d9d693, 0x9439936028864ac1, 0x0131108d97819edd, 0x42c177cf37b8193a];
    let pk91 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk91, &z91, &r91, &s91));

    // 88] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #92: special case hash
    let z92 = [0x8df56f36600e0f8b, 0xba20352117750229, 0xabad03e2fc662dc3, 0x958415d8ffffffff];
    let r92 = [0x50fb1aaa6ff6c9b2, 0x31e3bfe694f6b89c, 0x66a2c8065b541b3d, 0x08f16b8093a8fb4d];
    let s92 = [0x535ba3e5af81ca2e, 0x21f967410399b39b, 0x48573b611cb95d4a, 0x9d6455e2d5d17797];
    let pk92 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk92, &z92, &r92, &s92));

    // 89] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #93: special case hash
    let z93 = [0x954521b6975420f8, 0xe13deb04e1fbe8fb, 0xff1281093536f47f, 0xf1d8de4858ffffff];
    let r93 = [0xeed8dc2b338cb5f8, 0xc579b6938d19bce8, 0x19dd72ddb99ed8f8, 0xbe26231b6191658a];
    let s93 = [0xb9c5e96952575c89, 0xc943c14f79694a03, 0x37f0f22b2dcb57d5, 0xe1d9a32ee56cffed];
    let pk93 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk93, &z93, &r93, &s93));

    // 90] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #94: special case hash
    let z94 = [0x876b95c81fc31def, 0x32dc5d47c05ef6f1, 0xffff10782dd14a3b, 0x0927895f2802ffff];
    let r94 = [0x12638c455abe0443, 0x45f36a229d4aa4f8, 0x6204ac920a02d580, 0x15e76880898316b1];
    let s94 = [0x38196506a1939123, 0x55ca10e226e13f96, 0x5337bd6aba4178b4, 0xe74d357d3fcb5c8c];
    let pk94 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk94, &z94, &r94, &s94));

    // 91] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #95: special case hash
    let z95 = [0x24cf6a0c3ac80589, 0x0a57c3063fb5a306, 0xffffff4f332862a1, 0x60907984aa7e8eff];
    let r95 = [0x132315cc07f16dad, 0x31e6307d3ddbffc1, 0x3a45f9846fc28d1d, 0x352ecb53f8df2c50];
    let s95 = [0x899792887dd0a3c6, 0x436726ecd28258b1, 0xe1d05c5242ca1c39, 0x1348dfa9c482c558];
    let pk95 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk95, &z95, &r95, &s95));

    // 92] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #96: special case hash
    let z96 = [0x42d6b9b8cd6ae1e2, 0x50f9a5f50636ea69, 0xffffffff0af42cda, 0xc6ff198484939170];
    let r96 = [0x2c5bfa5f2a9558fb, 0x77b8642349ed3d65, 0x8a0da9882ab23c76, 0x4a40801a7e606ba7];
    let s96 = [0xea77dc5981725782, 0xdc24ed2925825bf8, 0x7f605f2832f7384b, 0x3a49b64848d682ef];
    let pk96 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk96, &z96, &r96, &s96));

    // 93] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #97: special case hash
    let z97 = [0x16dfbe4d27d7e68d, 0x9b9e0956cc43135d, 0x75ffffffff807479, 0xde030419345ca15c];
    let r97 = [0xe5e9e44df3d61e96, 0xb3511bac855c05c9, 0x2be412b078924b3b, 0xeacc5e1a8304a74d];
    let s97 = [0x08db8f714204f6d1, 0xec4bb0ed4c36ce98, 0x85dd827714847f96, 0x7451cd8e18d6ed18];
    let pk97 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk97, &z97, &r97, &s97));

    // 94] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #98: special case hash
    let z98 = [0x7e1ab78caaaac6ff, 0x665604d34acb1903, 0x2b88fffffffff6c8, 0x6f0e3eeaf42b2813];
    let r98 = [0x5f7de94c31577052, 0x4f8cd1214882adb6, 0xf30f67fdab61e8ce, 0x2f7a5e9e5771d424];
    let s98 = [0xb9528f8f78daa10c, 0xfb75dd050c5a449a, 0x44acb0b2bd889175, 0xac4e69808345809b];
    let pk98 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk98, &z98, &r98, &s98));

    // 95] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #99: special case hash
    let z99 = [0x2cb222d1f8017ab9, 0x48f7c0591ddcae7d, 0x3708d1ffffffffbe, 0xcdb549f773b3e62b];
    let r99 = [0x0a03d710b3300219, 0x7dddd7f6487621c3, 0x3e7e0f0e95e1a214, 0xffcda40f792ce4d9];
    let s99 = [0xd58c422c2453a49a, 0xfa77618f0b67add8, 0xd7ba9ade8f2065a1, 0x79938b55f8a17f7e];
    let pk99 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk99, &z99, &r99, &s99));

    // 96] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #100: special case hash
    let z100 = [0x24d8fd6f0edb0484, 0x9fd64886c1dc4f99, 0x1df4989bffffffff, 0x2c3f26f96a3ac005];
    let r100 = [0x8c17603a431e39a8, 0x48350f7ab3a588b2, 0x3d3e8c8c3fcc16a9, 0x81f2359c4faba6b5];
    let s100 = [0x7f9e101857f74300, 0x09e46d99fccefb9f, 0x0ff695d06c6860b5, 0xcd6f6a5cc3b55ead];
    let pk100 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk100, &z100, &r100, &s100));

    // 97] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #101: special case hash
    let z101 = [0x8476397c04edf411, 0xff5c31d89fda6a6b, 0x2cb7d53f9affffff, 0xac18f8418c55a250];
    let r101 = [0xc3f5f2aaf75ca808, 0xea130251a6fdffa5, 0xee1596fb073ea283, 0xdfc8bf520445cbb8];
    let s101 = [0xa7ac711e577e90e7, 0xbfd7d0dc7a4905b3, 0xd92823640e338e68, 0x048e33efce147c9d];
    let pk101 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk101, &z101, &r101, &s101));

    // 98] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #102: special case hash
    let z102 = [0x3e5a6ab8cf0ee610, 0xffffa2fd3e289368, 0xb24094f72bb5ffff, 0x4f9618f98e2d3a15];
    let r102 = [0x88227688ba6a5762, 0x6503a0e393e932f6, 0xefda70b46c53db16, 0xad019f74c6941d20];
    let s102 = [0xbc05efe16c199345, 0x7964ef2e0988e712, 0x5346bdbb3102cdcf, 0x93320eb7ca071025];
    let pk102 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk102, &z102, &r102, &s102));

    // 99] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #103: special case hash
    let z103 = [0x04caae73ab0bc75a, 0xffffff67edf7c402, 0x9cc21d31d37a25ff, 0x422e82a3d56ed10a];
    let r103 = [0xdeb7bd5a3ebc1883, 0xb54316bd3ebf7fff, 0xc34e78ce11dd71e4, 0xac8096842e8add68];
    let s103 = [0x9f21a3aac003b7a8, 0x36e3ce9f0ce21970, 0x2d4caf85d187215d, 0xf5ca2f4f23d67450];
    let pk103 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk103, &z103, &r103, &s103));

    // 100] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #104: special case hash
    let z104 = [0x2d9890b5cf95d018, 0x17a5ffffffffa084, 0x6e7b329ff738fbb4, 0x7075d245ccc3281b];
    let r104 = [0x54b4943693fb92f7, 0x89ddcd7b7b9d7768, 0xf939b70ea0022508, 0x677b2d3a59b18a5f];
    let s104 = [0xab6972cc0795db55, 0x5d2f63aee81efd0b, 0xf30307b21f3ccda3, 0x6b4ba856ade7677b];
    let pk104 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk104, &z104, &r104, &s104));

    // 101] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #105: special case hash
    let z105 = [0xc1847eb76c217a95, 0x7e280ebeffffffff, 0x9443d593fa4fd659, 0x3c80de54cd922698];
    let r105 = [0x05e1fc0d5957cfb0, 0xd84d31d4b7c30e1f, 0x379ba8e1b73d3115, 0x479e1ded14bcaed0];
    let s105 = [0x1e877027355b2443, 0x30857ca879f97c77, 0x7cf634a4f05b2e0c, 0x918f79e35b3d8948];
    let pk105 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk105, &z105, &r105, &s105));

    // 102] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #106: special case hash
    let z106 = [0xffc7906aa794b39b, 0x0ce891a8cdffffff, 0x980bef3d697ea277, 0xde21754e29b85601];
    let r106 = [0xb64840ead512a0a3, 0xd711e14b12ac5cf3, 0xd9a58f01164d55c3, 0x43dfccd0edb9e280];
    let s106 = [0x3199f49584389772, 0xca1174899b78ef9a, 0xcd5c4934365b3442, 0x1dbe33fa8ba84533];
    let pk106 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk106, &z106, &r106, &s106));

    // 103] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #107: special case hash
    let z107 = [0xffff2f1f2f57881c, 0x599e4d5f7289ffff, 0x84dd59623fb531bb, 0x8f65d92927cfb86a];
    let r107 = [0x38bb4085f0bbff11, 0xa20e9087c259d26a, 0xf4c7c7e4bca592fe, 0x5b09ab637bd4caf0];
    let s107 = [0xca8101de08eb0d75, 0xa24964e5a13f885b, 0x618e9d80d6fdcd6a, 0x45b7eb467b6748af];
    let pk107 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk107, &z107, &r107, &s107));

    // 104] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #108: special case hash
    let z108 = [0xfffffffafc8c3ca8, 0x2cc7cd0e8426cbff, 0x160bea3877dace8a, 0x6b63e9a74e092120];
    let r108 = [0x14a5039ed15ee06f, 0x667afa570a6cfa01, 0x5728c5c8af9b74e0, 0x5e9b1c5a028070df];
    let s108 = [0x44edaeb9ad990c20, 0x6c29eeffd3c50377, 0xad362bb8d7bd661b, 0xb1360907e2d9785e];
    let pk108 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk108, &z108, &r108, &s108));

    // 105] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #109: special case hash
    let z109 = [0xffffffffe852512e, 0xd094586e249c8699, 0xb6d75219444e8b43, 0xfc28259702a03845];
    let r109 = [0xd1a7a5fb8578f32e, 0x4890050f5a5712f6, 0x4a2fb0990e34538b, 0x0671a0a85c2b72d5];
    let s109 = [0xc720e5854713694c, 0x1808f27fd5bd4fda, 0x79ab9c3285ca4129, 0xdb1846bab6b73614];
    let pk109 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk109, &z109, &r109, &s109));

    // 106] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #110: special case hash
    let z110 = [0x1757ffffffffe20a, 0x74ecbcd52e8ceb57, 0xcee044ee8e8db7f7, 0x1273b4502ea4e3bc];
    let r110 = [0xbaedb35b2095103a, 0xc5d7d69859d301ab, 0x77dbbb0590a45492, 0x7673f85267484464];
    let s110 = [0x3807ef4422913d7c, 0x4dec0d417a414fed, 0x886bed9e6af02e0e, 0x3dc70ddf9c6b524d];
    let pk110 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk110, &z110, &r110, &s110));

    // 107] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #111: special case hash
    let z111 = [0xfb49ffffffffff6e, 0x4f8c53a15b96e602, 0x0c566c66228d8181, 0x08fb565610a79baa];
    let r111 = [0x9dfd657a796d12b5, 0x450d1a06c36d3ff3, 0xb21285089ebb1aa6, 0x7f085441070ecd2b];
    let s111 = [0xa9e4c5c54a2b9a8b, 0x92a5e6cb4b2d8daf, 0x2459d18d47da9aa4, 0x249712012029870a];
    let pk111 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk111, &z111, &r111, &s111));

    // 108] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #112: special case hash
    let z112 = [0x28ecaefeffffffff, 0xa2403f748e97d7cd, 0x87715fcb1aa4e79a, 0xd59291cc2cf89f30];
    let r112 = [0xa8e0f30a5d287348, 0xb76df04bc5aa6683, 0xc867398ea7322d5a, 0x914c67fb61dd1e27];
    let s112 = [0xc96d28f6d37304ea, 0xea7e66ec412b38d6, 0x4953e3ac1959ee8c, 0xfa07474031481dda];
    let pk112 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk112, &z112, &r112, &s112));

    // 109] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #113: k*G has a large x-coordinate
    let z113 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r113 = [0x0c46353d039cdaab, 0x4319055358e8617b, 0x0000000000000000, 0x0000000000000000];
    let s113 = [0xf3b9cac2fc63254e, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk113 = [
        0xa69874d2de5fe103,
        0x4d43784640855bf0,
        0x40031d72a9f5445a,
        0x0ad99500288d4669,
        0x22f0979ff0c3ba5e,
        0xba2c80c9244f4c54,
        0x50d5d3d29f99ae6e,
        0xc5011e6ef2c42dcd,
    ];
    assert!(ecdsa_verify_secp256r1(&pk113, &z113, &r113, &s113));

    // 110] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #114: r too large
    let z114 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r114 = [0xfffffffffffffffc, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let s114 = [0xf3b9cac2fc63254e, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk114 = [
        0xa69874d2de5fe103,
        0x4d43784640855bf0,
        0x40031d72a9f5445a,
        0x0ad99500288d4669,
        0x22f0979ff0c3ba5e,
        0xba2c80c9244f4c54,
        0x50d5d3d29f99ae6e,
        0xc5011e6ef2c42dcd,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk114, &z114, &r114, &s114));

    // 111] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #115: r,s are large
    let z115 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r115 = [0xf3b9cac2fc63254f, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s115 = [0xf3b9cac2fc63254e, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk115 = [
        0x013e09c582204554,
        0x193d0aa398f0fba8,
        0xe6f4819652d9fc69,
        0xab05fd9d0de26b9c,
        0x2f49435a1e9b8d45,
        0x2dd4103f19f6a8c3,
        0x59095d12b75af069,
        0x19235271228c7867,
    ];
    assert!(ecdsa_verify_secp256r1(&pk115, &z115, &r115, &s115));

    // 112] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #116: r and s^-1 have a large Hamming weight
    let z116 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r116 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s116 = [0xde54a36383df8dd4, 0x1453fe50914f3df2, 0x170f5ead2de4f651, 0x909135bdb6799286];
    let pk116 = [
        0x07badf6fdd4c6c56,
        0xfbfecf876219710b,
        0x6a68aa4201b6be5d,
        0x80984f39a1ff38a8,
        0xf1445019bb55ed95,
        0xd74415ed3cac2089,
        0x7a06dfb41871c940,
        0x11feb97390d9826e,
    ];
    assert!(ecdsa_verify_secp256r1(&pk116, &z116, &r116, &s116));

    // 113] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #117: r and s^-1 have a large Hamming weight
    let z117 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r117 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s117 = [0x360644669ca249a5, 0xf5deb773ad5f5a84, 0x71303fd5dd227dce, 0x27b4577ca009376f];
    let pk117 = [
        0x0a95bc602b4f7c05,
        0x6dd687495fcc19a7,
        0x3294f5baa9a3232b,
        0x4201b4272944201c,
        0x572c0c0a8fb0800e,
        0x36f463e3aef16629,
        0x1bb5ac6feaf753bc,
        0x95c37eba9ee8171c,
    ];
    assert!(ecdsa_verify_secp256r1(&pk117, &z117, &r117, &s117));

    // 114] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #118: small r and s
    let z118 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r118 = [0x0000000000000005, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s118 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk118 = [
        0xd91082c8725ac957,
        0x15ce88a4c9d25514,
        0x4e02b7922d66ce94,
        0xa71af64de5126a4a,
        0x2e7c9f11e872296b,
        0x30a435b993264548,
        0xb369fec9c2665d8e,
        0x5d47723c8fbe580b,
    ];
    assert!(ecdsa_verify_secp256r1(&pk118, &z118, &r118, &s118));

    // 115] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #120: small r and s
    let z119 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r119 = [0x0000000000000005, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s119 = [0x0000000000000003, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk119 = [
        0x2b31ee8ef16b1572,
        0x572f597d20df08fc,
        0x3fc2931f90ebe5b7,
        0x6627cec4f0731ea2,
        0xf2073e258fe694a5,
        0x3ee18f709bb275ea,
        0xc5c9c3c4c9be7f0d,
        0x6170ed77d8d0a14f,
    ];
    assert!(ecdsa_verify_secp256r1(&pk119, &z119, &r119, &s119));

    // 116] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #122: small r and s
    let z120 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r120 = [0x0000000000000005, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s120 = [0x0000000000000005, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk120 = [
        0x262ca7ec5a77f5bf,
        0x14afc010cb731343,
        0xe1f5e7544c54e73f,
        0x5a7c8825e85691cc,
        0xa16d3d7b2812f813,
        0x3c39bfce95f30e13,
        0xd7b147fb6c3d22af,
        0xef6edf62a4497c1b,
    ];
    assert!(ecdsa_verify_secp256r1(&pk120, &z120, &r120, &s120));

    // 117] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #124: small r and s
    let z121 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r121 = [0x0000000000000005, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s121 = [0x0000000000000006, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk121 = [
        0x3fa6caca7978c737,
        0x048e5e2fff996d88,
        0x64fedd603152990c,
        0xcbe0c29132cd7383,
        0x5189d4ab0d70e8c1,
        0x188e80bff7cc31ad,
        0x24b2603606f4c04d,
        0x70af6a8ce44cb412,
    ];
    assert!(ecdsa_verify_secp256r1(&pk121, &z121, &r121, &s121));

    // 118] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #126: r is larger than n
    let z122 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r122 = [0xf3b9cac2fc632556, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s122 = [0x0000000000000006, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk122 = [
        0x3fa6caca7978c737,
        0x048e5e2fff996d88,
        0x64fedd603152990c,
        0xcbe0c29132cd7383,
        0x5189d4ab0d70e8c1,
        0x188e80bff7cc31ad,
        0x24b2603606f4c04d,
        0x70af6a8ce44cb412,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk122, &z122, &r122, &s122));

    // 119] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #127: s is larger than n
    let z123 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r123 = [0x0000000000000005, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s123 = [0xf3b9cac2fc75fbd8, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk123 = [
        0x6db83444b037e139,
        0xd33a6795d02a2079,
        0xeab68f0d9a130e0e,
        0x4be4178097002f0d,
        0x075b51ae296d2d56,
        0x47caa669f193c1b4,
        0xce4dacea0f50d1f2,
        0x20f13051e0eecdcf,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk123, &z123, &r123, &s123));

    // 120] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #128: small r and s^-1
    let z124 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r124 = [0x0000000000000100, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s124 = [0x6e0d28c9bb75ea88, 0x516af4f63f2d74d7, 0xbb76eddbb76eddbb, 0x8f1e3c7862c58b16];
    let pk124 = [
        0x7783c97bf3e890d9,
        0x9f15313ebbba379d,
        0xd4be4329faa48d26,
        0xd0f73792203716af,
        0x68690d2363c89cc1,
        0x417e8f566549e6bc,
        0x21782bf5e275c714,
        0x971f4a3206605bec,
    ];
    assert!(ecdsa_verify_secp256r1(&pk124, &z124, &r124, &s124));

    // 121] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #129: smallish r and s^-1
    let z125 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r125 = [0x002d9b4d347952d6, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s125 = [0xbadd195a0ffe6d7a, 0x05ee1c87ff907bee, 0xb3974497710ab115, 0xef3043e7329581db];
    let pk125 = [
        0xccdebbb8054ce05f,
        0xb96ce83b7a254f71,
        0x80ef9e228140f9d9,
        0x4838b2be35a6276a,
        0x1fcb8aac738a6c6b,
        0x43bd660a82881405,
        0xe00238198d040690,
        0xfa9cbc123c919b19,
    ];
    assert!(ecdsa_verify_secp256r1(&pk125, &z125, &r125, &s125));

    // 122] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #130: 100-bit r and small s^-1
    let z126 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r126 = [0xb32b445580bf4eff, 0x0000001033e67e37, 0x0000000000000000, 0x0000000000000000];
    let s126 = [0xd87129b8e91d1b4d, 0x66e769ad4a16d3dc, 0x8b748b748b748b74, 0x8b748b7400000000];
    let pk126 = [
        0x71119aa4e74b0f64,
        0xab444ef520c0a8e7,
        0xbc4783dc9960746a,
        0x7393983ca30a520b,
        0xe2c6cd27b1857526,
        0xbaf32793afccf774,
        0x26e709863e6a486d,
        0xe9d7be1ab01a0bf6,
    ];
    assert!(ecdsa_verify_secp256r1(&pk126, &z126, &r126, &s126));

    // 123] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #131: small r and 100 bit s^-1
    let z127 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r127 = [0x0000000000000100, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s127 = [0xec3270fc4b81ef5b, 0xbe3cf9cb824a879f, 0x3178fa20b4aaad83, 0xef9f6ba4d97c09d0];
    let pk127 = [
        0x51b8a502d5dfcdc5,
        0x50588a05477e3088,
        0x697379f356a937f3,
        0x5ac331a1103fe966,
        0x02e65d408c871c0b,
        0x65204cfe03be995a,
        0x2b8da095bf6d7942,
        0xfe9993df4b57939b,
    ];
    assert!(ecdsa_verify_secp256r1(&pk127, &z127, &r127, &s127));

    // 124] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #132: 100-bit r and s^-1
    let z128 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r128 = [0xecbe7c39e93e7c25, 0x000000062522bbd3, 0x0000000000000000, 0x0000000000000000];
    let s128 = [0xec3270fc4b81ef5b, 0xbe3cf9cb824a879f, 0x3178fa20b4aaad83, 0xef9f6ba4d97c09d0];
    let pk128 = [
        0x8e5eae5767c41509,
        0xc458d926e27bb8e5,
        0x095a399d3904c74c,
        0x1d209be8de2de877,
        0x60a4f2c9d040d8c9,
        0xa6860e80163f38cc,
        0xdce351fc2a549893,
        0xdd59e04c214f7b18,
    ];
    assert!(ecdsa_verify_secp256r1(&pk128, &z128, &r128, &s128));

    // 125] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #133: r and s^-1 are close to n
    let z129 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r129 = [0xf3b9cac2fc6324d5, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s129 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let pk129 = [
        0xbecee0c133b10e99,
        0x392cef0633a1b8fa,
        0x3acaafa2fcb41349,
        0x083539fbee44625e,
        0x41d4d7616337911e,
        0xe2a402f26326bb7d,
        0x535196770a58047a,
        0x915c1ebe7bf00df8,
    ];
    assert!(ecdsa_verify_secp256r1(&pk129, &z129, &r129, &s129));

    // 126] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #134: s == 1
    let z130 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r130 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let s130 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk130 = [
        0xe1eb3d0e19373874,
        0x6a26f399e2d9734d,
        0x4abdea37390c0c1d,
        0x8aeb368a7027a4d6,
        0x3b19bf7a4adf576d,
        0x1b6691c7f7536aef,
        0xae9b875cf07bd55e,
        0x05bd13834715e1db,
    ];
    assert!(ecdsa_verify_secp256r1(&pk130, &z130, &r130, &s130));

    // 127] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #135: s == 0
    let z131 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r131 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let s131 = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk131 = [
        0xe1eb3d0e19373874,
        0x6a26f399e2d9734d,
        0x4abdea37390c0c1d,
        0x8aeb368a7027a4d6,
        0x3b19bf7a4adf576d,
        0x1b6691c7f7536aef,
        0xae9b875cf07bd55e,
        0x05bd13834715e1db,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk131, &z131, &r131, &s131));

    // 128] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #136: point at infinity during verify
    let z132 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r132 = [0x79dce5617e3192a8, 0xde737d56d38bcf42, 0x7fffffffffffffff, 0x7fffffff80000000];
    let s132 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let pk132 = [
        0x60e8ec07dd70f287,
        0x7e2c88fa0239e23f,
        0xe07757e55e6e516f,
        0xb533d4695dd5b8c5,
        0xe29d4eaf009afe47,
        0x881f7d4a39850143,
        0x8456863f33c3a85d,
        0x1b134ee58cc58327,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk132, &z132, &r132, &s132));

    // 129] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #137: edge case for signature malleability
    let z133 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r133 = [0x79dce5617e3192a9, 0xde737d56d38bcf42, 0x7fffffffffffffff, 0x7fffffff80000000];
    let s133 = [0x79dce5617e3192a8, 0xde737d56d38bcf42, 0x7fffffffffffffff, 0x7fffffff80000000];
    let pk133 = [
        0x628c8b4536787b86,
        0x8cbf2c57f9e284de,
        0xd14e1323523bc3aa,
        0xf50d371b91bfb1d7,
        0xb14cbb209f5fa2dd,
        0x1c553c9730405380,
        0x247cd2e7d0c8b129,
        0xf94ad887ac94d527,
    ];
    assert!(ecdsa_verify_secp256r1(&pk133, &z133, &r133, &s133));

    // 130] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #138: edge case for signature malleability
    let z134 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r134 = [0x79dce5617e3192a9, 0xde737d56d38bcf42, 0x7fffffffffffffff, 0x7fffffff80000000];
    let s134 = [0x79dce5617e3192a9, 0xde737d56d38bcf42, 0x7fffffffffffffff, 0x7fffffff80000000];
    let pk134 = [
        0x2eaeb0d857c4d946,
        0x7047c221bafc3a58,
        0x39156ce57a14b04a,
        0x68ec6e298eafe165,
        0x5bb385ac8ca6fb30,
        0x698ed16c426a2733,
        0xfdb39b2324f220a5,
        0x97bed1af17850117,
    ];
    assert!(ecdsa_verify_secp256r1(&pk134, &z134, &r134, &s134));

    // 131] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #139: u1 == 1
    let z135 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r135 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let s135 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let pk135 = [
        0x97bdf6557927c8b8,
        0xb781a0f1b08f6c88,
        0x0fece94019265fef,
        0x69da0364734d2e53,
        0x8b20f71e2a847002,
        0xa933d86ef8abbcce,
        0x3d726960f069ad71,
        0x66d2d3c7dcd518b2,
    ];
    assert!(ecdsa_verify_secp256r1(&pk135, &z135, &r135, &s135));

    // 132] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #140: u1 == n - 1
    let z136 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r136 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let s136 = [0xec15b0c43202d52e, 0xbcb012ea7bf091fc, 0x12bc9e0a6bdd5e1c, 0x44a5ad0ad0636d9f];
    let pk136 = [
        0x87bf067a1ac1ff32,
        0x1a471e2b23206201,
        0x2576e2b63e3e3062,
        0xd8adc00023a8edc0,
        0x32861576ba2362e1,
        0xa09a86b4ea9690aa,
        0xcb36131fff95ed12,
        0x33e2b50ec09807ac,
    ];
    assert!(ecdsa_verify_secp256r1(&pk136, &z136, &r136, &s136));

    // 133] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #141: u2 == 1
    let z137 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r137 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let s137 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let pk137 = [
        0xab3690dbe75ab785,
        0xedca02cfc7b2401f,
        0xfa6d882f03a7d5c7,
        0x3623ac973ced0a56,
        0x2e9bb3252be7f8fe,
        0x3da8e713ba0643b9,
        0x3da7257e737f3979,
        0x8db06908e64b2861,
    ];
    assert!(ecdsa_verify_secp256r1(&pk137, &z137, &r137, &s137));

    // 134] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #142: u2 == n - 1
    let z138 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r138 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let s138 = [0x4d26872ca84218e1, 0x7def51c91a0fbf03, 0xaaaaaaaaaaaaaaaa, 0xaaaaaaaa00000000];
    let pk138 = [
        0x90e5e04263f922f1,
        0x7b31959503b6fa38,
        0xd894b93ff52dc302,
        0xcf04ea77e9622523,
        0x1199bedeaecab2e9,
        0x1740c2f397543882,
        0x3c8b8400e57b4ed7,
        0xe8528fb7c006b398,
    ];
    assert!(ecdsa_verify_secp256r1(&pk138, &z138, &r138, &s138));

    // 135] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #143: edge case for u1
    let z139 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r139 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s139 = [0xfa5d3a8196623397, 0x7e019f0a28721885, 0xa46bcb51dc0b8b4b, 0xe91e1ba60fdedb76];
    let pk139 = [
        0x3e9f78dbeff77350,
        0xe683d49227996bda,
        0x929dc24077b508d7,
        0xdb7a2c8a1ab573e5,
        0x36eaf08a6c99a206,
        0x30cf7cc76a82f11a,
        0xc2e0aadd5a133117,
        0x4f417f3bc9a88075,
    ];
    assert!(ecdsa_verify_secp256r1(&pk139, &z139, &r139, &s139));

    // 136] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #144: edge case for u1
    let z140 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r140 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s140 = [0xc87b59b95b430ad9, 0x24f799e525b1e8e8, 0x94313ba4831b53fe, 0xfdea5843ffeb73af];
    let pk140 = [
        0xb413765ea80b6e1f,
        0xeff994efe9bbd05a,
        0x2f21974dc4752fad,
        0xdead11c7a5b39686,
        0x07aa0318fc7fe1ff,
        0xbb94078a343736df,
        0xcf89cff53c40e265,
        0x1de3f0640e8ac6ed,
    ];
    assert!(ecdsa_verify_secp256r1(&pk140, &z140, &r140, &s140));

    // 137] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #145: edge case for u1
    let z141 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r141 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s141 = [0x3c3dfc5e5bafc035, 0x994e41c5251cd73b, 0x65190db1680d62bb, 0x03ffcabf2f1b4d2a];
    let pk141 = [
        0xe80e00dfde67c7e9,
        0xbb1fea6f994326fb,
        0xaed3a6ef96c18613,
        0xd0bc472e0d7c81eb,
        0x667d1bb9fa619efd,
        0x3ad70ff17ba85335,
        0x389b946f64ad56c8,
        0x986c723ea4843d48,
    ];
    assert!(ecdsa_verify_secp256r1(&pk141, &z141, &r141, &s141));

    // 138] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #146: edge case for u1
    let z142 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r142 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s142 = [0x2847f74977534989, 0xfe4c1a88ae648e0d, 0x4b33dfdb17d0fed0, 0x4dfbc401f971cd30];
    let pk142 = [
        0x78495d458dd51c32,
        0xb2ad03776e02640f,
        0xcb736008b9c08d1a,
        0xa0a44ca947d66a2a,
        0x30a2392e40426add,
        0x294a4762420df43a,
        0x1f1c409dc2d872d4,
        0x6337fe5cf8c4604b,
    ];
    assert!(ecdsa_verify_secp256r1(&pk142, &z142, &r142, &s142));

    // 139] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #147: edge case for u1
    let z143 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r143 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s143 = [0x4971eba9cda5ca71, 0x988977055cd3a8e5, 0x3dfdb17d0fed112b, 0xbc4024761cd2ffd4];
    let pk143 = [
        0xd42b62c3ce8a96b7,
        0x298c25420b775019,
        0x5fb65fad0f602389,
        0xc9c2115290d008b4,
        0xcc3f06e9713973fd,
        0xc9dbefac46f9e601,
        0xd987ca730f0405c2,
        0x3877d25a8080dc02,
    ];
    assert!(ecdsa_verify_secp256r1(&pk143, &z143, &r143, &s143));

    // 140] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #148: edge case for u1
    let z144 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r144 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s144 = [0x9f2a0c909ee86f91, 0x742bf35d128fb345, 0x7bfb62fa1fda2257, 0x788048ed39a5ffa7];
    let pk144 = [
        0xfa83bc1a5ff6033e,
        0x4c0018962f3c5e7e,
        0x66b8bccf1b88e8a2,
        0x5eca1ef4c287dddc,
        0xecaef22f1c934a71,
        0x8d92a607c32cd407,
        0x45abdce8a8e4da75,
        0x5e79c4cb2c245b8c,
    ];
    assert!(ecdsa_verify_secp256r1(&pk144, &z144, &r144, &s144));

    // 141] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #149: edge case for u1
    let z145 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r145 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s145 = [0xdd8b23582b3cb15e, 0x5924b5ed5b11167e, 0x17d0fed112bc9e0a, 0x476d9131fd381bd9];
    let pk145 = [
        0x2d473e317029a47a,
        0x0a01e4130c3f8bf2,
        0x936bc7ab5a96353e,
        0x5caaa030e7fdf0e4,
        0xda926b42b178bef9,
        0xe9b201642005b3ce,
        0x2a20d371e9702254,
        0xdeb6adc462f7058f,
    ];
    assert!(ecdsa_verify_secp256r1(&pk145, &z145, &r145, &s145));

    // 142] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #150: edge case for u1
    let z146 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r146 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s146 = [0xf6cc0a19662d3601, 0xfafa8b19ce78d538, 0x4448d0a8f640fe46, 0x8374253e3e21bd15];
    let pk146 = [
        0x469b1a31f619b098,
        0xf83a1fc3501c8a66,
        0xb8ac0ce69eb1ea20,
        0xc2fd20bac06e555b,
        0xffc87ac397e6cbaf,
        0xca2ed32525c75f27,
        0x5bd7b8d76a25fc95,
        0x6237050779f52b61,
    ];
    assert!(ecdsa_verify_secp256r1(&pk146, &z146, &r146, &s146));

    // 143] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #151: edge case for u1
    let z147 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r147 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s147 = [0x49ae6a2d897a52d6, 0x2c11ee7fe14879e7, 0x3c5b9ede36cba545, 0x357cfd3be4d01d41];
    let pk147 = [
        0x17bedae4bba86ced,
        0x8426e11ea6ae78ce,
        0x0bbe726c37201006,
        0x3fd6a1ca7f77fb3b,
        0xddfc56e0db3c8ff4,
        0x18ad6f50b5461872,
        0xaab8745eac1cd690,
        0x03ce5516406bf8cf,
    ];
    assert!(ecdsa_verify_secp256r1(&pk147, &z147, &r147, &s147));

    // 144] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #152: edge case for u1
    let z148 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r148 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s148 = [0x7cd2f2bc27a0a6d8, 0xdf5225298e6ffc80, 0xa5e8e6b799fd86b8, 0x29798c5c0ee287d4];
    let pk148 = [
        0xe1edf7b086911114,
        0x4989db20e9bca3ed,
        0x624a60d6dc32734e,
        0x9cb8e51e27a5ae3b,
        0x5fc57322b4427544,
        0x410a19f2e277aa89,
        0x36d6556e8ad5f523,
        0xb4c104ab3c677e4b,
    ];
    assert!(ecdsa_verify_secp256r1(&pk148, &z148, &r148, &s148));

    // 145] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #153: edge case for u1
    let z149 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r149 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s149 = [0x7cae4820b30078dd, 0x1f72add1bf52c2ff, 0x2dca1a5711fa3a5a, 0x0b70f22c78109245];
    let pk149 = [
        0xc262512d8f49602a,
        0xbc78ef3d569e1223,
        0x02620b7955bc2b40,
        0xa3e52c156dcaf105,
        0xf192944977df147f,
        0x032355463486164c,
        0x4ad3cc86e57321de,
        0x4a2039f31c109702,
    ];
    assert!(ecdsa_verify_secp256r1(&pk149, &z149, &r149, &s149));

    // 146] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #154: edge case for u1
    let z150 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r150 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s150 = [0xf95c90416600f1ba, 0x3ee55ba37ea585fe, 0x5b9434ae23f474b4, 0x16e1e458f021248a];
    let pk150 = [
        0xc3f3c059b2655e88,
        0x5c37bf91b58a5157,
        0xe8e670fb90010fb1,
        0xf19b78928720d5be,
        0x074abd4329260509,
        0x468560c7cfeb942d,
        0xdcf273f5dc357e58,
        0xcf701ec962fb4a11,
    ];
    assert!(ecdsa_verify_secp256r1(&pk150, &z150, &r150, &s150));

    // 147] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #155: edge case for u1
    let z151 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r151 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s151 = [0x760ad86219016a97, 0x5e5809753df848fe, 0x895e4f0535eeaf0e, 0x2252d6856831b6cf];
    let pk151 = [
        0x4cb89345545c90a8,
        0x37482d242f235d7b,
        0xa5cf52b27a05bb73,
        0x83a744459ecdfb01,
        0x28121f37cc50de6e,
        0xd905df5f3c329458,
        0x3287de9ffe90355f,
        0xc05d49337b964981,
    ];
    assert!(ecdsa_verify_secp256r1(&pk151, &z151, &r151, &s151));

    // 148] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #156: edge case for u1
    let z152 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r152 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s152 = [0x17fbe390ac0972c3, 0xab1a9e39661a3ae0, 0xb28c86d8b406b15d, 0x81ffe55f178da695];
    let pk152 = [
        0x8ae51e5d6f3a21d7,
        0x4b19bbe88cee8e52,
        0xdae124f039dfd23f,
        0xdd13c6b34c56982d,
        0xbdae4bd3b42a45ff,
        0xe4c3345692fb5320,
        0xeb59ca974d039fc0,
        0xbfad4c2e6f263fe5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk152, &z152, &r152, &s152));

    // 149] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #157: edge case for u2
    let z153 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r153 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s153 = [0x513dee40fecbb71a, 0xe9a2538f37b28a2c, 0xffffffffffffffff, 0x7fffffffaaaaaaaa];
    let pk153 = [
        0xeed01b0f3deb7460,
        0xfad636bbf95192fe,
        0x2f65f094e94e5b4d,
        0x67e6f659cdde869a,
        0x3c62886437c38ba0,
        0x85bbe58712c8d923,
        0xb51dfe592f5cfd56,
        0xa37e0a51f258b7ae,
    ];
    assert!(ecdsa_verify_secp256r1(&pk153, &z153, &r153, &s153));

    // 150] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #158: edge case for u2
    let z154 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r154 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s154 = [0xde009e526adf21f2, 0x3ab3cccd0459b201, 0x6de86d42ad8a13da, 0xb62f26b5f2a2b26f];
    let pk154 = [
        0x9617bb367f9ecaaf,
        0x90d05511e8ec1f59,
        0x6545f029932087e4,
        0x2eb6412505aec05c,
        0xf6669af292895cb0,
        0x6a43fedcddb31830,
        0x3f9b1ae0124890f0,
        0x805f51efcc480340,
    ];
    assert!(ecdsa_verify_secp256r1(&pk154, &z154, &r154, &s154));

    // 151] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #159: edge case for u2
    let z155 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r155 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s155 = [0x0686aa7b4c90851e, 0xd57b38bb61403d70, 0xd02bbbe749bd351c, 0xbb1d9ac949dd748c];
    let pk155 = [
        0x0a854625fe0d7f35,
        0x5435e3a6b68d75a5,
        0x3a9fd80e056e2e85,
        0x84db645868eab35e,
        0xbf43a2ee39338cfe,
        0xbf92e72171570ef7,
        0x11ef3e075eddda9a,
        0x6d2589ac655edc9a,
    ];
    assert!(ecdsa_verify_secp256r1(&pk155, &z155, &r155, &s155));

    // 152] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #160: edge case for u2
    let z156 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r156 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s156 = [0x818f725b4f60aaf2, 0xe52545dac11f816e, 0x1c732513ca0234ec, 0x66755a00638cdaec];
    let pk156 = [
        0x7eb2975e386ad663,
        0x6aa5059b7a2ff763,
        0xd75c0983b22ca8ea,
        0x91b9e47c56278662,
        0x08302a16854ecfbd,
        0xd13c3c0310679c14,
        0x18d6d11dc062165f,
        0x49aa8ff283d0f77c,
    ];
    assert!(ecdsa_verify_secp256r1(&pk156, &z156, &r156, &s156));

    // 153] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #161: edge case for u2
    let z157 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r157 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s157 = [0x8ca48e982beb3669, 0xe98ebe492fdf02e4, 0x32513ca0234ecfff, 0x55a00c9fcdaebb60];
    let pk157 = [
        0x27f7ec5ee8e4834d,
        0xd4dc6b0a9e802e53,
        0x92b47fb4c5311fb6,
        0xf3ec2f13caf04d01,
        0x38ac321fefe5a432,
        0x531df87efdb47c13,
        0x67d6ecfe81e2b0f9,
        0xf97e3e468b7d0db8,
    ];
    assert!(ecdsa_verify_secp256r1(&pk157, &z157, &r157, &s157));

    // 154] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #162: edge case for u2
    let z158 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r158 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s158 = [0x19491d3057d66cd2, 0xd31d7c925fbe05c9, 0x64a27940469d9fff, 0xab40193f9b5d76c0];
    let pk158 = [
        0x3e4693c670fccc88,
        0x3180235b8f46b450,
        0x7dafd9acaf2fa10b,
        0xd92b200aefcab6ac,
        0xdc85b6b8ab922c72,
        0xefb7352d27e4ccca,
        0x75336256768f7c19,
        0x5ef2f3aebf5b3174,
    ];
    assert!(ecdsa_verify_secp256r1(&pk158, &z158, &r158, &s158));

    // 155] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #163: edge case for u2
    let z159 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r159 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s159 = [0xa26b4408d0dc8600, 0xcb0dadbbc7f549f8, 0xca0234ecffffffff, 0xca0234ebb5fdcb13];
    let pk159 = [
        0x140a3bcd881523cd,
        0x96bf179b3d76fc48,
        0x625b38e5f98bbabb,
        0x0a88361eb92ecca2,
        0x6edebf47298ad489,
        0x6aa2c96b86a41ccf,
        0x54035597375d9086,
        0xe6bdf56033f84a50,
    ];
    assert!(ecdsa_verify_secp256r1(&pk159, &z159, &r159, &s159));

    // 156] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #164: edge case for u2
    let z160 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r160 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s160 = [0x8711c77298815ad3, 0x19933a9e65b28559, 0x082b9310572620ae, 0xbfffffff3ea3677e];
    let pk160 = [
        0x2ba50469d84375e8,
        0x6e2b20e7f14a563a,
        0x7e0c1afc5d8d8036,
        0xd0fb17ccd8fafe82,
        0x1e236a7de7637d93,
        0x9ac602cc6349cf8c,
        0xf554355564646de9,
        0x68612569d39e2bb9,
    ];
    assert!(ecdsa_verify_secp256r1(&pk160, &z160, &r160, &s160));

    // 157] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #165: edge case for u2
    let z161 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r161 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s161 = [0x8f055d86e5cc41f4, 0x5b37902e023fab7c, 0xe666666666666666, 0x266666663bbbbbbb];
    let pk161 = [
        0x2b1e4309d3edb276,
        0xac4181076c9af0a2,
        0x3abbcef0d91f11e2,
        0x836f33bbc1dc0d3d,
        0x36f3a95bbe881f75,
        0xec2b0cb8120d7602,
        0xc773867582997c2b,
        0x9ab443ff6f901e30,
    ];
    assert!(ecdsa_verify_secp256r1(&pk161, &z161, &r161, &s161));

    // 158] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #166: edge case for u2
    let z162 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r162 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s162 = [0x08a443e258970b09, 0x146c573f4c6dfc8d, 0xa492492492492492, 0xbfffffff36db6db7];
    let pk162 = [
        0x5103cb33e55feeb8,
        0x1237034dec8d72ba,
        0x99719baee4b43274,
        0x92f99fbe973ed4a2,
        0x7a794cebd6e69697,
        0x1ac05767289280ee,
        0x174889f3ebcf1b7a,
        0x033dd0e91134c734,
    ];
    assert!(ecdsa_verify_secp256r1(&pk162, &z162, &r162, &s162));

    // 159] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #167: edge case for u2
    let z163 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r163 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s163 = [0xcb1ad3a27cfd49c4, 0xc815d0e60b3e596e, 0x7fffffffffffffff, 0xbfffffff2aaaaaab];
    let pk163 = [
        0x9d130bba434af09e,
        0xd12cffd73ebbb204,
        0x78e618ec0fa7e2e2,
        0xd35ba58da30197d3,
        0x82874c794635c1d2,
        0xc77cbb3c47919f8e,
        0xa432b7585a49b3a6,
        0xff83986e6875e41e,
    ];
    assert!(ecdsa_verify_secp256r1(&pk163, &z163, &r163, &s163));

    // 160] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #168: edge case for u2
    let z164 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r164 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s164 = [0xa27bdc81fd976e37, 0xd344a71e6f651458, 0xffffffffffffffff, 0x7fffffff55555555];
    let pk164 = [
        0xab0725c8d0793224,
        0x36697334a519d7dd,
        0x3f3ff475149be291,
        0x8651ce490f1b46d7,
        0x900bd825f590cc28,
        0x51ce21dd9003ae60,
        0xbc9ae82911f0b527,
        0xe11c65bd8ca92dc8,
    ];
    assert!(ecdsa_verify_secp256r1(&pk164, &z164, &r164, &s164));

    // 161] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #169: edge case for u2
    let z165 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r165 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s165 = [0x79dce5617e3192aa, 0xde737d56d38bcf42, 0x7fffffffffffffff, 0x3fffffff80000000];
    let pk165 = [
        0xcdaca9826b9cfc6d,
        0xd921d9e2f72b15b1,
        0x8795650ff95f101e,
        0x6d8e1b12c831a0da,
        0xa58c106ad486bf37,
        0xe6c7a6a637b20469,
        0x70394a4bc9f892d5,
        0xef6d63e2bc5c0895,
    ];
    assert!(ecdsa_verify_secp256r1(&pk165, &z165, &r165, &s165));

    // 162] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #170: edge case for u2
    let z166 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r166 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s166 = [0x0343553da648428f, 0x6abd9c5db0a01eb8, 0x6815ddf3a4de9a8e, 0x5d8ecd64a4eeba46];
    let pk166 = [
        0xff24cb4d920e1542,
        0xca9a410f627a0f7d,
        0x2997cbdbb0922328,
        0x0ae580bae933b4ef,
        0x8ba83c3949d893e3,
        0x2b99e309d8dcd9a9,
        0x88eb81421a361ccc,
        0x8911e7f8cc365a8a,
    ];
    assert!(ecdsa_verify_secp256r1(&pk166, &z166, &r166, &s166));

    // 163] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #171: point duplication during verification
    let z167 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r167 = [0x05ea6c42c4934569, 0x8c4aacafdfb6bcbe, 0x8fe0555ac3bc9904, 0x6f2347cab7dd7685];
    let s167 = [0x4d53f4301047856b, 0x435109cf9a15dd62, 0x9957a61e76e00c2c, 0xbb726660235793aa];
    let pk167 = [
        0x0e134c027fc46963,
        0x6983b442d2444fe7,
        0x835a849cce6fbdeb,
        0x5b812fd521aafa69,
        0x4e15eba5499249e9,
        0x38550ce672ce8b8d,
        0x004e92d8d940cf56,
        0x838a40f2a36092e9,
    ];
    assert!(ecdsa_verify_secp256r1(&pk167, &z167, &r167, &s167));

    // 164] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #172: duplication bug
    let z168 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r168 = [0x05ea6c42c4934569, 0x8c4aacafdfb6bcbe, 0x8fe0555ac3bc9904, 0x6f2347cab7dd7685];
    let s168 = [0x4d53f4301047856b, 0x435109cf9a15dd62, 0x9957a61e76e00c2c, 0xbb726660235793aa];
    let pk168 = [
        0x0e134c027fc46963,
        0x6983b442d2444fe7,
        0x835a849cce6fbdeb,
        0x5b812fd521aafa69,
        0xb1ea145ab66db616,
        0xc7aaf31a8d317472,
        0xffb16d2726bf30a9,
        0x7c75bf0c5c9f6d17,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk168, &z168, &r168, &s168));

    // 165] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #173: point with x-coordinate 0
    let z169 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r169 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s169 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let pk169 = [
        0x6222c34acfef72a6,
        0xb6da497f09c90317,
        0x319faa0d878665a6,
        0x6adda82b90261b0f,
        0x959a364c62e488d9,
        0xd71a41bf5e1f9df4,
        0x9b59f7602bb222fa,
        0x47e6f50dcc40ad5d,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk169, &z169, &r169, &s169));

    // 166] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #175: comparison with point at infinity
    let z170 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r170 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let s170 = [0x63f1f55a327a3aa9, 0x25c7cbbc549e52e7, 0x3333333333333333, 0x3333333300000000];
    let pk170 = [
        0x1a633db76665d250,
        0x3ff467f11ebd98a5,
        0x11083b78002081c5,
        0xdd86d3b5f4a13e85,
        0x1b7e17474ebc18f7,
        0x8dfaed6ff8d5cb3e,
        0x10d849349226d21d,
        0x45d5c8200c89f2fa,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk170, &z170, &r170, &s170));

    // 167] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #176: extreme value for k and edgecase s
    let z171 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r171 = [0xa60b48fc47669978, 0xc08969e277f21b35, 0x8a52380304b51ac3, 0x7cf27b188d034f7e];
    let s171 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let pk171 = [
        0x6591a93f5a0fbcc5,
        0x4b0f5a516e578c01,
        0x0c12c4cd0abfb4e6,
        0x4fea55b32cb32aca,
        0x85ed3be62ce4b280,
        0xf0fecd38a8a4b2c7,
        0x547b212f6bb14c88,
        0xd7d3fd10b2be668c,
    ];
    assert!(ecdsa_verify_secp256r1(&pk171, &z171, &r171, &s171));

    // 168] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #177: extreme value for k and s^-1
    let z172 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r172 = [0xa60b48fc47669978, 0xc08969e277f21b35, 0x8a52380304b51ac3, 0x7cf27b188d034f7e];
    let s172 = [0x1bcdd9f8fd6b63cc, 0x625bd7a09bec4ca8, 0x4924924924924924, 0xb6db6db624924925];
    let pk172 = [
        0x299802e32d7c3107,
        0xf32b7f98af669ead,
        0x92170a6f8eee735b,
        0xc6a7715270242277,
        0x412f726867db589e,
        0x61fe3a073e2ffd78,
        0xbd343572b3e56192,
        0xbc3b4b5e65ab887b,
    ];
    assert!(ecdsa_verify_secp256r1(&pk172, &z172, &r172, &s172));

    // 169] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #178: extreme value for k and s^-1
    let z173 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r173 = [0xa60b48fc47669978, 0xc08969e277f21b35, 0x8a52380304b51ac3, 0x7cf27b188d034f7e];
    let s173 = [0x8fc7d568c9e8eaa7, 0x971f2ef152794b9d, 0xcccccccccccccccc, 0xcccccccc00000000];
    let pk173 = [
        0x8de85a7d15b956ef,
        0xd6ec6d59b207fec9,
        0x7a9af99f49f03644,
        0x851c2bbad08e54ec,
        0x319f10ddeb0fe9d6,
        0x4b91aa2379f60727,
        0x684b410be8d0f749,
        0xcee9960283045075,
    ];
    assert!(ecdsa_verify_secp256r1(&pk173, &z173, &r173, &s173));

    // 170] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #179: extreme value for k and s^-1
    let z174 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r174 = [0xa60b48fc47669978, 0xc08969e277f21b35, 0x8a52380304b51ac3, 0x7cf27b188d034f7e];
    let s174 = [0x63f1f55a327a3aaa, 0x25c7cbbc549e52e7, 0x3333333333333333, 0x3333333300000000];
    let pk174 = [
        0x3061205acb19c48f,
        0x55911ff68318d1bf,
        0x88676949e53da7fc,
        0xf6417c8a670584e3,
        0x9ead43026ab6d43f,
        0x4779cd9ac916c366,
        0x2674acb750592978,
        0x8f2b743df34ad0f7,
    ];
    assert!(ecdsa_verify_secp256r1(&pk174, &z174, &r174, &s174));

    // 171] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #180: extreme value for k and s^-1
    let z175 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r175 = [0xa60b48fc47669978, 0xc08969e277f21b35, 0x8a52380304b51ac3, 0x7cf27b188d034f7e];
    let s175 = [0xd7ebf0c9fef7c185, 0x5a8b230d0b2b51dc, 0xb6db6db6db6db6db, 0x49249248db6db6db];
    let pk175 = [
        0x3f557faa7f8a0643,
        0x032565af420cf337,
        0xefec6c639930d636,
        0x501421277be45a5e,
        0x89cad195d0aa1371,
        0xac08d74501f2ae6e,
        0xcdc7dfe7384c8e5c,
        0x8673d6cb6076e1cf,
    ];
    assert!(ecdsa_verify_secp256r1(&pk175, &z175, &r175, &s175));

    // 172] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #181: extreme value for k
    let z176 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r176 = [0xa60b48fc47669978, 0xc08969e277f21b35, 0x8a52380304b51ac3, 0x7cf27b188d034f7e];
    let s176 = [0x6e0478c34416e3bb, 0x1584d13e18411e2f, 0xc82cbc9d1edd8c98, 0x16a4502e2781e11a];
    let pk176 = [
        0x3415ac84e808bb34,
        0x23ee01a4894adf0e,
        0x27735f729ca8a4ca,
        0x0d935bf9ffc115a5,
        0x50ce61d82eba33c5,
        0x70c3050893a43758,
        0x38912bd9ea6c4fde,
        0x3195a3762fea29ed,
    ];
    assert!(ecdsa_verify_secp256r1(&pk176, &z176, &r176, &s176));

    // 173] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #182: extreme value for k and edgecase s
    let z177 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r177 = [0xf4a13945d898c296, 0x77037d812deb33a0, 0xf8bce6e563a440f2, 0x6b17d1f2e12c4247];
    let s177 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let pk177 = [
        0x41e748e64e4dca21,
        0xb668fb670196206c,
        0xa589355014308e60,
        0x5e59f50708646be8,
        0xef38e213624a01de,
        0xeeeafbdf03aacbaf,
        0x7144d5b459982f52,
        0x5de37fee5c97bcaf,
    ];
    assert!(ecdsa_verify_secp256r1(&pk177, &z177, &r177, &s177));

    // 174] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #183: extreme value for k and s^-1
    let z178 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r178 = [0xf4a13945d898c296, 0x77037d812deb33a0, 0xf8bce6e563a440f2, 0x6b17d1f2e12c4247];
    let s178 = [0x1bcdd9f8fd6b63cc, 0x625bd7a09bec4ca8, 0x4924924924924924, 0xb6db6db624924925];
    let pk178 = [
        0xbfe924104b02db8e,
        0x2fd6226f7ef90ef0,
        0xff2f7a5b5445da9e,
        0x169fb797325843fa,
        0xb861b131d8a1d667,
        0x46d581d68878efb2,
        0xcf9b22f7a2e582bd,
        0x7bbb8de662c7b9b1,
    ];
    assert!(ecdsa_verify_secp256r1(&pk178, &z178, &r178, &s178));

    // 175] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #184: extreme value for k and s^-1
    let z179 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r179 = [0xf4a13945d898c296, 0x77037d812deb33a0, 0xf8bce6e563a440f2, 0x6b17d1f2e12c4247];
    let s179 = [0x8fc7d568c9e8eaa7, 0x971f2ef152794b9d, 0xcccccccccccccccc, 0xcccccccc00000000];
    let pk179 = [
        0xf8b7b54898148754,
        0xef2f7023d18affda,
        0x6b62d4e9e4ca885a,
        0x271cd89c00014309,
        0x02b2ca47fe8e4da5,
        0x81a609b9149ccb4b,
        0x35b55fa385b0f764,
        0x0a1c6e954e321084,
    ];
    assert!(ecdsa_verify_secp256r1(&pk179, &z179, &r179, &s179));

    // 176] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #185: extreme value for k and s^-1
    let z180 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r180 = [0xf4a13945d898c296, 0x77037d812deb33a0, 0xf8bce6e563a440f2, 0x6b17d1f2e12c4247];
    let s180 = [0x63f1f55a327a3aaa, 0x25c7cbbc549e52e7, 0x3333333333333333, 0x3333333300000000];
    let pk180 = [
        0x587a220afe499c12,
        0xb1563a9ab84bf524,
        0x7ddb46ebc1ed799a,
        0x3d0bc7ed8f09d2cb,
        0x4f78cb216fa3f8df,
        0xabf19ce7d68aa624,
        0x4f378d96adb0a408,
        0xe22dc3b3c103824a,
    ];
    assert!(ecdsa_verify_secp256r1(&pk180, &z180, &r180, &s180));

    // 177] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #186: extreme value for k and s^-1
    let z181 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r181 = [0xf4a13945d898c296, 0x77037d812deb33a0, 0xf8bce6e563a440f2, 0x6b17d1f2e12c4247];
    let s181 = [0xd7ebf0c9fef7c185, 0x5a8b230d0b2b51dc, 0xb6db6db6db6db6db, 0x49249248db6db6db];
    let pk181 = [
        0x721bcbd23663a9b7,
        0xb281797fa701288c,
        0xf9bb010d066974ab,
        0xa6c885ade1a4c566,
        0xf2058458f98af316,
        0x04a9c7d467e007e1,
        0x193a6096fc77a2b0,
        0x2e424b690957168d,
    ];
    assert!(ecdsa_verify_secp256r1(&pk181, &z181, &r181, &s181));

    // 178] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #187: extreme value for k
    let z182 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r182 = [0xf4a13945d898c296, 0x77037d812deb33a0, 0xf8bce6e563a440f2, 0x6b17d1f2e12c4247];
    let s182 = [0x6e0478c34416e3bb, 0x1584d13e18411e2f, 0xc82cbc9d1edd8c98, 0x16a4502e2781e11a];
    let pk182 = [
        0x30c3bdbad9ebfa5c,
        0x5bf75df62d87ab73,
        0x289e6ac3812572a2,
        0x8d3c2c2c3b765ba8,
        0x632e0b780c423f5d,
        0xcaa1621d1af241d4,
        0x238578d43aec54f7,
        0x4c6845442d66935b,
    ];
    assert!(ecdsa_verify_secp256r1(&pk182, &z182, &r182, &s182));

    // 179] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #188: testing point duplication
    let z183 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r183 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let s183 = [0x6bf5f864ff7be0c2, 0xad4591868595a8ee, 0xdb6db6db6db6db6d, 0x249249246db6db6d];
    let pk183 = [
        0xf4a13945d898c296,
        0x77037d812deb33a0,
        0xf8bce6e563a440f2,
        0x6b17d1f2e12c4247,
        0xcbb6406837bf51f5,
        0x2bce33576b315ece,
        0x8ee7eb4a7c0f9e16,
        0x4fe342e2fe1a7f9b,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk183, &z183, &r183, &s183));

    // 180] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #189: testing point duplication
    let z184 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r184 = [0xec15b0c43202d52e, 0xbcb012ea7bf091fc, 0x12bc9e0a6bdd5e1c, 0x44a5ad0ad0636d9f];
    let s184 = [0x6bf5f864ff7be0c2, 0xad4591868595a8ee, 0xdb6db6db6db6db6d, 0x249249246db6db6d];
    let pk184 = [
        0xf4a13945d898c296,
        0x77037d812deb33a0,
        0xf8bce6e563a440f2,
        0x6b17d1f2e12c4247,
        0xcbb6406837bf51f5,
        0x2bce33576b315ece,
        0x8ee7eb4a7c0f9e16,
        0x4fe342e2fe1a7f9b,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk184, &z184, &r184, &s184));

    // 181] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #190: testing point duplication
    let z185 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r185 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let s185 = [0x6bf5f864ff7be0c2, 0xad4591868595a8ee, 0xdb6db6db6db6db6d, 0x249249246db6db6d];
    let pk185 = [
        0xf4a13945d898c296,
        0x77037d812deb33a0,
        0xf8bce6e563a440f2,
        0x6b17d1f2e12c4247,
        0x3449bf97c840ae0a,
        0xd431cca994cea131,
        0x711814b583f061e9,
        0xb01cbd1c01e58065,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk185, &z185, &r185, &s185));

    // 182] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #191: testing point duplication
    let z186 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r186 = [0xec15b0c43202d52e, 0xbcb012ea7bf091fc, 0x12bc9e0a6bdd5e1c, 0x44a5ad0ad0636d9f];
    let s186 = [0x6bf5f864ff7be0c2, 0xad4591868595a8ee, 0xdb6db6db6db6db6d, 0x249249246db6db6d];
    let pk186 = [
        0xf4a13945d898c296,
        0x77037d812deb33a0,
        0xf8bce6e563a440f2,
        0x6b17d1f2e12c4247,
        0x3449bf97c840ae0a,
        0xd431cca994cea131,
        0x711814b583f061e9,
        0xb01cbd1c01e58065,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk186, &z186, &r186, &s186));

    // 183] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #192: pseudorandom signature
    let z187 = [0xa495991b7852b855, 0x27ae41e4649b934c, 0x9afbf4c8996fb924, 0xe3b0c44298fc1c14];
    let r187 = [0x8e76e09d8770b34a, 0x42d16e47f219f9e9, 0x7a305c951c0dcbcc, 0xb292a619339f6e56];
    let s187 = [0xab2abebdf89a62e2, 0xe59ec2a17ce5bd2d, 0x2f76f07bfe3661bd, 0x0177e60492c5a824];
    let pk187 = [
        0x5b522eba7240fad5,
        0x32e41495a944d004,
        0x13fb8a9e64da3b86,
        0x04aaec73635726f2,
        0x1aa546e8365d525d,
        0xaaf7b4e09fc81d6d,
        0xba01775787ced05e,
        0x87d9315798aaa3a5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk187, &z187, &r187, &s187));

    // 184] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #193: pseudorandom signature
    let z188 = [0xf59ec9dd1bb8c7b3, 0xe807bdf4c5332f19, 0x2856e7be399007c9, 0xdc1921946f4af96a];
    let r188 = [0xf5b8d2a2a6538e23, 0xcfbf33afe66dbadc, 0xba897f6b5fb59695, 0x530bd6b0c9af2d69];
    let s188 = [0xe934c72caa3f43e9, 0x0987e3e3f0f242ca, 0x55ededcedbf4cc0c, 0xd85e489cb7a161fd];
    let pk188 = [
        0x5b522eba7240fad5,
        0x32e41495a944d004,
        0x13fb8a9e64da3b86,
        0x04aaec73635726f2,
        0x1aa546e8365d525d,
        0xaaf7b4e09fc81d6d,
        0xba01775787ced05e,
        0x87d9315798aaa3a5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk188, &z188, &r188, &s188));

    // 185] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #194: pseudorandom signature
    let z189 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r189 = [0x73d7904519e51388, 0x2711f9917060406a, 0x381c4c1f1da8e9de, 0xa8ea150cb80125d7];
    let s189 = [0x7288293285449b86, 0x0c22c9d76ec21725, 0xa73b2d40480c2ba5, 0xf3ab9fa68bd47973];
    let pk189 = [
        0x5b522eba7240fad5,
        0x32e41495a944d004,
        0x13fb8a9e64da3b86,
        0x04aaec73635726f2,
        0x1aa546e8365d525d,
        0xaaf7b4e09fc81d6d,
        0xba01775787ced05e,
        0x87d9315798aaa3a5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk189, &z189, &r189, &s189));

    // 186] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #195: pseudorandom signature
    let z190 = [0x7f1b40c4cbd36f90, 0x93262cf06340c4fa, 0xdbb5f2c353e632c3, 0xde47c9b27eb8d300];
    let r190 = [0xc69178490d57fb71, 0x39aaf63f00a91f29, 0xe5aada139f52b705, 0x986e65933ef2ed4e];
    let s190 = [0x0f701aaa7a694b9c, 0xdabf0c0217d1c0ff, 0x372308cbf1489bbb, 0x3dafedfb8da6189d];
    let pk190 = [
        0x5b522eba7240fad5,
        0x32e41495a944d004,
        0x13fb8a9e64da3b86,
        0x04aaec73635726f2,
        0x1aa546e8365d525d,
        0xaaf7b4e09fc81d6d,
        0xba01775787ced05e,
        0x87d9315798aaa3a5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk190, &z190, &r190, &s190));

    // 187] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #196: x-coordinate of the public key has many trailing 0's
    let z191 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r191 = [0x7f29745eff3569f1, 0x50dd0fd5defa013c, 0x81e353a3565e4825, 0xd434e262a49eab77];
    let s191 = [0x844218305c6ba17a, 0x98953195d7bc10de, 0x52fd8077be769c2b, 0x9b0c0a93f267fb60];
    let pk191 = [
        0x9fbd816900000000,
        0xf3807eca11738023,
        0x05e4f1600ae2849d,
        0x4f337ccfd67726a8,
        0xd5df7ea60666d685,
        0x7eb504af43a3146c,
        0x416411e988c30f42,
        0xed9dea124cc8c396,
    ];
    assert!(ecdsa_verify_secp256r1(&pk191, &z191, &r191, &s191));

    // 188] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #197: x-coordinate of the public key has many trailing 0's
    let z192 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r192 = [0xadd0be9b1979110b, 0x1463489221bf0a33, 0xf76d79fd7a772e42, 0x0fe774355c04d060];
    let s192 = [0xac6181175df55737, 0x4ca8b91a1f325f3f, 0x43fa4f57f743ce12, 0x500dcba1c69a8fbd];
    let pk192 = [
        0x9fbd816900000000,
        0xf3807eca11738023,
        0x05e4f1600ae2849d,
        0x4f337ccfd67726a8,
        0xd5df7ea60666d685,
        0x7eb504af43a3146c,
        0x416411e988c30f42,
        0xed9dea124cc8c396,
    ];
    assert!(ecdsa_verify_secp256r1(&pk192, &z192, &r192, &s192));

    // 189] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #198: x-coordinate of the public key has many trailing 0's
    let z193 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r193 = [0xbfd06595ee1135e3, 0x8e3b2cd79693f125, 0x950c7d39f03d36dc, 0xbb40bf217bed3fb3];
    let s193 = [0xfa4780745bb55677, 0xc89a1e291ac692b3, 0x32710bdb6a1bf1bf, 0x541bf3532351ebb0];
    let pk193 = [
        0x9fbd816900000000,
        0xf3807eca11738023,
        0x05e4f1600ae2849d,
        0x4f337ccfd67726a8,
        0xd5df7ea60666d685,
        0x7eb504af43a3146c,
        0x416411e988c30f42,
        0xed9dea124cc8c396,
    ];
    assert!(ecdsa_verify_secp256r1(&pk193, &z193, &r193, &s193));

    // 190] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #199: y-coordinate of the public key has many trailing 0's
    let z194 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r194 = [0x556d3e75a233e73a, 0x05badd5ca99231ff, 0xdf3c86ea31389a54, 0x664eb7ee6db84a34];
    let s194 = [0x2e51a2901426a1bd, 0xe0badc678754b8f7, 0x137642490a51560c, 0x59f3c752e52eca46];
    let pk194 = [
        0x5c3f497265004935,
        0x618f06b8ff87e801,
        0xd499a07873fac281,
        0x3cf03d614d8939cf,
        0xe59cdbde80000000,
        0x7c7a1338a82f85a9,
        0x2ce3880a8960dd2a,
        0x84fa174d791c72bf,
    ];
    assert!(ecdsa_verify_secp256r1(&pk194, &z194, &r194, &s194));

    // 191] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #200: y-coordinate of the public key has many trailing 0's
    let z195 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r195 = [0x01985a79d1fd8b43, 0x9c3e42e2d1631fd0, 0x009d6fcd843d4ce3, 0x4cd0429bbabd2827];
    let s195 = [0xe466189d2acdabe3, 0xb7bca77a1a2b869a, 0xbe7ef1d0e0d98f08, 0x9638bf12dd682f60];
    let pk195 = [
        0x5c3f497265004935,
        0x618f06b8ff87e801,
        0xd499a07873fac281,
        0x3cf03d614d8939cf,
        0xe59cdbde80000000,
        0x7c7a1338a82f85a9,
        0x2ce3880a8960dd2a,
        0x84fa174d791c72bf,
    ];
    assert!(ecdsa_verify_secp256r1(&pk195, &z195, &r195, &s195));

    // 192] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #201: y-coordinate of the public key has many trailing 0's
    let z196 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r196 = [0x1e8added97c56c04, 0x60e3ce9aed5e5fd4, 0x1c44d8b6cb62b9f4, 0xe56c6ea2d1b01709];
    let s196 = [0x7fc1378180f89b55, 0x4fcf2b8025807820, 0xbe20b457e463440b, 0xa308ec31f281e955];
    let pk196 = [
        0x5c3f497265004935,
        0x618f06b8ff87e801,
        0xd499a07873fac281,
        0x3cf03d614d8939cf,
        0xe59cdbde80000000,
        0x7c7a1338a82f85a9,
        0x2ce3880a8960dd2a,
        0x84fa174d791c72bf,
    ];
    assert!(ecdsa_verify_secp256r1(&pk196, &z196, &r196, &s196));

    // 193] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #202: y-coordinate of the public key has many trailing 1's
    let z197 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r197 = [0x011f8fbbf3466830, 0x57c176356a2624fb, 0xcabed3346d891eee, 0x1158a08d291500b4];
    let s197 = [0xa46798c18f285519, 0xc91f378b75d487dd, 0xe082325b85290c5b, 0x228a8c486a736006];
    let pk197 = [
        0x5c3f497265004935,
        0x618f06b8ff87e801,
        0xd499a07873fac281,
        0x3cf03d614d8939cf,
        0x1a6324217fffffff,
        0x8385ecc857d07a56,
        0xd31c77f5769f22d5,
        0x7b05e8b186e38d41,
    ];
    assert!(ecdsa_verify_secp256r1(&pk197, &z197, &r197, &s197));

    // 194] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #203: y-coordinate of the public key has many trailing 1's
    let z198 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r198 = [0x3e0dde56d309fa9d, 0x2687b29176939dd2, 0x0ea36b0c0fc8d6aa, 0xb1db9289649f5941];
    let s198 = [0x4e1c3f48a1251336, 0x3a6d1af5c23c7d58, 0x5b0dbd987366dcf4, 0x3e1535e428055901];
    let pk198 = [
        0x5c3f497265004935,
        0x618f06b8ff87e801,
        0xd499a07873fac281,
        0x3cf03d614d8939cf,
        0x1a6324217fffffff,
        0x8385ecc857d07a56,
        0xd31c77f5769f22d5,
        0x7b05e8b186e38d41,
    ];
    assert!(ecdsa_verify_secp256r1(&pk198, &z198, &r198, &s198));

    // 195] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #204: y-coordinate of the public key has many trailing 1's
    let z199 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r199 = [0x0ac6f0ca4e24ed86, 0x0a341a79f2dd1a22, 0x446aa8d4e6e7578b, 0xb7b16e762286cb96];
    let s199 = [0x5e55234ecb8f12bc, 0x1780146df799ccf5, 0x661c547d07bbb072, 0xddc60a700a139b04];
    let pk199 = [
        0x5c3f497265004935,
        0x618f06b8ff87e801,
        0xd499a07873fac281,
        0x3cf03d614d8939cf,
        0x1a6324217fffffff,
        0x8385ecc857d07a56,
        0xd31c77f5769f22d5,
        0x7b05e8b186e38d41,
    ];
    assert!(ecdsa_verify_secp256r1(&pk199, &z199, &r199, &s199));

    // 196] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #205: x-coordinate of the public key has many trailing 1's
    let z200 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r200 = [0xd1c91c670d9105b4, 0xd796edad36bc6e6b, 0xc8e00d8df963ff35, 0xd82a7c2717261187];
    let s200 = [0x680d07debd139929, 0x351ecd5988efb23f, 0xf4603e7cbac0f3c0, 0x3dcabddaf8fcaa61];
    let pk200 = [
        0xdfa5ff8effffffff,
        0x45956ebcfe8ad0f6,
        0x344ed94bca3fcd05,
        0x2829c31faa2e400e,
        0xeb3bd37ebeb9222e,
        0x4113099052df57e7,
        0x5855afa7676ade28,
        0xa01aafaf000e5258,
    ];
    assert!(ecdsa_verify_secp256r1(&pk200, &z200, &r200, &s200));

    // 197] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #206: x-coordinate of the public key has many trailing 1's
    let z201 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r201 = [0x6a5cba063254af78, 0x7787802baff30ce9, 0x3d5befe719f462d7, 0x5eb9c8845de68eb1];
    let s201 = [0x2b87ddbe2ef66fb5, 0x44972186228ee9a6, 0x7ca0ff9bbd92fb6e, 0x2c026ae9be2e2a5e];
    let pk201 = [
        0xdfa5ff8effffffff,
        0x45956ebcfe8ad0f6,
        0x344ed94bca3fcd05,
        0x2829c31faa2e400e,
        0xeb3bd37ebeb9222e,
        0x4113099052df57e7,
        0x5855afa7676ade28,
        0xa01aafaf000e5258,
    ];
    assert!(ecdsa_verify_secp256r1(&pk201, &z201, &r201, &s201));

    // 198] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #207: x-coordinate of the public key has many trailing 1's
    let z202 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r202 = [0x404a8e4e36230c28, 0xf277921becc117d0, 0xf3b782b170239f90, 0x96843dd03c22abd2];
    let s202 = [0x19e1ede123dd991d, 0x9a31214eb4d7e6db, 0x43f67165976de9ed, 0xf2be378f526f74a5];
    let pk202 = [
        0xdfa5ff8effffffff,
        0x45956ebcfe8ad0f6,
        0x344ed94bca3fcd05,
        0x2829c31faa2e400e,
        0xeb3bd37ebeb9222e,
        0x4113099052df57e7,
        0x5855afa7676ade28,
        0xa01aafaf000e5258,
    ];
    assert!(ecdsa_verify_secp256r1(&pk202, &z202, &r202, &s202));

    // 199] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #208: x-coordinate of the public key is large
    let z203 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r203 = [0x3b760297067421f6, 0x4d27e9d98edc2d0e, 0x6f9996af72933946, 0x766456dce1857c90];
    let s203 = [0x3646bfbbf19d0b41, 0x4e55376eced699e9, 0x81dccaf5d19037ec, 0x402385ecadae0d80];
    let pk203 = [
        0x08318c4ca9a7a4f5,
        0x65ff9059ad6aac07,
        0x0458dd8f9e738f26,
        0xfffffff948081e6a,
        0x78e6529a1663bd73,
        0xe0c0fb89557ad0bf,
        0x311ee54149b973ca,
        0x5a8abcba2dda8474,
    ];
    assert!(ecdsa_verify_secp256r1(&pk203, &z203, &r203, &s203));

    // 200] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #209: x-coordinate of the public key is large
    let z204 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r204 = [0x9c34f777de7b9fd9, 0xb97ed8b07cced0b1, 0x19e6518a11b2dbc2, 0xc605c4b2edeab204];
    let s204 = [0xff5e159d47326dba, 0xb2cde2eda700fb1c, 0xc719647bc8af1b29, 0xedf0f612c5f46e03];
    let pk204 = [
        0x08318c4ca9a7a4f5,
        0x65ff9059ad6aac07,
        0x0458dd8f9e738f26,
        0xfffffff948081e6a,
        0x78e6529a1663bd73,
        0xe0c0fb89557ad0bf,
        0x311ee54149b973ca,
        0x5a8abcba2dda8474,
    ];
    assert!(ecdsa_verify_secp256r1(&pk204, &z204, &r204, &s204));

    // 201] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #210: x-coordinate of the public key is large
    let z205 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r205 = [0xb732bfe3b7eb8a84, 0x10e64485d9929ad7, 0xf6141c9ac54141f2, 0xd48b68e6cabfe03c];
    let s205 = [0x08f0772315b6c941, 0x4508c389109ad2f2, 0x19dc26f9b7e2265e, 0xfeedae50c61bd00e];
    let pk205 = [
        0x08318c4ca9a7a4f5,
        0x65ff9059ad6aac07,
        0x0458dd8f9e738f26,
        0xfffffff948081e6a,
        0x78e6529a1663bd73,
        0xe0c0fb89557ad0bf,
        0x311ee54149b973ca,
        0x5a8abcba2dda8474,
    ];
    assert!(ecdsa_verify_secp256r1(&pk205, &z205, &r205, &s205));

    // 202] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #211: x-coordinate of the public key is small
    let z206 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r206 = [0xc35a93d12a5dd4c7, 0x710ad7f6595d5874, 0x65957098569f0479, 0xb7c81457d4aeb6aa];
    let s206 = [0x4b9e3a05c0a1cdb3, 0x1a9199f2ca574dad, 0xd568069a432ca18a, 0xb7961a0b652878c2];
    let pk206 = [
        0xbff1173937ba748e,
        0x6f9e0015eeb23aeb,
        0x949d5f03a6f5c7f8,
        0x00000003fa15f963,
        0x548ca48af2ba7e71,
        0xfadcfcb0023ea889,
        0x555fa13659cca5d7,
        0x1099872070e8e87c,
    ];
    assert!(ecdsa_verify_secp256r1(&pk206, &z206, &r206, &s206));

    // 203] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #212: x-coordinate of the public key is small
    let z207 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r207 = [0xde5e9652e76ff3f7, 0xe3cf97e263e669f8, 0xa30a1321d5858e1e, 0x6b01332ddb6edfa9];
    let s207 = [0xcc58f9e69e96cd5a, 0x139c8f7d86b02cb1, 0x9a6a04ace2bd0f70, 0x5939545fced45730];
    let pk207 = [
        0xbff1173937ba748e,
        0x6f9e0015eeb23aeb,
        0x949d5f03a6f5c7f8,
        0x00000003fa15f963,
        0x548ca48af2ba7e71,
        0xfadcfcb0023ea889,
        0x555fa13659cca5d7,
        0x1099872070e8e87c,
    ];
    assert!(ecdsa_verify_secp256r1(&pk207, &z207, &r207, &s207));

    // 204] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #213: x-coordinate of the public key is small
    let z208 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r208 = [0x0e6a4fb93f106361, 0x4101cd2fd8436b7d, 0x349f9fc356b6c034, 0xefdb884720eaeadc];
    let s208 = [0xe48cb60d8113385d, 0xcba9e77de7d69b6c, 0x613975473aadf3aa, 0xf24bee6ad5dc05f7];
    let pk208 = [
        0xbff1173937ba748e,
        0x6f9e0015eeb23aeb,
        0x949d5f03a6f5c7f8,
        0x00000003fa15f963,
        0x548ca48af2ba7e71,
        0xfadcfcb0023ea889,
        0x555fa13659cca5d7,
        0x1099872070e8e87c,
    ];
    assert!(ecdsa_verify_secp256r1(&pk208, &z208, &r208, &s208));

    // 205] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #214: y-coordinate of the public key is small
    let z209 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r209 = [0x8014c87b8b20eb07, 0x9b23a23dd973dcbe, 0xb88fb5a646836aea, 0x31230428405560dc];
    let s209 = [0x8bd7ae3d9bd0beff, 0xaf97374e19f3c5fb, 0x6646747694a41b0a, 0x0f9344d6e812ce16];
    let pk209 = [
        0x25e9f851c18af015,
        0xbe5d2d6796707d81,
        0xaa6ecbbc612816b3,
        0xbcbb2914c79f045e,
        0xf300a698a7193bc2,
        0xdd684ade5a1127bc,
        0x0fa2ea4cceb9ab63,
        0x000000001352bb4a,
    ];
    assert!(ecdsa_verify_secp256r1(&pk209, &z209, &r209, &s209));

    // 206] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #215: y-coordinate of the public key is small
    let z210 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r210 = [0x9174db34c4855743, 0x94359c7db9841d67, 0x0d5c470cda0b36b2, 0xcaa797da65b320ab];
    let s210 = [0x3de6d9b36242e5a0, 0x123d2685ee3b941d, 0x45391aaf7505f345, 0xcf543a62f23e2127];
    let pk210 = [
        0x25e9f851c18af015,
        0xbe5d2d6796707d81,
        0xaa6ecbbc612816b3,
        0xbcbb2914c79f045e,
        0xf300a698a7193bc2,
        0xdd684ade5a1127bc,
        0x0fa2ea4cceb9ab63,
        0x000000001352bb4a,
    ];
    assert!(ecdsa_verify_secp256r1(&pk210, &z210, &r210, &s210));

    // 207] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #216: y-coordinate of the public key is small
    let z211 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r211 = [0x1c336ed800185945, 0x19bc54084536e7d2, 0xd7867657e5d6d365, 0x7e5f0ab5d900d3d3];
    let s211 = [0xe727ff0b19b646aa, 0x6688294aad35aa72, 0x4b82dfb322e5ac67, 0x9450c07f201faec9];
    let pk211 = [
        0x25e9f851c18af015,
        0xbe5d2d6796707d81,
        0xaa6ecbbc612816b3,
        0xbcbb2914c79f045e,
        0xf300a698a7193bc2,
        0xdd684ade5a1127bc,
        0x0fa2ea4cceb9ab63,
        0x000000001352bb4a,
    ];
    assert!(ecdsa_verify_secp256r1(&pk211, &z211, &r211, &s211));

    // 208] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #217: y-coordinate of the public key is large
    let z212 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r212 = [0x03aa69f0ca25b356, 0x3f8a1e4a2136fe4b, 0x6dc6a480bf037ae2, 0xd7d70c581ae9e3f6];
    let s212 = [0xaf41d9127cc47224, 0x13e85658e62a59e2, 0xba962c8a3ee833a4, 0x89c460f8a5a5c2bb];
    let pk212 = [
        0x25e9f851c18af015,
        0xbe5d2d6796707d81,
        0xaa6ecbbc612816b3,
        0xbcbb2914c79f045e,
        0x0cff596758e6c43d,
        0x2297b522a5eed843,
        0xf05d15b33146549c,
        0xfffffffeecad44b6,
    ];
    assert!(ecdsa_verify_secp256r1(&pk212, &z212, &r212, &s212));

    // 209] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #218: y-coordinate of the public key is large
    let z213 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r213 = [0xeee34bb396266b34, 0xb7aa20c625975e5e, 0xe0dfa0bf68bcdf4b, 0x341c1b9ff3c83dd5];
    let s213 = [0x902a67099e0a4469, 0x49c634e77765a017, 0x121b22b11366fad5, 0x72b69f061b750fd5];
    let pk213 = [
        0x25e9f851c18af015,
        0xbe5d2d6796707d81,
        0xaa6ecbbc612816b3,
        0xbcbb2914c79f045e,
        0x0cff596758e6c43d,
        0x2297b522a5eed843,
        0xf05d15b33146549c,
        0xfffffffeecad44b6,
    ];
    assert!(ecdsa_verify_secp256r1(&pk213, &z213, &r213, &s213));

    // 210] wycheproof/ecdsa_secp256r1_sha256_p1363_test.json EcdsaP1363Verify SHA-256 #219: y-coordinate of the public key is large
    let z214 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r214 = [0x7628d313a3814f67, 0x9bd1781a59180994, 0x72a42f0d87387935, 0x70bebe684cdcb5ca];
    let s214 = [0x2dcde6b2094798a9, 0xc0e464b1c3577f4c, 0xd535fa31027bbe9c, 0xaec03aca8f5587a4];
    let pk214 = [
        0x25e9f851c18af015,
        0xbe5d2d6796707d81,
        0xaa6ecbbc612816b3,
        0xbcbb2914c79f045e,
        0x0cff596758e6c43d,
        0x2297b522a5eed843,
        0xf05d15b33146549c,
        0xfffffffeecad44b6,
    ];
    assert!(ecdsa_verify_secp256r1(&pk214, &z214, &r214, &s214));

    // 211] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #1: signature malleability
    let z215 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r215 = [0xb8cc6af9bd5c2e18, 0xffe50d85a1eee859, 0x80a6d9d1190a436e, 0x2ba3a8be6b94d5ec];
    let s215 = [0x77a67f79e6fadd76, 0x525fe710fab9aa7c, 0x3c7b11eb6c4e0ae7, 0x4cd60b855d442f5b];
    let pk215 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk215, &z215, &r215, &s215));

    // 212] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #2: Legacy:ASN encoding of s misses leading 0
    let z216 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r216 = [0xb8cc6af9bd5c2e18, 0xffe50d85a1eee859, 0x80a6d9d1190a436e, 0x2ba3a8be6b94d5ec];
    let s216 = [0x7c134b49156847db, 0x6a87139cac5df408, 0xc384ee1493b1f518, 0xb329f479a2bbd0a5];
    let pk216 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk216, &z216, &r216, &s216));

    // 213] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #3: valid
    let z217 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r217 = [0xb8cc6af9bd5c2e18, 0xffe50d85a1eee859, 0x80a6d9d1190a436e, 0x2ba3a8be6b94d5ec];
    let s217 = [0x7c134b49156847db, 0x6a87139cac5df408, 0xc384ee1493b1f518, 0xb329f479a2bbd0a5];
    let pk217 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk217, &z217, &r217, &s217));

    // 214] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #118: modify first byte of integer
    let z218 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r218 = [0xb8cc6af9bd5c2e18, 0xffe50d85a1eee859, 0x80a6d9d1190a436e, 0x29a3a8be6b94d5ec];
    let s218 = [0x7c134b49156847db, 0x6a87139cac5df408, 0xc384ee1493b1f518, 0xb329f479a2bbd0a5];
    let pk218 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk218, &z218, &r218, &s218));

    // 215] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #120: modify last byte of integer
    let z219 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r219 = [0xb8cc6af9bd5c2e98, 0xffe50d85a1eee859, 0x80a6d9d1190a436e, 0x2ba3a8be6b94d5ec];
    let s219 = [0x7c134b49156847db, 0x6a87139cac5df408, 0xc384ee1493b1f518, 0xb329f479a2bbd0a5];
    let pk219 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk219, &z219, &r219, &s219));

    // 216] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #121: modify last byte of integer
    let z220 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r220 = [0xb8cc6af9bd5c2e18, 0xffe50d85a1eee859, 0x80a6d9d1190a436e, 0x2ba3a8be6b94d5ec];
    let s220 = [0x7c134b491568475b, 0x6a87139cac5df408, 0xc384ee1493b1f518, 0xb329f479a2bbd0a5];
    let pk220 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk220, &z220, &r220, &s220));

    // 217] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #124: truncated integer
    let z221 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r221 = [0xb8cc6af9bd5c2e18, 0xffe50d85a1eee859, 0x80a6d9d1190a436e, 0x2ba3a8be6b94d5ec];
    let s221 = [0x087c134b49156847, 0x186a87139cac5df4, 0xa5c384ee1493b1f5, 0x00b329f479a2bbd0];
    let pk221 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk221, &z221, &r221, &s221));

    // 218] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #133: Modified r or s, e.g. by adding or subtracting the order of the group
    let z222 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r222 = [0x4733950642a3d1e8, 0x001af27a5e1117a6, 0x7f59262ee6f5bc91, 0xd45c5741946b2a13];
    let s222 = [0x7c134b49156847db, 0x6a87139cac5df408, 0xc384ee1493b1f518, 0xb329f479a2bbd0a5];
    let pk222 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk222, &z222, &r222, &s222));

    // 219] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #134: Modified r or s, e.g. by adding or subtracting the order of the group
    let z223 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r223 = [0x3aed5fc93f06f739, 0xbd01ed280528b62b, 0x7f59262ee6f5bc90, 0xd45c5740946b2a14];
    let s223 = [0x7c134b49156847db, 0x6a87139cac5df408, 0xc384ee1493b1f518, 0xb329f479a2bbd0a5];
    let pk223 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk223, &z223, &r223, &s223));

    // 220] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #137: Modified r or s, e.g. by adding or subtracting the order of the group
    let z224 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r224 = [0x4733950642a3d1e8, 0x001af27a5e1117a6, 0x7f59262ee6f5bc91, 0xd45c5741946b2a13];
    let s224 = [0x7c134b49156847db, 0x6a87139cac5df408, 0xc384ee1493b1f518, 0xb329f479a2bbd0a5];
    let pk224 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk224, &z224, &r224, &s224));

    // 221] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #139: Modified r or s, e.g. by adding or subtracting the order of the group
    let z225 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r225 = [0xb8cc6af9bd5c2e18, 0xffe50d85a1eee859, 0x80a6d9d1190a436e, 0x2ba3a8be6b94d5ec];
    let s225 = [0x885980861905228a, 0xada018ef05465583, 0xc384ee1493b1f518, 0xb329f47aa2bbd0a4];
    let pk225 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk225, &z225, &r225, &s225));

    // 222] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #143: Modified r or s, e.g. by adding or subtracting the order of the group
    let z226 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r226 = [0xb8cc6af9bd5c2e18, 0xffe50d85a1eee859, 0x80a6d9d1190a436e, 0x2ba3a8be6b94d5ec];
    let s226 = [0x83ecb4b6ea97b825, 0x9578ec6353a20bf7, 0x3c7b11eb6c4e0ae7, 0x4cd60b865d442f5a];
    let pk226 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk226, &z226, &r226, &s226));

    // 223] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #177: Signature with special case values for r and s
    let z227 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r227 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s227 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk227 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk227, &z227, &r227, &s227));

    // 224] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #178: Signature with special case values for r and s
    let z228 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r228 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s228 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk228 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk228, &z228, &r228, &s228));

    // 225] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #179: Signature with special case values for r and s
    let z229 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r229 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s229 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk229 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk229, &z229, &r229, &s229));

    // 226] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #180: Signature with special case values for r and s
    let z230 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r230 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s230 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let pk230 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk230, &z230, &r230, &s230));

    // 227] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #181: Signature with special case values for r and s
    let z231 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r231 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s231 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let pk231 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk231, &z231, &r231, &s231));

    // 228] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #187: Signature with special case values for r and s
    let z232 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r232 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s232 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk232 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk232, &z232, &r232, &s232));

    // 229] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #188: Signature with special case values for r and s
    let z233 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r233 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s233 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk233 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk233, &z233, &r233, &s233));

    // 230] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #189: Signature with special case values for r and s
    let z234 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r234 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s234 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk234 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk234, &z234, &r234, &s234));

    // 231] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #190: Signature with special case values for r and s
    let z235 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r235 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s235 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let pk235 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk235, &z235, &r235, &s235));

    // 232] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #191: Signature with special case values for r and s
    let z236 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r236 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s236 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let pk236 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk236, &z236, &r236, &s236));

    // 233] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #197: Signature with special case values for r and s
    let z237 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r237 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s237 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk237 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk237, &z237, &r237, &s237));

    // 234] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #198: Signature with special case values for r and s
    let z238 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r238 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s238 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk238 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk238, &z238, &r238, &s238));

    // 235] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #199: Signature with special case values for r and s
    let z239 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r239 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s239 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk239 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk239, &z239, &r239, &s239));

    // 236] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #200: Signature with special case values for r and s
    let z240 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r240 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s240 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let pk240 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk240, &z240, &r240, &s240));

    // 237] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #201: Signature with special case values for r and s
    let z241 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r241 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s241 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let pk241 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk241, &z241, &r241, &s241));

    // 238] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #207: Signature with special case values for r and s
    let z242 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r242 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let s242 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk242 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk242, &z242, &r242, &s242));

    // 239] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #208: Signature with special case values for r and s
    let z243 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r243 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let s243 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk243 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk243, &z243, &r243, &s243));

    // 240] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #209: Signature with special case values for r and s
    let z244 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r244 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let s244 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk244 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk244, &z244, &r244, &s244));

    // 241] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #210: Signature with special case values for r and s
    let z245 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r245 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let s245 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let pk245 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk245, &z245, &r245, &s245));

    // 242] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #211: Signature with special case values for r and s
    let z246 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r246 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let s246 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let pk246 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk246, &z246, &r246, &s246));

    // 243] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #217: Signature with special case values for r and s
    let z247 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r247 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let s247 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk247 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk247, &z247, &r247, &s247));

    // 244] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #218: Signature with special case values for r and s
    let z248 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r248 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let s248 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk248 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk248, &z248, &r248, &s248));

    // 245] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #219: Signature with special case values for r and s
    let z249 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r249 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let s249 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk249 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk249, &z249, &r249, &s249));

    // 246] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #220: Signature with special case values for r and s
    let z250 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r250 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let s250 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let pk250 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk250, &z250, &r250, &s250));

    // 247] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #221: Signature with special case values for r and s
    let z251 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r251 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let s251 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let pk251 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk251, &z251, &r251, &s251));

    // 248] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #230: Edge case for Shamir multiplication
    let z252 = [0x2fa50c772ed6f807, 0x2f2627416faf2f07, 0xc422f44dea4ed1a5, 0x70239dd877f7c944];
    let r252 = [0x11547c97711c898e, 0x8ff312334e2ba16d, 0x4f3e2fc02bdee9be, 0x64a1aab5000d0e80];
    let s252 = [0xfd683b9bb2cf4f1b, 0x7772a2f91d73286f, 0xd1a206d4e013e099, 0x6af015971cc30be6];
    let pk252 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk252, &z252, &r252, &s252));

    // 249] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #231: special case hash
    let z253 = [0x7ead3645f356e7a9, 0x84bcd58a1bb5e747, 0xccf17803ebe2bd08, 0x00000000690ed426];
    let r253 = [0x1e19a0ec580bf266, 0xded7d397738448de, 0x6f78c81c91fc7e8b, 0x16aea964a2f6506d];
    let s253 = [0x38c3ff033be928e9, 0x391e8e80c578d1cd, 0xcfe8b7bc47d27d78, 0x252cd762130c6667];
    let pk253 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk253, &z253, &r253, &s253));

    // 250] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #232: special case hash
    let z254 = [0x140697ad25770d91, 0xf696ad3ebb5ee47f, 0x525c6035725235c2, 0x7300000000213f2a];
    let r254 = [0x7c665baccb23c882, 0xf2d26d6ef524af91, 0xf476dfc26b9b733d, 0x9cc98be2347d469b];
    let s254 = [0xa631dacb16b56c32, 0x0ec1b7847929d10e, 0xd70727b82462f61d, 0x093496459effe2d8];
    let pk254 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk254, &z254, &r254, &s254));

    // 251] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #233: special case hash
    let z255 = [0x4a0161c27fe06045, 0x8afd25daadeb3edb, 0xe0635b245f0b9797, 0xddf2000000005e0b];
    let r255 = [0x093999f07ab8aa43, 0x03dce3dea0d53fa8, 0x058164524dde8927, 0x73b3c90ecd390028];
    let s255 = [0x188c0c4075c88634, 0x2ed25a395387b5f4, 0x5bb7d8bf0a651c80, 0x2f67b0b8e2063669];
    let pk255 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk255, &z255, &r255, &s255));

    // 252] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #234: special case hash
    let z256 = [0x5be1ec355d0841a0, 0x642b8499588b8985, 0x4769c4ecb9e164d6, 0x67ab190000000078];
    let r256 = [0xf37e90119d5ba3dd, 0x1a7f0eb390763378, 0x28fadf2f89b95c85, 0xbfab3098252847b3];
    let s256 = [0x1e2da9b8b4987e3b, 0x8195ccebb65c2aaf, 0x67c2d058ccb44d97, 0xbdd64e234e832b10];
    let pk256 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk256, &z256, &r256, &s256));

    // 253] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #235: special case hash
    let z257 = [0xe296b6350fc311cf, 0x02095dff252ee905, 0x76d7dbeffe125eaf, 0xa2bf094600000000];
    let r257 = [0xd17093c5cd21d2cd, 0xf1c9aaab168b1596, 0x8bf8bf04a4ceb1c1, 0x204a9784074b246d];
    let s257 = [0x582fe648d1d88b52, 0xa406c2506fe17975, 0xdc06a759c8847868, 0x51cce41670636783];
    let pk257 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk257, &z257, &r257, &s257));

    // 254] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #236: special case hash
    let z258 = [0x29e15c544e4f0e65, 0xa0a3531711608581, 0x00e1e75e624a06b3, 0x3554e827c7000000];
    let r258 = [0x027bca0f1ceeaa03, 0x0031a91d1314f835, 0xf63d4aa4f81fe2cb, 0xed66dc34f551ac82];
    let s258 = [0xbb8953d67c0c48c7, 0x67623c3f6e5d4d6a, 0x194a422e18d5fda1, 0x99ca123aa09b13cd];
    let pk258 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk258, &z258, &r258, &s258));

    // 255] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #237: special case hash
    let z259 = [0x26e3a54b9fc6965c, 0x3255ea4c9fd0cb34, 0x000026941a0f0bb5, 0x9b6cd3b812610000];
    let r259 = [0x56bf0f60a237012b, 0x126b062023ccc3c0, 0x899d44f2356a578d, 0x060b700bef665c68];
    let s259 = [0x6be5d581c11d3610, 0xedbb410cbef3f26d, 0x4fcc78a3366ca95d, 0x8d186c027832965f];
    let pk259 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk259, &z259, &r259, &s259));

    // 256] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #238: special case hash
    let z260 = [0x77162f93c4ae0186, 0x82a52baa51c71ca8, 0x000000e7561c26fc, 0x883ae39f50bf0100];
    let r260 = [0x2bb0c8e38c96831d, 0xc93ea76cd313c913, 0x24d7aa7934b6cf29, 0x9f6adfe8d5eb5b2c];
    let s260 = [0x051593883b5e9902, 0x906a33e66b5bd15e, 0x890c944cf271756c, 0xb26a9c9e40e55ee0];
    let pk260 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk260, &z260, &r260, &s260));

    // 257] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #239: special case hash
    let z261 = [0x01fe9fce011d0ba6, 0x10540f420fb4ff74, 0x0000000000fa7cd0, 0xa1ce5d6e5ecaf28b];
    let r261 = [0x8868f4ba273f16b7, 0xa1abf6da168cebfa, 0x3ad2f33615e56174, 0xa1af03ca91677b67];
    let s261 = [0x5caf24c8c5e06b1c, 0x77d69022e7d098d7, 0x35cd258b173d0c23, 0x20aa73ffe48afa64];
    let pk261 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk261, &z261, &r261, &s261));

    // 258] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #240: special case hash
    let z262 = [0x5494cdffd5ee8054, 0x97330012a8ee836c, 0x9300000000383453, 0x8ea5f645f373f580];
    let r262 = [0xe327a28c11893db9, 0x659355507b843da6, 0x11a6c99a71c973d5, 0xfdc70602766f8eed];
    let s262 = [0xa7f83f2b10d21350, 0x0f6d15ec0078ca60, 0x37b1eacf456a9e9e, 0x3df5349688a085b1];
    let pk262 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk262, &z262, &r262, &s262));

    // 259] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #241: special case hash
    let z263 = [0x8d9c1bbdcb5ef305, 0xd65ce93eabb7d60d, 0xa734000000008792, 0x660570d323e9f75f];
    let r263 = [0x0dc738f7b876e675, 0x23456f63c643cf8e, 0xd6537f6a6c49966c, 0xb516a314f2fce530];
    let s263 = [0xa66b0120cd16fff2, 0x967c4bd80954479b, 0x17dd536fbc5efdf1, 0xd39ffd033c92b6d7];
    let pk263 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk263, &z263, &r263, &s263));

    // 260] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #242: special case hash
    let z264 = [0x46ada2de4c568c34, 0x8d35f1f45cf9c3bf, 0x7dde8800000000e9, 0xd0462673154cce58];
    let r264 = [0xa485c101e29ff0a8, 0x82717bebb6492fd0, 0x2ecb7984d4758315, 0x3b2cbf046eac4584];
    let s264 = [0xc8595fc1c1d99258, 0x701099cac5f76e68, 0xde512bc9313aaf51, 0x4c9b7b47a98b0f82];
    let pk264 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk264, &z264, &r264, &s264));

    // 261] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #243: special case hash
    let z265 = [0xb83e7b4418d7278f, 0x0caef15a6171059a, 0x80cedfef00000000, 0xbd90640269a78226];
    let r265 = [0x6c3fb15bfde48dcf, 0xd79d0312cfa1ab65, 0x841f14af54e2f9ed, 0x30c87d35e636f540];
    let s265 = [0x0db9abf6340677ed, 0x71409ede23efd08e, 0xc85a692bd6ecafeb, 0x47c15a5a82d24b75];
    let pk265 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk265, &z265, &r265, &s265));

    // 262] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #244: special case hash
    let z266 = [0x4beae8e284788a73, 0x00d2dcceb301c54b, 0x512e41222a000000, 0x33239a52d72f1311];
    let r266 = [0x68ff262113760f52, 0xe2e8176d168dec3c, 0xbc43b58cfe6647b9, 0x38686ff0fda2cef6];
    let s266 = [0xc2ddabb3fde9d67d, 0xe976e2db5e6a4cf7, 0x9601662167fa8717, 0x067ec3b651f42266];
    let pk266 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk266, &z266, &r266, &s266));

    // 263] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #245: special case hash
    let z267 = [0x1dc84c2d941ffaf1, 0x00007ee4a21a1cbe, 0x1365d4e6d95c0000, 0xb8d64fbcd4a1c10f];
    let r267 = [0x225985ab6e2775cf, 0xf3e17d27f5ee844b, 0x44fc25c7f2de8b6a, 0x44a3e23bf314f2b3];
    let s267 = [0x93c9cc3f4dd15e86, 0x84f0411f57295004, 0x1ddc87be532abed5, 0x2d48e223205e9804];
    let pk267 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk267, &z267, &r267, &s267));

    // 264] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #246: special case hash
    let z268 = [0x4088b20fe0e9d84a, 0x0000003a227420db, 0xa3fef3183ed09200, 0x01603d3982bf77d7];
    let r268 = [0x0eb9d638781688e9, 0x41b99db3b5aa8d33, 0xf11f967a3d95110c, 0x2ded5b7ec8e90e7b];
    let s268 = [0xec69238a009808f9, 0x8de049c328ae1f44, 0x1bfc46fb1a67e308, 0x7d5792c53628155e];
    let pk268 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk268, &z268, &r268, &s268));

    // 265] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #247: special case hash
    let z269 = [0xb7e9eb0cfbff7363, 0x000000004d89ef50, 0x599aa02e6cf66d9c, 0x9ea6994f1e0384c8];
    let r269 = [0x05976f15137d8b8f, 0x3eaccafcd40ec2f6, 0xefd3bc3d31870f92, 0xbdae7bcb580bf335];
    let s269 = [0x24838122ce7ec3c7, 0x9f373a4fb318994f, 0x0b0106eecfe25749, 0xf6dfa12f19e52527];
    let pk269 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk269, &z269, &r269, &s269));

    // 266] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #248: special case hash
    let z270 = [0xf692bc670905b18c, 0x4700000000e2fa5b, 0x693979371a01068a, 0xd03215a8401bcf16];
    let r270 = [0x1ece251c2401f1c6, 0x99209b78596956d2, 0x62720957ffff5137, 0x50f9c4f0cd6940e1];
    let s270 = [0xaa5167dfab244726, 0x5a4355e411a59c32, 0x889defaaabb106b9, 0xd7033a0a787d338e];
    let pk270 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk270, &z270, &r270, &s270));

    // 267] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #249: special case hash
    let z271 = [0xfd5f64b582e3bb14, 0xc87e000000008408, 0x9c84bf83f0300e5d, 0x307bfaaffb650c88];
    let r271 = [0xbe90924ead5c860d, 0x0982e29575d019aa, 0x1906066a378d6754, 0xf612820687604fa0];
    let s271 = [0x328230ce294b0fef, 0x1a99f4857b316525, 0x75ea98afd20e328a, 0x3f9367702dd7dd4f];
    let pk271 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk271, &z271, &r271, &s271));

    // 268] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #250: special case hash
    let z272 = [0xaf574bb4d54ea6b8, 0x51527c00000000e4, 0x33324d36bb0c1575, 0xbab5c4f4df540d7b];
    let r272 = [0x0f2f507da5782a7a, 0x1f61980c1949f56b, 0xc93db5da7aa6f508, 0x9505e407657d6e8b];
    let s272 = [0x5e7f71784f9c5021, 0x08e0ed5cb92b3cfa, 0x8ffbeccab6c3656c, 0xc60d31904e366973];
    let pk272 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk272, &z272, &r272, &s272));

    // 269] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #251: special case hash
    let z273 = [0xc3b869197ef5e15e, 0xc2456f5b00000000, 0xe4f58d8036f9c36e, 0xd4ba47f6ae28f274];
    let r273 = [0x3e1c68a40404517d, 0x08735aed37173272, 0xd83e6a7787cd691b, 0xbbd16fbbb656b6d0];
    let s273 = [0x560e3e7fd25c0f00, 0x7d2d097be5e8ee34, 0x787d91315be67587, 0x9d8e35dba96028b7];
    let pk273 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk273, &z273, &r273, &s273));

    // 270] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #252: special case hash
    let z274 = [0x00801e47f8c184e1, 0xfe0f10aafd000000, 0xf29f1fa00984342a, 0x79fd19c7235ea212];
    let r274 = [0xcf57c61e92df327e, 0x442d2ceef7559a30, 0x06ea76848d35a6da, 0x2ec9760122db98fd];
    let s274 = [0xc4963625c0a19878, 0x393fb6814c27b760, 0x701fccf86e462ee3, 0x7ab271da90859479];
    let pk274 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk274, &z274, &r274, &s274));

    // 271] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #253: special case hash
    let z275 = [0x0000a37ea6700cda, 0x79cbeb7ac9730000, 0xaf9aba5c0583462d, 0x8c291e8eeaa45adb];
    let r275 = [0x4f1005a89fe00c59, 0xd9ba9dd463221f7a, 0xaa6a7fc49b1c51ee, 0x54e76b7683b6650b];
    let s275 = [0x52f2f7806a31c8fd, 0xcfd11b1c1ae11661, 0x37ec1cc8374b7915, 0x2ea076886c773eb9];
    let pk275 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk275, &z275, &r275, &s275));

    // 272] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #254: special case hash
    let z276 = [0x0000003c278a6b21, 0xf4cdcf66c3f78a00, 0x9803efbfb8140732, 0x0eaae8641084fa97];
    let r276 = [0x10419c0c496c9466, 0x7a74abdbb69be4fb, 0xbce6e3c26f602109, 0x5291deaf24659ffb];
    let s276 = [0xbf83469270a03dc3, 0x827f84742f29f10a, 0xcdb982bb4e4ecef5, 0x65d6fcf336d27cc7];
    let pk276 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk276, &z276, &r276, &s276));

    // 273] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #255: special case hash
    let z277 = [0x00000000afc0f89d, 0xef17c6d96e13846c, 0x0068399bf01bab42, 0xe02716d01fb23a5a];
    let r277 = [0xd15166a88479f107, 0x003b33fc17eb50f9, 0x47419dc58efb05e8, 0x207a3241812d75d9];
    let s277 = [0x82d5caadf7592767, 0xf1c5d70793cf55e3, 0x3ce80b32d0574f62, 0xcdee749f2e492b21];
    let pk277 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk277, &z277, &r277, &s277));

    // 274] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #256: special case hash
    let z278 = [0x9a00000000fc7de1, 0x9061768af89d0065, 0x194e9a16bc7dab2a, 0x9eb0bf583a1a6b9a];
    let r278 = [0xc0dee3cf81aa7728, 0xbe84437a355a0a37, 0x4328ac94913bf01b, 0x6554e49f82a85520];
    let s278 = [0x86effe7f22b4f929, 0x16250a2eaebc8be4, 0xc94e1e126980d3df, 0xaea00de2507ddaf5];
    let pk278 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk278, &z278, &r278, &s278));

    // 275] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #257: special case hash
    let z279 = [0x690e00000000cd15, 0x6e1030cb53d9a82b, 0x2c214f0d5e72ef28, 0x62aac98818b3b84a];
    let r279 = [0x2990ac82707efdfc, 0x6c6e19b4d80a8c60, 0xbff06f71c88216c2, 0xa54c5062648339d2];
    let s279 = [0xff09be73c9731b0d, 0x1056317f467ad09a, 0x69fd016777517aa0, 0xe99bbe7fcfafae3e];
    let pk279 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk279, &z279, &r279, &s279));

    // 276] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #258: special case hash
    let z280 = [0x464b9300000000c8, 0xd2b6f552ea4b6895, 0xf29ae43732e513ef, 0x3760a7f37cf96218];
    let r280 = [0x4ca8b059cff37eaf, 0xd23096593133e71b, 0x309f1f444012b1a1, 0x975bd7157a8d363b];
    let s280 = [0xacc46786bf919622, 0xd4c69840fe090f2a, 0xa241793f2abc930b, 0x7faa7a28b1c822ba];
    let pk280 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk280, &z280, &r280, &s280));

    // 277] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #259: special case hash
    let z281 = [0xbb6ff6c800000000, 0x6b4320bea836cd9c, 0x3834f2098c088009, 0x0da0a1d2851d3302];
    let r281 = [0x7b95b3e0da43885e, 0xde9ec90305afb135, 0x276afd2ebcfe4d61, 0x5694a6f84b8f875c];
    let s281 = [0x3b6ccc7c679cbaa4, 0x8ee2dc5c7870c082, 0x8051dec02ebdf70d, 0x0dffad9ffd0b757d];
    let pk281 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk281, &z281, &r281, &s281));

    // 278] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #260: special case hash
    let z282 = [0xa764a231e82d289a, 0x0fe975f735887194, 0x086fd567aafd598f, 0xffffffff293886d3];
    let r282 = [0xd7454ba9790f1ba6, 0xf7098f1a98d21620, 0xb4968a27d16a6d08, 0xa0c30e8026fdb2b4];
    let s282 = [0x8bd2760c65424339, 0xacc5ca6445914968, 0x5baf463f9deceb53, 0x5e470453a8a399f1];
    let pk282 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk282, &z282, &r282, &s282));

    // 279] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #261: special case hash
    let z283 = [0x0e8d9ca99527e7b7, 0x26acdc4ce127ec2e, 0xe3c03445a072e243, 0x7bffffffff2376d1];
    let r283 = [0x2aa0228cf7b99a88, 0x1dfebebd5ad8aca5, 0xdd73602cd4bb4eea, 0x614ea84acf736527];
    let s283 = [0x2a4dd193195c902f, 0xde14368e96a9482c, 0xd1b8183f3ed490e4, 0x737cc85f5f2d2f60];
    let pk283 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk283, &z283, &r283, &s283));

    // 280] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #262: special case hash
    let z284 = [0xfd016807e97fa395, 0xbc80872602a6e467, 0x51b085377605a224, 0xa2b5ffffffffebb2];
    let r284 = [0xa8d74dfbd0f942fa, 0x45377338febfd439, 0x0d3fb2ea00b17329, 0xbead6734ebe44b81];
    let s284 = [0x36a46b103ef56e2a, 0xf4bbe7a10f73b3e0, 0x3cad35919fd21a8a, 0x6bb18eae36616a7d];
    let pk284 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk284, &z284, &r284, &s284));

    // 281] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #263: special case hash
    let z285 = [0x7b83d0967d4b20c0, 0xc1a3c256870d45a6, 0x1b96fa5f097fcf3c, 0x641227ffffffff6f];
    let r285 = [0x654fae182df9bad2, 0x8d922cbf212703e9, 0xd4db9d9ce64854c9, 0x499625479e161dac];
    let s285 = [0x95b64fca76d9d693, 0x9439936028864ac1, 0x0131108d97819edd, 0x42c177cf37b8193a];
    let pk285 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk285, &z285, &r285, &s285));

    // 282] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #264: special case hash
    let z286 = [0x8df56f36600e0f8b, 0xba20352117750229, 0xabad03e2fc662dc3, 0x958415d8ffffffff];
    let r286 = [0x50fb1aaa6ff6c9b2, 0x31e3bfe694f6b89c, 0x66a2c8065b541b3d, 0x08f16b8093a8fb4d];
    let s286 = [0x535ba3e5af81ca2e, 0x21f967410399b39b, 0x48573b611cb95d4a, 0x9d6455e2d5d17797];
    let pk286 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk286, &z286, &r286, &s286));

    // 283] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #265: special case hash
    let z287 = [0x954521b6975420f8, 0xe13deb04e1fbe8fb, 0xff1281093536f47f, 0xf1d8de4858ffffff];
    let r287 = [0xeed8dc2b338cb5f8, 0xc579b6938d19bce8, 0x19dd72ddb99ed8f8, 0xbe26231b6191658a];
    let s287 = [0xb9c5e96952575c89, 0xc943c14f79694a03, 0x37f0f22b2dcb57d5, 0xe1d9a32ee56cffed];
    let pk287 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk287, &z287, &r287, &s287));

    // 284] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #266: special case hash
    let z288 = [0x876b95c81fc31def, 0x32dc5d47c05ef6f1, 0xffff10782dd14a3b, 0x0927895f2802ffff];
    let r288 = [0x12638c455abe0443, 0x45f36a229d4aa4f8, 0x6204ac920a02d580, 0x15e76880898316b1];
    let s288 = [0x38196506a1939123, 0x55ca10e226e13f96, 0x5337bd6aba4178b4, 0xe74d357d3fcb5c8c];
    let pk288 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk288, &z288, &r288, &s288));

    // 285] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #267: special case hash
    let z289 = [0x24cf6a0c3ac80589, 0x0a57c3063fb5a306, 0xffffff4f332862a1, 0x60907984aa7e8eff];
    let r289 = [0x132315cc07f16dad, 0x31e6307d3ddbffc1, 0x3a45f9846fc28d1d, 0x352ecb53f8df2c50];
    let s289 = [0x899792887dd0a3c6, 0x436726ecd28258b1, 0xe1d05c5242ca1c39, 0x1348dfa9c482c558];
    let pk289 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk289, &z289, &r289, &s289));

    // 286] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #268: special case hash
    let z290 = [0x42d6b9b8cd6ae1e2, 0x50f9a5f50636ea69, 0xffffffff0af42cda, 0xc6ff198484939170];
    let r290 = [0x2c5bfa5f2a9558fb, 0x77b8642349ed3d65, 0x8a0da9882ab23c76, 0x4a40801a7e606ba7];
    let s290 = [0xea77dc5981725782, 0xdc24ed2925825bf8, 0x7f605f2832f7384b, 0x3a49b64848d682ef];
    let pk290 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk290, &z290, &r290, &s290));

    // 287] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #269: special case hash
    let z291 = [0x16dfbe4d27d7e68d, 0x9b9e0956cc43135d, 0x75ffffffff807479, 0xde030419345ca15c];
    let r291 = [0xe5e9e44df3d61e96, 0xb3511bac855c05c9, 0x2be412b078924b3b, 0xeacc5e1a8304a74d];
    let s291 = [0x08db8f714204f6d1, 0xec4bb0ed4c36ce98, 0x85dd827714847f96, 0x7451cd8e18d6ed18];
    let pk291 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk291, &z291, &r291, &s291));

    // 288] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #270: special case hash
    let z292 = [0x7e1ab78caaaac6ff, 0x665604d34acb1903, 0x2b88fffffffff6c8, 0x6f0e3eeaf42b2813];
    let r292 = [0x5f7de94c31577052, 0x4f8cd1214882adb6, 0xf30f67fdab61e8ce, 0x2f7a5e9e5771d424];
    let s292 = [0xb9528f8f78daa10c, 0xfb75dd050c5a449a, 0x44acb0b2bd889175, 0xac4e69808345809b];
    let pk292 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk292, &z292, &r292, &s292));

    // 289] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #271: special case hash
    let z293 = [0x2cb222d1f8017ab9, 0x48f7c0591ddcae7d, 0x3708d1ffffffffbe, 0xcdb549f773b3e62b];
    let r293 = [0x0a03d710b3300219, 0x7dddd7f6487621c3, 0x3e7e0f0e95e1a214, 0xffcda40f792ce4d9];
    let s293 = [0xd58c422c2453a49a, 0xfa77618f0b67add8, 0xd7ba9ade8f2065a1, 0x79938b55f8a17f7e];
    let pk293 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk293, &z293, &r293, &s293));

    // 290] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #272: special case hash
    let z294 = [0x24d8fd6f0edb0484, 0x9fd64886c1dc4f99, 0x1df4989bffffffff, 0x2c3f26f96a3ac005];
    let r294 = [0x8c17603a431e39a8, 0x48350f7ab3a588b2, 0x3d3e8c8c3fcc16a9, 0x81f2359c4faba6b5];
    let s294 = [0x7f9e101857f74300, 0x09e46d99fccefb9f, 0x0ff695d06c6860b5, 0xcd6f6a5cc3b55ead];
    let pk294 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk294, &z294, &r294, &s294));

    // 291] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #273: special case hash
    let z295 = [0x8476397c04edf411, 0xff5c31d89fda6a6b, 0x2cb7d53f9affffff, 0xac18f8418c55a250];
    let r295 = [0xc3f5f2aaf75ca808, 0xea130251a6fdffa5, 0xee1596fb073ea283, 0xdfc8bf520445cbb8];
    let s295 = [0xa7ac711e577e90e7, 0xbfd7d0dc7a4905b3, 0xd92823640e338e68, 0x048e33efce147c9d];
    let pk295 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk295, &z295, &r295, &s295));

    // 292] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #274: special case hash
    let z296 = [0x3e5a6ab8cf0ee610, 0xffffa2fd3e289368, 0xb24094f72bb5ffff, 0x4f9618f98e2d3a15];
    let r296 = [0x88227688ba6a5762, 0x6503a0e393e932f6, 0xefda70b46c53db16, 0xad019f74c6941d20];
    let s296 = [0xbc05efe16c199345, 0x7964ef2e0988e712, 0x5346bdbb3102cdcf, 0x93320eb7ca071025];
    let pk296 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk296, &z296, &r296, &s296));

    // 293] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #275: special case hash
    let z297 = [0x04caae73ab0bc75a, 0xffffff67edf7c402, 0x9cc21d31d37a25ff, 0x422e82a3d56ed10a];
    let r297 = [0xdeb7bd5a3ebc1883, 0xb54316bd3ebf7fff, 0xc34e78ce11dd71e4, 0xac8096842e8add68];
    let s297 = [0x9f21a3aac003b7a8, 0x36e3ce9f0ce21970, 0x2d4caf85d187215d, 0xf5ca2f4f23d67450];
    let pk297 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk297, &z297, &r297, &s297));

    // 294] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #276: special case hash
    let z298 = [0x2d9890b5cf95d018, 0x17a5ffffffffa084, 0x6e7b329ff738fbb4, 0x7075d245ccc3281b];
    let r298 = [0x54b4943693fb92f7, 0x89ddcd7b7b9d7768, 0xf939b70ea0022508, 0x677b2d3a59b18a5f];
    let s298 = [0xab6972cc0795db55, 0x5d2f63aee81efd0b, 0xf30307b21f3ccda3, 0x6b4ba856ade7677b];
    let pk298 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk298, &z298, &r298, &s298));

    // 295] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #277: special case hash
    let z299 = [0xc1847eb76c217a95, 0x7e280ebeffffffff, 0x9443d593fa4fd659, 0x3c80de54cd922698];
    let r299 = [0x05e1fc0d5957cfb0, 0xd84d31d4b7c30e1f, 0x379ba8e1b73d3115, 0x479e1ded14bcaed0];
    let s299 = [0x1e877027355b2443, 0x30857ca879f97c77, 0x7cf634a4f05b2e0c, 0x918f79e35b3d8948];
    let pk299 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk299, &z299, &r299, &s299));

    // 296] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #278: special case hash
    let z300 = [0xffc7906aa794b39b, 0x0ce891a8cdffffff, 0x980bef3d697ea277, 0xde21754e29b85601];
    let r300 = [0xb64840ead512a0a3, 0xd711e14b12ac5cf3, 0xd9a58f01164d55c3, 0x43dfccd0edb9e280];
    let s300 = [0x3199f49584389772, 0xca1174899b78ef9a, 0xcd5c4934365b3442, 0x1dbe33fa8ba84533];
    let pk300 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk300, &z300, &r300, &s300));

    // 297] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #279: special case hash
    let z301 = [0xffff2f1f2f57881c, 0x599e4d5f7289ffff, 0x84dd59623fb531bb, 0x8f65d92927cfb86a];
    let r301 = [0x38bb4085f0bbff11, 0xa20e9087c259d26a, 0xf4c7c7e4bca592fe, 0x5b09ab637bd4caf0];
    let s301 = [0xca8101de08eb0d75, 0xa24964e5a13f885b, 0x618e9d80d6fdcd6a, 0x45b7eb467b6748af];
    let pk301 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk301, &z301, &r301, &s301));

    // 298] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #280: special case hash
    let z302 = [0xfffffffafc8c3ca8, 0x2cc7cd0e8426cbff, 0x160bea3877dace8a, 0x6b63e9a74e092120];
    let r302 = [0x14a5039ed15ee06f, 0x667afa570a6cfa01, 0x5728c5c8af9b74e0, 0x5e9b1c5a028070df];
    let s302 = [0x44edaeb9ad990c20, 0x6c29eeffd3c50377, 0xad362bb8d7bd661b, 0xb1360907e2d9785e];
    let pk302 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk302, &z302, &r302, &s302));

    // 299] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #281: special case hash
    let z303 = [0xffffffffe852512e, 0xd094586e249c8699, 0xb6d75219444e8b43, 0xfc28259702a03845];
    let r303 = [0xd1a7a5fb8578f32e, 0x4890050f5a5712f6, 0x4a2fb0990e34538b, 0x0671a0a85c2b72d5];
    let s303 = [0xc720e5854713694c, 0x1808f27fd5bd4fda, 0x79ab9c3285ca4129, 0xdb1846bab6b73614];
    let pk303 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk303, &z303, &r303, &s303));

    // 300] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #282: special case hash
    let z304 = [0x1757ffffffffe20a, 0x74ecbcd52e8ceb57, 0xcee044ee8e8db7f7, 0x1273b4502ea4e3bc];
    let r304 = [0xbaedb35b2095103a, 0xc5d7d69859d301ab, 0x77dbbb0590a45492, 0x7673f85267484464];
    let s304 = [0x3807ef4422913d7c, 0x4dec0d417a414fed, 0x886bed9e6af02e0e, 0x3dc70ddf9c6b524d];
    let pk304 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk304, &z304, &r304, &s304));

    // 301] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #283: special case hash
    let z305 = [0xfb49ffffffffff6e, 0x4f8c53a15b96e602, 0x0c566c66228d8181, 0x08fb565610a79baa];
    let r305 = [0x9dfd657a796d12b5, 0x450d1a06c36d3ff3, 0xb21285089ebb1aa6, 0x7f085441070ecd2b];
    let s305 = [0xa9e4c5c54a2b9a8b, 0x92a5e6cb4b2d8daf, 0x2459d18d47da9aa4, 0x249712012029870a];
    let pk305 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk305, &z305, &r305, &s305));

    // 302] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #284: special case hash
    let z306 = [0x28ecaefeffffffff, 0xa2403f748e97d7cd, 0x87715fcb1aa4e79a, 0xd59291cc2cf89f30];
    let r306 = [0xa8e0f30a5d287348, 0xb76df04bc5aa6683, 0xc867398ea7322d5a, 0x914c67fb61dd1e27];
    let s306 = [0xc96d28f6d37304ea, 0xea7e66ec412b38d6, 0x4953e3ac1959ee8c, 0xfa07474031481dda];
    let pk306 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk306, &z306, &r306, &s306));

    // 303] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #286: r too large
    let z307 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r307 = [0xfffffffffffffffc, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let s307 = [0xf3b9cac2fc63254e, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk307 = [
        0xa69874d2de5fe103,
        0x4d43784640855bf0,
        0x40031d72a9f5445a,
        0x0ad99500288d4669,
        0x22f0979ff0c3ba5e,
        0xba2c80c9244f4c54,
        0x50d5d3d29f99ae6e,
        0xc5011e6ef2c42dcd,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk307, &z307, &r307, &s307));

    // 304] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #287: r,s are large
    let z308 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r308 = [0xf3b9cac2fc63254f, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s308 = [0xf3b9cac2fc63254e, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk308 = [
        0x013e09c582204554,
        0x193d0aa398f0fba8,
        0xe6f4819652d9fc69,
        0xab05fd9d0de26b9c,
        0x2f49435a1e9b8d45,
        0x2dd4103f19f6a8c3,
        0x59095d12b75af069,
        0x19235271228c7867,
    ];
    assert!(ecdsa_verify_secp256r1(&pk308, &z308, &r308, &s308));

    // 305] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #288: r and s^-1 have a large Hamming weight
    let z309 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r309 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s309 = [0xde54a36383df8dd4, 0x1453fe50914f3df2, 0x170f5ead2de4f651, 0x909135bdb6799286];
    let pk309 = [
        0x07badf6fdd4c6c56,
        0xfbfecf876219710b,
        0x6a68aa4201b6be5d,
        0x80984f39a1ff38a8,
        0xf1445019bb55ed95,
        0xd74415ed3cac2089,
        0x7a06dfb41871c940,
        0x11feb97390d9826e,
    ];
    assert!(ecdsa_verify_secp256r1(&pk309, &z309, &r309, &s309));

    // 306] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #289: r and s^-1 have a large Hamming weight
    let z310 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r310 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s310 = [0x360644669ca249a5, 0xf5deb773ad5f5a84, 0x71303fd5dd227dce, 0x27b4577ca009376f];
    let pk310 = [
        0x0a95bc602b4f7c05,
        0x6dd687495fcc19a7,
        0x3294f5baa9a3232b,
        0x4201b4272944201c,
        0x572c0c0a8fb0800e,
        0x36f463e3aef16629,
        0x1bb5ac6feaf753bc,
        0x95c37eba9ee8171c,
    ];
    assert!(ecdsa_verify_secp256r1(&pk310, &z310, &r310, &s310));

    // 307] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #301: r and s^-1 are close to n
    let z311 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r311 = [0xf3b9cac2fc6324d5, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s311 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let pk311 = [
        0xbecee0c133b10e99,
        0x392cef0633a1b8fa,
        0x3acaafa2fcb41349,
        0x083539fbee44625e,
        0x41d4d7616337911e,
        0xe2a402f26326bb7d,
        0x535196770a58047a,
        0x915c1ebe7bf00df8,
    ];
    assert!(ecdsa_verify_secp256r1(&pk311, &z311, &r311, &s311));

    // 308] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #304: point at infinity during verify
    let z312 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r312 = [0x79dce5617e3192a8, 0xde737d56d38bcf42, 0x7fffffffffffffff, 0x7fffffff80000000];
    let s312 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let pk312 = [
        0x60e8ec07dd70f287,
        0x7e2c88fa0239e23f,
        0xe07757e55e6e516f,
        0xb533d4695dd5b8c5,
        0xe29d4eaf009afe47,
        0x881f7d4a39850143,
        0x8456863f33c3a85d,
        0x1b134ee58cc58327,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk312, &z312, &r312, &s312));

    // 309] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #305: edge case for signature malleability
    let z313 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r313 = [0x79dce5617e3192a9, 0xde737d56d38bcf42, 0x7fffffffffffffff, 0x7fffffff80000000];
    let s313 = [0x79dce5617e3192a8, 0xde737d56d38bcf42, 0x7fffffffffffffff, 0x7fffffff80000000];
    let pk313 = [
        0x628c8b4536787b86,
        0x8cbf2c57f9e284de,
        0xd14e1323523bc3aa,
        0xf50d371b91bfb1d7,
        0xb14cbb209f5fa2dd,
        0x1c553c9730405380,
        0x247cd2e7d0c8b129,
        0xf94ad887ac94d527,
    ];
    assert!(ecdsa_verify_secp256r1(&pk313, &z313, &r313, &s313));

    // 310] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #306: edge case for signature malleability
    let z314 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r314 = [0x79dce5617e3192a9, 0xde737d56d38bcf42, 0x7fffffffffffffff, 0x7fffffff80000000];
    let s314 = [0x79dce5617e3192a9, 0xde737d56d38bcf42, 0x7fffffffffffffff, 0x7fffffff80000000];
    let pk314 = [
        0x2eaeb0d857c4d946,
        0x7047c221bafc3a58,
        0x39156ce57a14b04a,
        0x68ec6e298eafe165,
        0x5bb385ac8ca6fb30,
        0x698ed16c426a2733,
        0xfdb39b2324f220a5,
        0x97bed1af17850117,
    ];
    assert!(ecdsa_verify_secp256r1(&pk314, &z314, &r314, &s314));

    // 311] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #307: u1 == 1
    let z315 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r315 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let s315 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let pk315 = [
        0x97bdf6557927c8b8,
        0xb781a0f1b08f6c88,
        0x0fece94019265fef,
        0x69da0364734d2e53,
        0x8b20f71e2a847002,
        0xa933d86ef8abbcce,
        0x3d726960f069ad71,
        0x66d2d3c7dcd518b2,
    ];
    assert!(ecdsa_verify_secp256r1(&pk315, &z315, &r315, &s315));

    // 312] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #308: u1 == n - 1
    let z316 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r316 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let s316 = [0xec15b0c43202d52e, 0xbcb012ea7bf091fc, 0x12bc9e0a6bdd5e1c, 0x44a5ad0ad0636d9f];
    let pk316 = [
        0x87bf067a1ac1ff32,
        0x1a471e2b23206201,
        0x2576e2b63e3e3062,
        0xd8adc00023a8edc0,
        0x32861576ba2362e1,
        0xa09a86b4ea9690aa,
        0xcb36131fff95ed12,
        0x33e2b50ec09807ac,
    ];
    assert!(ecdsa_verify_secp256r1(&pk316, &z316, &r316, &s316));

    // 313] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #309: u2 == 1
    let z317 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r317 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let s317 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let pk317 = [
        0xab3690dbe75ab785,
        0xedca02cfc7b2401f,
        0xfa6d882f03a7d5c7,
        0x3623ac973ced0a56,
        0x2e9bb3252be7f8fe,
        0x3da8e713ba0643b9,
        0x3da7257e737f3979,
        0x8db06908e64b2861,
    ];
    assert!(ecdsa_verify_secp256r1(&pk317, &z317, &r317, &s317));

    // 314] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #310: u2 == n - 1
    let z318 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r318 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let s318 = [0x4d26872ca84218e1, 0x7def51c91a0fbf03, 0xaaaaaaaaaaaaaaaa, 0xaaaaaaaa00000000];
    let pk318 = [
        0x90e5e04263f922f1,
        0x7b31959503b6fa38,
        0xd894b93ff52dc302,
        0xcf04ea77e9622523,
        0x1199bedeaecab2e9,
        0x1740c2f397543882,
        0x3c8b8400e57b4ed7,
        0xe8528fb7c006b398,
    ];
    assert!(ecdsa_verify_secp256r1(&pk318, &z318, &r318, &s318));

    // 315] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #311: edge case for u1
    let z319 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r319 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s319 = [0xfa5d3a8196623397, 0x7e019f0a28721885, 0xa46bcb51dc0b8b4b, 0xe91e1ba60fdedb76];
    let pk319 = [
        0x3e9f78dbeff77350,
        0xe683d49227996bda,
        0x929dc24077b508d7,
        0xdb7a2c8a1ab573e5,
        0x36eaf08a6c99a206,
        0x30cf7cc76a82f11a,
        0xc2e0aadd5a133117,
        0x4f417f3bc9a88075,
    ];
    assert!(ecdsa_verify_secp256r1(&pk319, &z319, &r319, &s319));

    // 316] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #312: edge case for u1
    let z320 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r320 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s320 = [0xc87b59b95b430ad9, 0x24f799e525b1e8e8, 0x94313ba4831b53fe, 0xfdea5843ffeb73af];
    let pk320 = [
        0xb413765ea80b6e1f,
        0xeff994efe9bbd05a,
        0x2f21974dc4752fad,
        0xdead11c7a5b39686,
        0x07aa0318fc7fe1ff,
        0xbb94078a343736df,
        0xcf89cff53c40e265,
        0x1de3f0640e8ac6ed,
    ];
    assert!(ecdsa_verify_secp256r1(&pk320, &z320, &r320, &s320));

    // 317] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #313: edge case for u1
    let z321 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r321 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s321 = [0x3c3dfc5e5bafc035, 0x994e41c5251cd73b, 0x65190db1680d62bb, 0x03ffcabf2f1b4d2a];
    let pk321 = [
        0xe80e00dfde67c7e9,
        0xbb1fea6f994326fb,
        0xaed3a6ef96c18613,
        0xd0bc472e0d7c81eb,
        0x667d1bb9fa619efd,
        0x3ad70ff17ba85335,
        0x389b946f64ad56c8,
        0x986c723ea4843d48,
    ];
    assert!(ecdsa_verify_secp256r1(&pk321, &z321, &r321, &s321));

    // 318] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #314: edge case for u1
    let z322 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r322 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s322 = [0x2847f74977534989, 0xfe4c1a88ae648e0d, 0x4b33dfdb17d0fed0, 0x4dfbc401f971cd30];
    let pk322 = [
        0x78495d458dd51c32,
        0xb2ad03776e02640f,
        0xcb736008b9c08d1a,
        0xa0a44ca947d66a2a,
        0x30a2392e40426add,
        0x294a4762420df43a,
        0x1f1c409dc2d872d4,
        0x6337fe5cf8c4604b,
    ];
    assert!(ecdsa_verify_secp256r1(&pk322, &z322, &r322, &s322));

    // 319] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #315: edge case for u1
    let z323 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r323 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s323 = [0x4971eba9cda5ca71, 0x988977055cd3a8e5, 0x3dfdb17d0fed112b, 0xbc4024761cd2ffd4];
    let pk323 = [
        0xd42b62c3ce8a96b7,
        0x298c25420b775019,
        0x5fb65fad0f602389,
        0xc9c2115290d008b4,
        0xcc3f06e9713973fd,
        0xc9dbefac46f9e601,
        0xd987ca730f0405c2,
        0x3877d25a8080dc02,
    ];
    assert!(ecdsa_verify_secp256r1(&pk323, &z323, &r323, &s323));

    // 320] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #316: edge case for u1
    let z324 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r324 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s324 = [0x9f2a0c909ee86f91, 0x742bf35d128fb345, 0x7bfb62fa1fda2257, 0x788048ed39a5ffa7];
    let pk324 = [
        0xfa83bc1a5ff6033e,
        0x4c0018962f3c5e7e,
        0x66b8bccf1b88e8a2,
        0x5eca1ef4c287dddc,
        0xecaef22f1c934a71,
        0x8d92a607c32cd407,
        0x45abdce8a8e4da75,
        0x5e79c4cb2c245b8c,
    ];
    assert!(ecdsa_verify_secp256r1(&pk324, &z324, &r324, &s324));

    // 321] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #317: edge case for u1
    let z325 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r325 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s325 = [0xdd8b23582b3cb15e, 0x5924b5ed5b11167e, 0x17d0fed112bc9e0a, 0x476d9131fd381bd9];
    let pk325 = [
        0x2d473e317029a47a,
        0x0a01e4130c3f8bf2,
        0x936bc7ab5a96353e,
        0x5caaa030e7fdf0e4,
        0xda926b42b178bef9,
        0xe9b201642005b3ce,
        0x2a20d371e9702254,
        0xdeb6adc462f7058f,
    ];
    assert!(ecdsa_verify_secp256r1(&pk325, &z325, &r325, &s325));

    // 322] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #318: edge case for u1
    let z326 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r326 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s326 = [0xf6cc0a19662d3601, 0xfafa8b19ce78d538, 0x4448d0a8f640fe46, 0x8374253e3e21bd15];
    let pk326 = [
        0x469b1a31f619b098,
        0xf83a1fc3501c8a66,
        0xb8ac0ce69eb1ea20,
        0xc2fd20bac06e555b,
        0xffc87ac397e6cbaf,
        0xca2ed32525c75f27,
        0x5bd7b8d76a25fc95,
        0x6237050779f52b61,
    ];
    assert!(ecdsa_verify_secp256r1(&pk326, &z326, &r326, &s326));

    // 323] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #319: edge case for u1
    let z327 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r327 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s327 = [0x49ae6a2d897a52d6, 0x2c11ee7fe14879e7, 0x3c5b9ede36cba545, 0x357cfd3be4d01d41];
    let pk327 = [
        0x17bedae4bba86ced,
        0x8426e11ea6ae78ce,
        0x0bbe726c37201006,
        0x3fd6a1ca7f77fb3b,
        0xddfc56e0db3c8ff4,
        0x18ad6f50b5461872,
        0xaab8745eac1cd690,
        0x03ce5516406bf8cf,
    ];
    assert!(ecdsa_verify_secp256r1(&pk327, &z327, &r327, &s327));

    // 324] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #320: edge case for u1
    let z328 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r328 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s328 = [0x7cd2f2bc27a0a6d8, 0xdf5225298e6ffc80, 0xa5e8e6b799fd86b8, 0x29798c5c0ee287d4];
    let pk328 = [
        0xe1edf7b086911114,
        0x4989db20e9bca3ed,
        0x624a60d6dc32734e,
        0x9cb8e51e27a5ae3b,
        0x5fc57322b4427544,
        0x410a19f2e277aa89,
        0x36d6556e8ad5f523,
        0xb4c104ab3c677e4b,
    ];
    assert!(ecdsa_verify_secp256r1(&pk328, &z328, &r328, &s328));

    // 325] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #321: edge case for u1
    let z329 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r329 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s329 = [0x7cae4820b30078dd, 0x1f72add1bf52c2ff, 0x2dca1a5711fa3a5a, 0x0b70f22c78109245];
    let pk329 = [
        0xc262512d8f49602a,
        0xbc78ef3d569e1223,
        0x02620b7955bc2b40,
        0xa3e52c156dcaf105,
        0xf192944977df147f,
        0x032355463486164c,
        0x4ad3cc86e57321de,
        0x4a2039f31c109702,
    ];
    assert!(ecdsa_verify_secp256r1(&pk329, &z329, &r329, &s329));

    // 326] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #322: edge case for u1
    let z330 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r330 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s330 = [0xf95c90416600f1ba, 0x3ee55ba37ea585fe, 0x5b9434ae23f474b4, 0x16e1e458f021248a];
    let pk330 = [
        0xc3f3c059b2655e88,
        0x5c37bf91b58a5157,
        0xe8e670fb90010fb1,
        0xf19b78928720d5be,
        0x074abd4329260509,
        0x468560c7cfeb942d,
        0xdcf273f5dc357e58,
        0xcf701ec962fb4a11,
    ];
    assert!(ecdsa_verify_secp256r1(&pk330, &z330, &r330, &s330));

    // 327] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #323: edge case for u1
    let z331 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r331 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s331 = [0x760ad86219016a97, 0x5e5809753df848fe, 0x895e4f0535eeaf0e, 0x2252d6856831b6cf];
    let pk331 = [
        0x4cb89345545c90a8,
        0x37482d242f235d7b,
        0xa5cf52b27a05bb73,
        0x83a744459ecdfb01,
        0x28121f37cc50de6e,
        0xd905df5f3c329458,
        0x3287de9ffe90355f,
        0xc05d49337b964981,
    ];
    assert!(ecdsa_verify_secp256r1(&pk331, &z331, &r331, &s331));

    // 328] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #324: edge case for u1
    let z332 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r332 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s332 = [0x17fbe390ac0972c3, 0xab1a9e39661a3ae0, 0xb28c86d8b406b15d, 0x81ffe55f178da695];
    let pk332 = [
        0x8ae51e5d6f3a21d7,
        0x4b19bbe88cee8e52,
        0xdae124f039dfd23f,
        0xdd13c6b34c56982d,
        0xbdae4bd3b42a45ff,
        0xe4c3345692fb5320,
        0xeb59ca974d039fc0,
        0xbfad4c2e6f263fe5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk332, &z332, &r332, &s332));

    // 329] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #325: edge case for u2
    let z333 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r333 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s333 = [0x513dee40fecbb71a, 0xe9a2538f37b28a2c, 0xffffffffffffffff, 0x7fffffffaaaaaaaa];
    let pk333 = [
        0xeed01b0f3deb7460,
        0xfad636bbf95192fe,
        0x2f65f094e94e5b4d,
        0x67e6f659cdde869a,
        0x3c62886437c38ba0,
        0x85bbe58712c8d923,
        0xb51dfe592f5cfd56,
        0xa37e0a51f258b7ae,
    ];
    assert!(ecdsa_verify_secp256r1(&pk333, &z333, &r333, &s333));

    // 330] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #326: edge case for u2
    let z334 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r334 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s334 = [0xde009e526adf21f2, 0x3ab3cccd0459b201, 0x6de86d42ad8a13da, 0xb62f26b5f2a2b26f];
    let pk334 = [
        0x9617bb367f9ecaaf,
        0x90d05511e8ec1f59,
        0x6545f029932087e4,
        0x2eb6412505aec05c,
        0xf6669af292895cb0,
        0x6a43fedcddb31830,
        0x3f9b1ae0124890f0,
        0x805f51efcc480340,
    ];
    assert!(ecdsa_verify_secp256r1(&pk334, &z334, &r334, &s334));

    // 331] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #327: edge case for u2
    let z335 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r335 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s335 = [0x0686aa7b4c90851e, 0xd57b38bb61403d70, 0xd02bbbe749bd351c, 0xbb1d9ac949dd748c];
    let pk335 = [
        0x0a854625fe0d7f35,
        0x5435e3a6b68d75a5,
        0x3a9fd80e056e2e85,
        0x84db645868eab35e,
        0xbf43a2ee39338cfe,
        0xbf92e72171570ef7,
        0x11ef3e075eddda9a,
        0x6d2589ac655edc9a,
    ];
    assert!(ecdsa_verify_secp256r1(&pk335, &z335, &r335, &s335));

    // 332] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #328: edge case for u2
    let z336 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r336 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s336 = [0x818f725b4f60aaf2, 0xe52545dac11f816e, 0x1c732513ca0234ec, 0x66755a00638cdaec];
    let pk336 = [
        0x7eb2975e386ad663,
        0x6aa5059b7a2ff763,
        0xd75c0983b22ca8ea,
        0x91b9e47c56278662,
        0x08302a16854ecfbd,
        0xd13c3c0310679c14,
        0x18d6d11dc062165f,
        0x49aa8ff283d0f77c,
    ];
    assert!(ecdsa_verify_secp256r1(&pk336, &z336, &r336, &s336));

    // 333] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #329: edge case for u2
    let z337 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r337 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s337 = [0x8ca48e982beb3669, 0xe98ebe492fdf02e4, 0x32513ca0234ecfff, 0x55a00c9fcdaebb60];
    let pk337 = [
        0x27f7ec5ee8e4834d,
        0xd4dc6b0a9e802e53,
        0x92b47fb4c5311fb6,
        0xf3ec2f13caf04d01,
        0x38ac321fefe5a432,
        0x531df87efdb47c13,
        0x67d6ecfe81e2b0f9,
        0xf97e3e468b7d0db8,
    ];
    assert!(ecdsa_verify_secp256r1(&pk337, &z337, &r337, &s337));

    // 334] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #330: edge case for u2
    let z338 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r338 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s338 = [0x19491d3057d66cd2, 0xd31d7c925fbe05c9, 0x64a27940469d9fff, 0xab40193f9b5d76c0];
    let pk338 = [
        0x3e4693c670fccc88,
        0x3180235b8f46b450,
        0x7dafd9acaf2fa10b,
        0xd92b200aefcab6ac,
        0xdc85b6b8ab922c72,
        0xefb7352d27e4ccca,
        0x75336256768f7c19,
        0x5ef2f3aebf5b3174,
    ];
    assert!(ecdsa_verify_secp256r1(&pk338, &z338, &r338, &s338));

    // 335] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #331: edge case for u2
    let z339 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r339 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s339 = [0xa26b4408d0dc8600, 0xcb0dadbbc7f549f8, 0xca0234ecffffffff, 0xca0234ebb5fdcb13];
    let pk339 = [
        0x140a3bcd881523cd,
        0x96bf179b3d76fc48,
        0x625b38e5f98bbabb,
        0x0a88361eb92ecca2,
        0x6edebf47298ad489,
        0x6aa2c96b86a41ccf,
        0x54035597375d9086,
        0xe6bdf56033f84a50,
    ];
    assert!(ecdsa_verify_secp256r1(&pk339, &z339, &r339, &s339));

    // 336] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #332: edge case for u2
    let z340 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r340 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s340 = [0x8711c77298815ad3, 0x19933a9e65b28559, 0x082b9310572620ae, 0xbfffffff3ea3677e];
    let pk340 = [
        0x2ba50469d84375e8,
        0x6e2b20e7f14a563a,
        0x7e0c1afc5d8d8036,
        0xd0fb17ccd8fafe82,
        0x1e236a7de7637d93,
        0x9ac602cc6349cf8c,
        0xf554355564646de9,
        0x68612569d39e2bb9,
    ];
    assert!(ecdsa_verify_secp256r1(&pk340, &z340, &r340, &s340));

    // 337] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #333: edge case for u2
    let z341 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r341 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s341 = [0x8f055d86e5cc41f4, 0x5b37902e023fab7c, 0xe666666666666666, 0x266666663bbbbbbb];
    let pk341 = [
        0x2b1e4309d3edb276,
        0xac4181076c9af0a2,
        0x3abbcef0d91f11e2,
        0x836f33bbc1dc0d3d,
        0x36f3a95bbe881f75,
        0xec2b0cb8120d7602,
        0xc773867582997c2b,
        0x9ab443ff6f901e30,
    ];
    assert!(ecdsa_verify_secp256r1(&pk341, &z341, &r341, &s341));

    // 338] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #334: edge case for u2
    let z342 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r342 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s342 = [0x08a443e258970b09, 0x146c573f4c6dfc8d, 0xa492492492492492, 0xbfffffff36db6db7];
    let pk342 = [
        0x5103cb33e55feeb8,
        0x1237034dec8d72ba,
        0x99719baee4b43274,
        0x92f99fbe973ed4a2,
        0x7a794cebd6e69697,
        0x1ac05767289280ee,
        0x174889f3ebcf1b7a,
        0x033dd0e91134c734,
    ];
    assert!(ecdsa_verify_secp256r1(&pk342, &z342, &r342, &s342));

    // 339] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #335: edge case for u2
    let z343 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r343 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s343 = [0xcb1ad3a27cfd49c4, 0xc815d0e60b3e596e, 0x7fffffffffffffff, 0xbfffffff2aaaaaab];
    let pk343 = [
        0x9d130bba434af09e,
        0xd12cffd73ebbb204,
        0x78e618ec0fa7e2e2,
        0xd35ba58da30197d3,
        0x82874c794635c1d2,
        0xc77cbb3c47919f8e,
        0xa432b7585a49b3a6,
        0xff83986e6875e41e,
    ];
    assert!(ecdsa_verify_secp256r1(&pk343, &z343, &r343, &s343));

    // 340] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #336: edge case for u2
    let z344 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r344 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s344 = [0xa27bdc81fd976e37, 0xd344a71e6f651458, 0xffffffffffffffff, 0x7fffffff55555555];
    let pk344 = [
        0xab0725c8d0793224,
        0x36697334a519d7dd,
        0x3f3ff475149be291,
        0x8651ce490f1b46d7,
        0x900bd825f590cc28,
        0x51ce21dd9003ae60,
        0xbc9ae82911f0b527,
        0xe11c65bd8ca92dc8,
    ];
    assert!(ecdsa_verify_secp256r1(&pk344, &z344, &r344, &s344));

    // 341] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #337: edge case for u2
    let z345 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r345 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s345 = [0x79dce5617e3192aa, 0xde737d56d38bcf42, 0x7fffffffffffffff, 0x3fffffff80000000];
    let pk345 = [
        0xcdaca9826b9cfc6d,
        0xd921d9e2f72b15b1,
        0x8795650ff95f101e,
        0x6d8e1b12c831a0da,
        0xa58c106ad486bf37,
        0xe6c7a6a637b20469,
        0x70394a4bc9f892d5,
        0xef6d63e2bc5c0895,
    ];
    assert!(ecdsa_verify_secp256r1(&pk345, &z345, &r345, &s345));

    // 342] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #338: edge case for u2
    let z346 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r346 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s346 = [0x0343553da648428f, 0x6abd9c5db0a01eb8, 0x6815ddf3a4de9a8e, 0x5d8ecd64a4eeba46];
    let pk346 = [
        0xff24cb4d920e1542,
        0xca9a410f627a0f7d,
        0x2997cbdbb0922328,
        0x0ae580bae933b4ef,
        0x8ba83c3949d893e3,
        0x2b99e309d8dcd9a9,
        0x88eb81421a361ccc,
        0x8911e7f8cc365a8a,
    ];
    assert!(ecdsa_verify_secp256r1(&pk346, &z346, &r346, &s346));

    // 343] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #339: point duplication during verification
    let z347 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r347 = [0x05ea6c42c4934569, 0x8c4aacafdfb6bcbe, 0x8fe0555ac3bc9904, 0x6f2347cab7dd7685];
    let s347 = [0x4d53f4301047856b, 0x435109cf9a15dd62, 0x9957a61e76e00c2c, 0xbb726660235793aa];
    let pk347 = [
        0x0e134c027fc46963,
        0x6983b442d2444fe7,
        0x835a849cce6fbdeb,
        0x5b812fd521aafa69,
        0x4e15eba5499249e9,
        0x38550ce672ce8b8d,
        0x004e92d8d940cf56,
        0x838a40f2a36092e9,
    ];
    assert!(ecdsa_verify_secp256r1(&pk347, &z347, &r347, &s347));

    // 344] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #340: duplication bug
    let z348 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r348 = [0x05ea6c42c4934569, 0x8c4aacafdfb6bcbe, 0x8fe0555ac3bc9904, 0x6f2347cab7dd7685];
    let s348 = [0x4d53f4301047856b, 0x435109cf9a15dd62, 0x9957a61e76e00c2c, 0xbb726660235793aa];
    let pk348 = [
        0x0e134c027fc46963,
        0x6983b442d2444fe7,
        0x835a849cce6fbdeb,
        0x5b812fd521aafa69,
        0xb1ea145ab66db616,
        0xc7aaf31a8d317472,
        0xffb16d2726bf30a9,
        0x7c75bf0c5c9f6d17,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk348, &z348, &r348, &s348));

    // 345] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #343: comparison with point at infinity
    let z349 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r349 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let s349 = [0x63f1f55a327a3aa9, 0x25c7cbbc549e52e7, 0x3333333333333333, 0x3333333300000000];
    let pk349 = [
        0x1a633db76665d250,
        0x3ff467f11ebd98a5,
        0x11083b78002081c5,
        0xdd86d3b5f4a13e85,
        0x1b7e17474ebc18f7,
        0x8dfaed6ff8d5cb3e,
        0x10d849349226d21d,
        0x45d5c8200c89f2fa,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk349, &z349, &r349, &s349));

    // 346] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #344: extreme value for k and edgecase s
    let z350 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r350 = [0xa60b48fc47669978, 0xc08969e277f21b35, 0x8a52380304b51ac3, 0x7cf27b188d034f7e];
    let s350 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let pk350 = [
        0x6591a93f5a0fbcc5,
        0x4b0f5a516e578c01,
        0x0c12c4cd0abfb4e6,
        0x4fea55b32cb32aca,
        0x85ed3be62ce4b280,
        0xf0fecd38a8a4b2c7,
        0x547b212f6bb14c88,
        0xd7d3fd10b2be668c,
    ];
    assert!(ecdsa_verify_secp256r1(&pk350, &z350, &r350, &s350));

    // 347] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #345: extreme value for k and s^-1
    let z351 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r351 = [0xa60b48fc47669978, 0xc08969e277f21b35, 0x8a52380304b51ac3, 0x7cf27b188d034f7e];
    let s351 = [0x1bcdd9f8fd6b63cc, 0x625bd7a09bec4ca8, 0x4924924924924924, 0xb6db6db624924925];
    let pk351 = [
        0x299802e32d7c3107,
        0xf32b7f98af669ead,
        0x92170a6f8eee735b,
        0xc6a7715270242277,
        0x412f726867db589e,
        0x61fe3a073e2ffd78,
        0xbd343572b3e56192,
        0xbc3b4b5e65ab887b,
    ];
    assert!(ecdsa_verify_secp256r1(&pk351, &z351, &r351, &s351));

    // 348] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #346: extreme value for k and s^-1
    let z352 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r352 = [0xa60b48fc47669978, 0xc08969e277f21b35, 0x8a52380304b51ac3, 0x7cf27b188d034f7e];
    let s352 = [0x8fc7d568c9e8eaa7, 0x971f2ef152794b9d, 0xcccccccccccccccc, 0xcccccccc00000000];
    let pk352 = [
        0x8de85a7d15b956ef,
        0xd6ec6d59b207fec9,
        0x7a9af99f49f03644,
        0x851c2bbad08e54ec,
        0x319f10ddeb0fe9d6,
        0x4b91aa2379f60727,
        0x684b410be8d0f749,
        0xcee9960283045075,
    ];
    assert!(ecdsa_verify_secp256r1(&pk352, &z352, &r352, &s352));

    // 349] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #347: extreme value for k and s^-1
    let z353 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r353 = [0xa60b48fc47669978, 0xc08969e277f21b35, 0x8a52380304b51ac3, 0x7cf27b188d034f7e];
    let s353 = [0x63f1f55a327a3aaa, 0x25c7cbbc549e52e7, 0x3333333333333333, 0x3333333300000000];
    let pk353 = [
        0x3061205acb19c48f,
        0x55911ff68318d1bf,
        0x88676949e53da7fc,
        0xf6417c8a670584e3,
        0x9ead43026ab6d43f,
        0x4779cd9ac916c366,
        0x2674acb750592978,
        0x8f2b743df34ad0f7,
    ];
    assert!(ecdsa_verify_secp256r1(&pk353, &z353, &r353, &s353));

    // 350] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #348: extreme value for k and s^-1
    let z354 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r354 = [0xa60b48fc47669978, 0xc08969e277f21b35, 0x8a52380304b51ac3, 0x7cf27b188d034f7e];
    let s354 = [0xd7ebf0c9fef7c185, 0x5a8b230d0b2b51dc, 0xb6db6db6db6db6db, 0x49249248db6db6db];
    let pk354 = [
        0x3f557faa7f8a0643,
        0x032565af420cf337,
        0xefec6c639930d636,
        0x501421277be45a5e,
        0x89cad195d0aa1371,
        0xac08d74501f2ae6e,
        0xcdc7dfe7384c8e5c,
        0x8673d6cb6076e1cf,
    ];
    assert!(ecdsa_verify_secp256r1(&pk354, &z354, &r354, &s354));

    // 351] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #349: extreme value for k
    let z355 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r355 = [0xa60b48fc47669978, 0xc08969e277f21b35, 0x8a52380304b51ac3, 0x7cf27b188d034f7e];
    let s355 = [0x6e0478c34416e3bb, 0x1584d13e18411e2f, 0xc82cbc9d1edd8c98, 0x16a4502e2781e11a];
    let pk355 = [
        0x3415ac84e808bb34,
        0x23ee01a4894adf0e,
        0x27735f729ca8a4ca,
        0x0d935bf9ffc115a5,
        0x50ce61d82eba33c5,
        0x70c3050893a43758,
        0x38912bd9ea6c4fde,
        0x3195a3762fea29ed,
    ];
    assert!(ecdsa_verify_secp256r1(&pk355, &z355, &r355, &s355));

    // 352] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #350: extreme value for k and edgecase s
    let z356 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r356 = [0xf4a13945d898c296, 0x77037d812deb33a0, 0xf8bce6e563a440f2, 0x6b17d1f2e12c4247];
    let s356 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let pk356 = [
        0x41e748e64e4dca21,
        0xb668fb670196206c,
        0xa589355014308e60,
        0x5e59f50708646be8,
        0xef38e213624a01de,
        0xeeeafbdf03aacbaf,
        0x7144d5b459982f52,
        0x5de37fee5c97bcaf,
    ];
    assert!(ecdsa_verify_secp256r1(&pk356, &z356, &r356, &s356));

    // 353] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #351: extreme value for k and s^-1
    let z357 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r357 = [0xf4a13945d898c296, 0x77037d812deb33a0, 0xf8bce6e563a440f2, 0x6b17d1f2e12c4247];
    let s357 = [0x1bcdd9f8fd6b63cc, 0x625bd7a09bec4ca8, 0x4924924924924924, 0xb6db6db624924925];
    let pk357 = [
        0xbfe924104b02db8e,
        0x2fd6226f7ef90ef0,
        0xff2f7a5b5445da9e,
        0x169fb797325843fa,
        0xb861b131d8a1d667,
        0x46d581d68878efb2,
        0xcf9b22f7a2e582bd,
        0x7bbb8de662c7b9b1,
    ];
    assert!(ecdsa_verify_secp256r1(&pk357, &z357, &r357, &s357));

    // 354] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #352: extreme value for k and s^-1
    let z358 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r358 = [0xf4a13945d898c296, 0x77037d812deb33a0, 0xf8bce6e563a440f2, 0x6b17d1f2e12c4247];
    let s358 = [0x8fc7d568c9e8eaa7, 0x971f2ef152794b9d, 0xcccccccccccccccc, 0xcccccccc00000000];
    let pk358 = [
        0xf8b7b54898148754,
        0xef2f7023d18affda,
        0x6b62d4e9e4ca885a,
        0x271cd89c00014309,
        0x02b2ca47fe8e4da5,
        0x81a609b9149ccb4b,
        0x35b55fa385b0f764,
        0x0a1c6e954e321084,
    ];
    assert!(ecdsa_verify_secp256r1(&pk358, &z358, &r358, &s358));

    // 355] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #353: extreme value for k and s^-1
    let z359 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r359 = [0xf4a13945d898c296, 0x77037d812deb33a0, 0xf8bce6e563a440f2, 0x6b17d1f2e12c4247];
    let s359 = [0x63f1f55a327a3aaa, 0x25c7cbbc549e52e7, 0x3333333333333333, 0x3333333300000000];
    let pk359 = [
        0x587a220afe499c12,
        0xb1563a9ab84bf524,
        0x7ddb46ebc1ed799a,
        0x3d0bc7ed8f09d2cb,
        0x4f78cb216fa3f8df,
        0xabf19ce7d68aa624,
        0x4f378d96adb0a408,
        0xe22dc3b3c103824a,
    ];
    assert!(ecdsa_verify_secp256r1(&pk359, &z359, &r359, &s359));

    // 356] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #354: extreme value for k and s^-1
    let z360 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r360 = [0xf4a13945d898c296, 0x77037d812deb33a0, 0xf8bce6e563a440f2, 0x6b17d1f2e12c4247];
    let s360 = [0xd7ebf0c9fef7c185, 0x5a8b230d0b2b51dc, 0xb6db6db6db6db6db, 0x49249248db6db6db];
    let pk360 = [
        0x721bcbd23663a9b7,
        0xb281797fa701288c,
        0xf9bb010d066974ab,
        0xa6c885ade1a4c566,
        0xf2058458f98af316,
        0x04a9c7d467e007e1,
        0x193a6096fc77a2b0,
        0x2e424b690957168d,
    ];
    assert!(ecdsa_verify_secp256r1(&pk360, &z360, &r360, &s360));

    // 357] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #355: extreme value for k
    let z361 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r361 = [0xf4a13945d898c296, 0x77037d812deb33a0, 0xf8bce6e563a440f2, 0x6b17d1f2e12c4247];
    let s361 = [0x6e0478c34416e3bb, 0x1584d13e18411e2f, 0xc82cbc9d1edd8c98, 0x16a4502e2781e11a];
    let pk361 = [
        0x30c3bdbad9ebfa5c,
        0x5bf75df62d87ab73,
        0x289e6ac3812572a2,
        0x8d3c2c2c3b765ba8,
        0x632e0b780c423f5d,
        0xcaa1621d1af241d4,
        0x238578d43aec54f7,
        0x4c6845442d66935b,
    ];
    assert!(ecdsa_verify_secp256r1(&pk361, &z361, &r361, &s361));

    // 358] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #356: testing point duplication
    let z362 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r362 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let s362 = [0x6bf5f864ff7be0c2, 0xad4591868595a8ee, 0xdb6db6db6db6db6d, 0x249249246db6db6d];
    let pk362 = [
        0xf4a13945d898c296,
        0x77037d812deb33a0,
        0xf8bce6e563a440f2,
        0x6b17d1f2e12c4247,
        0xcbb6406837bf51f5,
        0x2bce33576b315ece,
        0x8ee7eb4a7c0f9e16,
        0x4fe342e2fe1a7f9b,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk362, &z362, &r362, &s362));

    // 359] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #357: testing point duplication
    let z363 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r363 = [0xec15b0c43202d52e, 0xbcb012ea7bf091fc, 0x12bc9e0a6bdd5e1c, 0x44a5ad0ad0636d9f];
    let s363 = [0x6bf5f864ff7be0c2, 0xad4591868595a8ee, 0xdb6db6db6db6db6d, 0x249249246db6db6d];
    let pk363 = [
        0xf4a13945d898c296,
        0x77037d812deb33a0,
        0xf8bce6e563a440f2,
        0x6b17d1f2e12c4247,
        0xcbb6406837bf51f5,
        0x2bce33576b315ece,
        0x8ee7eb4a7c0f9e16,
        0x4fe342e2fe1a7f9b,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk363, &z363, &r363, &s363));

    // 360] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #358: testing point duplication
    let z364 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r364 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let s364 = [0x6bf5f864ff7be0c2, 0xad4591868595a8ee, 0xdb6db6db6db6db6d, 0x249249246db6db6d];
    let pk364 = [
        0xf4a13945d898c296,
        0x77037d812deb33a0,
        0xf8bce6e563a440f2,
        0x6b17d1f2e12c4247,
        0x3449bf97c840ae0a,
        0xd431cca994cea131,
        0x711814b583f061e9,
        0xb01cbd1c01e58065,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk364, &z364, &r364, &s364));

    // 361] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #359: testing point duplication
    let z365 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r365 = [0xec15b0c43202d52e, 0xbcb012ea7bf091fc, 0x12bc9e0a6bdd5e1c, 0x44a5ad0ad0636d9f];
    let s365 = [0x6bf5f864ff7be0c2, 0xad4591868595a8ee, 0xdb6db6db6db6db6d, 0x249249246db6db6d];
    let pk365 = [
        0xf4a13945d898c296,
        0x77037d812deb33a0,
        0xf8bce6e563a440f2,
        0x6b17d1f2e12c4247,
        0x3449bf97c840ae0a,
        0xd431cca994cea131,
        0x711814b583f061e9,
        0xb01cbd1c01e58065,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk365, &z365, &r365, &s365));

    // 362] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #360: pseudorandom signature
    let z366 = [0xa495991b7852b855, 0x27ae41e4649b934c, 0x9afbf4c8996fb924, 0xe3b0c44298fc1c14];
    let r366 = [0x8e76e09d8770b34a, 0x42d16e47f219f9e9, 0x7a305c951c0dcbcc, 0xb292a619339f6e56];
    let s366 = [0xab2abebdf89a62e2, 0xe59ec2a17ce5bd2d, 0x2f76f07bfe3661bd, 0x0177e60492c5a824];
    let pk366 = [
        0x5b522eba7240fad5,
        0x32e41495a944d004,
        0x13fb8a9e64da3b86,
        0x04aaec73635726f2,
        0x1aa546e8365d525d,
        0xaaf7b4e09fc81d6d,
        0xba01775787ced05e,
        0x87d9315798aaa3a5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk366, &z366, &r366, &s366));

    // 363] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #361: pseudorandom signature
    let z367 = [0xf59ec9dd1bb8c7b3, 0xe807bdf4c5332f19, 0x2856e7be399007c9, 0xdc1921946f4af96a];
    let r367 = [0xf5b8d2a2a6538e23, 0xcfbf33afe66dbadc, 0xba897f6b5fb59695, 0x530bd6b0c9af2d69];
    let s367 = [0xe934c72caa3f43e9, 0x0987e3e3f0f242ca, 0x55ededcedbf4cc0c, 0xd85e489cb7a161fd];
    let pk367 = [
        0x5b522eba7240fad5,
        0x32e41495a944d004,
        0x13fb8a9e64da3b86,
        0x04aaec73635726f2,
        0x1aa546e8365d525d,
        0xaaf7b4e09fc81d6d,
        0xba01775787ced05e,
        0x87d9315798aaa3a5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk367, &z367, &r367, &s367));

    // 364] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #362: pseudorandom signature
    let z368 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r368 = [0x73d7904519e51388, 0x2711f9917060406a, 0x381c4c1f1da8e9de, 0xa8ea150cb80125d7];
    let s368 = [0x7288293285449b86, 0x0c22c9d76ec21725, 0xa73b2d40480c2ba5, 0xf3ab9fa68bd47973];
    let pk368 = [
        0x5b522eba7240fad5,
        0x32e41495a944d004,
        0x13fb8a9e64da3b86,
        0x04aaec73635726f2,
        0x1aa546e8365d525d,
        0xaaf7b4e09fc81d6d,
        0xba01775787ced05e,
        0x87d9315798aaa3a5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk368, &z368, &r368, &s368));

    // 365] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #363: pseudorandom signature
    let z369 = [0x7f1b40c4cbd36f90, 0x93262cf06340c4fa, 0xdbb5f2c353e632c3, 0xde47c9b27eb8d300];
    let r369 = [0xc69178490d57fb71, 0x39aaf63f00a91f29, 0xe5aada139f52b705, 0x986e65933ef2ed4e];
    let s369 = [0x0f701aaa7a694b9c, 0xdabf0c0217d1c0ff, 0x372308cbf1489bbb, 0x3dafedfb8da6189d];
    let pk369 = [
        0x5b522eba7240fad5,
        0x32e41495a944d004,
        0x13fb8a9e64da3b86,
        0x04aaec73635726f2,
        0x1aa546e8365d525d,
        0xaaf7b4e09fc81d6d,
        0xba01775787ced05e,
        0x87d9315798aaa3a5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk369, &z369, &r369, &s369));

    // 366] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #364: x-coordinate of the public key has many trailing 0's
    let z370 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r370 = [0x7f29745eff3569f1, 0x50dd0fd5defa013c, 0x81e353a3565e4825, 0xd434e262a49eab77];
    let s370 = [0x844218305c6ba17a, 0x98953195d7bc10de, 0x52fd8077be769c2b, 0x9b0c0a93f267fb60];
    let pk370 = [
        0x9fbd816900000000,
        0xf3807eca11738023,
        0x05e4f1600ae2849d,
        0x4f337ccfd67726a8,
        0xd5df7ea60666d685,
        0x7eb504af43a3146c,
        0x416411e988c30f42,
        0xed9dea124cc8c396,
    ];
    assert!(ecdsa_verify_secp256r1(&pk370, &z370, &r370, &s370));

    // 367] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #365: x-coordinate of the public key has many trailing 0's
    let z371 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r371 = [0xadd0be9b1979110b, 0x1463489221bf0a33, 0xf76d79fd7a772e42, 0x0fe774355c04d060];
    let s371 = [0xac6181175df55737, 0x4ca8b91a1f325f3f, 0x43fa4f57f743ce12, 0x500dcba1c69a8fbd];
    let pk371 = [
        0x9fbd816900000000,
        0xf3807eca11738023,
        0x05e4f1600ae2849d,
        0x4f337ccfd67726a8,
        0xd5df7ea60666d685,
        0x7eb504af43a3146c,
        0x416411e988c30f42,
        0xed9dea124cc8c396,
    ];
    assert!(ecdsa_verify_secp256r1(&pk371, &z371, &r371, &s371));

    // 368] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #366: x-coordinate of the public key has many trailing 0's
    let z372 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r372 = [0xbfd06595ee1135e3, 0x8e3b2cd79693f125, 0x950c7d39f03d36dc, 0xbb40bf217bed3fb3];
    let s372 = [0xfa4780745bb55677, 0xc89a1e291ac692b3, 0x32710bdb6a1bf1bf, 0x541bf3532351ebb0];
    let pk372 = [
        0x9fbd816900000000,
        0xf3807eca11738023,
        0x05e4f1600ae2849d,
        0x4f337ccfd67726a8,
        0xd5df7ea60666d685,
        0x7eb504af43a3146c,
        0x416411e988c30f42,
        0xed9dea124cc8c396,
    ];
    assert!(ecdsa_verify_secp256r1(&pk372, &z372, &r372, &s372));

    // 369] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #367: y-coordinate of the public key has many trailing 0's
    let z373 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r373 = [0x556d3e75a233e73a, 0x05badd5ca99231ff, 0xdf3c86ea31389a54, 0x664eb7ee6db84a34];
    let s373 = [0x2e51a2901426a1bd, 0xe0badc678754b8f7, 0x137642490a51560c, 0x59f3c752e52eca46];
    let pk373 = [
        0x5c3f497265004935,
        0x618f06b8ff87e801,
        0xd499a07873fac281,
        0x3cf03d614d8939cf,
        0xe59cdbde80000000,
        0x7c7a1338a82f85a9,
        0x2ce3880a8960dd2a,
        0x84fa174d791c72bf,
    ];
    assert!(ecdsa_verify_secp256r1(&pk373, &z373, &r373, &s373));

    // 370] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #368: y-coordinate of the public key has many trailing 0's
    let z374 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r374 = [0x01985a79d1fd8b43, 0x9c3e42e2d1631fd0, 0x009d6fcd843d4ce3, 0x4cd0429bbabd2827];
    let s374 = [0xe466189d2acdabe3, 0xb7bca77a1a2b869a, 0xbe7ef1d0e0d98f08, 0x9638bf12dd682f60];
    let pk374 = [
        0x5c3f497265004935,
        0x618f06b8ff87e801,
        0xd499a07873fac281,
        0x3cf03d614d8939cf,
        0xe59cdbde80000000,
        0x7c7a1338a82f85a9,
        0x2ce3880a8960dd2a,
        0x84fa174d791c72bf,
    ];
    assert!(ecdsa_verify_secp256r1(&pk374, &z374, &r374, &s374));

    // 371] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #369: y-coordinate of the public key has many trailing 0's
    let z375 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r375 = [0x1e8added97c56c04, 0x60e3ce9aed5e5fd4, 0x1c44d8b6cb62b9f4, 0xe56c6ea2d1b01709];
    let s375 = [0x7fc1378180f89b55, 0x4fcf2b8025807820, 0xbe20b457e463440b, 0xa308ec31f281e955];
    let pk375 = [
        0x5c3f497265004935,
        0x618f06b8ff87e801,
        0xd499a07873fac281,
        0x3cf03d614d8939cf,
        0xe59cdbde80000000,
        0x7c7a1338a82f85a9,
        0x2ce3880a8960dd2a,
        0x84fa174d791c72bf,
    ];
    assert!(ecdsa_verify_secp256r1(&pk375, &z375, &r375, &s375));

    // 372] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #370: y-coordinate of the public key has many trailing 1's
    let z376 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r376 = [0x011f8fbbf3466830, 0x57c176356a2624fb, 0xcabed3346d891eee, 0x1158a08d291500b4];
    let s376 = [0xa46798c18f285519, 0xc91f378b75d487dd, 0xe082325b85290c5b, 0x228a8c486a736006];
    let pk376 = [
        0x5c3f497265004935,
        0x618f06b8ff87e801,
        0xd499a07873fac281,
        0x3cf03d614d8939cf,
        0x1a6324217fffffff,
        0x8385ecc857d07a56,
        0xd31c77f5769f22d5,
        0x7b05e8b186e38d41,
    ];
    assert!(ecdsa_verify_secp256r1(&pk376, &z376, &r376, &s376));

    // 373] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #371: y-coordinate of the public key has many trailing 1's
    let z377 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r377 = [0x3e0dde56d309fa9d, 0x2687b29176939dd2, 0x0ea36b0c0fc8d6aa, 0xb1db9289649f5941];
    let s377 = [0x4e1c3f48a1251336, 0x3a6d1af5c23c7d58, 0x5b0dbd987366dcf4, 0x3e1535e428055901];
    let pk377 = [
        0x5c3f497265004935,
        0x618f06b8ff87e801,
        0xd499a07873fac281,
        0x3cf03d614d8939cf,
        0x1a6324217fffffff,
        0x8385ecc857d07a56,
        0xd31c77f5769f22d5,
        0x7b05e8b186e38d41,
    ];
    assert!(ecdsa_verify_secp256r1(&pk377, &z377, &r377, &s377));

    // 374] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #372: y-coordinate of the public key has many trailing 1's
    let z378 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r378 = [0x0ac6f0ca4e24ed86, 0x0a341a79f2dd1a22, 0x446aa8d4e6e7578b, 0xb7b16e762286cb96];
    let s378 = [0x5e55234ecb8f12bc, 0x1780146df799ccf5, 0x661c547d07bbb072, 0xddc60a700a139b04];
    let pk378 = [
        0x5c3f497265004935,
        0x618f06b8ff87e801,
        0xd499a07873fac281,
        0x3cf03d614d8939cf,
        0x1a6324217fffffff,
        0x8385ecc857d07a56,
        0xd31c77f5769f22d5,
        0x7b05e8b186e38d41,
    ];
    assert!(ecdsa_verify_secp256r1(&pk378, &z378, &r378, &s378));

    // 375] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #373: x-coordinate of the public key has many trailing 1's
    let z379 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r379 = [0xd1c91c670d9105b4, 0xd796edad36bc6e6b, 0xc8e00d8df963ff35, 0xd82a7c2717261187];
    let s379 = [0x680d07debd139929, 0x351ecd5988efb23f, 0xf4603e7cbac0f3c0, 0x3dcabddaf8fcaa61];
    let pk379 = [
        0xdfa5ff8effffffff,
        0x45956ebcfe8ad0f6,
        0x344ed94bca3fcd05,
        0x2829c31faa2e400e,
        0xeb3bd37ebeb9222e,
        0x4113099052df57e7,
        0x5855afa7676ade28,
        0xa01aafaf000e5258,
    ];
    assert!(ecdsa_verify_secp256r1(&pk379, &z379, &r379, &s379));

    // 376] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #374: x-coordinate of the public key has many trailing 1's
    let z380 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r380 = [0x6a5cba063254af78, 0x7787802baff30ce9, 0x3d5befe719f462d7, 0x5eb9c8845de68eb1];
    let s380 = [0x2b87ddbe2ef66fb5, 0x44972186228ee9a6, 0x7ca0ff9bbd92fb6e, 0x2c026ae9be2e2a5e];
    let pk380 = [
        0xdfa5ff8effffffff,
        0x45956ebcfe8ad0f6,
        0x344ed94bca3fcd05,
        0x2829c31faa2e400e,
        0xeb3bd37ebeb9222e,
        0x4113099052df57e7,
        0x5855afa7676ade28,
        0xa01aafaf000e5258,
    ];
    assert!(ecdsa_verify_secp256r1(&pk380, &z380, &r380, &s380));

    // 377] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #375: x-coordinate of the public key has many trailing 1's
    let z381 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r381 = [0x404a8e4e36230c28, 0xf277921becc117d0, 0xf3b782b170239f90, 0x96843dd03c22abd2];
    let s381 = [0x19e1ede123dd991d, 0x9a31214eb4d7e6db, 0x43f67165976de9ed, 0xf2be378f526f74a5];
    let pk381 = [
        0xdfa5ff8effffffff,
        0x45956ebcfe8ad0f6,
        0x344ed94bca3fcd05,
        0x2829c31faa2e400e,
        0xeb3bd37ebeb9222e,
        0x4113099052df57e7,
        0x5855afa7676ade28,
        0xa01aafaf000e5258,
    ];
    assert!(ecdsa_verify_secp256r1(&pk381, &z381, &r381, &s381));

    // 378] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #376: x-coordinate of the public key is large
    let z382 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r382 = [0x3b760297067421f6, 0x4d27e9d98edc2d0e, 0x6f9996af72933946, 0x766456dce1857c90];
    let s382 = [0x3646bfbbf19d0b41, 0x4e55376eced699e9, 0x81dccaf5d19037ec, 0x402385ecadae0d80];
    let pk382 = [
        0x08318c4ca9a7a4f5,
        0x65ff9059ad6aac07,
        0x0458dd8f9e738f26,
        0xfffffff948081e6a,
        0x78e6529a1663bd73,
        0xe0c0fb89557ad0bf,
        0x311ee54149b973ca,
        0x5a8abcba2dda8474,
    ];
    assert!(ecdsa_verify_secp256r1(&pk382, &z382, &r382, &s382));

    // 379] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #377: x-coordinate of the public key is large
    let z383 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r383 = [0x9c34f777de7b9fd9, 0xb97ed8b07cced0b1, 0x19e6518a11b2dbc2, 0xc605c4b2edeab204];
    let s383 = [0xff5e159d47326dba, 0xb2cde2eda700fb1c, 0xc719647bc8af1b29, 0xedf0f612c5f46e03];
    let pk383 = [
        0x08318c4ca9a7a4f5,
        0x65ff9059ad6aac07,
        0x0458dd8f9e738f26,
        0xfffffff948081e6a,
        0x78e6529a1663bd73,
        0xe0c0fb89557ad0bf,
        0x311ee54149b973ca,
        0x5a8abcba2dda8474,
    ];
    assert!(ecdsa_verify_secp256r1(&pk383, &z383, &r383, &s383));

    // 380] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #378: x-coordinate of the public key is large
    let z384 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r384 = [0xb732bfe3b7eb8a84, 0x10e64485d9929ad7, 0xf6141c9ac54141f2, 0xd48b68e6cabfe03c];
    let s384 = [0x08f0772315b6c941, 0x4508c389109ad2f2, 0x19dc26f9b7e2265e, 0xfeedae50c61bd00e];
    let pk384 = [
        0x08318c4ca9a7a4f5,
        0x65ff9059ad6aac07,
        0x0458dd8f9e738f26,
        0xfffffff948081e6a,
        0x78e6529a1663bd73,
        0xe0c0fb89557ad0bf,
        0x311ee54149b973ca,
        0x5a8abcba2dda8474,
    ];
    assert!(ecdsa_verify_secp256r1(&pk384, &z384, &r384, &s384));

    // 381] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #379: x-coordinate of the public key is small
    let z385 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r385 = [0xc35a93d12a5dd4c7, 0x710ad7f6595d5874, 0x65957098569f0479, 0xb7c81457d4aeb6aa];
    let s385 = [0x4b9e3a05c0a1cdb3, 0x1a9199f2ca574dad, 0xd568069a432ca18a, 0xb7961a0b652878c2];
    let pk385 = [
        0xbff1173937ba748e,
        0x6f9e0015eeb23aeb,
        0x949d5f03a6f5c7f8,
        0x00000003fa15f963,
        0x548ca48af2ba7e71,
        0xfadcfcb0023ea889,
        0x555fa13659cca5d7,
        0x1099872070e8e87c,
    ];
    assert!(ecdsa_verify_secp256r1(&pk385, &z385, &r385, &s385));

    // 382] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #380: x-coordinate of the public key is small
    let z386 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r386 = [0xde5e9652e76ff3f7, 0xe3cf97e263e669f8, 0xa30a1321d5858e1e, 0x6b01332ddb6edfa9];
    let s386 = [0xcc58f9e69e96cd5a, 0x139c8f7d86b02cb1, 0x9a6a04ace2bd0f70, 0x5939545fced45730];
    let pk386 = [
        0xbff1173937ba748e,
        0x6f9e0015eeb23aeb,
        0x949d5f03a6f5c7f8,
        0x00000003fa15f963,
        0x548ca48af2ba7e71,
        0xfadcfcb0023ea889,
        0x555fa13659cca5d7,
        0x1099872070e8e87c,
    ];
    assert!(ecdsa_verify_secp256r1(&pk386, &z386, &r386, &s386));

    // 383] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #381: x-coordinate of the public key is small
    let z387 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r387 = [0x0e6a4fb93f106361, 0x4101cd2fd8436b7d, 0x349f9fc356b6c034, 0xefdb884720eaeadc];
    let s387 = [0xe48cb60d8113385d, 0xcba9e77de7d69b6c, 0x613975473aadf3aa, 0xf24bee6ad5dc05f7];
    let pk387 = [
        0xbff1173937ba748e,
        0x6f9e0015eeb23aeb,
        0x949d5f03a6f5c7f8,
        0x00000003fa15f963,
        0x548ca48af2ba7e71,
        0xfadcfcb0023ea889,
        0x555fa13659cca5d7,
        0x1099872070e8e87c,
    ];
    assert!(ecdsa_verify_secp256r1(&pk387, &z387, &r387, &s387));

    // 384] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #382: y-coordinate of the public key is small
    let z388 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r388 = [0x8014c87b8b20eb07, 0x9b23a23dd973dcbe, 0xb88fb5a646836aea, 0x31230428405560dc];
    let s388 = [0x8bd7ae3d9bd0beff, 0xaf97374e19f3c5fb, 0x6646747694a41b0a, 0x0f9344d6e812ce16];
    let pk388 = [
        0x25e9f851c18af015,
        0xbe5d2d6796707d81,
        0xaa6ecbbc612816b3,
        0xbcbb2914c79f045e,
        0xf300a698a7193bc2,
        0xdd684ade5a1127bc,
        0x0fa2ea4cceb9ab63,
        0x000000001352bb4a,
    ];
    assert!(ecdsa_verify_secp256r1(&pk388, &z388, &r388, &s388));

    // 385] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #383: y-coordinate of the public key is small
    let z389 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r389 = [0x9174db34c4855743, 0x94359c7db9841d67, 0x0d5c470cda0b36b2, 0xcaa797da65b320ab];
    let s389 = [0x3de6d9b36242e5a0, 0x123d2685ee3b941d, 0x45391aaf7505f345, 0xcf543a62f23e2127];
    let pk389 = [
        0x25e9f851c18af015,
        0xbe5d2d6796707d81,
        0xaa6ecbbc612816b3,
        0xbcbb2914c79f045e,
        0xf300a698a7193bc2,
        0xdd684ade5a1127bc,
        0x0fa2ea4cceb9ab63,
        0x000000001352bb4a,
    ];
    assert!(ecdsa_verify_secp256r1(&pk389, &z389, &r389, &s389));

    // 386] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #384: y-coordinate of the public key is small
    let z390 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r390 = [0x1c336ed800185945, 0x19bc54084536e7d2, 0xd7867657e5d6d365, 0x7e5f0ab5d900d3d3];
    let s390 = [0xe727ff0b19b646aa, 0x6688294aad35aa72, 0x4b82dfb322e5ac67, 0x9450c07f201faec9];
    let pk390 = [
        0x25e9f851c18af015,
        0xbe5d2d6796707d81,
        0xaa6ecbbc612816b3,
        0xbcbb2914c79f045e,
        0xf300a698a7193bc2,
        0xdd684ade5a1127bc,
        0x0fa2ea4cceb9ab63,
        0x000000001352bb4a,
    ];
    assert!(ecdsa_verify_secp256r1(&pk390, &z390, &r390, &s390));

    // 387] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #385: y-coordinate of the public key is large
    let z391 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r391 = [0x03aa69f0ca25b356, 0x3f8a1e4a2136fe4b, 0x6dc6a480bf037ae2, 0xd7d70c581ae9e3f6];
    let s391 = [0xaf41d9127cc47224, 0x13e85658e62a59e2, 0xba962c8a3ee833a4, 0x89c460f8a5a5c2bb];
    let pk391 = [
        0x25e9f851c18af015,
        0xbe5d2d6796707d81,
        0xaa6ecbbc612816b3,
        0xbcbb2914c79f045e,
        0x0cff596758e6c43d,
        0x2297b522a5eed843,
        0xf05d15b33146549c,
        0xfffffffeecad44b6,
    ];
    assert!(ecdsa_verify_secp256r1(&pk391, &z391, &r391, &s391));

    // 388] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #386: y-coordinate of the public key is large
    let z392 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r392 = [0xeee34bb396266b34, 0xb7aa20c625975e5e, 0xe0dfa0bf68bcdf4b, 0x341c1b9ff3c83dd5];
    let s392 = [0x902a67099e0a4469, 0x49c634e77765a017, 0x121b22b11366fad5, 0x72b69f061b750fd5];
    let pk392 = [
        0x25e9f851c18af015,
        0xbe5d2d6796707d81,
        0xaa6ecbbc612816b3,
        0xbcbb2914c79f045e,
        0x0cff596758e6c43d,
        0x2297b522a5eed843,
        0xf05d15b33146549c,
        0xfffffffeecad44b6,
    ];
    assert!(ecdsa_verify_secp256r1(&pk392, &z392, &r392, &s392));

    // 389] wycheproof/ecdsa_secp256r1_sha256_test.json EcdsaVerify SHA-256 #387: y-coordinate of the public key is large
    let z393 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r393 = [0x7628d313a3814f67, 0x9bd1781a59180994, 0x72a42f0d87387935, 0x70bebe684cdcb5ca];
    let s393 = [0x2dcde6b2094798a9, 0xc0e464b1c3577f4c, 0xd535fa31027bbe9c, 0xaec03aca8f5587a4];
    let pk393 = [
        0x25e9f851c18af015,
        0xbe5d2d6796707d81,
        0xaa6ecbbc612816b3,
        0xbcbb2914c79f045e,
        0x0cff596758e6c43d,
        0x2297b522a5eed843,
        0xf05d15b33146549c,
        0xfffffffeecad44b6,
    ];
    assert!(ecdsa_verify_secp256r1(&pk393, &z393, &r393, &s393));

    // 390] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #1: signature malleability
    let z394 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r394 = [0xb8cc6af9bd5c2e18, 0xffe50d85a1eee859, 0x80a6d9d1190a436e, 0x2ba3a8be6b94d5ec];
    let s394 = [0x77a67f79e6fadd76, 0x525fe710fab9aa7c, 0x3c7b11eb6c4e0ae7, 0x4cd60b855d442f5b];
    let pk394 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk394, &z394, &r394, &s394));

    // 391] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #2: Legacy:ASN encoding of s misses leading 0
    let z395 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r395 = [0xb8cc6af9bd5c2e18, 0xffe50d85a1eee859, 0x80a6d9d1190a436e, 0x2ba3a8be6b94d5ec];
    let s395 = [0x7c134b49156847db, 0x6a87139cac5df408, 0xc384ee1493b1f518, 0xb329f479a2bbd0a5];
    let pk395 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk395, &z395, &r395, &s395));

    // 392] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #3: valid
    let z396 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r396 = [0xb8cc6af9bd5c2e18, 0xffe50d85a1eee859, 0x80a6d9d1190a436e, 0x2ba3a8be6b94d5ec];
    let s396 = [0x7c134b49156847db, 0x6a87139cac5df408, 0xc384ee1493b1f518, 0xb329f479a2bbd0a5];
    let pk396 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk396, &z396, &r396, &s396));

    // 393] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #118: modify first byte of integer
    let z397 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r397 = [0xb8cc6af9bd5c2e18, 0xffe50d85a1eee859, 0x80a6d9d1190a436e, 0x29a3a8be6b94d5ec];
    let s397 = [0x7c134b49156847db, 0x6a87139cac5df408, 0xc384ee1493b1f518, 0xb329f479a2bbd0a5];
    let pk397 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk397, &z397, &r397, &s397));

    // 394] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #120: modify last byte of integer
    let z398 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r398 = [0xb8cc6af9bd5c2e98, 0xffe50d85a1eee859, 0x80a6d9d1190a436e, 0x2ba3a8be6b94d5ec];
    let s398 = [0x7c134b49156847db, 0x6a87139cac5df408, 0xc384ee1493b1f518, 0xb329f479a2bbd0a5];
    let pk398 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk398, &z398, &r398, &s398));

    // 395] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #121: modify last byte of integer
    let z399 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r399 = [0xb8cc6af9bd5c2e18, 0xffe50d85a1eee859, 0x80a6d9d1190a436e, 0x2ba3a8be6b94d5ec];
    let s399 = [0x7c134b491568475b, 0x6a87139cac5df408, 0xc384ee1493b1f518, 0xb329f479a2bbd0a5];
    let pk399 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk399, &z399, &r399, &s399));

    // 396] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #124: truncated integer
    let z400 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r400 = [0xb8cc6af9bd5c2e18, 0xffe50d85a1eee859, 0x80a6d9d1190a436e, 0x2ba3a8be6b94d5ec];
    let s400 = [0x087c134b49156847, 0x186a87139cac5df4, 0xa5c384ee1493b1f5, 0x00b329f479a2bbd0];
    let pk400 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk400, &z400, &r400, &s400));

    // 397] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #133: Modified r or s, e.g. by adding or subtracting the order of the group
    let z401 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r401 = [0x4733950642a3d1e8, 0x001af27a5e1117a6, 0x7f59262ee6f5bc91, 0xd45c5741946b2a13];
    let s401 = [0x7c134b49156847db, 0x6a87139cac5df408, 0xc384ee1493b1f518, 0xb329f479a2bbd0a5];
    let pk401 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk401, &z401, &r401, &s401));

    // 398] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #134: Modified r or s, e.g. by adding or subtracting the order of the group
    let z402 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r402 = [0x3aed5fc93f06f739, 0xbd01ed280528b62b, 0x7f59262ee6f5bc90, 0xd45c5740946b2a14];
    let s402 = [0x7c134b49156847db, 0x6a87139cac5df408, 0xc384ee1493b1f518, 0xb329f479a2bbd0a5];
    let pk402 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk402, &z402, &r402, &s402));

    // 399] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #137: Modified r or s, e.g. by adding or subtracting the order of the group
    let z403 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r403 = [0x4733950642a3d1e8, 0x001af27a5e1117a6, 0x7f59262ee6f5bc91, 0xd45c5741946b2a13];
    let s403 = [0x7c134b49156847db, 0x6a87139cac5df408, 0xc384ee1493b1f518, 0xb329f479a2bbd0a5];
    let pk403 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk403, &z403, &r403, &s403));

    // 400] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #139: Modified r or s, e.g. by adding or subtracting the order of the group
    let z404 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r404 = [0xb8cc6af9bd5c2e18, 0xffe50d85a1eee859, 0x80a6d9d1190a436e, 0x2ba3a8be6b94d5ec];
    let s404 = [0x885980861905228a, 0xada018ef05465583, 0xc384ee1493b1f518, 0xb329f47aa2bbd0a4];
    let pk404 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk404, &z404, &r404, &s404));

    // 401] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #143: Modified r or s, e.g. by adding or subtracting the order of the group
    let z405 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r405 = [0xb8cc6af9bd5c2e18, 0xffe50d85a1eee859, 0x80a6d9d1190a436e, 0x2ba3a8be6b94d5ec];
    let s405 = [0x83ecb4b6ea97b825, 0x9578ec6353a20bf7, 0x3c7b11eb6c4e0ae7, 0x4cd60b865d442f5a];
    let pk405 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk405, &z405, &r405, &s405));

    // 402] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #177: Signature with special case values for r and s
    let z406 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r406 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s406 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk406 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk406, &z406, &r406, &s406));

    // 403] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #178: Signature with special case values for r and s
    let z407 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r407 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s407 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk407 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk407, &z407, &r407, &s407));

    // 404] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #179: Signature with special case values for r and s
    let z408 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r408 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s408 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk408 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk408, &z408, &r408, &s408));

    // 405] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #180: Signature with special case values for r and s
    let z409 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r409 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s409 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let pk409 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk409, &z409, &r409, &s409));

    // 406] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #181: Signature with special case values for r and s
    let z410 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r410 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s410 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let pk410 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk410, &z410, &r410, &s410));

    // 407] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #187: Signature with special case values for r and s
    let z411 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r411 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s411 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk411 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk411, &z411, &r411, &s411));

    // 408] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #188: Signature with special case values for r and s
    let z412 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r412 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s412 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk412 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk412, &z412, &r412, &s412));

    // 409] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #189: Signature with special case values for r and s
    let z413 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r413 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s413 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk413 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk413, &z413, &r413, &s413));

    // 410] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #190: Signature with special case values for r and s
    let z414 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r414 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s414 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let pk414 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk414, &z414, &r414, &s414));

    // 411] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #191: Signature with special case values for r and s
    let z415 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r415 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s415 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let pk415 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk415, &z415, &r415, &s415));

    // 412] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #197: Signature with special case values for r and s
    let z416 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r416 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s416 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk416 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk416, &z416, &r416, &s416));

    // 413] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #198: Signature with special case values for r and s
    let z417 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r417 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s417 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk417 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk417, &z417, &r417, &s417));

    // 414] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #199: Signature with special case values for r and s
    let z418 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r418 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s418 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk418 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk418, &z418, &r418, &s418));

    // 415] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #200: Signature with special case values for r and s
    let z419 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r419 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s419 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let pk419 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk419, &z419, &r419, &s419));

    // 416] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #201: Signature with special case values for r and s
    let z420 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r420 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s420 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let pk420 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk420, &z420, &r420, &s420));

    // 417] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #207: Signature with special case values for r and s
    let z421 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r421 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let s421 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk421 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk421, &z421, &r421, &s421));

    // 418] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #208: Signature with special case values for r and s
    let z422 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r422 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let s422 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk422 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk422, &z422, &r422, &s422));

    // 419] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #209: Signature with special case values for r and s
    let z423 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r423 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let s423 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk423 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk423, &z423, &r423, &s423));

    // 420] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #210: Signature with special case values for r and s
    let z424 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r424 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let s424 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let pk424 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk424, &z424, &r424, &s424));

    // 421] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #211: Signature with special case values for r and s
    let z425 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r425 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let s425 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let pk425 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk425, &z425, &r425, &s425));

    // 422] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #217: Signature with special case values for r and s
    let z426 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r426 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let s426 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk426 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk426, &z426, &r426, &s426));

    // 423] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #218: Signature with special case values for r and s
    let z427 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r427 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let s427 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk427 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk427, &z427, &r427, &s427));

    // 424] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #219: Signature with special case values for r and s
    let z428 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r428 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let s428 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk428 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk428, &z428, &r428, &s428));

    // 425] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #220: Signature with special case values for r and s
    let z429 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r429 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let s429 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let pk429 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk429, &z429, &r429, &s429));

    // 426] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #221: Signature with special case values for r and s
    let z430 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r430 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let s430 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let pk430 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk430, &z430, &r430, &s430));

    // 427] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #230: Edge case for Shamir multiplication
    let z431 = [0x2fa50c772ed6f807, 0x2f2627416faf2f07, 0xc422f44dea4ed1a5, 0x70239dd877f7c944];
    let r431 = [0x11547c97711c898e, 0x8ff312334e2ba16d, 0x4f3e2fc02bdee9be, 0x64a1aab5000d0e80];
    let s431 = [0xfd683b9bb2cf4f1b, 0x7772a2f91d73286f, 0xd1a206d4e013e099, 0x6af015971cc30be6];
    let pk431 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk431, &z431, &r431, &s431));

    // 428] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #231: special case hash
    let z432 = [0x7ead3645f356e7a9, 0x84bcd58a1bb5e747, 0xccf17803ebe2bd08, 0x00000000690ed426];
    let r432 = [0x1e19a0ec580bf266, 0xded7d397738448de, 0x6f78c81c91fc7e8b, 0x16aea964a2f6506d];
    let s432 = [0x38c3ff033be928e9, 0x391e8e80c578d1cd, 0xcfe8b7bc47d27d78, 0x252cd762130c6667];
    let pk432 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk432, &z432, &r432, &s432));

    // 429] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #232: special case hash
    let z433 = [0x140697ad25770d91, 0xf696ad3ebb5ee47f, 0x525c6035725235c2, 0x7300000000213f2a];
    let r433 = [0x7c665baccb23c882, 0xf2d26d6ef524af91, 0xf476dfc26b9b733d, 0x9cc98be2347d469b];
    let s433 = [0xa631dacb16b56c32, 0x0ec1b7847929d10e, 0xd70727b82462f61d, 0x093496459effe2d8];
    let pk433 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk433, &z433, &r433, &s433));

    // 430] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #233: special case hash
    let z434 = [0x4a0161c27fe06045, 0x8afd25daadeb3edb, 0xe0635b245f0b9797, 0xddf2000000005e0b];
    let r434 = [0x093999f07ab8aa43, 0x03dce3dea0d53fa8, 0x058164524dde8927, 0x73b3c90ecd390028];
    let s434 = [0x188c0c4075c88634, 0x2ed25a395387b5f4, 0x5bb7d8bf0a651c80, 0x2f67b0b8e2063669];
    let pk434 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk434, &z434, &r434, &s434));

    // 431] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #234: special case hash
    let z435 = [0x5be1ec355d0841a0, 0x642b8499588b8985, 0x4769c4ecb9e164d6, 0x67ab190000000078];
    let r435 = [0xf37e90119d5ba3dd, 0x1a7f0eb390763378, 0x28fadf2f89b95c85, 0xbfab3098252847b3];
    let s435 = [0x1e2da9b8b4987e3b, 0x8195ccebb65c2aaf, 0x67c2d058ccb44d97, 0xbdd64e234e832b10];
    let pk435 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk435, &z435, &r435, &s435));

    // 432] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #235: special case hash
    let z436 = [0xe296b6350fc311cf, 0x02095dff252ee905, 0x76d7dbeffe125eaf, 0xa2bf094600000000];
    let r436 = [0xd17093c5cd21d2cd, 0xf1c9aaab168b1596, 0x8bf8bf04a4ceb1c1, 0x204a9784074b246d];
    let s436 = [0x582fe648d1d88b52, 0xa406c2506fe17975, 0xdc06a759c8847868, 0x51cce41670636783];
    let pk436 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk436, &z436, &r436, &s436));

    // 433] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #236: special case hash
    let z437 = [0x29e15c544e4f0e65, 0xa0a3531711608581, 0x00e1e75e624a06b3, 0x3554e827c7000000];
    let r437 = [0x027bca0f1ceeaa03, 0x0031a91d1314f835, 0xf63d4aa4f81fe2cb, 0xed66dc34f551ac82];
    let s437 = [0xbb8953d67c0c48c7, 0x67623c3f6e5d4d6a, 0x194a422e18d5fda1, 0x99ca123aa09b13cd];
    let pk437 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk437, &z437, &r437, &s437));

    // 434] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #237: special case hash
    let z438 = [0x26e3a54b9fc6965c, 0x3255ea4c9fd0cb34, 0x000026941a0f0bb5, 0x9b6cd3b812610000];
    let r438 = [0x56bf0f60a237012b, 0x126b062023ccc3c0, 0x899d44f2356a578d, 0x060b700bef665c68];
    let s438 = [0x6be5d581c11d3610, 0xedbb410cbef3f26d, 0x4fcc78a3366ca95d, 0x8d186c027832965f];
    let pk438 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk438, &z438, &r438, &s438));

    // 435] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #238: special case hash
    let z439 = [0x77162f93c4ae0186, 0x82a52baa51c71ca8, 0x000000e7561c26fc, 0x883ae39f50bf0100];
    let r439 = [0x2bb0c8e38c96831d, 0xc93ea76cd313c913, 0x24d7aa7934b6cf29, 0x9f6adfe8d5eb5b2c];
    let s439 = [0x051593883b5e9902, 0x906a33e66b5bd15e, 0x890c944cf271756c, 0xb26a9c9e40e55ee0];
    let pk439 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk439, &z439, &r439, &s439));

    // 436] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #239: special case hash
    let z440 = [0x01fe9fce011d0ba6, 0x10540f420fb4ff74, 0x0000000000fa7cd0, 0xa1ce5d6e5ecaf28b];
    let r440 = [0x8868f4ba273f16b7, 0xa1abf6da168cebfa, 0x3ad2f33615e56174, 0xa1af03ca91677b67];
    let s440 = [0x5caf24c8c5e06b1c, 0x77d69022e7d098d7, 0x35cd258b173d0c23, 0x20aa73ffe48afa64];
    let pk440 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk440, &z440, &r440, &s440));

    // 437] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #240: special case hash
    let z441 = [0x5494cdffd5ee8054, 0x97330012a8ee836c, 0x9300000000383453, 0x8ea5f645f373f580];
    let r441 = [0xe327a28c11893db9, 0x659355507b843da6, 0x11a6c99a71c973d5, 0xfdc70602766f8eed];
    let s441 = [0xa7f83f2b10d21350, 0x0f6d15ec0078ca60, 0x37b1eacf456a9e9e, 0x3df5349688a085b1];
    let pk441 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk441, &z441, &r441, &s441));

    // 438] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #241: special case hash
    let z442 = [0x8d9c1bbdcb5ef305, 0xd65ce93eabb7d60d, 0xa734000000008792, 0x660570d323e9f75f];
    let r442 = [0x0dc738f7b876e675, 0x23456f63c643cf8e, 0xd6537f6a6c49966c, 0xb516a314f2fce530];
    let s442 = [0xa66b0120cd16fff2, 0x967c4bd80954479b, 0x17dd536fbc5efdf1, 0xd39ffd033c92b6d7];
    let pk442 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk442, &z442, &r442, &s442));

    // 439] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #242: special case hash
    let z443 = [0x46ada2de4c568c34, 0x8d35f1f45cf9c3bf, 0x7dde8800000000e9, 0xd0462673154cce58];
    let r443 = [0xa485c101e29ff0a8, 0x82717bebb6492fd0, 0x2ecb7984d4758315, 0x3b2cbf046eac4584];
    let s443 = [0xc8595fc1c1d99258, 0x701099cac5f76e68, 0xde512bc9313aaf51, 0x4c9b7b47a98b0f82];
    let pk443 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk443, &z443, &r443, &s443));

    // 440] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #243: special case hash
    let z444 = [0xb83e7b4418d7278f, 0x0caef15a6171059a, 0x80cedfef00000000, 0xbd90640269a78226];
    let r444 = [0x6c3fb15bfde48dcf, 0xd79d0312cfa1ab65, 0x841f14af54e2f9ed, 0x30c87d35e636f540];
    let s444 = [0x0db9abf6340677ed, 0x71409ede23efd08e, 0xc85a692bd6ecafeb, 0x47c15a5a82d24b75];
    let pk444 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk444, &z444, &r444, &s444));

    // 441] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #244: special case hash
    let z445 = [0x4beae8e284788a73, 0x00d2dcceb301c54b, 0x512e41222a000000, 0x33239a52d72f1311];
    let r445 = [0x68ff262113760f52, 0xe2e8176d168dec3c, 0xbc43b58cfe6647b9, 0x38686ff0fda2cef6];
    let s445 = [0xc2ddabb3fde9d67d, 0xe976e2db5e6a4cf7, 0x9601662167fa8717, 0x067ec3b651f42266];
    let pk445 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk445, &z445, &r445, &s445));

    // 442] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #245: special case hash
    let z446 = [0x1dc84c2d941ffaf1, 0x00007ee4a21a1cbe, 0x1365d4e6d95c0000, 0xb8d64fbcd4a1c10f];
    let r446 = [0x225985ab6e2775cf, 0xf3e17d27f5ee844b, 0x44fc25c7f2de8b6a, 0x44a3e23bf314f2b3];
    let s446 = [0x93c9cc3f4dd15e86, 0x84f0411f57295004, 0x1ddc87be532abed5, 0x2d48e223205e9804];
    let pk446 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk446, &z446, &r446, &s446));

    // 443] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #246: special case hash
    let z447 = [0x4088b20fe0e9d84a, 0x0000003a227420db, 0xa3fef3183ed09200, 0x01603d3982bf77d7];
    let r447 = [0x0eb9d638781688e9, 0x41b99db3b5aa8d33, 0xf11f967a3d95110c, 0x2ded5b7ec8e90e7b];
    let s447 = [0xec69238a009808f9, 0x8de049c328ae1f44, 0x1bfc46fb1a67e308, 0x7d5792c53628155e];
    let pk447 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk447, &z447, &r447, &s447));

    // 444] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #247: special case hash
    let z448 = [0xb7e9eb0cfbff7363, 0x000000004d89ef50, 0x599aa02e6cf66d9c, 0x9ea6994f1e0384c8];
    let r448 = [0x05976f15137d8b8f, 0x3eaccafcd40ec2f6, 0xefd3bc3d31870f92, 0xbdae7bcb580bf335];
    let s448 = [0x24838122ce7ec3c7, 0x9f373a4fb318994f, 0x0b0106eecfe25749, 0xf6dfa12f19e52527];
    let pk448 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk448, &z448, &r448, &s448));

    // 445] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #248: special case hash
    let z449 = [0xf692bc670905b18c, 0x4700000000e2fa5b, 0x693979371a01068a, 0xd03215a8401bcf16];
    let r449 = [0x1ece251c2401f1c6, 0x99209b78596956d2, 0x62720957ffff5137, 0x50f9c4f0cd6940e1];
    let s449 = [0xaa5167dfab244726, 0x5a4355e411a59c32, 0x889defaaabb106b9, 0xd7033a0a787d338e];
    let pk449 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk449, &z449, &r449, &s449));

    // 446] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #249: special case hash
    let z450 = [0xfd5f64b582e3bb14, 0xc87e000000008408, 0x9c84bf83f0300e5d, 0x307bfaaffb650c88];
    let r450 = [0xbe90924ead5c860d, 0x0982e29575d019aa, 0x1906066a378d6754, 0xf612820687604fa0];
    let s450 = [0x328230ce294b0fef, 0x1a99f4857b316525, 0x75ea98afd20e328a, 0x3f9367702dd7dd4f];
    let pk450 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk450, &z450, &r450, &s450));

    // 447] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #250: special case hash
    let z451 = [0xaf574bb4d54ea6b8, 0x51527c00000000e4, 0x33324d36bb0c1575, 0xbab5c4f4df540d7b];
    let r451 = [0x0f2f507da5782a7a, 0x1f61980c1949f56b, 0xc93db5da7aa6f508, 0x9505e407657d6e8b];
    let s451 = [0x5e7f71784f9c5021, 0x08e0ed5cb92b3cfa, 0x8ffbeccab6c3656c, 0xc60d31904e366973];
    let pk451 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk451, &z451, &r451, &s451));

    // 448] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #251: special case hash
    let z452 = [0xc3b869197ef5e15e, 0xc2456f5b00000000, 0xe4f58d8036f9c36e, 0xd4ba47f6ae28f274];
    let r452 = [0x3e1c68a40404517d, 0x08735aed37173272, 0xd83e6a7787cd691b, 0xbbd16fbbb656b6d0];
    let s452 = [0x560e3e7fd25c0f00, 0x7d2d097be5e8ee34, 0x787d91315be67587, 0x9d8e35dba96028b7];
    let pk452 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk452, &z452, &r452, &s452));

    // 449] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #252: special case hash
    let z453 = [0x00801e47f8c184e1, 0xfe0f10aafd000000, 0xf29f1fa00984342a, 0x79fd19c7235ea212];
    let r453 = [0xcf57c61e92df327e, 0x442d2ceef7559a30, 0x06ea76848d35a6da, 0x2ec9760122db98fd];
    let s453 = [0xc4963625c0a19878, 0x393fb6814c27b760, 0x701fccf86e462ee3, 0x7ab271da90859479];
    let pk453 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk453, &z453, &r453, &s453));

    // 450] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #253: special case hash
    let z454 = [0x0000a37ea6700cda, 0x79cbeb7ac9730000, 0xaf9aba5c0583462d, 0x8c291e8eeaa45adb];
    let r454 = [0x4f1005a89fe00c59, 0xd9ba9dd463221f7a, 0xaa6a7fc49b1c51ee, 0x54e76b7683b6650b];
    let s454 = [0x52f2f7806a31c8fd, 0xcfd11b1c1ae11661, 0x37ec1cc8374b7915, 0x2ea076886c773eb9];
    let pk454 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk454, &z454, &r454, &s454));

    // 451] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #254: special case hash
    let z455 = [0x0000003c278a6b21, 0xf4cdcf66c3f78a00, 0x9803efbfb8140732, 0x0eaae8641084fa97];
    let r455 = [0x10419c0c496c9466, 0x7a74abdbb69be4fb, 0xbce6e3c26f602109, 0x5291deaf24659ffb];
    let s455 = [0xbf83469270a03dc3, 0x827f84742f29f10a, 0xcdb982bb4e4ecef5, 0x65d6fcf336d27cc7];
    let pk455 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk455, &z455, &r455, &s455));

    // 452] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #255: special case hash
    let z456 = [0x00000000afc0f89d, 0xef17c6d96e13846c, 0x0068399bf01bab42, 0xe02716d01fb23a5a];
    let r456 = [0xd15166a88479f107, 0x003b33fc17eb50f9, 0x47419dc58efb05e8, 0x207a3241812d75d9];
    let s456 = [0x82d5caadf7592767, 0xf1c5d70793cf55e3, 0x3ce80b32d0574f62, 0xcdee749f2e492b21];
    let pk456 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk456, &z456, &r456, &s456));

    // 453] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #256: special case hash
    let z457 = [0x9a00000000fc7de1, 0x9061768af89d0065, 0x194e9a16bc7dab2a, 0x9eb0bf583a1a6b9a];
    let r457 = [0xc0dee3cf81aa7728, 0xbe84437a355a0a37, 0x4328ac94913bf01b, 0x6554e49f82a85520];
    let s457 = [0x86effe7f22b4f929, 0x16250a2eaebc8be4, 0xc94e1e126980d3df, 0xaea00de2507ddaf5];
    let pk457 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk457, &z457, &r457, &s457));

    // 454] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #257: special case hash
    let z458 = [0x690e00000000cd15, 0x6e1030cb53d9a82b, 0x2c214f0d5e72ef28, 0x62aac98818b3b84a];
    let r458 = [0x2990ac82707efdfc, 0x6c6e19b4d80a8c60, 0xbff06f71c88216c2, 0xa54c5062648339d2];
    let s458 = [0xff09be73c9731b0d, 0x1056317f467ad09a, 0x69fd016777517aa0, 0xe99bbe7fcfafae3e];
    let pk458 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk458, &z458, &r458, &s458));

    // 455] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #258: special case hash
    let z459 = [0x464b9300000000c8, 0xd2b6f552ea4b6895, 0xf29ae43732e513ef, 0x3760a7f37cf96218];
    let r459 = [0x4ca8b059cff37eaf, 0xd23096593133e71b, 0x309f1f444012b1a1, 0x975bd7157a8d363b];
    let s459 = [0xacc46786bf919622, 0xd4c69840fe090f2a, 0xa241793f2abc930b, 0x7faa7a28b1c822ba];
    let pk459 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk459, &z459, &r459, &s459));

    // 456] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #259: special case hash
    let z460 = [0xbb6ff6c800000000, 0x6b4320bea836cd9c, 0x3834f2098c088009, 0x0da0a1d2851d3302];
    let r460 = [0x7b95b3e0da43885e, 0xde9ec90305afb135, 0x276afd2ebcfe4d61, 0x5694a6f84b8f875c];
    let s460 = [0x3b6ccc7c679cbaa4, 0x8ee2dc5c7870c082, 0x8051dec02ebdf70d, 0x0dffad9ffd0b757d];
    let pk460 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk460, &z460, &r460, &s460));

    // 457] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #260: special case hash
    let z461 = [0xa764a231e82d289a, 0x0fe975f735887194, 0x086fd567aafd598f, 0xffffffff293886d3];
    let r461 = [0xd7454ba9790f1ba6, 0xf7098f1a98d21620, 0xb4968a27d16a6d08, 0xa0c30e8026fdb2b4];
    let s461 = [0x8bd2760c65424339, 0xacc5ca6445914968, 0x5baf463f9deceb53, 0x5e470453a8a399f1];
    let pk461 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk461, &z461, &r461, &s461));

    // 458] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #261: special case hash
    let z462 = [0x0e8d9ca99527e7b7, 0x26acdc4ce127ec2e, 0xe3c03445a072e243, 0x7bffffffff2376d1];
    let r462 = [0x2aa0228cf7b99a88, 0x1dfebebd5ad8aca5, 0xdd73602cd4bb4eea, 0x614ea84acf736527];
    let s462 = [0x2a4dd193195c902f, 0xde14368e96a9482c, 0xd1b8183f3ed490e4, 0x737cc85f5f2d2f60];
    let pk462 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk462, &z462, &r462, &s462));

    // 459] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #262: special case hash
    let z463 = [0xfd016807e97fa395, 0xbc80872602a6e467, 0x51b085377605a224, 0xa2b5ffffffffebb2];
    let r463 = [0xa8d74dfbd0f942fa, 0x45377338febfd439, 0x0d3fb2ea00b17329, 0xbead6734ebe44b81];
    let s463 = [0x36a46b103ef56e2a, 0xf4bbe7a10f73b3e0, 0x3cad35919fd21a8a, 0x6bb18eae36616a7d];
    let pk463 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk463, &z463, &r463, &s463));

    // 460] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #263: special case hash
    let z464 = [0x7b83d0967d4b20c0, 0xc1a3c256870d45a6, 0x1b96fa5f097fcf3c, 0x641227ffffffff6f];
    let r464 = [0x654fae182df9bad2, 0x8d922cbf212703e9, 0xd4db9d9ce64854c9, 0x499625479e161dac];
    let s464 = [0x95b64fca76d9d693, 0x9439936028864ac1, 0x0131108d97819edd, 0x42c177cf37b8193a];
    let pk464 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk464, &z464, &r464, &s464));

    // 461] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #264: special case hash
    let z465 = [0x8df56f36600e0f8b, 0xba20352117750229, 0xabad03e2fc662dc3, 0x958415d8ffffffff];
    let r465 = [0x50fb1aaa6ff6c9b2, 0x31e3bfe694f6b89c, 0x66a2c8065b541b3d, 0x08f16b8093a8fb4d];
    let s465 = [0x535ba3e5af81ca2e, 0x21f967410399b39b, 0x48573b611cb95d4a, 0x9d6455e2d5d17797];
    let pk465 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk465, &z465, &r465, &s465));

    // 462] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #265: special case hash
    let z466 = [0x954521b6975420f8, 0xe13deb04e1fbe8fb, 0xff1281093536f47f, 0xf1d8de4858ffffff];
    let r466 = [0xeed8dc2b338cb5f8, 0xc579b6938d19bce8, 0x19dd72ddb99ed8f8, 0xbe26231b6191658a];
    let s466 = [0xb9c5e96952575c89, 0xc943c14f79694a03, 0x37f0f22b2dcb57d5, 0xe1d9a32ee56cffed];
    let pk466 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk466, &z466, &r466, &s466));

    // 463] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #266: special case hash
    let z467 = [0x876b95c81fc31def, 0x32dc5d47c05ef6f1, 0xffff10782dd14a3b, 0x0927895f2802ffff];
    let r467 = [0x12638c455abe0443, 0x45f36a229d4aa4f8, 0x6204ac920a02d580, 0x15e76880898316b1];
    let s467 = [0x38196506a1939123, 0x55ca10e226e13f96, 0x5337bd6aba4178b4, 0xe74d357d3fcb5c8c];
    let pk467 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk467, &z467, &r467, &s467));

    // 464] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #267: special case hash
    let z468 = [0x24cf6a0c3ac80589, 0x0a57c3063fb5a306, 0xffffff4f332862a1, 0x60907984aa7e8eff];
    let r468 = [0x132315cc07f16dad, 0x31e6307d3ddbffc1, 0x3a45f9846fc28d1d, 0x352ecb53f8df2c50];
    let s468 = [0x899792887dd0a3c6, 0x436726ecd28258b1, 0xe1d05c5242ca1c39, 0x1348dfa9c482c558];
    let pk468 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk468, &z468, &r468, &s468));

    // 465] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #268: special case hash
    let z469 = [0x42d6b9b8cd6ae1e2, 0x50f9a5f50636ea69, 0xffffffff0af42cda, 0xc6ff198484939170];
    let r469 = [0x2c5bfa5f2a9558fb, 0x77b8642349ed3d65, 0x8a0da9882ab23c76, 0x4a40801a7e606ba7];
    let s469 = [0xea77dc5981725782, 0xdc24ed2925825bf8, 0x7f605f2832f7384b, 0x3a49b64848d682ef];
    let pk469 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk469, &z469, &r469, &s469));

    // 466] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #269: special case hash
    let z470 = [0x16dfbe4d27d7e68d, 0x9b9e0956cc43135d, 0x75ffffffff807479, 0xde030419345ca15c];
    let r470 = [0xe5e9e44df3d61e96, 0xb3511bac855c05c9, 0x2be412b078924b3b, 0xeacc5e1a8304a74d];
    let s470 = [0x08db8f714204f6d1, 0xec4bb0ed4c36ce98, 0x85dd827714847f96, 0x7451cd8e18d6ed18];
    let pk470 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk470, &z470, &r470, &s470));

    // 467] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #270: special case hash
    let z471 = [0x7e1ab78caaaac6ff, 0x665604d34acb1903, 0x2b88fffffffff6c8, 0x6f0e3eeaf42b2813];
    let r471 = [0x5f7de94c31577052, 0x4f8cd1214882adb6, 0xf30f67fdab61e8ce, 0x2f7a5e9e5771d424];
    let s471 = [0xb9528f8f78daa10c, 0xfb75dd050c5a449a, 0x44acb0b2bd889175, 0xac4e69808345809b];
    let pk471 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk471, &z471, &r471, &s471));

    // 468] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #271: special case hash
    let z472 = [0x2cb222d1f8017ab9, 0x48f7c0591ddcae7d, 0x3708d1ffffffffbe, 0xcdb549f773b3e62b];
    let r472 = [0x0a03d710b3300219, 0x7dddd7f6487621c3, 0x3e7e0f0e95e1a214, 0xffcda40f792ce4d9];
    let s472 = [0xd58c422c2453a49a, 0xfa77618f0b67add8, 0xd7ba9ade8f2065a1, 0x79938b55f8a17f7e];
    let pk472 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk472, &z472, &r472, &s472));

    // 469] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #272: special case hash
    let z473 = [0x24d8fd6f0edb0484, 0x9fd64886c1dc4f99, 0x1df4989bffffffff, 0x2c3f26f96a3ac005];
    let r473 = [0x8c17603a431e39a8, 0x48350f7ab3a588b2, 0x3d3e8c8c3fcc16a9, 0x81f2359c4faba6b5];
    let s473 = [0x7f9e101857f74300, 0x09e46d99fccefb9f, 0x0ff695d06c6860b5, 0xcd6f6a5cc3b55ead];
    let pk473 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk473, &z473, &r473, &s473));

    // 470] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #273: special case hash
    let z474 = [0x8476397c04edf411, 0xff5c31d89fda6a6b, 0x2cb7d53f9affffff, 0xac18f8418c55a250];
    let r474 = [0xc3f5f2aaf75ca808, 0xea130251a6fdffa5, 0xee1596fb073ea283, 0xdfc8bf520445cbb8];
    let s474 = [0xa7ac711e577e90e7, 0xbfd7d0dc7a4905b3, 0xd92823640e338e68, 0x048e33efce147c9d];
    let pk474 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk474, &z474, &r474, &s474));

    // 471] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #274: special case hash
    let z475 = [0x3e5a6ab8cf0ee610, 0xffffa2fd3e289368, 0xb24094f72bb5ffff, 0x4f9618f98e2d3a15];
    let r475 = [0x88227688ba6a5762, 0x6503a0e393e932f6, 0xefda70b46c53db16, 0xad019f74c6941d20];
    let s475 = [0xbc05efe16c199345, 0x7964ef2e0988e712, 0x5346bdbb3102cdcf, 0x93320eb7ca071025];
    let pk475 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk475, &z475, &r475, &s475));

    // 472] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #275: special case hash
    let z476 = [0x04caae73ab0bc75a, 0xffffff67edf7c402, 0x9cc21d31d37a25ff, 0x422e82a3d56ed10a];
    let r476 = [0xdeb7bd5a3ebc1883, 0xb54316bd3ebf7fff, 0xc34e78ce11dd71e4, 0xac8096842e8add68];
    let s476 = [0x9f21a3aac003b7a8, 0x36e3ce9f0ce21970, 0x2d4caf85d187215d, 0xf5ca2f4f23d67450];
    let pk476 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk476, &z476, &r476, &s476));

    // 473] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #276: special case hash
    let z477 = [0x2d9890b5cf95d018, 0x17a5ffffffffa084, 0x6e7b329ff738fbb4, 0x7075d245ccc3281b];
    let r477 = [0x54b4943693fb92f7, 0x89ddcd7b7b9d7768, 0xf939b70ea0022508, 0x677b2d3a59b18a5f];
    let s477 = [0xab6972cc0795db55, 0x5d2f63aee81efd0b, 0xf30307b21f3ccda3, 0x6b4ba856ade7677b];
    let pk477 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk477, &z477, &r477, &s477));

    // 474] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #277: special case hash
    let z478 = [0xc1847eb76c217a95, 0x7e280ebeffffffff, 0x9443d593fa4fd659, 0x3c80de54cd922698];
    let r478 = [0x05e1fc0d5957cfb0, 0xd84d31d4b7c30e1f, 0x379ba8e1b73d3115, 0x479e1ded14bcaed0];
    let s478 = [0x1e877027355b2443, 0x30857ca879f97c77, 0x7cf634a4f05b2e0c, 0x918f79e35b3d8948];
    let pk478 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk478, &z478, &r478, &s478));

    // 475] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #278: special case hash
    let z479 = [0xffc7906aa794b39b, 0x0ce891a8cdffffff, 0x980bef3d697ea277, 0xde21754e29b85601];
    let r479 = [0xb64840ead512a0a3, 0xd711e14b12ac5cf3, 0xd9a58f01164d55c3, 0x43dfccd0edb9e280];
    let s479 = [0x3199f49584389772, 0xca1174899b78ef9a, 0xcd5c4934365b3442, 0x1dbe33fa8ba84533];
    let pk479 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk479, &z479, &r479, &s479));

    // 476] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #279: special case hash
    let z480 = [0xffff2f1f2f57881c, 0x599e4d5f7289ffff, 0x84dd59623fb531bb, 0x8f65d92927cfb86a];
    let r480 = [0x38bb4085f0bbff11, 0xa20e9087c259d26a, 0xf4c7c7e4bca592fe, 0x5b09ab637bd4caf0];
    let s480 = [0xca8101de08eb0d75, 0xa24964e5a13f885b, 0x618e9d80d6fdcd6a, 0x45b7eb467b6748af];
    let pk480 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk480, &z480, &r480, &s480));

    // 477] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #280: special case hash
    let z481 = [0xfffffffafc8c3ca8, 0x2cc7cd0e8426cbff, 0x160bea3877dace8a, 0x6b63e9a74e092120];
    let r481 = [0x14a5039ed15ee06f, 0x667afa570a6cfa01, 0x5728c5c8af9b74e0, 0x5e9b1c5a028070df];
    let s481 = [0x44edaeb9ad990c20, 0x6c29eeffd3c50377, 0xad362bb8d7bd661b, 0xb1360907e2d9785e];
    let pk481 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk481, &z481, &r481, &s481));

    // 478] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #281: special case hash
    let z482 = [0xffffffffe852512e, 0xd094586e249c8699, 0xb6d75219444e8b43, 0xfc28259702a03845];
    let r482 = [0xd1a7a5fb8578f32e, 0x4890050f5a5712f6, 0x4a2fb0990e34538b, 0x0671a0a85c2b72d5];
    let s482 = [0xc720e5854713694c, 0x1808f27fd5bd4fda, 0x79ab9c3285ca4129, 0xdb1846bab6b73614];
    let pk482 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk482, &z482, &r482, &s482));

    // 479] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #282: special case hash
    let z483 = [0x1757ffffffffe20a, 0x74ecbcd52e8ceb57, 0xcee044ee8e8db7f7, 0x1273b4502ea4e3bc];
    let r483 = [0xbaedb35b2095103a, 0xc5d7d69859d301ab, 0x77dbbb0590a45492, 0x7673f85267484464];
    let s483 = [0x3807ef4422913d7c, 0x4dec0d417a414fed, 0x886bed9e6af02e0e, 0x3dc70ddf9c6b524d];
    let pk483 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk483, &z483, &r483, &s483));

    // 480] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #283: special case hash
    let z484 = [0xfb49ffffffffff6e, 0x4f8c53a15b96e602, 0x0c566c66228d8181, 0x08fb565610a79baa];
    let r484 = [0x9dfd657a796d12b5, 0x450d1a06c36d3ff3, 0xb21285089ebb1aa6, 0x7f085441070ecd2b];
    let s484 = [0xa9e4c5c54a2b9a8b, 0x92a5e6cb4b2d8daf, 0x2459d18d47da9aa4, 0x249712012029870a];
    let pk484 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk484, &z484, &r484, &s484));

    // 481] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #284: special case hash
    let z485 = [0x28ecaefeffffffff, 0xa2403f748e97d7cd, 0x87715fcb1aa4e79a, 0xd59291cc2cf89f30];
    let r485 = [0xa8e0f30a5d287348, 0xb76df04bc5aa6683, 0xc867398ea7322d5a, 0x914c67fb61dd1e27];
    let s485 = [0xc96d28f6d37304ea, 0xea7e66ec412b38d6, 0x4953e3ac1959ee8c, 0xfa07474031481dda];
    let pk485 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk485, &z485, &r485, &s485));

    // 482] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #636: r too large
    let z486 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r486 = [0xfffffffffffffffc, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let s486 = [0xf3b9cac2fc63254e, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk486 = [
        0x82415c8361baaca4,
        0xebf7d10fa5151531,
        0x9b1a6957d29ce22f,
        0xd705d16f80987e2d,
        0x60819e8682160926,
        0xa6f83625593620d4,
        0x14ec1238beae2037,
        0xb1fc105ee5ce80d5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk486, &z486, &r486, &s486));

    // 483] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #637: r,s are large
    let z487 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r487 = [0xf3b9cac2fc63254f, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s487 = [0xf3b9cac2fc63254e, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk487 = [
        0xafa52c8501387d59,
        0xcd2ef67056893ead,
        0x844c09d7b560d527,
        0x3cd8d2f81d6953b0,
        0x8903485c0bb6dc2d,
        0xa490b62a6b771906,
        0x7a0c5e3b747adfa3,
        0xee41fdb4d10402ce,
    ];
    assert!(ecdsa_verify_secp256r1(&pk487, &z487, &r487, &s487));

    // 484] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #638: r and s^-1 have a large Hamming weight
    let z488 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r488 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s488 = [0xde54a36383df8dd4, 0x1453fe50914f3df2, 0x170f5ead2de4f651, 0x909135bdb6799286];
    let pk488 = [
        0x189b481196851378,
        0x0e81f332c4545d41,
        0x936133508c391510,
        0x8240cd81edd91cb6,
        0x837dc432f9ce89d9,
        0xea6dd6d9c0ae27b7,
        0x80ea5db514aa2f93,
        0xe05b06e72d4a1bff,
    ];
    assert!(ecdsa_verify_secp256r1(&pk488, &z488, &r488, &s488));

    // 485] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #639: r and s^-1 have a large Hamming weight
    let z489 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r489 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s489 = [0x360644669ca249a5, 0xf5deb773ad5f5a84, 0x71303fd5dd227dce, 0x27b4577ca009376f];
    let pk489 = [
        0x616d91eaad13df2c,
        0xa6e1bfe6779756fa,
        0xc17f1704c65aa1dc,
        0xb062947356748b0f,
        0x431113f1b2fb579d,
        0x12b84a4f8432293b,
        0x409cfc5992a99fff,
        0x0b38c17f3d0672e7,
    ];
    assert!(ecdsa_verify_secp256r1(&pk489, &z489, &r489, &s489));

    // 486] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #651: r and s^-1 are close to n
    let z490 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r490 = [0xf3b9cac2fc6324d5, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s490 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let pk490 = [
        0xfcf3fd88f8e07ede,
        0x3b499a96afa7aaa3,
        0x2bbe25a34ea4e363,
        0x7a736d8e326a9ca6,
        0xfcc48a5934864627,
        0xeda7bf9ae46aa3ea,
        0xe818443a686e869e,
        0xb3e45879d8622b93,
    ];
    assert!(ecdsa_verify_secp256r1(&pk490, &z490, &r490, &s490));

    // 487] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #654: point at infinity during verify
    let z491 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r491 = [0x79dce5617e3192a8, 0xde737d56d38bcf42, 0x7fffffffffffffff, 0x7fffffff80000000];
    let s491 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let pk491 = [
        0xae67c467de045034,
        0x15259240aa78d08a,
        0xd8d7a0c80f66dddd,
        0x0203736fcb198b15,
        0xb072f8f20e87a996,
        0x471b160c6bcf2568,
        0xa387ee8e4d4e84b4,
        0x34383438d5041ea9,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk491, &z491, &r491, &s491));

    // 488] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #655: edge case for signature malleability
    let z492 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r492 = [0x79dce5617e3192a9, 0xde737d56d38bcf42, 0x7fffffffffffffff, 0x7fffffff80000000];
    let s492 = [0x79dce5617e3192a8, 0xde737d56d38bcf42, 0x7fffffffffffffff, 0x7fffffff80000000];
    let pk492 = [
        0x4e37dcd2bfea02e1,
        0x99fe2e70a1848238,
        0x1f2a39730da5d8cd,
        0x78d844dc7f16b73b,
        0x3a8183c26e75d336,
        0xd3b9a6a6dea99aa4,
        0x13d02c666c45ef22,
        0xed6572e01eb7a8d1,
    ];
    assert!(ecdsa_verify_secp256r1(&pk492, &z492, &r492, &s492));

    // 489] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #656: edge case for signature malleability
    let z493 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r493 = [0x79dce5617e3192a9, 0xde737d56d38bcf42, 0x7fffffffffffffff, 0x7fffffff80000000];
    let s493 = [0x79dce5617e3192a9, 0xde737d56d38bcf42, 0x7fffffffffffffff, 0x7fffffff80000000];
    let pk493 = [
        0x4d62da9599a74014,
        0xcc5beb81a958b02b,
        0x0eacc8c09d2e5789,
        0xdec6c8257dde9411,
        0xca4e344fdd690f1d,
        0x7b06dd6f4e9c56ba,
        0x970b83f652442106,
        0x66fae1614174be63,
    ];
    assert!(ecdsa_verify_secp256r1(&pk493, &z493, &r493, &s493));

    // 490] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #657: u1 == 1
    let z494 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r494 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let s494 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let pk494 = [
        0xc3d0cad0988cabc0,
        0x92db0c23f0c2ea24,
        0x23ca5cbf1f919512,
        0xa17f5b75a35ed646,
        0x8b3a3f6300424dc6,
        0xecbb2fc20fdde7c5,
        0x40730b4fa3ee64fa,
        0x83a7a618625c2289,
    ];
    assert!(ecdsa_verify_secp256r1(&pk494, &z494, &r494, &s494));

    // 491] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #658: u1 == n - 1
    let z495 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r495 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let s495 = [0x9eac5054ed2ec72c, 0x9c400e9c69af7beb, 0x4089464733ff7cd3, 0xacd155416a8b77f3];
    let pk495 = [
        0xe9eaa1ebcc9fb5c3,
        0x04ec8393a0200419,
        0x13f33bf90dab628c,
        0x04ba0cba291a37db,
        0x49ecf4265dc12f62,
        0x47970fc3428f0f00,
        0x625ad57b12a32d40,
        0x1f3a0a0e6823a49b,
    ];
    assert!(ecdsa_verify_secp256r1(&pk495, &z495, &r495, &s495));

    // 492] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #659: u2 == 1
    let z496 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r496 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let s496 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let pk496 = [
        0x9f204a434b8900ef,
        0xcbe8723a1ed39f22,
        0x3d8aeaa2b7322f9c,
        0x692b6c828e0feed6,
        0x3329069ae4dd5716,
        0x274af56a8c5628dc,
        0x8fde38b98c7c271f,
        0xa1f6f6abcb38ea3b,
    ];
    assert!(ecdsa_verify_secp256r1(&pk496, &z496, &r496, &s496));

    // 493] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #660: u2 == n - 1
    let z497 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r497 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let s497 = [0x4d26872ca84218e1, 0x7def51c91a0fbf03, 0xaaaaaaaaaaaaaaaa, 0xaaaaaaaa00000000];
    let pk497 = [
        0x0f84871874ccef09,
        0x5ebb5a3ef7632f80,
        0xcb93687a9cd8f975,
        0x00cefd9162d13e64,
        0x7dbbef2c54bc0cb1,
        0xb8480d2587404ebf,
        0xf721be2fb5f549e4,
        0x543ecbeaf7e8044e,
    ];
    assert!(ecdsa_verify_secp256r1(&pk497, &z497, &r497, &s497));

    // 494] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #661: edge case for u1
    let z498 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r498 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s498 = [0x6d97c1bb03dd2bd2, 0x49d9f794f6d5405f, 0x3fd23de844002bb9, 0x710f8e3edc7c2d5a];
    let pk498 = [
        0xde03efa3f0f24486,
        0x12f50c8c85a4beb9,
        0x2f291d5c1921fd5e,
        0xb975183b42551cf5,
        0x78c406b25ab43091,
        0xf21e242ce3fb15bc,
        0x2dc313612020311f,
        0x2243018e6866df92,
    ];
    assert!(ecdsa_verify_secp256r1(&pk498, &z498, &r498, &s498));

    // 495] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #662: edge case for u1
    let z499 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r499 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s499 = [0xa8e269274ffe4e1b, 0x1a58525c7b4db2e7, 0x3069a7e5f40335a6, 0xedffbc270f722c24];
    let pk499 = [
        0xd717149274466999,
        0x4d48b8d17191d74e,
        0xdf042a26f8abf609,
        0xc25f1d166f3e211c,
        0xcfb52a114e77ccdb,
        0x51969adf9604b5ac,
        0x9e8b4c5da6bb9228,
        0x65d06dd6a88abfa4,
    ];
    assert!(ecdsa_verify_secp256r1(&pk499, &z499, &r499, &s499));

    // 496] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #663: edge case for u1
    let z500 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r500 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s500 = [0x7904b68919ba4d53, 0x3314c3e178525d00, 0x4f95d2344e24ee52, 0xa25adcae105ed7ff];
    let pk500 = [
        0x008d7f0164cbc0ca,
        0xd6eee398a23c3a0b,
        0xa004236218a3c3a2,
        0x8fe5e88243a76e41,
        0x86b8cb387af7f240,
        0x82d40127c897697c,
        0x3c7cfd9b83c63e3a,
        0x98a20d1bdcf57351,
    ];
    assert!(ecdsa_verify_secp256r1(&pk500, &z500, &r500, &s500));

    // 497] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #664: edge case for u1
    let z501 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r501 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s501 = [0x0297e766b5805ebb, 0x346924b2f64bd3dd, 0x6760d773de3f3e87, 0x2e4348c645707dce];
    let pk501 = [
        0xeca2efb37e8dff2c,
        0x3ecee6d5a840a37b,
        0x70c7b341970b3824,
        0x02148256b530fbc4,
        0x53a83573473cb30d,
        0xa987eeb6ddb738af,
        0x489ca703a399864b,
        0xc0adbea0882482a7,
    ];
    assert!(ecdsa_verify_secp256r1(&pk501, &z501, &r501, &s501));

    // 498] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #665: edge case for u1
    let z502 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r502 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s502 = [0xf9684fb67753d1dc, 0x869e916dbcf797d8, 0x0d773de3f3e87408, 0x348c673b07dce392];
    let pk502 = [
        0xb3a621d021c76f8e,
        0xd698e19615124273,
        0x9c7375c5fcf3e54e,
        0xa34db012ce6eda1e,
        0xfdde13d1d6df7f14,
        0xbb4fbb7ddf08d8d8,
        0x221e39e1205d5510,
        0x777458d6f55a364c,
    ];
    assert!(ecdsa_verify_secp256r1(&pk502, &z502, &r502, &s502));

    // 499] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #666: edge case for u1
    let z503 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r503 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s503 = [0xf2d09f6ceea7a3b8, 0x0d3d22db79ef2fb1, 0x1aee7bc7e7d0e811, 0x6918ce760fb9c724];
    let pk503 = [
        0x7ae37b4e7778041d,
        0xdb6dd2a1b315b2ce,
        0x912b6271dd8a43ba,
        0xb97af3fe78be15f2,
        0x2d0fa4c479b278e7,
        0x154c305307d1dcd5,
        0x6495c42102d08e81,
        0x930d71ee1992d246,
    ];
    assert!(ecdsa_verify_secp256r1(&pk503, &z503, &r503, &s503));

    // 500] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #667: edge case for u1
    let z504 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r504 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s504 = [0xd864d648969f5b5a, 0x15ac20e4c126bbf6, 0xde3f3e8740894647, 0x73b3c694391d8ead];
    let pk504 = [
        0x59139af3135dbcbb,
        0xbf81108e6c35cd85,
        0x1cedc7a1d6eff6e9,
        0x81e7198a3c3f2390,
        0xb737525b5d580034,
        0xba990d4570a4e3b7,
        0x61b90c9f4285eefc,
        0x9ef1568530291a80,
    ];
    assert!(ecdsa_verify_secp256r1(&pk504, &z504, &r504, &s504));

    // 501] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #668: edge case for u1
    let z505 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r505 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s505 = [0x6877e53c41457f28, 0xb89ce11259519765, 0x2989a16db1930ef1, 0xbb07ac7a86948c2c];
    let pk505 = [
        0x3cc9960f188ddf73,
        0xab573e8becc6ddff,
        0xa39cb9de645149c2,
        0xab4d792ca121d1db,
        0x56386de68285a3c8,
        0x5858d7be1315a694,
        0x3262ff7335541519,
        0x7f90ba23664153e9,
    ];
    assert!(ecdsa_verify_secp256r1(&pk505, &z505, &r505, &s505));

    // 502] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #669: edge case for u1
    let z506 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r506 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s506 = [0x3f62c861187db648, 0xd198662d6f229944, 0x9337c69bf9332ed3, 0x27e4d82cb6c061dd];
    let pk506 = [
        0xf587c9c2652f88ef,
        0x51fbfa9e5be80563,
        0x084476a68d59bbde,
        0x518412b69af43aae,
        0xc1c70503fc10f233,
        0xfbc24afed8523ede,
        0x7b0c55e5240a3a98,
        0x2d3b90d25baa6bdb,
    ];
    assert!(ecdsa_verify_secp256r1(&pk506, &z506, &r506, &s506));

    // 503] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #670: edge case for u1
    let z507 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r507 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s507 = [0x5c3dd81ae94609a4, 0x2d13b356dfe9ec27, 0x3b77850515fff6a1, 0xe7c5cf3aac2e8892];
    let pk507 = [
        0xf5d370af34f8352d,
        0xd1f66fe6cd373aa7,
        0xdffea4761ebaf592,
        0xa08f14a644b9a935,
        0xd732a5741c7aaaf5,
        0xec7a396d0a7affca,
        0x900a914c2934ec2f,
        0xa54b5bc4025cf335,
    ];
    assert!(ecdsa_verify_secp256r1(&pk507, &z507, &r507, &s507));

    // 504] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #671: edge case for u1
    let z508 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r508 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s508 = [0x3cede9e57a748f68, 0x17f9fee32bacfe55, 0xe016e10bddffea23, 0xc77838df91c1e953];
    let pk508 = [
        0x39241061e33f8f8c,
        0x0e9f45715b900446,
        0x90739d38af4ae3a2,
        0xccf2296a6a89b62b,
        0xc7e058034412ae08,
        0xaf83e7ff1bb84438,
        0xc6e9a472b96d88f4,
        0xaace0046491eeaa1,
    ];
    assert!(ecdsa_verify_secp256r1(&pk508, &z508, &r508, &s508));

    // 505] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #672: edge case for u1
    let z509 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r509 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s509 = [0x86220907f885f97f, 0x730d0318b0425e25, 0xc02dc217bbffd446, 0x8ef071c02383d2a6];
    let pk509 = [
        0xe165f962c86e3927,
        0x6c02b23e04002276,
        0x2b1f34895e5819a0,
        0x94b0fc1525bcabf8,
        0xfb29fbc89a9c3376,
        0x2792225e16a6d2db,
        0x204fb32a1f829290,
        0xbe7c2ab4d0b25303,
    ];
    assert!(ecdsa_verify_secp256r1(&pk509, &z509, &r509, &s509));

    // 506] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #673: edge case for u1
    let z510 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r510 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s510 = [0xcf56282a76976396, 0xce20074e34d7bdf5, 0xa044a32399ffbe69, 0x5668aaa0b545bbf9];
    let pk510 = [
        0x0627cadfd16de6ec,
        0xccdcf2efca407edb,
        0x508527d89882d183,
        0x5351f37e1de0c88c,
        0x81b766a1a1300349,
        0x8425853b5b675eb7,
        0xebcc4c97847eed21,
        0x44b4b57cdf960d32,
    ];
    assert!(ecdsa_verify_secp256r1(&pk510, &z510, &r510, &s510));

    // 507] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #674: edge case for u1
    let z511 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r511 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s511 = [0xb65f40a60b0eb952, 0xf7fddf478fb4fdc2, 0x27cae91a27127728, 0xd12d6e56882f6c00];
    let pk511 = [
        0x5cb2fb276ac971a6,
        0xc2b5d147bdc83132,
        0xcb64019710a269c6,
        0x748bbafc320e6735,
        0x7005177578f51163,
        0xd93a7a49a8c5ccd3,
        0x00ad21ee3fd4d980,
        0x9d655e9a755bc9d8,
    ];
    assert!(ecdsa_verify_secp256r1(&pk511, &z511, &r511, &s511));

    // 508] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #675: edge case for u2
    let z512 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r512 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s512 = [0x513dee40fecbb71a, 0xe9a2538f37b28a2c, 0xffffffffffffffff, 0x7fffffffaaaaaaaa];
    let pk512 = [
        0x1c33038964fd85cc,
        0x12410b3b90fa97a3,
        0x36535a934d4ab851,
        0x14b3bbd75c5e1c0c,
        0x9705561dd6631883,
        0x18f2b50c5d00fb3f,
        0xb460d636c965a5f8,
        0x112f7d837f8f9c36,
    ];
    assert!(ecdsa_verify_secp256r1(&pk512, &z512, &r512, &s512));

    // 509] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #676: edge case for u2
    let z513 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r513 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s513 = [0xde009e526adf21f2, 0x3ab3cccd0459b201, 0x6de86d42ad8a13da, 0xb62f26b5f2a2b26f];
    let pk513 = [
        0x56935671ae9305bf,
        0x4a9bafa2f14a5903,
        0x6d6f950a8e08ade0,
        0xd823533c04cd8edc,
        0x59d7797303123775,
        0xb58312907b195acb,
        0x96924c265f0ddb75,
        0x43178d1f88b6a57a,
    ];
    assert!(ecdsa_verify_secp256r1(&pk513, &z513, &r513, &s513));

    // 510] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #677: edge case for u2
    let z514 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r514 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s514 = [0x0686aa7b4c90851e, 0xd57b38bb61403d70, 0xd02bbbe749bd351c, 0xbb1d9ac949dd748c];
    let pk514 = [
        0xd209b92e654bab69,
        0xec108c105575c2f3,
        0x030624c6328e8ce3,
        0xdb2b3408b3167d91,
        0xc3d6be82836fa258,
        0x800df7c996d5d7b7,
        0x2c6e612f0fd3189d,
        0xc34318139c50b080,
    ];
    assert!(ecdsa_verify_secp256r1(&pk514, &z514, &r514, &s514));

    // 511] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #678: edge case for u2
    let z515 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r515 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s515 = [0x818f725b4f60aaf2, 0xe52545dac11f816e, 0x1c732513ca0234ec, 0x66755a00638cdaec];
    let pk515 = [
        0x1dd7ab6063852742,
        0x78c24837dfae26bc,
        0x2216453b2ac1e9d1,
        0x09179ce7c5922539,
        0xdcc3b691f95a9255,
        0x45c2860a59f2be1d,
        0xb826b2db7a86d19d,
        0x5556b42e330289f3,
    ];
    assert!(ecdsa_verify_secp256r1(&pk515, &z515, &r515, &s515));

    // 512] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #679: edge case for u2
    let z516 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r516 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s516 = [0x8ca48e982beb3669, 0xe98ebe492fdf02e4, 0x32513ca0234ecfff, 0x55a00c9fcdaebb60];
    let pk516 = [
        0x6b1eccebd6568d7e,
        0xd0c2fb29d70ff19b,
        0x467b7e4b214ea4c2,
        0x01959fb8deda56e5,
        0x211c39cc3a413398,
        0x25167db5a14d098a,
        0x970bff01e1343f69,
        0xd9dbd77a918297fd,
    ];
    assert!(ecdsa_verify_secp256r1(&pk516, &z516, &r516, &s516));

    // 513] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #680: edge case for u2
    let z517 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r517 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s517 = [0x19491d3057d66cd2, 0xd31d7c925fbe05c9, 0x64a27940469d9fff, 0xab40193f9b5d76c0];
    let pk517 = [
        0x5938245dd6bcab3a,
        0x947e1c5dd7ccc61a,
        0xc852b4e8f8ba9d6d,
        0x567f1fdc387e5350,
        0xb1f3eb1011130a11,
        0x2857970e26662267,
        0x9535c22eaaf0b581,
        0x9960bebaf919514f,
    ];
    assert!(ecdsa_verify_secp256r1(&pk517, &z517, &r517, &s517));

    // 514] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #681: edge case for u2
    let z518 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r518 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s518 = [0xa26b4408d0dc8600, 0xcb0dadbbc7f549f8, 0xca0234ecffffffff, 0xca0234ebb5fdcb13];
    let pk518 = [
        0x60b36d46d3e4bec2,
        0x2f9dd6dd28552626,
        0xb2f51682fd5f5176,
        0x3499f974ff4ca6bb,
        0x4546630f0d5c5e81,
        0xc64d4fa46ddce85c,
        0x20119152f0122476,
        0xf498fae2487807e2,
    ];
    assert!(ecdsa_verify_secp256r1(&pk518, &z518, &r518, &s518));

    // 515] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #682: edge case for u2
    let z519 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r519 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s519 = [0x8711c77298815ad3, 0x19933a9e65b28559, 0x082b9310572620ae, 0xbfffffff3ea3677e];
    let pk519 = [
        0x51b0f27094473426,
        0xcf30d0f3ec4b9f03,
        0x29596257db13b26e,
        0x2c5c01662cf00c19,
        0x1d9fdafa484e4ac7,
        0x7a0154b57f7a69c5,
        0xee822ddd2fc74424,
        0xe986a086060d086e,
    ];
    assert!(ecdsa_verify_secp256r1(&pk519, &z519, &r519, &s519));

    // 516] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #683: edge case for u2
    let z520 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r520 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s520 = [0x8f055d86e5cc41f4, 0x5b37902e023fab7c, 0xe666666666666666, 0x266666663bbbbbbb];
    let pk520 = [
        0xd9f3cf010b160501,
        0x15774183be7ba5b2,
        0xdbae94c23be6f52c,
        0x91d4cba813a04d86,
        0xe4b1874b02fd544a,
        0x541bf4b952b0ad7b,
        0x9a9ac080d516025a,
        0x900b8adfea649101,
    ];
    assert!(ecdsa_verify_secp256r1(&pk520, &z520, &r520, &s520));

    // 517] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #684: edge case for u2
    let z521 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r521 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s521 = [0x08a443e258970b09, 0x146c573f4c6dfc8d, 0xa492492492492492, 0xbfffffff36db6db7];
    let pk521 = [
        0x0c614b948e8aa124,
        0x02af36960831d021,
        0x8330ecad41e1a3b3,
        0xef7fd0a3a3638663,
        0x34116e35a8c7d098,
        0xd8cab5ab59c730eb,
        0xd3c1be0fdeaf11fc,
        0xef0d6d800e4047d6,
    ];
    assert!(ecdsa_verify_secp256r1(&pk521, &z521, &r521, &s521));

    // 518] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #685: edge case for u2
    let z522 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r522 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s522 = [0xcb1ad3a27cfd49c4, 0xc815d0e60b3e596e, 0x7fffffffffffffff, 0xbfffffff2aaaaaab];
    let pk522 = [
        0x8cea92eafe93df2a,
        0x6c55cc3ca5dbeb86,
        0x8ca77035a607fea0,
        0xa521dab13cc9152d,
        0xa36500418a2f43de,
        0x6ce1111bdb9c2e0c,
        0x5e6a5ccaa2826a40,
        0x7bfb9b2853199663,
    ];
    assert!(ecdsa_verify_secp256r1(&pk522, &z522, &r522, &s522));

    // 519] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #686: edge case for u2
    let z523 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r523 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s523 = [0xa27bdc81fd976e37, 0xd344a71e6f651458, 0xffffffffffffffff, 0x7fffffff55555555];
    let pk523 = [
        0xc01d1237a81a1097,
        0xe7a2683a12f38b4f,
        0x565f2187fe11d4e8,
        0x474d58a4eec16e0d,
        0xfde3a517a6ded4cd,
        0x9df2b67920fb5945,
        0xbdb67ef77f6fd296,
        0x6e55f73bb7cdda46,
    ];
    assert!(ecdsa_verify_secp256r1(&pk523, &z523, &r523, &s523));

    // 520] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #687: edge case for u2
    let z524 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r524 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s524 = [0x79dce5617e3192aa, 0xde737d56d38bcf42, 0x7fffffffffffffff, 0x3fffffff80000000];
    let pk524 = [
        0xf47d223a5b23a621,
        0x879f7b57208cdabb,
        0xe5cb525c37da8fa0,
        0x692da5cd4309d9a6,
        0x7d2902e9125e6ab4,
        0x7fc5fc3e6a5ed339,
        0xa7389aaed61738b1,
        0x40e0daa78cfdd207,
    ];
    assert!(ecdsa_verify_secp256r1(&pk524, &z524, &r524, &s524));

    // 521] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #688: edge case for u2
    let z525 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r525 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s525 = [0x0343553da648428f, 0x6abd9c5db0a01eb8, 0x6815ddf3a4de9a8e, 0x5d8ecd64a4eeba46];
    let pk525 = [
        0xe9b8805c570a0670,
        0xfcd4d1f1679274f4,
        0x8a90279f14a8082c,
        0x85689b3e0775c771,
        0x70d3f240ebe705b1,
        0x15b9b7ca661ec7ff,
        0x09afa3640f4a034e,
        0x167fcc5ca734552e,
    ];
    assert!(ecdsa_verify_secp256r1(&pk525, &z525, &r525, &s525));

    // 522] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #689: point duplication during verification
    let z526 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r526 = [0x05ea6c42c4934569, 0x8c4aacafdfb6bcbe, 0x8fe0555ac3bc9904, 0x6f2347cab7dd7685];
    let s526 = [0x3a41de562aa18ed8, 0x3f54ddf7383e4402, 0xc4fa1f4703c1e50d, 0xf21d907e3890916d];
    let pk526 = [
        0x455edaef42cf237e,
        0x3cb2ef63b2ba2c0d,
        0x97a90d4ca8887e02,
        0x0158137755b901f7,
        0xe17bd1ba5677edcd,
        0xa7c7b9fd2b41d6e0,
        0x92b8b61aafa7a4aa,
        0x2a964fc00d377a85,
    ];
    assert!(ecdsa_verify_secp256r1(&pk526, &z526, &r526, &s526));

    // 523] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #690: duplication bug
    let z527 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r527 = [0x05ea6c42c4934569, 0x8c4aacafdfb6bcbe, 0x8fe0555ac3bc9904, 0x6f2347cab7dd7685];
    let s527 = [0x3a41de562aa18ed8, 0x3f54ddf7383e4402, 0xc4fa1f4703c1e50d, 0xf21d907e3890916d];
    let pk527 = [
        0x455edaef42cf237e,
        0x3cb2ef63b2ba2c0d,
        0x97a90d4ca8887e02,
        0x0158137755b901f7,
        0x1e842e45a9881232,
        0x58384603d4be291f,
        0x6d4749e550585b55,
        0xd569b03ef2c8857b,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk527, &z527, &r527, &s527));

    // 524] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #693: comparison with point at infinity
    let z528 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r528 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let s528 = [0x63f1f55a327a3aa9, 0x25c7cbbc549e52e7, 0x3333333333333333, 0x3333333300000000];
    let pk528 = [
        0xadb6f2c10aac831e,
        0xb36b7cdc54e33b84,
        0x8bdb2e61201b4549,
        0x664ce273320d918d,
        0xc7c915c736cef1f4,
        0xe1cceed2dd862e2d,
        0x73ac3d76bfbc8c5e,
        0x49e68831f18bda29,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk528, &z528, &r528, &s528));

    // 525] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #694: extreme value for k and edgecase s
    let z529 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r529 = [0xa60b48fc47669978, 0xc08969e277f21b35, 0x8a52380304b51ac3, 0x7cf27b188d034f7e];
    let s529 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let pk529 = [
        0x1add395efff1e0fe,
        0xc27d7089faeb3ddd,
        0x301dbbad4d86247e,
        0x961691a5e960d07a,
        0x231bd260a9e78aeb,
        0x37d1f1519817f09a,
        0xdf990d2c5377790e,
        0x7254622cc371866c,
    ];
    assert!(ecdsa_verify_secp256r1(&pk529, &z529, &r529, &s529));

    // 526] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #695: extreme value for k and s^-1
    let z530 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r530 = [0xa60b48fc47669978, 0xc08969e277f21b35, 0x8a52380304b51ac3, 0x7cf27b188d034f7e];
    let s530 = [0x1bcdd9f8fd6b63cc, 0x625bd7a09bec4ca8, 0x4924924924924924, 0xb6db6db624924925];
    let pk530 = [
        0x250e71e2aca63e9c,
        0xf1074793274e2928,
        0xa868e3b0fb33e6b4,
        0x5d283e13ce8ca60d,
        0xe0f44505a84886ce,
        0xbfd6d0c8bb6591d3,
        0x4d9e506d418ed9a1,
        0x214dc74fa25371fb,
    ];
    assert!(ecdsa_verify_secp256r1(&pk530, &z530, &r530, &s530));

    // 527] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #696: extreme value for k and s^-1
    let z531 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r531 = [0xa60b48fc47669978, 0xc08969e277f21b35, 0x8a52380304b51ac3, 0x7cf27b188d034f7e];
    let s531 = [0x8fc7d568c9e8eaa7, 0x971f2ef152794b9d, 0xcccccccccccccccc, 0xcccccccc00000000];
    let pk531 = [
        0xa9d297f792eef6a3,
        0xf9f8216551d9315a,
        0x3bd1d86514ae0462,
        0x0fc351da038ae080,
        0x5fc339d634019c73,
        0x753f00d6077a1e9e,
        0xda35360ca7aa925e,
        0x41c74eed786f2d33,
    ];
    assert!(ecdsa_verify_secp256r1(&pk531, &z531, &r531, &s531));

    // 528] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #697: extreme value for k and s^-1
    let z532 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r532 = [0xa60b48fc47669978, 0xc08969e277f21b35, 0x8a52380304b51ac3, 0x7cf27b188d034f7e];
    let s532 = [0x63f1f55a327a3aaa, 0x25c7cbbc549e52e7, 0x3333333333333333, 0x3333333300000000];
    let pk532 = [
        0x322bba9430ce4b60,
        0xfd4de7550065f638,
        0x3fee55c080547c2b,
        0xa1e34c8f16d13867,
        0x20b7d8a6b81ac936,
        0xc5d44a7bdf424366,
        0x4d7df8ab3f3b4181,
        0x662be9bb512663aa,
    ];
    assert!(ecdsa_verify_secp256r1(&pk532, &z532, &r532, &s532));

    // 529] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #698: extreme value for k and s^-1
    let z533 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r533 = [0xa60b48fc47669978, 0xc08969e277f21b35, 0x8a52380304b51ac3, 0x7cf27b188d034f7e];
    let s533 = [0xd7ebf0c9fef7c185, 0x5a8b230d0b2b51dc, 0xb6db6db6db6db6db, 0x49249248db6db6db];
    let pk533 = [
        0x7cda6e52817c1bdf,
        0xa87a23c7186150ed,
        0xf41d322a302d2078,
        0x7e1a8a8338d7fd8c,
        0x8d59ee34c615377f,
        0x254d748272b2d4eb,
        0x21e29014b2898349,
        0xd0a9135a89d21ce8,
    ];
    assert!(ecdsa_verify_secp256r1(&pk533, &z533, &r533, &s533));

    // 530] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #699: extreme value for k
    let z534 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r534 = [0xa60b48fc47669978, 0xc08969e277f21b35, 0x8a52380304b51ac3, 0x7cf27b188d034f7e];
    let s534 = [0x6e0478c34416e3bb, 0x1584d13e18411e2f, 0xc82cbc9d1edd8c98, 0x16a4502e2781e11a];
    let pk534 = [
        0xf76a686f64be078b,
        0x1b2c6f663ea33583,
        0x5c61ee7a018cc957,
        0x5c19fe227a61abc6,
        0x06c0541d17b24ddb,
        0xcf78492490a5cc56,
        0xd52bc48673b457c2,
        0x7b4a0d734940f613,
    ];
    assert!(ecdsa_verify_secp256r1(&pk534, &z534, &r534, &s534));

    // 531] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #700: extreme value for k and edgecase s
    let z535 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r535 = [0xf4a13945d898c296, 0x77037d812deb33a0, 0xf8bce6e563a440f2, 0x6b17d1f2e12c4247];
    let s535 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let pk535 = [
        0x5db63abeb2739666,
        0x208eed08c2d4189a,
        0x9d9ef9e47419dba3,
        0xdb02d1f3421d600e,
        0x1715e6b24125512a,
        0xd210d5fd8ec628e3,
        0xd7ffe480827f90a0,
        0xe0ed26967b9ada9e,
    ];
    assert!(ecdsa_verify_secp256r1(&pk535, &z535, &r535, &s535));

    // 532] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #701: extreme value for k and s^-1
    let z536 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r536 = [0xf4a13945d898c296, 0x77037d812deb33a0, 0xf8bce6e563a440f2, 0x6b17d1f2e12c4247];
    let s536 = [0x1bcdd9f8fd6b63cc, 0x625bd7a09bec4ca8, 0x4924924924924924, 0xb6db6db624924925];
    let pk536 = [
        0xcfab338b88229c4b,
        0x5711bd3ed5a0ef72,
        0x93c29e441395b6c0,
        0x6222d19626555018,
        0xf1c54311d8e2fd23,
        0xac2626423b0bf81a,
        0x70362aaa520ee24c,
        0xaaae079cb44a1af0,
    ];
    assert!(ecdsa_verify_secp256r1(&pk536, &z536, &r536, &s536));

    // 533] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #702: extreme value for k and s^-1
    let z537 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r537 = [0xf4a13945d898c296, 0x77037d812deb33a0, 0xf8bce6e563a440f2, 0x6b17d1f2e12c4247];
    let s537 = [0x8fc7d568c9e8eaa7, 0x971f2ef152794b9d, 0xcccccccccccccccc, 0xcccccccc00000000];
    let pk537 = [
        0x361da184b04cdca5,
        0x9c0952ba599f4c03,
        0xfa81bc99c70bb041,
        0x4ccfa24c67f3def7,
        0xf6ca7a0a82153bfa,
        0x9728df870800be8c,
        0x729a2219478a7e62,
        0xdb76b797f7f41d9c,
    ];
    assert!(ecdsa_verify_secp256r1(&pk537, &z537, &r537, &s537));

    // 534] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #703: extreme value for k and s^-1
    let z538 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r538 = [0xf4a13945d898c296, 0x77037d812deb33a0, 0xf8bce6e563a440f2, 0x6b17d1f2e12c4247];
    let s538 = [0x63f1f55a327a3aaa, 0x25c7cbbc549e52e7, 0x3333333333333333, 0x3333333300000000];
    let pk538 = [
        0x61e99fefff9d84da,
        0xf3dbde7a99dc5740,
        0xac71402b6e9ecc4a,
        0xea1c72c91034036b,
        0x2bc2ea918c18cb63,
        0x29d5d055408c90d0,
        0xf56e34eb048f0a9d,
        0xb7dd057e75b78ac6,
    ];
    assert!(ecdsa_verify_secp256r1(&pk538, &z538, &r538, &s538));

    // 535] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #704: extreme value for k and s^-1
    let z539 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r539 = [0xf4a13945d898c296, 0x77037d812deb33a0, 0xf8bce6e563a440f2, 0x6b17d1f2e12c4247];
    let s539 = [0xd7ebf0c9fef7c185, 0x5a8b230d0b2b51dc, 0xb6db6db6db6db6db, 0x49249248db6db6db];
    let pk539 = [
        0x0d936988e90e79bc,
        0x38924f7817d1cd35,
        0x820b7795da2da62b,
        0xc2879a66d86cb20b,
        0x3aaaa11fa3b6a083,
        0xcb0177216db6fd1f,
        0x7a759de024eff90b,
        0x5431a7268ff6931c,
    ];
    assert!(ecdsa_verify_secp256r1(&pk539, &z539, &r539, &s539));

    // 536] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #705: extreme value for k
    let z540 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r540 = [0xf4a13945d898c296, 0x77037d812deb33a0, 0xf8bce6e563a440f2, 0x6b17d1f2e12c4247];
    let s540 = [0x6e0478c34416e3bb, 0x1584d13e18411e2f, 0xc82cbc9d1edd8c98, 0x16a4502e2781e11a];
    let pk540 = [
        0x58f455079aee0ba3,
        0x54c26df27711b065,
        0xb848c75006f2ef3c,
        0xab1c0f273f74abc2,
        0xe4d6c37fa48b47f2,
        0x6c179f0a13af1771,
        0x5997c776f14ad645,
        0xdf510f2ecef6d9a0,
    ];
    assert!(ecdsa_verify_secp256r1(&pk540, &z540, &r540, &s540));

    // 537] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #706: testing point duplication
    let z541 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r541 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let s541 = [0x6bf5f864ff7be0c2, 0xad4591868595a8ee, 0xdb6db6db6db6db6d, 0x249249246db6db6d];
    let pk541 = [
        0xf4a13945d898c296,
        0x77037d812deb33a0,
        0xf8bce6e563a440f2,
        0x6b17d1f2e12c4247,
        0xcbb6406837bf51f5,
        0x2bce33576b315ece,
        0x8ee7eb4a7c0f9e16,
        0x4fe342e2fe1a7f9b,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk541, &z541, &r541, &s541));

    // 538] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #707: testing point duplication
    let z542 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r542 = [0x9eac5054ed2ec72c, 0x9c400e9c69af7beb, 0x4089464733ff7cd3, 0xacd155416a8b77f3];
    let s542 = [0x6bf5f864ff7be0c2, 0xad4591868595a8ee, 0xdb6db6db6db6db6d, 0x249249246db6db6d];
    let pk542 = [
        0xf4a13945d898c296,
        0x77037d812deb33a0,
        0xf8bce6e563a440f2,
        0x6b17d1f2e12c4247,
        0xcbb6406837bf51f5,
        0x2bce33576b315ece,
        0x8ee7eb4a7c0f9e16,
        0x4fe342e2fe1a7f9b,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk542, &z542, &r542, &s542));

    // 539] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #708: testing point duplication
    let z543 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r543 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let s543 = [0x6bf5f864ff7be0c2, 0xad4591868595a8ee, 0xdb6db6db6db6db6d, 0x249249246db6db6d];
    let pk543 = [
        0xf4a13945d898c296,
        0x77037d812deb33a0,
        0xf8bce6e563a440f2,
        0x6b17d1f2e12c4247,
        0x3449bf97c840ae0a,
        0xd431cca994cea131,
        0x711814b583f061e9,
        0xb01cbd1c01e58065,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk543, &z543, &r543, &s543));

    // 540] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #709: testing point duplication
    let z544 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r544 = [0x9eac5054ed2ec72c, 0x9c400e9c69af7beb, 0x4089464733ff7cd3, 0xacd155416a8b77f3];
    let s544 = [0x6bf5f864ff7be0c2, 0xad4591868595a8ee, 0xdb6db6db6db6db6d, 0x249249246db6db6d];
    let pk544 = [
        0xf4a13945d898c296,
        0x77037d812deb33a0,
        0xf8bce6e563a440f2,
        0x6b17d1f2e12c4247,
        0x3449bf97c840ae0a,
        0xd431cca994cea131,
        0x711814b583f061e9,
        0xb01cbd1c01e58065,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk544, &z544, &r544, &s544));

    // 541] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #1210: pseudorandom signature
    let z545 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r545 = [0x73d7904519e51388, 0x2711f9917060406a, 0x381c4c1f1da8e9de, 0xa8ea150cb80125d7];
    let s545 = [0x7288293285449b86, 0x0c22c9d76ec21725, 0xa73b2d40480c2ba5, 0xf3ab9fa68bd47973];
    let pk545 = [
        0x5b522eba7240fad5,
        0x32e41495a944d004,
        0x13fb8a9e64da3b86,
        0x04aaec73635726f2,
        0x1aa546e8365d525d,
        0xaaf7b4e09fc81d6d,
        0xba01775787ced05e,
        0x87d9315798aaa3a5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk545, &z545, &r545, &s545));

    // 542] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #1211: pseudorandom signature
    let z546 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r546 = [0x88268822c253bcce, 0x15d8c43a1365713c, 0x065a051bc7adc206, 0x30e782f964b2e2ff];
    let s546 = [0x0428d2d3f4e08ed5, 0x2e84cacfa7c6eec3, 0xdc8b46c515f9604e, 0x5b16df652aa1ecb2];
    let pk546 = [
        0x5b522eba7240fad5,
        0x32e41495a944d004,
        0x13fb8a9e64da3b86,
        0x04aaec73635726f2,
        0x1aa546e8365d525d,
        0xaaf7b4e09fc81d6d,
        0xba01775787ced05e,
        0x87d9315798aaa3a5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk546, &z546, &r546, &s546));

    // 543] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #1212: pseudorandom signature
    let z547 = [0xa495991b7852b855, 0x27ae41e4649b934c, 0x9afbf4c8996fb924, 0xe3b0c44298fc1c14];
    let r547 = [0x8e76e09d8770b34a, 0x42d16e47f219f9e9, 0x7a305c951c0dcbcc, 0xb292a619339f6e56];
    let s547 = [0xab2abebdf89a62e2, 0xe59ec2a17ce5bd2d, 0x2f76f07bfe3661bd, 0x0177e60492c5a824];
    let pk547 = [
        0x5b522eba7240fad5,
        0x32e41495a944d004,
        0x13fb8a9e64da3b86,
        0x04aaec73635726f2,
        0x1aa546e8365d525d,
        0xaaf7b4e09fc81d6d,
        0xba01775787ced05e,
        0x87d9315798aaa3a5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk547, &z547, &r547, &s547));

    // 544] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #1213: pseudorandom signature
    let z548 = [0x7f1b40c4cbd36f90, 0x93262cf06340c4fa, 0xdbb5f2c353e632c3, 0xde47c9b27eb8d300];
    let r548 = [0xc69178490d57fb71, 0x39aaf63f00a91f29, 0xe5aada139f52b705, 0x986e65933ef2ed4e];
    let s548 = [0x0f701aaa7a694b9c, 0xdabf0c0217d1c0ff, 0x372308cbf1489bbb, 0x3dafedfb8da6189d];
    let pk548 = [
        0x5b522eba7240fad5,
        0x32e41495a944d004,
        0x13fb8a9e64da3b86,
        0x04aaec73635726f2,
        0x1aa546e8365d525d,
        0xaaf7b4e09fc81d6d,
        0xba01775787ced05e,
        0x87d9315798aaa3a5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk548, &z548, &r548, &s548));

    // 545] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #1303: x-coordinate of the public key has many trailing 0's
    let z549 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r549 = [0x7f29745eff3569f1, 0x50dd0fd5defa013c, 0x81e353a3565e4825, 0xd434e262a49eab77];
    let s549 = [0x844218305c6ba17a, 0x98953195d7bc10de, 0x52fd8077be769c2b, 0x9b0c0a93f267fb60];
    let pk549 = [
        0x9fbd816900000000,
        0xf3807eca11738023,
        0x05e4f1600ae2849d,
        0x4f337ccfd67726a8,
        0xd5df7ea60666d685,
        0x7eb504af43a3146c,
        0x416411e988c30f42,
        0xed9dea124cc8c396,
    ];
    assert!(ecdsa_verify_secp256r1(&pk549, &z549, &r549, &s549));

    // 546] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #1304: x-coordinate of the public key has many trailing 0's
    let z550 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r550 = [0xadd0be9b1979110b, 0x1463489221bf0a33, 0xf76d79fd7a772e42, 0x0fe774355c04d060];
    let s550 = [0xac6181175df55737, 0x4ca8b91a1f325f3f, 0x43fa4f57f743ce12, 0x500dcba1c69a8fbd];
    let pk550 = [
        0x9fbd816900000000,
        0xf3807eca11738023,
        0x05e4f1600ae2849d,
        0x4f337ccfd67726a8,
        0xd5df7ea60666d685,
        0x7eb504af43a3146c,
        0x416411e988c30f42,
        0xed9dea124cc8c396,
    ];
    assert!(ecdsa_verify_secp256r1(&pk550, &z550, &r550, &s550));

    // 547] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #1305: x-coordinate of the public key has many trailing 0's
    let z551 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r551 = [0xbfd06595ee1135e3, 0x8e3b2cd79693f125, 0x950c7d39f03d36dc, 0xbb40bf217bed3fb3];
    let s551 = [0xfa4780745bb55677, 0xc89a1e291ac692b3, 0x32710bdb6a1bf1bf, 0x541bf3532351ebb0];
    let pk551 = [
        0x9fbd816900000000,
        0xf3807eca11738023,
        0x05e4f1600ae2849d,
        0x4f337ccfd67726a8,
        0xd5df7ea60666d685,
        0x7eb504af43a3146c,
        0x416411e988c30f42,
        0xed9dea124cc8c396,
    ];
    assert!(ecdsa_verify_secp256r1(&pk551, &z551, &r551, &s551));

    // 548] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #1306: y-coordinate of the public key has many trailing 0's
    let z552 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r552 = [0x556d3e75a233e73a, 0x05badd5ca99231ff, 0xdf3c86ea31389a54, 0x664eb7ee6db84a34];
    let s552 = [0x2e51a2901426a1bd, 0xe0badc678754b8f7, 0x137642490a51560c, 0x59f3c752e52eca46];
    let pk552 = [
        0x5c3f497265004935,
        0x618f06b8ff87e801,
        0xd499a07873fac281,
        0x3cf03d614d8939cf,
        0xe59cdbde80000000,
        0x7c7a1338a82f85a9,
        0x2ce3880a8960dd2a,
        0x84fa174d791c72bf,
    ];
    assert!(ecdsa_verify_secp256r1(&pk552, &z552, &r552, &s552));

    // 549] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #1307: y-coordinate of the public key has many trailing 0's
    let z553 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r553 = [0x01985a79d1fd8b43, 0x9c3e42e2d1631fd0, 0x009d6fcd843d4ce3, 0x4cd0429bbabd2827];
    let s553 = [0xe466189d2acdabe3, 0xb7bca77a1a2b869a, 0xbe7ef1d0e0d98f08, 0x9638bf12dd682f60];
    let pk553 = [
        0x5c3f497265004935,
        0x618f06b8ff87e801,
        0xd499a07873fac281,
        0x3cf03d614d8939cf,
        0xe59cdbde80000000,
        0x7c7a1338a82f85a9,
        0x2ce3880a8960dd2a,
        0x84fa174d791c72bf,
    ];
    assert!(ecdsa_verify_secp256r1(&pk553, &z553, &r553, &s553));

    // 550] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #1308: y-coordinate of the public key has many trailing 0's
    let z554 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r554 = [0x1e8added97c56c04, 0x60e3ce9aed5e5fd4, 0x1c44d8b6cb62b9f4, 0xe56c6ea2d1b01709];
    let s554 = [0x7fc1378180f89b55, 0x4fcf2b8025807820, 0xbe20b457e463440b, 0xa308ec31f281e955];
    let pk554 = [
        0x5c3f497265004935,
        0x618f06b8ff87e801,
        0xd499a07873fac281,
        0x3cf03d614d8939cf,
        0xe59cdbde80000000,
        0x7c7a1338a82f85a9,
        0x2ce3880a8960dd2a,
        0x84fa174d791c72bf,
    ];
    assert!(ecdsa_verify_secp256r1(&pk554, &z554, &r554, &s554));

    // 551] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #1309: y-coordinate of the public key has many trailing 1's
    let z555 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r555 = [0x011f8fbbf3466830, 0x57c176356a2624fb, 0xcabed3346d891eee, 0x1158a08d291500b4];
    let s555 = [0xa46798c18f285519, 0xc91f378b75d487dd, 0xe082325b85290c5b, 0x228a8c486a736006];
    let pk555 = [
        0x5c3f497265004935,
        0x618f06b8ff87e801,
        0xd499a07873fac281,
        0x3cf03d614d8939cf,
        0x1a6324217fffffff,
        0x8385ecc857d07a56,
        0xd31c77f5769f22d5,
        0x7b05e8b186e38d41,
    ];
    assert!(ecdsa_verify_secp256r1(&pk555, &z555, &r555, &s555));

    // 552] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #1310: y-coordinate of the public key has many trailing 1's
    let z556 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r556 = [0x3e0dde56d309fa9d, 0x2687b29176939dd2, 0x0ea36b0c0fc8d6aa, 0xb1db9289649f5941];
    let s556 = [0x4e1c3f48a1251336, 0x3a6d1af5c23c7d58, 0x5b0dbd987366dcf4, 0x3e1535e428055901];
    let pk556 = [
        0x5c3f497265004935,
        0x618f06b8ff87e801,
        0xd499a07873fac281,
        0x3cf03d614d8939cf,
        0x1a6324217fffffff,
        0x8385ecc857d07a56,
        0xd31c77f5769f22d5,
        0x7b05e8b186e38d41,
    ];
    assert!(ecdsa_verify_secp256r1(&pk556, &z556, &r556, &s556));

    // 553] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #1311: y-coordinate of the public key has many trailing 1's
    let z557 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r557 = [0x0ac6f0ca4e24ed86, 0x0a341a79f2dd1a22, 0x446aa8d4e6e7578b, 0xb7b16e762286cb96];
    let s557 = [0x5e55234ecb8f12bc, 0x1780146df799ccf5, 0x661c547d07bbb072, 0xddc60a700a139b04];
    let pk557 = [
        0x5c3f497265004935,
        0x618f06b8ff87e801,
        0xd499a07873fac281,
        0x3cf03d614d8939cf,
        0x1a6324217fffffff,
        0x8385ecc857d07a56,
        0xd31c77f5769f22d5,
        0x7b05e8b186e38d41,
    ];
    assert!(ecdsa_verify_secp256r1(&pk557, &z557, &r557, &s557));

    // 554] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #1312: x-coordinate of the public key has many trailing 1's
    let z558 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r558 = [0xd1c91c670d9105b4, 0xd796edad36bc6e6b, 0xc8e00d8df963ff35, 0xd82a7c2717261187];
    let s558 = [0x680d07debd139929, 0x351ecd5988efb23f, 0xf4603e7cbac0f3c0, 0x3dcabddaf8fcaa61];
    let pk558 = [
        0xdfa5ff8effffffff,
        0x45956ebcfe8ad0f6,
        0x344ed94bca3fcd05,
        0x2829c31faa2e400e,
        0xeb3bd37ebeb9222e,
        0x4113099052df57e7,
        0x5855afa7676ade28,
        0xa01aafaf000e5258,
    ];
    assert!(ecdsa_verify_secp256r1(&pk558, &z558, &r558, &s558));

    // 555] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #1313: x-coordinate of the public key has many trailing 1's
    let z559 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r559 = [0x6a5cba063254af78, 0x7787802baff30ce9, 0x3d5befe719f462d7, 0x5eb9c8845de68eb1];
    let s559 = [0x2b87ddbe2ef66fb5, 0x44972186228ee9a6, 0x7ca0ff9bbd92fb6e, 0x2c026ae9be2e2a5e];
    let pk559 = [
        0xdfa5ff8effffffff,
        0x45956ebcfe8ad0f6,
        0x344ed94bca3fcd05,
        0x2829c31faa2e400e,
        0xeb3bd37ebeb9222e,
        0x4113099052df57e7,
        0x5855afa7676ade28,
        0xa01aafaf000e5258,
    ];
    assert!(ecdsa_verify_secp256r1(&pk559, &z559, &r559, &s559));

    // 556] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #1314: x-coordinate of the public key has many trailing 1's
    let z560 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r560 = [0x404a8e4e36230c28, 0xf277921becc117d0, 0xf3b782b170239f90, 0x96843dd03c22abd2];
    let s560 = [0x19e1ede123dd991d, 0x9a31214eb4d7e6db, 0x43f67165976de9ed, 0xf2be378f526f74a5];
    let pk560 = [
        0xdfa5ff8effffffff,
        0x45956ebcfe8ad0f6,
        0x344ed94bca3fcd05,
        0x2829c31faa2e400e,
        0xeb3bd37ebeb9222e,
        0x4113099052df57e7,
        0x5855afa7676ade28,
        0xa01aafaf000e5258,
    ];
    assert!(ecdsa_verify_secp256r1(&pk560, &z560, &r560, &s560));

    // 557] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #1315: x-coordinate of the public key is large
    let z561 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r561 = [0x3b760297067421f6, 0x4d27e9d98edc2d0e, 0x6f9996af72933946, 0x766456dce1857c90];
    let s561 = [0x3646bfbbf19d0b41, 0x4e55376eced699e9, 0x81dccaf5d19037ec, 0x402385ecadae0d80];
    let pk561 = [
        0x08318c4ca9a7a4f5,
        0x65ff9059ad6aac07,
        0x0458dd8f9e738f26,
        0xfffffff948081e6a,
        0x78e6529a1663bd73,
        0xe0c0fb89557ad0bf,
        0x311ee54149b973ca,
        0x5a8abcba2dda8474,
    ];
    assert!(ecdsa_verify_secp256r1(&pk561, &z561, &r561, &s561));

    // 558] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #1316: x-coordinate of the public key is large
    let z562 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r562 = [0x9c34f777de7b9fd9, 0xb97ed8b07cced0b1, 0x19e6518a11b2dbc2, 0xc605c4b2edeab204];
    let s562 = [0xff5e159d47326dba, 0xb2cde2eda700fb1c, 0xc719647bc8af1b29, 0xedf0f612c5f46e03];
    let pk562 = [
        0x08318c4ca9a7a4f5,
        0x65ff9059ad6aac07,
        0x0458dd8f9e738f26,
        0xfffffff948081e6a,
        0x78e6529a1663bd73,
        0xe0c0fb89557ad0bf,
        0x311ee54149b973ca,
        0x5a8abcba2dda8474,
    ];
    assert!(ecdsa_verify_secp256r1(&pk562, &z562, &r562, &s562));

    // 559] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #1317: x-coordinate of the public key is large
    let z563 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r563 = [0xb732bfe3b7eb8a84, 0x10e64485d9929ad7, 0xf6141c9ac54141f2, 0xd48b68e6cabfe03c];
    let s563 = [0x08f0772315b6c941, 0x4508c389109ad2f2, 0x19dc26f9b7e2265e, 0xfeedae50c61bd00e];
    let pk563 = [
        0x08318c4ca9a7a4f5,
        0x65ff9059ad6aac07,
        0x0458dd8f9e738f26,
        0xfffffff948081e6a,
        0x78e6529a1663bd73,
        0xe0c0fb89557ad0bf,
        0x311ee54149b973ca,
        0x5a8abcba2dda8474,
    ];
    assert!(ecdsa_verify_secp256r1(&pk563, &z563, &r563, &s563));

    // 560] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #1318: x-coordinate of the public key is small
    let z564 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r564 = [0xc35a93d12a5dd4c7, 0x710ad7f6595d5874, 0x65957098569f0479, 0xb7c81457d4aeb6aa];
    let s564 = [0x4b9e3a05c0a1cdb3, 0x1a9199f2ca574dad, 0xd568069a432ca18a, 0xb7961a0b652878c2];
    let pk564 = [
        0xbff1173937ba748e,
        0x6f9e0015eeb23aeb,
        0x949d5f03a6f5c7f8,
        0x00000003fa15f963,
        0x548ca48af2ba7e71,
        0xfadcfcb0023ea889,
        0x555fa13659cca5d7,
        0x1099872070e8e87c,
    ];
    assert!(ecdsa_verify_secp256r1(&pk564, &z564, &r564, &s564));

    // 561] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #1319: x-coordinate of the public key is small
    let z565 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r565 = [0xde5e9652e76ff3f7, 0xe3cf97e263e669f8, 0xa30a1321d5858e1e, 0x6b01332ddb6edfa9];
    let s565 = [0xcc58f9e69e96cd5a, 0x139c8f7d86b02cb1, 0x9a6a04ace2bd0f70, 0x5939545fced45730];
    let pk565 = [
        0xbff1173937ba748e,
        0x6f9e0015eeb23aeb,
        0x949d5f03a6f5c7f8,
        0x00000003fa15f963,
        0x548ca48af2ba7e71,
        0xfadcfcb0023ea889,
        0x555fa13659cca5d7,
        0x1099872070e8e87c,
    ];
    assert!(ecdsa_verify_secp256r1(&pk565, &z565, &r565, &s565));

    // 562] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #1320: x-coordinate of the public key is small
    let z566 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r566 = [0x0e6a4fb93f106361, 0x4101cd2fd8436b7d, 0x349f9fc356b6c034, 0xefdb884720eaeadc];
    let s566 = [0xe48cb60d8113385d, 0xcba9e77de7d69b6c, 0x613975473aadf3aa, 0xf24bee6ad5dc05f7];
    let pk566 = [
        0xbff1173937ba748e,
        0x6f9e0015eeb23aeb,
        0x949d5f03a6f5c7f8,
        0x00000003fa15f963,
        0x548ca48af2ba7e71,
        0xfadcfcb0023ea889,
        0x555fa13659cca5d7,
        0x1099872070e8e87c,
    ];
    assert!(ecdsa_verify_secp256r1(&pk566, &z566, &r566, &s566));

    // 563] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #1321: y-coordinate of the public key is small
    let z567 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r567 = [0x8014c87b8b20eb07, 0x9b23a23dd973dcbe, 0xb88fb5a646836aea, 0x31230428405560dc];
    let s567 = [0x8bd7ae3d9bd0beff, 0xaf97374e19f3c5fb, 0x6646747694a41b0a, 0x0f9344d6e812ce16];
    let pk567 = [
        0x25e9f851c18af015,
        0xbe5d2d6796707d81,
        0xaa6ecbbc612816b3,
        0xbcbb2914c79f045e,
        0xf300a698a7193bc2,
        0xdd684ade5a1127bc,
        0x0fa2ea4cceb9ab63,
        0x000000001352bb4a,
    ];
    assert!(ecdsa_verify_secp256r1(&pk567, &z567, &r567, &s567));

    // 564] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #1322: y-coordinate of the public key is small
    let z568 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r568 = [0x9174db34c4855743, 0x94359c7db9841d67, 0x0d5c470cda0b36b2, 0xcaa797da65b320ab];
    let s568 = [0x3de6d9b36242e5a0, 0x123d2685ee3b941d, 0x45391aaf7505f345, 0xcf543a62f23e2127];
    let pk568 = [
        0x25e9f851c18af015,
        0xbe5d2d6796707d81,
        0xaa6ecbbc612816b3,
        0xbcbb2914c79f045e,
        0xf300a698a7193bc2,
        0xdd684ade5a1127bc,
        0x0fa2ea4cceb9ab63,
        0x000000001352bb4a,
    ];
    assert!(ecdsa_verify_secp256r1(&pk568, &z568, &r568, &s568));

    // 565] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #1323: y-coordinate of the public key is small
    let z569 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r569 = [0x1c336ed800185945, 0x19bc54084536e7d2, 0xd7867657e5d6d365, 0x7e5f0ab5d900d3d3];
    let s569 = [0xe727ff0b19b646aa, 0x6688294aad35aa72, 0x4b82dfb322e5ac67, 0x9450c07f201faec9];
    let pk569 = [
        0x25e9f851c18af015,
        0xbe5d2d6796707d81,
        0xaa6ecbbc612816b3,
        0xbcbb2914c79f045e,
        0xf300a698a7193bc2,
        0xdd684ade5a1127bc,
        0x0fa2ea4cceb9ab63,
        0x000000001352bb4a,
    ];
    assert!(ecdsa_verify_secp256r1(&pk569, &z569, &r569, &s569));

    // 566] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #1324: y-coordinate of the public key is large
    let z570 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r570 = [0x03aa69f0ca25b356, 0x3f8a1e4a2136fe4b, 0x6dc6a480bf037ae2, 0xd7d70c581ae9e3f6];
    let s570 = [0xaf41d9127cc47224, 0x13e85658e62a59e2, 0xba962c8a3ee833a4, 0x89c460f8a5a5c2bb];
    let pk570 = [
        0x25e9f851c18af015,
        0xbe5d2d6796707d81,
        0xaa6ecbbc612816b3,
        0xbcbb2914c79f045e,
        0x0cff596758e6c43d,
        0x2297b522a5eed843,
        0xf05d15b33146549c,
        0xfffffffeecad44b6,
    ];
    assert!(ecdsa_verify_secp256r1(&pk570, &z570, &r570, &s570));

    // 567] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #1325: y-coordinate of the public key is large
    let z571 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r571 = [0xeee34bb396266b34, 0xb7aa20c625975e5e, 0xe0dfa0bf68bcdf4b, 0x341c1b9ff3c83dd5];
    let s571 = [0x902a67099e0a4469, 0x49c634e77765a017, 0x121b22b11366fad5, 0x72b69f061b750fd5];
    let pk571 = [
        0x25e9f851c18af015,
        0xbe5d2d6796707d81,
        0xaa6ecbbc612816b3,
        0xbcbb2914c79f045e,
        0x0cff596758e6c43d,
        0x2297b522a5eed843,
        0xf05d15b33146549c,
        0xfffffffeecad44b6,
    ];
    assert!(ecdsa_verify_secp256r1(&pk571, &z571, &r571, &s571));

    // 568] wycheproof/ecdsa_test.json EcdsaVerify SHA-256 #1326: y-coordinate of the public key is large
    let z572 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r572 = [0x7628d313a3814f67, 0x9bd1781a59180994, 0x72a42f0d87387935, 0x70bebe684cdcb5ca];
    let s572 = [0x2dcde6b2094798a9, 0xc0e464b1c3577f4c, 0xd535fa31027bbe9c, 0xaec03aca8f5587a4];
    let pk572 = [
        0x25e9f851c18af015,
        0xbe5d2d6796707d81,
        0xaa6ecbbc612816b3,
        0xbcbb2914c79f045e,
        0x0cff596758e6c43d,
        0x2297b522a5eed843,
        0xf05d15b33146549c,
        0xfffffffeecad44b6,
    ];
    assert!(ecdsa_verify_secp256r1(&pk572, &z572, &r572, &s572));

    // 569] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #1: signature malleability
    let z573 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r573 = [0xb8cc6af9bd5c2e18, 0xffe50d85a1eee859, 0x80a6d9d1190a436e, 0x2ba3a8be6b94d5ec];
    let s573 = [0x77a67f79e6fadd76, 0x525fe710fab9aa7c, 0x3c7b11eb6c4e0ae7, 0x4cd60b855d442f5b];
    let pk573 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk573, &z573, &r573, &s573));

    // 570] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #3: Modified r or s, e.g. by adding or subtracting the order of the group
    let z574 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r574 = [0x3aed5fc93f06f739, 0xbd01ed280528b62b, 0x7f59262ee6f5bc90, 0xd45c5740946b2a14];
    let s574 = [0x7c134b49156847db, 0x6a87139cac5df408, 0xc384ee1493b1f518, 0xb329f479a2bbd0a5];
    let pk574 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk574, &z574, &r574, &s574));

    // 571] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #5: Modified r or s, e.g. by adding or subtracting the order of the group
    let z575 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r575 = [0x4733950642a3d1e8, 0x001af27a5e1117a6, 0x7f59262ee6f5bc91, 0xd45c5741946b2a13];
    let s575 = [0x7c134b49156847db, 0x6a87139cac5df408, 0xc384ee1493b1f518, 0xb329f479a2bbd0a5];
    let pk575 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk575, &z575, &r575, &s575));

    // 572] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #8: Modified r or s, e.g. by adding or subtracting the order of the group
    let z576 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r576 = [0xb8cc6af9bd5c2e18, 0xffe50d85a1eee859, 0x80a6d9d1190a436e, 0x2ba3a8be6b94d5ec];
    let s576 = [0x83ecb4b6ea97b825, 0x9578ec6353a20bf7, 0x3c7b11eb6c4e0ae7, 0x4cd60b865d442f5a];
    let pk576 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk576, &z576, &r576, &s576));

    // 573] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #9: Signature with special case values for r and s
    let z577 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r577 = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s577 = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk577 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk577, &z577, &r577, &s577));

    // 574] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #10: Signature with special case values for r and s
    let z578 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r578 = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s578 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk578 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk578, &z578, &r578, &s578));

    // 575] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #11: Signature with special case values for r and s
    let z579 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r579 = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s579 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk579 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk579, &z579, &r579, &s579));

    // 576] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #12: Signature with special case values for r and s
    let z580 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r580 = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s580 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk580 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk580, &z580, &r580, &s580));

    // 577] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #13: Signature with special case values for r and s
    let z581 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r581 = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s581 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk581 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk581, &z581, &r581, &s581));

    // 578] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #14: Signature with special case values for r and s
    let z582 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r582 = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s582 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let pk582 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk582, &z582, &r582, &s582));

    // 579] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #15: Signature with special case values for r and s
    let z583 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r583 = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s583 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let pk583 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk583, &z583, &r583, &s583));

    // 580] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #16: Signature with special case values for r and s
    let z584 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r584 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s584 = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk584 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk584, &z584, &r584, &s584));

    // 581] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #17: Signature with special case values for r and s
    let z585 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r585 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s585 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk585 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk585, &z585, &r585, &s585));

    // 582] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #18: Signature with special case values for r and s
    let z586 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r586 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s586 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk586 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk586, &z586, &r586, &s586));

    // 583] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #19: Signature with special case values for r and s
    let z587 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r587 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s587 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk587 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk587, &z587, &r587, &s587));

    // 584] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #20: Signature with special case values for r and s
    let z588 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r588 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s588 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk588 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk588, &z588, &r588, &s588));

    // 585] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #21: Signature with special case values for r and s
    let z589 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r589 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s589 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let pk589 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk589, &z589, &r589, &s589));

    // 586] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #22: Signature with special case values for r and s
    let z590 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r590 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s590 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let pk590 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk590, &z590, &r590, &s590));

    // 587] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #23: Signature with special case values for r and s
    let z591 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r591 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s591 = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk591 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk591, &z591, &r591, &s591));

    // 588] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #24: Signature with special case values for r and s
    let z592 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r592 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s592 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk592 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk592, &z592, &r592, &s592));

    // 589] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #25: Signature with special case values for r and s
    let z593 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r593 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s593 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk593 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk593, &z593, &r593, &s593));

    // 590] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #26: Signature with special case values for r and s
    let z594 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r594 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s594 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk594 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk594, &z594, &r594, &s594));

    // 591] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #27: Signature with special case values for r and s
    let z595 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r595 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s595 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk595 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk595, &z595, &r595, &s595));

    // 592] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #28: Signature with special case values for r and s
    let z596 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r596 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s596 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let pk596 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk596, &z596, &r596, &s596));

    // 593] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #29: Signature with special case values for r and s
    let z597 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r597 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s597 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let pk597 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk597, &z597, &r597, &s597));

    // 594] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #30: Signature with special case values for r and s
    let z598 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r598 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s598 = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk598 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk598, &z598, &r598, &s598));

    // 595] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #31: Signature with special case values for r and s
    let z599 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r599 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s599 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk599 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk599, &z599, &r599, &s599));

    // 596] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #32: Signature with special case values for r and s
    let z600 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r600 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s600 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk600 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk600, &z600, &r600, &s600));

    // 597] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #33: Signature with special case values for r and s
    let z601 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r601 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s601 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk601 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk601, &z601, &r601, &s601));

    // 598] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #34: Signature with special case values for r and s
    let z602 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r602 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s602 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk602 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk602, &z602, &r602, &s602));

    // 599] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #35: Signature with special case values for r and s
    let z603 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r603 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s603 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let pk603 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk603, &z603, &r603, &s603));

    // 600] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #36: Signature with special case values for r and s
    let z604 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r604 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s604 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let pk604 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk604, &z604, &r604, &s604));

    // 601] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #37: Signature with special case values for r and s
    let z605 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r605 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s605 = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk605 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk605, &z605, &r605, &s605));

    // 602] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #38: Signature with special case values for r and s
    let z606 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r606 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s606 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk606 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk606, &z606, &r606, &s606));

    // 603] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #39: Signature with special case values for r and s
    let z607 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r607 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s607 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk607 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk607, &z607, &r607, &s607));

    // 604] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #40: Signature with special case values for r and s
    let z608 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r608 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s608 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk608 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk608, &z608, &r608, &s608));

    // 605] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #41: Signature with special case values for r and s
    let z609 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r609 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s609 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk609 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk609, &z609, &r609, &s609));

    // 606] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #42: Signature with special case values for r and s
    let z610 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r610 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s610 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let pk610 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk610, &z610, &r610, &s610));

    // 607] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #43: Signature with special case values for r and s
    let z611 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r611 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s611 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let pk611 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk611, &z611, &r611, &s611));

    // 608] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #44: Signature with special case values for r and s
    let z612 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r612 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let s612 = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk612 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk612, &z612, &r612, &s612));

    // 609] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #45: Signature with special case values for r and s
    let z613 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r613 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let s613 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk613 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk613, &z613, &r613, &s613));

    // 610] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #46: Signature with special case values for r and s
    let z614 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r614 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let s614 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk614 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk614, &z614, &r614, &s614));

    // 611] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #47: Signature with special case values for r and s
    let z615 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r615 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let s615 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk615 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk615, &z615, &r615, &s615));

    // 612] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #48: Signature with special case values for r and s
    let z616 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r616 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let s616 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk616 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk616, &z616, &r616, &s616));

    // 613] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #49: Signature with special case values for r and s
    let z617 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r617 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let s617 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let pk617 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk617, &z617, &r617, &s617));

    // 614] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #50: Signature with special case values for r and s
    let z618 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r618 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let s618 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let pk618 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk618, &z618, &r618, &s618));

    // 615] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #51: Signature with special case values for r and s
    let z619 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r619 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let s619 = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk619 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk619, &z619, &r619, &s619));

    // 616] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #52: Signature with special case values for r and s
    let z620 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r620 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let s620 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk620 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk620, &z620, &r620, &s620));

    // 617] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #53: Signature with special case values for r and s
    let z621 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r621 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let s621 = [0xf3b9cac2fc632551, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk621 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk621, &z621, &r621, &s621));

    // 618] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #54: Signature with special case values for r and s
    let z622 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r622 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let s622 = [0xf3b9cac2fc632550, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk622 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk622, &z622, &r622, &s622));

    // 619] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #55: Signature with special case values for r and s
    let z623 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r623 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let s623 = [0xf3b9cac2fc632552, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk623 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk623, &z623, &r623, &s623));

    // 620] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #56: Signature with special case values for r and s
    let z624 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r624 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let s624 = [0xffffffffffffffff, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let pk624 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk624, &z624, &r624, &s624));

    // 621] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #57: Signature with special case values for r and s
    let z625 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r625 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let s625 = [0x0000000000000000, 0x0000000100000000, 0x0000000000000000, 0xffffffff00000001];
    let pk625 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk625, &z625, &r625, &s625));

    // 622] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #58: Edge case for Shamir multiplication
    let z626 = [0x2fa50c772ed6f807, 0x2f2627416faf2f07, 0xc422f44dea4ed1a5, 0x70239dd877f7c944];
    let r626 = [0x11547c97711c898e, 0x8ff312334e2ba16d, 0x4f3e2fc02bdee9be, 0x64a1aab5000d0e80];
    let s626 = [0xfd683b9bb2cf4f1b, 0x7772a2f91d73286f, 0xd1a206d4e013e099, 0x6af015971cc30be6];
    let pk626 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk626, &z626, &r626, &s626));

    // 623] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #59: special case hash
    let z627 = [0x7ead3645f356e7a9, 0x84bcd58a1bb5e747, 0xccf17803ebe2bd08, 0x00000000690ed426];
    let r627 = [0x1e19a0ec580bf266, 0xded7d397738448de, 0x6f78c81c91fc7e8b, 0x16aea964a2f6506d];
    let s627 = [0x38c3ff033be928e9, 0x391e8e80c578d1cd, 0xcfe8b7bc47d27d78, 0x252cd762130c6667];
    let pk627 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk627, &z627, &r627, &s627));

    // 624] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #60: special case hash
    let z628 = [0x140697ad25770d91, 0xf696ad3ebb5ee47f, 0x525c6035725235c2, 0x7300000000213f2a];
    let r628 = [0x7c665baccb23c882, 0xf2d26d6ef524af91, 0xf476dfc26b9b733d, 0x9cc98be2347d469b];
    let s628 = [0xa631dacb16b56c32, 0x0ec1b7847929d10e, 0xd70727b82462f61d, 0x093496459effe2d8];
    let pk628 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk628, &z628, &r628, &s628));

    // 625] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #61: special case hash
    let z629 = [0x4a0161c27fe06045, 0x8afd25daadeb3edb, 0xe0635b245f0b9797, 0xddf2000000005e0b];
    let r629 = [0x093999f07ab8aa43, 0x03dce3dea0d53fa8, 0x058164524dde8927, 0x73b3c90ecd390028];
    let s629 = [0x188c0c4075c88634, 0x2ed25a395387b5f4, 0x5bb7d8bf0a651c80, 0x2f67b0b8e2063669];
    let pk629 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk629, &z629, &r629, &s629));

    // 626] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #62: special case hash
    let z630 = [0x5be1ec355d0841a0, 0x642b8499588b8985, 0x4769c4ecb9e164d6, 0x67ab190000000078];
    let r630 = [0xf37e90119d5ba3dd, 0x1a7f0eb390763378, 0x28fadf2f89b95c85, 0xbfab3098252847b3];
    let s630 = [0x1e2da9b8b4987e3b, 0x8195ccebb65c2aaf, 0x67c2d058ccb44d97, 0xbdd64e234e832b10];
    let pk630 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk630, &z630, &r630, &s630));

    // 627] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #63: special case hash
    let z631 = [0xe296b6350fc311cf, 0x02095dff252ee905, 0x76d7dbeffe125eaf, 0xa2bf094600000000];
    let r631 = [0xd17093c5cd21d2cd, 0xf1c9aaab168b1596, 0x8bf8bf04a4ceb1c1, 0x204a9784074b246d];
    let s631 = [0x582fe648d1d88b52, 0xa406c2506fe17975, 0xdc06a759c8847868, 0x51cce41670636783];
    let pk631 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk631, &z631, &r631, &s631));

    // 628] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #64: special case hash
    let z632 = [0x29e15c544e4f0e65, 0xa0a3531711608581, 0x00e1e75e624a06b3, 0x3554e827c7000000];
    let r632 = [0x027bca0f1ceeaa03, 0x0031a91d1314f835, 0xf63d4aa4f81fe2cb, 0xed66dc34f551ac82];
    let s632 = [0xbb8953d67c0c48c7, 0x67623c3f6e5d4d6a, 0x194a422e18d5fda1, 0x99ca123aa09b13cd];
    let pk632 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk632, &z632, &r632, &s632));

    // 629] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #65: special case hash
    let z633 = [0x26e3a54b9fc6965c, 0x3255ea4c9fd0cb34, 0x000026941a0f0bb5, 0x9b6cd3b812610000];
    let r633 = [0x56bf0f60a237012b, 0x126b062023ccc3c0, 0x899d44f2356a578d, 0x060b700bef665c68];
    let s633 = [0x6be5d581c11d3610, 0xedbb410cbef3f26d, 0x4fcc78a3366ca95d, 0x8d186c027832965f];
    let pk633 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk633, &z633, &r633, &s633));

    // 630] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #66: special case hash
    let z634 = [0x77162f93c4ae0186, 0x82a52baa51c71ca8, 0x000000e7561c26fc, 0x883ae39f50bf0100];
    let r634 = [0x2bb0c8e38c96831d, 0xc93ea76cd313c913, 0x24d7aa7934b6cf29, 0x9f6adfe8d5eb5b2c];
    let s634 = [0x051593883b5e9902, 0x906a33e66b5bd15e, 0x890c944cf271756c, 0xb26a9c9e40e55ee0];
    let pk634 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk634, &z634, &r634, &s634));

    // 631] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #67: special case hash
    let z635 = [0x01fe9fce011d0ba6, 0x10540f420fb4ff74, 0x0000000000fa7cd0, 0xa1ce5d6e5ecaf28b];
    let r635 = [0x8868f4ba273f16b7, 0xa1abf6da168cebfa, 0x3ad2f33615e56174, 0xa1af03ca91677b67];
    let s635 = [0x5caf24c8c5e06b1c, 0x77d69022e7d098d7, 0x35cd258b173d0c23, 0x20aa73ffe48afa64];
    let pk635 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk635, &z635, &r635, &s635));

    // 632] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #68: special case hash
    let z636 = [0x5494cdffd5ee8054, 0x97330012a8ee836c, 0x9300000000383453, 0x8ea5f645f373f580];
    let r636 = [0xe327a28c11893db9, 0x659355507b843da6, 0x11a6c99a71c973d5, 0xfdc70602766f8eed];
    let s636 = [0xa7f83f2b10d21350, 0x0f6d15ec0078ca60, 0x37b1eacf456a9e9e, 0x3df5349688a085b1];
    let pk636 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk636, &z636, &r636, &s636));

    // 633] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #69: special case hash
    let z637 = [0x8d9c1bbdcb5ef305, 0xd65ce93eabb7d60d, 0xa734000000008792, 0x660570d323e9f75f];
    let r637 = [0x0dc738f7b876e675, 0x23456f63c643cf8e, 0xd6537f6a6c49966c, 0xb516a314f2fce530];
    let s637 = [0xa66b0120cd16fff2, 0x967c4bd80954479b, 0x17dd536fbc5efdf1, 0xd39ffd033c92b6d7];
    let pk637 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk637, &z637, &r637, &s637));

    // 634] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #70: special case hash
    let z638 = [0x46ada2de4c568c34, 0x8d35f1f45cf9c3bf, 0x7dde8800000000e9, 0xd0462673154cce58];
    let r638 = [0xa485c101e29ff0a8, 0x82717bebb6492fd0, 0x2ecb7984d4758315, 0x3b2cbf046eac4584];
    let s638 = [0xc8595fc1c1d99258, 0x701099cac5f76e68, 0xde512bc9313aaf51, 0x4c9b7b47a98b0f82];
    let pk638 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk638, &z638, &r638, &s638));

    // 635] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #71: special case hash
    let z639 = [0xb83e7b4418d7278f, 0x0caef15a6171059a, 0x80cedfef00000000, 0xbd90640269a78226];
    let r639 = [0x6c3fb15bfde48dcf, 0xd79d0312cfa1ab65, 0x841f14af54e2f9ed, 0x30c87d35e636f540];
    let s639 = [0x0db9abf6340677ed, 0x71409ede23efd08e, 0xc85a692bd6ecafeb, 0x47c15a5a82d24b75];
    let pk639 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk639, &z639, &r639, &s639));

    // 636] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #72: special case hash
    let z640 = [0x4beae8e284788a73, 0x00d2dcceb301c54b, 0x512e41222a000000, 0x33239a52d72f1311];
    let r640 = [0x68ff262113760f52, 0xe2e8176d168dec3c, 0xbc43b58cfe6647b9, 0x38686ff0fda2cef6];
    let s640 = [0xc2ddabb3fde9d67d, 0xe976e2db5e6a4cf7, 0x9601662167fa8717, 0x067ec3b651f42266];
    let pk640 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk640, &z640, &r640, &s640));

    // 637] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #73: special case hash
    let z641 = [0x1dc84c2d941ffaf1, 0x00007ee4a21a1cbe, 0x1365d4e6d95c0000, 0xb8d64fbcd4a1c10f];
    let r641 = [0x225985ab6e2775cf, 0xf3e17d27f5ee844b, 0x44fc25c7f2de8b6a, 0x44a3e23bf314f2b3];
    let s641 = [0x93c9cc3f4dd15e86, 0x84f0411f57295004, 0x1ddc87be532abed5, 0x2d48e223205e9804];
    let pk641 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk641, &z641, &r641, &s641));

    // 638] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #74: special case hash
    let z642 = [0x4088b20fe0e9d84a, 0x0000003a227420db, 0xa3fef3183ed09200, 0x01603d3982bf77d7];
    let r642 = [0x0eb9d638781688e9, 0x41b99db3b5aa8d33, 0xf11f967a3d95110c, 0x2ded5b7ec8e90e7b];
    let s642 = [0xec69238a009808f9, 0x8de049c328ae1f44, 0x1bfc46fb1a67e308, 0x7d5792c53628155e];
    let pk642 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk642, &z642, &r642, &s642));

    // 639] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #75: special case hash
    let z643 = [0xb7e9eb0cfbff7363, 0x000000004d89ef50, 0x599aa02e6cf66d9c, 0x9ea6994f1e0384c8];
    let r643 = [0x05976f15137d8b8f, 0x3eaccafcd40ec2f6, 0xefd3bc3d31870f92, 0xbdae7bcb580bf335];
    let s643 = [0x24838122ce7ec3c7, 0x9f373a4fb318994f, 0x0b0106eecfe25749, 0xf6dfa12f19e52527];
    let pk643 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk643, &z643, &r643, &s643));

    // 640] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #76: special case hash
    let z644 = [0xf692bc670905b18c, 0x4700000000e2fa5b, 0x693979371a01068a, 0xd03215a8401bcf16];
    let r644 = [0x1ece251c2401f1c6, 0x99209b78596956d2, 0x62720957ffff5137, 0x50f9c4f0cd6940e1];
    let s644 = [0xaa5167dfab244726, 0x5a4355e411a59c32, 0x889defaaabb106b9, 0xd7033a0a787d338e];
    let pk644 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk644, &z644, &r644, &s644));

    // 641] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #77: special case hash
    let z645 = [0xfd5f64b582e3bb14, 0xc87e000000008408, 0x9c84bf83f0300e5d, 0x307bfaaffb650c88];
    let r645 = [0xbe90924ead5c860d, 0x0982e29575d019aa, 0x1906066a378d6754, 0xf612820687604fa0];
    let s645 = [0x328230ce294b0fef, 0x1a99f4857b316525, 0x75ea98afd20e328a, 0x3f9367702dd7dd4f];
    let pk645 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk645, &z645, &r645, &s645));

    // 642] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #78: special case hash
    let z646 = [0xaf574bb4d54ea6b8, 0x51527c00000000e4, 0x33324d36bb0c1575, 0xbab5c4f4df540d7b];
    let r646 = [0x0f2f507da5782a7a, 0x1f61980c1949f56b, 0xc93db5da7aa6f508, 0x9505e407657d6e8b];
    let s646 = [0x5e7f71784f9c5021, 0x08e0ed5cb92b3cfa, 0x8ffbeccab6c3656c, 0xc60d31904e366973];
    let pk646 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk646, &z646, &r646, &s646));

    // 643] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #79: special case hash
    let z647 = [0xc3b869197ef5e15e, 0xc2456f5b00000000, 0xe4f58d8036f9c36e, 0xd4ba47f6ae28f274];
    let r647 = [0x3e1c68a40404517d, 0x08735aed37173272, 0xd83e6a7787cd691b, 0xbbd16fbbb656b6d0];
    let s647 = [0x560e3e7fd25c0f00, 0x7d2d097be5e8ee34, 0x787d91315be67587, 0x9d8e35dba96028b7];
    let pk647 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk647, &z647, &r647, &s647));

    // 644] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #80: special case hash
    let z648 = [0x00801e47f8c184e1, 0xfe0f10aafd000000, 0xf29f1fa00984342a, 0x79fd19c7235ea212];
    let r648 = [0xcf57c61e92df327e, 0x442d2ceef7559a30, 0x06ea76848d35a6da, 0x2ec9760122db98fd];
    let s648 = [0xc4963625c0a19878, 0x393fb6814c27b760, 0x701fccf86e462ee3, 0x7ab271da90859479];
    let pk648 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk648, &z648, &r648, &s648));

    // 645] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #81: special case hash
    let z649 = [0x0000a37ea6700cda, 0x79cbeb7ac9730000, 0xaf9aba5c0583462d, 0x8c291e8eeaa45adb];
    let r649 = [0x4f1005a89fe00c59, 0xd9ba9dd463221f7a, 0xaa6a7fc49b1c51ee, 0x54e76b7683b6650b];
    let s649 = [0x52f2f7806a31c8fd, 0xcfd11b1c1ae11661, 0x37ec1cc8374b7915, 0x2ea076886c773eb9];
    let pk649 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk649, &z649, &r649, &s649));

    // 646] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #82: special case hash
    let z650 = [0x0000003c278a6b21, 0xf4cdcf66c3f78a00, 0x9803efbfb8140732, 0x0eaae8641084fa97];
    let r650 = [0x10419c0c496c9466, 0x7a74abdbb69be4fb, 0xbce6e3c26f602109, 0x5291deaf24659ffb];
    let s650 = [0xbf83469270a03dc3, 0x827f84742f29f10a, 0xcdb982bb4e4ecef5, 0x65d6fcf336d27cc7];
    let pk650 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk650, &z650, &r650, &s650));

    // 647] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #83: special case hash
    let z651 = [0x00000000afc0f89d, 0xef17c6d96e13846c, 0x0068399bf01bab42, 0xe02716d01fb23a5a];
    let r651 = [0xd15166a88479f107, 0x003b33fc17eb50f9, 0x47419dc58efb05e8, 0x207a3241812d75d9];
    let s651 = [0x82d5caadf7592767, 0xf1c5d70793cf55e3, 0x3ce80b32d0574f62, 0xcdee749f2e492b21];
    let pk651 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk651, &z651, &r651, &s651));

    // 648] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #84: special case hash
    let z652 = [0x9a00000000fc7de1, 0x9061768af89d0065, 0x194e9a16bc7dab2a, 0x9eb0bf583a1a6b9a];
    let r652 = [0xc0dee3cf81aa7728, 0xbe84437a355a0a37, 0x4328ac94913bf01b, 0x6554e49f82a85520];
    let s652 = [0x86effe7f22b4f929, 0x16250a2eaebc8be4, 0xc94e1e126980d3df, 0xaea00de2507ddaf5];
    let pk652 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk652, &z652, &r652, &s652));

    // 649] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #85: special case hash
    let z653 = [0x690e00000000cd15, 0x6e1030cb53d9a82b, 0x2c214f0d5e72ef28, 0x62aac98818b3b84a];
    let r653 = [0x2990ac82707efdfc, 0x6c6e19b4d80a8c60, 0xbff06f71c88216c2, 0xa54c5062648339d2];
    let s653 = [0xff09be73c9731b0d, 0x1056317f467ad09a, 0x69fd016777517aa0, 0xe99bbe7fcfafae3e];
    let pk653 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk653, &z653, &r653, &s653));

    // 650] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #86: special case hash
    let z654 = [0x464b9300000000c8, 0xd2b6f552ea4b6895, 0xf29ae43732e513ef, 0x3760a7f37cf96218];
    let r654 = [0x4ca8b059cff37eaf, 0xd23096593133e71b, 0x309f1f444012b1a1, 0x975bd7157a8d363b];
    let s654 = [0xacc46786bf919622, 0xd4c69840fe090f2a, 0xa241793f2abc930b, 0x7faa7a28b1c822ba];
    let pk654 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk654, &z654, &r654, &s654));

    // 651] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #87: special case hash
    let z655 = [0xbb6ff6c800000000, 0x6b4320bea836cd9c, 0x3834f2098c088009, 0x0da0a1d2851d3302];
    let r655 = [0x7b95b3e0da43885e, 0xde9ec90305afb135, 0x276afd2ebcfe4d61, 0x5694a6f84b8f875c];
    let s655 = [0x3b6ccc7c679cbaa4, 0x8ee2dc5c7870c082, 0x8051dec02ebdf70d, 0x0dffad9ffd0b757d];
    let pk655 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk655, &z655, &r655, &s655));

    // 652] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #88: special case hash
    let z656 = [0xa764a231e82d289a, 0x0fe975f735887194, 0x086fd567aafd598f, 0xffffffff293886d3];
    let r656 = [0xd7454ba9790f1ba6, 0xf7098f1a98d21620, 0xb4968a27d16a6d08, 0xa0c30e8026fdb2b4];
    let s656 = [0x8bd2760c65424339, 0xacc5ca6445914968, 0x5baf463f9deceb53, 0x5e470453a8a399f1];
    let pk656 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk656, &z656, &r656, &s656));

    // 653] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #89: special case hash
    let z657 = [0x0e8d9ca99527e7b7, 0x26acdc4ce127ec2e, 0xe3c03445a072e243, 0x7bffffffff2376d1];
    let r657 = [0x2aa0228cf7b99a88, 0x1dfebebd5ad8aca5, 0xdd73602cd4bb4eea, 0x614ea84acf736527];
    let s657 = [0x2a4dd193195c902f, 0xde14368e96a9482c, 0xd1b8183f3ed490e4, 0x737cc85f5f2d2f60];
    let pk657 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk657, &z657, &r657, &s657));

    // 654] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #90: special case hash
    let z658 = [0xfd016807e97fa395, 0xbc80872602a6e467, 0x51b085377605a224, 0xa2b5ffffffffebb2];
    let r658 = [0xa8d74dfbd0f942fa, 0x45377338febfd439, 0x0d3fb2ea00b17329, 0xbead6734ebe44b81];
    let s658 = [0x36a46b103ef56e2a, 0xf4bbe7a10f73b3e0, 0x3cad35919fd21a8a, 0x6bb18eae36616a7d];
    let pk658 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk658, &z658, &r658, &s658));

    // 655] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #91: special case hash
    let z659 = [0x7b83d0967d4b20c0, 0xc1a3c256870d45a6, 0x1b96fa5f097fcf3c, 0x641227ffffffff6f];
    let r659 = [0x654fae182df9bad2, 0x8d922cbf212703e9, 0xd4db9d9ce64854c9, 0x499625479e161dac];
    let s659 = [0x95b64fca76d9d693, 0x9439936028864ac1, 0x0131108d97819edd, 0x42c177cf37b8193a];
    let pk659 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk659, &z659, &r659, &s659));

    // 656] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #92: special case hash
    let z660 = [0x8df56f36600e0f8b, 0xba20352117750229, 0xabad03e2fc662dc3, 0x958415d8ffffffff];
    let r660 = [0x50fb1aaa6ff6c9b2, 0x31e3bfe694f6b89c, 0x66a2c8065b541b3d, 0x08f16b8093a8fb4d];
    let s660 = [0x535ba3e5af81ca2e, 0x21f967410399b39b, 0x48573b611cb95d4a, 0x9d6455e2d5d17797];
    let pk660 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk660, &z660, &r660, &s660));

    // 657] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #93: special case hash
    let z661 = [0x954521b6975420f8, 0xe13deb04e1fbe8fb, 0xff1281093536f47f, 0xf1d8de4858ffffff];
    let r661 = [0xeed8dc2b338cb5f8, 0xc579b6938d19bce8, 0x19dd72ddb99ed8f8, 0xbe26231b6191658a];
    let s661 = [0xb9c5e96952575c89, 0xc943c14f79694a03, 0x37f0f22b2dcb57d5, 0xe1d9a32ee56cffed];
    let pk661 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk661, &z661, &r661, &s661));

    // 658] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #94: special case hash
    let z662 = [0x876b95c81fc31def, 0x32dc5d47c05ef6f1, 0xffff10782dd14a3b, 0x0927895f2802ffff];
    let r662 = [0x12638c455abe0443, 0x45f36a229d4aa4f8, 0x6204ac920a02d580, 0x15e76880898316b1];
    let s662 = [0x38196506a1939123, 0x55ca10e226e13f96, 0x5337bd6aba4178b4, 0xe74d357d3fcb5c8c];
    let pk662 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk662, &z662, &r662, &s662));

    // 659] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #95: special case hash
    let z663 = [0x24cf6a0c3ac80589, 0x0a57c3063fb5a306, 0xffffff4f332862a1, 0x60907984aa7e8eff];
    let r663 = [0x132315cc07f16dad, 0x31e6307d3ddbffc1, 0x3a45f9846fc28d1d, 0x352ecb53f8df2c50];
    let s663 = [0x899792887dd0a3c6, 0x436726ecd28258b1, 0xe1d05c5242ca1c39, 0x1348dfa9c482c558];
    let pk663 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk663, &z663, &r663, &s663));

    // 660] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #96: special case hash
    let z664 = [0x42d6b9b8cd6ae1e2, 0x50f9a5f50636ea69, 0xffffffff0af42cda, 0xc6ff198484939170];
    let r664 = [0x2c5bfa5f2a9558fb, 0x77b8642349ed3d65, 0x8a0da9882ab23c76, 0x4a40801a7e606ba7];
    let s664 = [0xea77dc5981725782, 0xdc24ed2925825bf8, 0x7f605f2832f7384b, 0x3a49b64848d682ef];
    let pk664 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk664, &z664, &r664, &s664));

    // 661] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #97: special case hash
    let z665 = [0x16dfbe4d27d7e68d, 0x9b9e0956cc43135d, 0x75ffffffff807479, 0xde030419345ca15c];
    let r665 = [0xe5e9e44df3d61e96, 0xb3511bac855c05c9, 0x2be412b078924b3b, 0xeacc5e1a8304a74d];
    let s665 = [0x08db8f714204f6d1, 0xec4bb0ed4c36ce98, 0x85dd827714847f96, 0x7451cd8e18d6ed18];
    let pk665 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk665, &z665, &r665, &s665));

    // 662] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #98: special case hash
    let z666 = [0x7e1ab78caaaac6ff, 0x665604d34acb1903, 0x2b88fffffffff6c8, 0x6f0e3eeaf42b2813];
    let r666 = [0x5f7de94c31577052, 0x4f8cd1214882adb6, 0xf30f67fdab61e8ce, 0x2f7a5e9e5771d424];
    let s666 = [0xb9528f8f78daa10c, 0xfb75dd050c5a449a, 0x44acb0b2bd889175, 0xac4e69808345809b];
    let pk666 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk666, &z666, &r666, &s666));

    // 663] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #99: special case hash
    let z667 = [0x2cb222d1f8017ab9, 0x48f7c0591ddcae7d, 0x3708d1ffffffffbe, 0xcdb549f773b3e62b];
    let r667 = [0x0a03d710b3300219, 0x7dddd7f6487621c3, 0x3e7e0f0e95e1a214, 0xffcda40f792ce4d9];
    let s667 = [0xd58c422c2453a49a, 0xfa77618f0b67add8, 0xd7ba9ade8f2065a1, 0x79938b55f8a17f7e];
    let pk667 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk667, &z667, &r667, &s667));

    // 664] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #100: special case hash
    let z668 = [0x24d8fd6f0edb0484, 0x9fd64886c1dc4f99, 0x1df4989bffffffff, 0x2c3f26f96a3ac005];
    let r668 = [0x8c17603a431e39a8, 0x48350f7ab3a588b2, 0x3d3e8c8c3fcc16a9, 0x81f2359c4faba6b5];
    let s668 = [0x7f9e101857f74300, 0x09e46d99fccefb9f, 0x0ff695d06c6860b5, 0xcd6f6a5cc3b55ead];
    let pk668 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk668, &z668, &r668, &s668));

    // 665] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #101: special case hash
    let z669 = [0x8476397c04edf411, 0xff5c31d89fda6a6b, 0x2cb7d53f9affffff, 0xac18f8418c55a250];
    let r669 = [0xc3f5f2aaf75ca808, 0xea130251a6fdffa5, 0xee1596fb073ea283, 0xdfc8bf520445cbb8];
    let s669 = [0xa7ac711e577e90e7, 0xbfd7d0dc7a4905b3, 0xd92823640e338e68, 0x048e33efce147c9d];
    let pk669 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk669, &z669, &r669, &s669));

    // 666] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #102: special case hash
    let z670 = [0x3e5a6ab8cf0ee610, 0xffffa2fd3e289368, 0xb24094f72bb5ffff, 0x4f9618f98e2d3a15];
    let r670 = [0x88227688ba6a5762, 0x6503a0e393e932f6, 0xefda70b46c53db16, 0xad019f74c6941d20];
    let s670 = [0xbc05efe16c199345, 0x7964ef2e0988e712, 0x5346bdbb3102cdcf, 0x93320eb7ca071025];
    let pk670 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk670, &z670, &r670, &s670));

    // 667] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #103: special case hash
    let z671 = [0x04caae73ab0bc75a, 0xffffff67edf7c402, 0x9cc21d31d37a25ff, 0x422e82a3d56ed10a];
    let r671 = [0xdeb7bd5a3ebc1883, 0xb54316bd3ebf7fff, 0xc34e78ce11dd71e4, 0xac8096842e8add68];
    let s671 = [0x9f21a3aac003b7a8, 0x36e3ce9f0ce21970, 0x2d4caf85d187215d, 0xf5ca2f4f23d67450];
    let pk671 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk671, &z671, &r671, &s671));

    // 668] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #104: special case hash
    let z672 = [0x2d9890b5cf95d018, 0x17a5ffffffffa084, 0x6e7b329ff738fbb4, 0x7075d245ccc3281b];
    let r672 = [0x54b4943693fb92f7, 0x89ddcd7b7b9d7768, 0xf939b70ea0022508, 0x677b2d3a59b18a5f];
    let s672 = [0xab6972cc0795db55, 0x5d2f63aee81efd0b, 0xf30307b21f3ccda3, 0x6b4ba856ade7677b];
    let pk672 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk672, &z672, &r672, &s672));

    // 669] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #105: special case hash
    let z673 = [0xc1847eb76c217a95, 0x7e280ebeffffffff, 0x9443d593fa4fd659, 0x3c80de54cd922698];
    let r673 = [0x05e1fc0d5957cfb0, 0xd84d31d4b7c30e1f, 0x379ba8e1b73d3115, 0x479e1ded14bcaed0];
    let s673 = [0x1e877027355b2443, 0x30857ca879f97c77, 0x7cf634a4f05b2e0c, 0x918f79e35b3d8948];
    let pk673 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk673, &z673, &r673, &s673));

    // 670] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #106: special case hash
    let z674 = [0xffc7906aa794b39b, 0x0ce891a8cdffffff, 0x980bef3d697ea277, 0xde21754e29b85601];
    let r674 = [0xb64840ead512a0a3, 0xd711e14b12ac5cf3, 0xd9a58f01164d55c3, 0x43dfccd0edb9e280];
    let s674 = [0x3199f49584389772, 0xca1174899b78ef9a, 0xcd5c4934365b3442, 0x1dbe33fa8ba84533];
    let pk674 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk674, &z674, &r674, &s674));

    // 671] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #107: special case hash
    let z675 = [0xffff2f1f2f57881c, 0x599e4d5f7289ffff, 0x84dd59623fb531bb, 0x8f65d92927cfb86a];
    let r675 = [0x38bb4085f0bbff11, 0xa20e9087c259d26a, 0xf4c7c7e4bca592fe, 0x5b09ab637bd4caf0];
    let s675 = [0xca8101de08eb0d75, 0xa24964e5a13f885b, 0x618e9d80d6fdcd6a, 0x45b7eb467b6748af];
    let pk675 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk675, &z675, &r675, &s675));

    // 672] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #108: special case hash
    let z676 = [0xfffffffafc8c3ca8, 0x2cc7cd0e8426cbff, 0x160bea3877dace8a, 0x6b63e9a74e092120];
    let r676 = [0x14a5039ed15ee06f, 0x667afa570a6cfa01, 0x5728c5c8af9b74e0, 0x5e9b1c5a028070df];
    let s676 = [0x44edaeb9ad990c20, 0x6c29eeffd3c50377, 0xad362bb8d7bd661b, 0xb1360907e2d9785e];
    let pk676 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk676, &z676, &r676, &s676));

    // 673] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #109: special case hash
    let z677 = [0xffffffffe852512e, 0xd094586e249c8699, 0xb6d75219444e8b43, 0xfc28259702a03845];
    let r677 = [0xd1a7a5fb8578f32e, 0x4890050f5a5712f6, 0x4a2fb0990e34538b, 0x0671a0a85c2b72d5];
    let s677 = [0xc720e5854713694c, 0x1808f27fd5bd4fda, 0x79ab9c3285ca4129, 0xdb1846bab6b73614];
    let pk677 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk677, &z677, &r677, &s677));

    // 674] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #110: special case hash
    let z678 = [0x1757ffffffffe20a, 0x74ecbcd52e8ceb57, 0xcee044ee8e8db7f7, 0x1273b4502ea4e3bc];
    let r678 = [0xbaedb35b2095103a, 0xc5d7d69859d301ab, 0x77dbbb0590a45492, 0x7673f85267484464];
    let s678 = [0x3807ef4422913d7c, 0x4dec0d417a414fed, 0x886bed9e6af02e0e, 0x3dc70ddf9c6b524d];
    let pk678 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk678, &z678, &r678, &s678));

    // 675] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #111: special case hash
    let z679 = [0xfb49ffffffffff6e, 0x4f8c53a15b96e602, 0x0c566c66228d8181, 0x08fb565610a79baa];
    let r679 = [0x9dfd657a796d12b5, 0x450d1a06c36d3ff3, 0xb21285089ebb1aa6, 0x7f085441070ecd2b];
    let s679 = [0xa9e4c5c54a2b9a8b, 0x92a5e6cb4b2d8daf, 0x2459d18d47da9aa4, 0x249712012029870a];
    let pk679 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk679, &z679, &r679, &s679));

    // 676] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #112: special case hash
    let z680 = [0x28ecaefeffffffff, 0xa2403f748e97d7cd, 0x87715fcb1aa4e79a, 0xd59291cc2cf89f30];
    let r680 = [0xa8e0f30a5d287348, 0xb76df04bc5aa6683, 0xc867398ea7322d5a, 0x914c67fb61dd1e27];
    let s680 = [0xc96d28f6d37304ea, 0xea7e66ec412b38d6, 0x4953e3ac1959ee8c, 0xfa07474031481dda];
    let pk680 = [
        0x69c8c4df6c732838,
        0x2903269919f70860,
        0xdcfe467828128bad,
        0x2927b10512bae3ed,
        0x8d1a974e7341513e,
        0x6766b3d968500155,
        0x921fb1498a60f460,
        0xc7787964eaac00e5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk680, &z680, &r680, &s680));

    // 677] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #113: k*G has a large x-coordinate
    let z681 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r681 = [0x0c46353d039cdaab, 0x4319055358e8617b, 0x0000000000000000, 0x0000000000000000];
    let s681 = [0xf3b9cac2fc63254e, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk681 = [
        0x82415c8361baaca4,
        0xebf7d10fa5151531,
        0x9b1a6957d29ce22f,
        0xd705d16f80987e2d,
        0x60819e8682160926,
        0xa6f83625593620d4,
        0x14ec1238beae2037,
        0xb1fc105ee5ce80d5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk681, &z681, &r681, &s681));

    // 678] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #114: r too large
    let z682 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r682 = [0xfffffffffffffffc, 0x00000000ffffffff, 0x0000000000000000, 0xffffffff00000001];
    let s682 = [0xf3b9cac2fc63254e, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk682 = [
        0x82415c8361baaca4,
        0xebf7d10fa5151531,
        0x9b1a6957d29ce22f,
        0xd705d16f80987e2d,
        0x60819e8682160926,
        0xa6f83625593620d4,
        0x14ec1238beae2037,
        0xb1fc105ee5ce80d5,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk682, &z682, &r682, &s682));

    // 679] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #115: r,s are large
    let z683 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r683 = [0xf3b9cac2fc63254f, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s683 = [0xf3b9cac2fc63254e, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk683 = [
        0xafa52c8501387d59,
        0xcd2ef67056893ead,
        0x844c09d7b560d527,
        0x3cd8d2f81d6953b0,
        0x8903485c0bb6dc2d,
        0xa490b62a6b771906,
        0x7a0c5e3b747adfa3,
        0xee41fdb4d10402ce,
    ];
    assert!(ecdsa_verify_secp256r1(&pk683, &z683, &r683, &s683));

    // 680] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #116: r and s^-1 have a large Hamming weight
    let z684 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r684 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s684 = [0xde54a36383df8dd4, 0x1453fe50914f3df2, 0x170f5ead2de4f651, 0x909135bdb6799286];
    let pk684 = [
        0x189b481196851378,
        0x0e81f332c4545d41,
        0x936133508c391510,
        0x8240cd81edd91cb6,
        0x837dc432f9ce89d9,
        0xea6dd6d9c0ae27b7,
        0x80ea5db514aa2f93,
        0xe05b06e72d4a1bff,
    ];
    assert!(ecdsa_verify_secp256r1(&pk684, &z684, &r684, &s684));

    // 681] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #117: r and s^-1 have a large Hamming weight
    let z685 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r685 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s685 = [0x360644669ca249a5, 0xf5deb773ad5f5a84, 0x71303fd5dd227dce, 0x27b4577ca009376f];
    let pk685 = [
        0x616d91eaad13df2c,
        0xa6e1bfe6779756fa,
        0xc17f1704c65aa1dc,
        0xb062947356748b0f,
        0x431113f1b2fb579d,
        0x12b84a4f8432293b,
        0x409cfc5992a99fff,
        0x0b38c17f3d0672e7,
    ];
    assert!(ecdsa_verify_secp256r1(&pk685, &z685, &r685, &s685));

    // 682] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #118: small r and s
    let z686 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r686 = [0x0000000000000005, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s686 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk686 = [
        0x2813b3a19a1eb5e5,
        0x80fa0dc43171d771,
        0xafa601072489a563,
        0x4a03ef9f92eb268c,
        0xa8ba90622df6f2f0,
        0x018a79b3e0263d91,
        0x2f4a17fd830c6654,
        0x3e213e28a608ce9a,
    ];
    assert!(ecdsa_verify_secp256r1(&pk686, &z686, &r686, &s686));

    // 683] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #120: small r and s
    let z687 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r687 = [0x0000000000000005, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s687 = [0x0000000000000003, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk687 = [
        0x0b601ea1f859e701,
        0x41cef26177ada885,
        0xe286b4833701606a,
        0x091194c1cba17f34,
        0x5db9ad77767f55eb,
        0xa7984e6209f4d6b9,
        0x58403ce2fe501983,
        0x27242fcec7088287,
    ];
    assert!(ecdsa_verify_secp256r1(&pk687, &z687, &r687, &s687));

    // 684] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #122: small r and s
    let z688 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r688 = [0x0000000000000005, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s688 = [0x0000000000000005, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk688 = [
        0x95812c96dcfb41a7,
        0x48e81c2bdbdd39c1,
        0xea8f56fee3a4b2b1,
        0x103c6ecceff59e71,
        0xcb16c448d8e57bf5,
        0xb4ebce8b09042c2e,
        0x50b883d770ec51eb,
        0x2303a193dc591be1,
    ];
    assert!(ecdsa_verify_secp256r1(&pk688, &z688, &r688, &s688));

    // 685] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #124: small r and s
    let z689 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r689 = [0x0000000000000005, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s689 = [0x0000000000000006, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk689 = [
        0x51468927e87fb6ea,
        0x67390c20111bd2b4,
        0xbcb2bfe8c22228be,
        0x3b66b829fe604638,
        0x6712fa9e9c4ac212,
        0xde485a3ed09dade7,
        0xb274ba2cad36b58f,
        0xbc8e59c009361758,
    ];
    assert!(ecdsa_verify_secp256r1(&pk689, &z689, &r689, &s689));

    // 686] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #126: r is larger than n
    let z690 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r690 = [0xf3b9cac2fc632556, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s690 = [0x0000000000000006, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk690 = [
        0x51468927e87fb6ea,
        0x67390c20111bd2b4,
        0xbcb2bfe8c22228be,
        0x3b66b829fe604638,
        0x6712fa9e9c4ac212,
        0xde485a3ed09dade7,
        0xb274ba2cad36b58f,
        0xbc8e59c009361758,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk690, &z690, &r690, &s690));

    // 687] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #127: s is larger than n
    let z691 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r691 = [0x0000000000000005, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s691 = [0xf3b9cac2fc75fbd8, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let pk691 = [
        0x94582092cbc50c30,
        0x3961b874b8c8e0eb,
        0x71c09fdcbc74a623,
        0x4ff2f6c24e4a33cd,
        0x9ae82d61ead26420,
        0xa120486b534139d5,
        0x335f3f937d4c79af,
        0x84fa9547afda5c66,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk691, &z691, &r691, &s691));

    // 688] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #128: small r and s^-1
    let z692 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r692 = [0x0000000000000100, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s692 = [0x6e0d28c9bb75ea88, 0x516af4f63f2d74d7, 0xbb76eddbb76eddbb, 0x8f1e3c7862c58b16];
    let pk692 = [
        0x9b5bc657ac588175,
        0x60cdaa8ee0058788,
        0xcd53c2fb973cf14d,
        0x84b959080bb30859,
        0x0b0c45af2c3cd7ca,
        0x60e5ea7850b0f665,
        0x113c78b4cb8dc7d3,
        0xa02ce5c1e53cb196,
    ];
    assert!(ecdsa_verify_secp256r1(&pk692, &z692, &r692, &s692));

    // 689] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #129: smallish r and s^-1
    let z693 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r693 = [0x002d9b4d347952d6, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s693 = [0xbadd195a0ffe6d7a, 0x05ee1c87ff907bee, 0xb3974497710ab115, 0xef3043e7329581db];
    let pk693 = [
        0xe1ff32a563354e99,
        0xf74a07ebb91e0570,
        0x77ae578e5d835fa7,
        0xdf4083bd6ecbda5a,
        0x18edaf9071e311f8,
        0xd4bc4f2deec57238,
        0xf647df28e2d9acd0,
        0x25af80b09a167d9e,
    ];
    assert!(ecdsa_verify_secp256r1(&pk693, &z693, &r693, &s693));

    // 690] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #130: 100-bit r and small s^-1
    let z694 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r694 = [0xb32b445580bf4eff, 0x0000001033e67e37, 0x0000000000000000, 0x0000000000000000];
    let s694 = [0xd87129b8e91d1b4d, 0x66e769ad4a16d3dc, 0x8b748b748b748b74, 0x8b748b7400000000];
    let pk694 = [
        0x14a34a7c956a0377,
        0xc8679d278f3736b4,
        0x8ca821f7ba6f000c,
        0xc2569a3c9bf8c183,
        0xd441a4e9683aeb09,
        0x434c975806795ab7,
        0x4b4a91c9b7d65bc6,
        0x0387ea85bc4f2880,
    ];
    assert!(ecdsa_verify_secp256r1(&pk694, &z694, &r694, &s694));

    // 691] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #131: small r and 100 bit s^-1
    let z695 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r695 = [0x0000000000000100, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s695 = [0xec3270fc4b81ef5b, 0xbe3cf9cb824a879f, 0x3178fa20b4aaad83, 0xef9f6ba4d97c09d0];
    let pk695 = [
        0x4a6496d80670968a,
        0xc586357c978256f4,
        0x6540c271774a6bf1,
        0x4a9f7da2a6c359a1,
        0xb03924755beb40d4,
        0x04c86f2c508eb777,
        0x56fbd7bb9e4e3ae3,
        0xc496e73a44563f8d,
    ];
    assert!(ecdsa_verify_secp256r1(&pk695, &z695, &r695, &s695));

    // 692] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #132: 100-bit r and s^-1
    let z696 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r696 = [0xecbe7c39e93e7c25, 0x000000062522bbd3, 0x0000000000000000, 0x0000000000000000];
    let s696 = [0xec3270fc4b81ef5b, 0xbe3cf9cb824a879f, 0x3178fa20b4aaad83, 0xef9f6ba4d97c09d0];
    let pk696 = [
        0x8ff23a8e95ca106b,
        0x6067d466dde4917a,
        0xe26204c0a3413699,
        0x874146432b3cd2c9,
        0x3ac4866703a6608c,
        0x10bdc6edd465e6f4,
        0x85a813bc35f3a207,
        0x709b3d50976ef8b3,
    ];
    assert!(ecdsa_verify_secp256r1(&pk696, &z696, &r696, &s696));

    // 693] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #133: r and s^-1 are close to n
    let z697 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r697 = [0xf3b9cac2fc6324d5, 0xbce6faada7179e84, 0xffffffffffffffff, 0xffffffff00000000];
    let s697 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let pk697 = [
        0xfcf3fd88f8e07ede,
        0x3b499a96afa7aaa3,
        0x2bbe25a34ea4e363,
        0x7a736d8e326a9ca6,
        0xfcc48a5934864627,
        0xeda7bf9ae46aa3ea,
        0xe818443a686e869e,
        0xb3e45879d8622b93,
    ];
    assert!(ecdsa_verify_secp256r1(&pk697, &z697, &r697, &s697));

    // 694] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #134: s == 1
    let z698 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r698 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let s698 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk698 = [
        0xc69748a03f0d5988,
        0xec1ecb41e55172e9,
        0x382630f99725e423,
        0xe84d9b232e971a43,
        0xe31753793c7588d4,
        0xf2ee923714e7d1df,
        0x3bd041ff75fac98e,
        0x618b15b427ad8336,
    ];
    assert!(ecdsa_verify_secp256r1(&pk698, &z698, &r698, &s698));

    // 695] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #135: s == 0
    let z699 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r699 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let s699 = [0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let pk699 = [
        0xc69748a03f0d5988,
        0xec1ecb41e55172e9,
        0x382630f99725e423,
        0xe84d9b232e971a43,
        0xe31753793c7588d4,
        0xf2ee923714e7d1df,
        0x3bd041ff75fac98e,
        0x618b15b427ad8336,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk699, &z699, &r699, &s699));

    // 696] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #136: point at infinity during verify
    let z700 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r700 = [0x79dce5617e3192a8, 0xde737d56d38bcf42, 0x7fffffffffffffff, 0x7fffffff80000000];
    let s700 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let pk700 = [
        0xae67c467de045034,
        0x15259240aa78d08a,
        0xd8d7a0c80f66dddd,
        0x0203736fcb198b15,
        0xb072f8f20e87a996,
        0x471b160c6bcf2568,
        0xa387ee8e4d4e84b4,
        0x34383438d5041ea9,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk700, &z700, &r700, &s700));

    // 697] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #137: edge case for signature malleability
    let z701 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r701 = [0x79dce5617e3192a9, 0xde737d56d38bcf42, 0x7fffffffffffffff, 0x7fffffff80000000];
    let s701 = [0x79dce5617e3192a8, 0xde737d56d38bcf42, 0x7fffffffffffffff, 0x7fffffff80000000];
    let pk701 = [
        0x4e37dcd2bfea02e1,
        0x99fe2e70a1848238,
        0x1f2a39730da5d8cd,
        0x78d844dc7f16b73b,
        0x3a8183c26e75d336,
        0xd3b9a6a6dea99aa4,
        0x13d02c666c45ef22,
        0xed6572e01eb7a8d1,
    ];
    assert!(ecdsa_verify_secp256r1(&pk701, &z701, &r701, &s701));

    // 698] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #138: edge case for signature malleability
    let z702 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r702 = [0x79dce5617e3192a9, 0xde737d56d38bcf42, 0x7fffffffffffffff, 0x7fffffff80000000];
    let s702 = [0x79dce5617e3192a9, 0xde737d56d38bcf42, 0x7fffffffffffffff, 0x7fffffff80000000];
    let pk702 = [
        0x4d62da9599a74014,
        0xcc5beb81a958b02b,
        0x0eacc8c09d2e5789,
        0xdec6c8257dde9411,
        0xca4e344fdd690f1d,
        0x7b06dd6f4e9c56ba,
        0x970b83f652442106,
        0x66fae1614174be63,
    ];
    assert!(ecdsa_verify_secp256r1(&pk702, &z702, &r702, &s702));

    // 699] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #139: u1 == 1
    let z703 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r703 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let s703 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let pk703 = [
        0xc3d0cad0988cabc0,
        0x92db0c23f0c2ea24,
        0x23ca5cbf1f919512,
        0xa17f5b75a35ed646,
        0x8b3a3f6300424dc6,
        0xecbb2fc20fdde7c5,
        0x40730b4fa3ee64fa,
        0x83a7a618625c2289,
    ];
    assert!(ecdsa_verify_secp256r1(&pk703, &z703, &r703, &s703));

    // 700] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #140: u1 == n - 1
    let z704 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r704 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let s704 = [0x9eac5054ed2ec72c, 0x9c400e9c69af7beb, 0x4089464733ff7cd3, 0xacd155416a8b77f3];
    let pk704 = [
        0xe9eaa1ebcc9fb5c3,
        0x04ec8393a0200419,
        0x13f33bf90dab628c,
        0x04ba0cba291a37db,
        0x49ecf4265dc12f62,
        0x47970fc3428f0f00,
        0x625ad57b12a32d40,
        0x1f3a0a0e6823a49b,
    ];
    assert!(ecdsa_verify_secp256r1(&pk704, &z704, &r704, &s704));

    // 701] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #141: u2 == 1
    let z705 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r705 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let s705 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let pk705 = [
        0x9f204a434b8900ef,
        0xcbe8723a1ed39f22,
        0x3d8aeaa2b7322f9c,
        0x692b6c828e0feed6,
        0x3329069ae4dd5716,
        0x274af56a8c5628dc,
        0x8fde38b98c7c271f,
        0xa1f6f6abcb38ea3b,
    ];
    assert!(ecdsa_verify_secp256r1(&pk705, &z705, &r705, &s705));

    // 702] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #142: u2 == n - 1
    let z706 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r706 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let s706 = [0x4d26872ca84218e1, 0x7def51c91a0fbf03, 0xaaaaaaaaaaaaaaaa, 0xaaaaaaaa00000000];
    let pk706 = [
        0x0f84871874ccef09,
        0x5ebb5a3ef7632f80,
        0xcb93687a9cd8f975,
        0x00cefd9162d13e64,
        0x7dbbef2c54bc0cb1,
        0xb8480d2587404ebf,
        0xf721be2fb5f549e4,
        0x543ecbeaf7e8044e,
    ];
    assert!(ecdsa_verify_secp256r1(&pk706, &z706, &r706, &s706));

    // 703] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #143: edge case for u1
    let z707 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r707 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s707 = [0x6d97c1bb03dd2bd2, 0x49d9f794f6d5405f, 0x3fd23de844002bb9, 0x710f8e3edc7c2d5a];
    let pk707 = [
        0xde03efa3f0f24486,
        0x12f50c8c85a4beb9,
        0x2f291d5c1921fd5e,
        0xb975183b42551cf5,
        0x78c406b25ab43091,
        0xf21e242ce3fb15bc,
        0x2dc313612020311f,
        0x2243018e6866df92,
    ];
    assert!(ecdsa_verify_secp256r1(&pk707, &z707, &r707, &s707));

    // 704] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #144: edge case for u1
    let z708 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r708 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s708 = [0xa8e269274ffe4e1b, 0x1a58525c7b4db2e7, 0x3069a7e5f40335a6, 0xedffbc270f722c24];
    let pk708 = [
        0xd717149274466999,
        0x4d48b8d17191d74e,
        0xdf042a26f8abf609,
        0xc25f1d166f3e211c,
        0xcfb52a114e77ccdb,
        0x51969adf9604b5ac,
        0x9e8b4c5da6bb9228,
        0x65d06dd6a88abfa4,
    ];
    assert!(ecdsa_verify_secp256r1(&pk708, &z708, &r708, &s708));

    // 705] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #145: edge case for u1
    let z709 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r709 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s709 = [0x7904b68919ba4d53, 0x3314c3e178525d00, 0x4f95d2344e24ee52, 0xa25adcae105ed7ff];
    let pk709 = [
        0x008d7f0164cbc0ca,
        0xd6eee398a23c3a0b,
        0xa004236218a3c3a2,
        0x8fe5e88243a76e41,
        0x86b8cb387af7f240,
        0x82d40127c897697c,
        0x3c7cfd9b83c63e3a,
        0x98a20d1bdcf57351,
    ];
    assert!(ecdsa_verify_secp256r1(&pk709, &z709, &r709, &s709));

    // 706] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #146: edge case for u1
    let z710 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r710 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s710 = [0x0297e766b5805ebb, 0x346924b2f64bd3dd, 0x6760d773de3f3e87, 0x2e4348c645707dce];
    let pk710 = [
        0xeca2efb37e8dff2c,
        0x3ecee6d5a840a37b,
        0x70c7b341970b3824,
        0x02148256b530fbc4,
        0x53a83573473cb30d,
        0xa987eeb6ddb738af,
        0x489ca703a399864b,
        0xc0adbea0882482a7,
    ];
    assert!(ecdsa_verify_secp256r1(&pk710, &z710, &r710, &s710));

    // 707] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #147: edge case for u1
    let z711 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r711 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s711 = [0xf9684fb67753d1dc, 0x869e916dbcf797d8, 0x0d773de3f3e87408, 0x348c673b07dce392];
    let pk711 = [
        0xb3a621d021c76f8e,
        0xd698e19615124273,
        0x9c7375c5fcf3e54e,
        0xa34db012ce6eda1e,
        0xfdde13d1d6df7f14,
        0xbb4fbb7ddf08d8d8,
        0x221e39e1205d5510,
        0x777458d6f55a364c,
    ];
    assert!(ecdsa_verify_secp256r1(&pk711, &z711, &r711, &s711));

    // 708] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #148: edge case for u1
    let z712 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r712 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s712 = [0xf2d09f6ceea7a3b8, 0x0d3d22db79ef2fb1, 0x1aee7bc7e7d0e811, 0x6918ce760fb9c724];
    let pk712 = [
        0x7ae37b4e7778041d,
        0xdb6dd2a1b315b2ce,
        0x912b6271dd8a43ba,
        0xb97af3fe78be15f2,
        0x2d0fa4c479b278e7,
        0x154c305307d1dcd5,
        0x6495c42102d08e81,
        0x930d71ee1992d246,
    ];
    assert!(ecdsa_verify_secp256r1(&pk712, &z712, &r712, &s712));

    // 709] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #149: edge case for u1
    let z713 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r713 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s713 = [0xd864d648969f5b5a, 0x15ac20e4c126bbf6, 0xde3f3e8740894647, 0x73b3c694391d8ead];
    let pk713 = [
        0x59139af3135dbcbb,
        0xbf81108e6c35cd85,
        0x1cedc7a1d6eff6e9,
        0x81e7198a3c3f2390,
        0xb737525b5d580034,
        0xba990d4570a4e3b7,
        0x61b90c9f4285eefc,
        0x9ef1568530291a80,
    ];
    assert!(ecdsa_verify_secp256r1(&pk713, &z713, &r713, &s713));

    // 710] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #150: edge case for u1
    let z714 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r714 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s714 = [0x6877e53c41457f28, 0xb89ce11259519765, 0x2989a16db1930ef1, 0xbb07ac7a86948c2c];
    let pk714 = [
        0x3cc9960f188ddf73,
        0xab573e8becc6ddff,
        0xa39cb9de645149c2,
        0xab4d792ca121d1db,
        0x56386de68285a3c8,
        0x5858d7be1315a694,
        0x3262ff7335541519,
        0x7f90ba23664153e9,
    ];
    assert!(ecdsa_verify_secp256r1(&pk714, &z714, &r714, &s714));

    // 711] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #151: edge case for u1
    let z715 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r715 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s715 = [0x3f62c861187db648, 0xd198662d6f229944, 0x9337c69bf9332ed3, 0x27e4d82cb6c061dd];
    let pk715 = [
        0xf587c9c2652f88ef,
        0x51fbfa9e5be80563,
        0x084476a68d59bbde,
        0x518412b69af43aae,
        0xc1c70503fc10f233,
        0xfbc24afed8523ede,
        0x7b0c55e5240a3a98,
        0x2d3b90d25baa6bdb,
    ];
    assert!(ecdsa_verify_secp256r1(&pk715, &z715, &r715, &s715));

    // 712] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #152: edge case for u1
    let z716 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r716 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s716 = [0x5c3dd81ae94609a4, 0x2d13b356dfe9ec27, 0x3b77850515fff6a1, 0xe7c5cf3aac2e8892];
    let pk716 = [
        0xf5d370af34f8352d,
        0xd1f66fe6cd373aa7,
        0xdffea4761ebaf592,
        0xa08f14a644b9a935,
        0xd732a5741c7aaaf5,
        0xec7a396d0a7affca,
        0x900a914c2934ec2f,
        0xa54b5bc4025cf335,
    ];
    assert!(ecdsa_verify_secp256r1(&pk716, &z716, &r716, &s716));

    // 713] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #153: edge case for u1
    let z717 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r717 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s717 = [0x3cede9e57a748f68, 0x17f9fee32bacfe55, 0xe016e10bddffea23, 0xc77838df91c1e953];
    let pk717 = [
        0x39241061e33f8f8c,
        0x0e9f45715b900446,
        0x90739d38af4ae3a2,
        0xccf2296a6a89b62b,
        0xc7e058034412ae08,
        0xaf83e7ff1bb84438,
        0xc6e9a472b96d88f4,
        0xaace0046491eeaa1,
    ];
    assert!(ecdsa_verify_secp256r1(&pk717, &z717, &r717, &s717));

    // 714] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #154: edge case for u1
    let z718 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r718 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s718 = [0x86220907f885f97f, 0x730d0318b0425e25, 0xc02dc217bbffd446, 0x8ef071c02383d2a6];
    let pk718 = [
        0xe165f962c86e3927,
        0x6c02b23e04002276,
        0x2b1f34895e5819a0,
        0x94b0fc1525bcabf8,
        0xfb29fbc89a9c3376,
        0x2792225e16a6d2db,
        0x204fb32a1f829290,
        0xbe7c2ab4d0b25303,
    ];
    assert!(ecdsa_verify_secp256r1(&pk718, &z718, &r718, &s718));

    // 715] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #155: edge case for u1
    let z719 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r719 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s719 = [0xcf56282a76976396, 0xce20074e34d7bdf5, 0xa044a32399ffbe69, 0x5668aaa0b545bbf9];
    let pk719 = [
        0x0627cadfd16de6ec,
        0xccdcf2efca407edb,
        0x508527d89882d183,
        0x5351f37e1de0c88c,
        0x81b766a1a1300349,
        0x8425853b5b675eb7,
        0xebcc4c97847eed21,
        0x44b4b57cdf960d32,
    ];
    assert!(ecdsa_verify_secp256r1(&pk719, &z719, &r719, &s719));

    // 716] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #156: edge case for u1
    let z720 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r720 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s720 = [0xb65f40a60b0eb952, 0xf7fddf478fb4fdc2, 0x27cae91a27127728, 0xd12d6e56882f6c00];
    let pk720 = [
        0x5cb2fb276ac971a6,
        0xc2b5d147bdc83132,
        0xcb64019710a269c6,
        0x748bbafc320e6735,
        0x7005177578f51163,
        0xd93a7a49a8c5ccd3,
        0x00ad21ee3fd4d980,
        0x9d655e9a755bc9d8,
    ];
    assert!(ecdsa_verify_secp256r1(&pk720, &z720, &r720, &s720));

    // 717] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #157: edge case for u2
    let z721 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r721 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s721 = [0x513dee40fecbb71a, 0xe9a2538f37b28a2c, 0xffffffffffffffff, 0x7fffffffaaaaaaaa];
    let pk721 = [
        0x1c33038964fd85cc,
        0x12410b3b90fa97a3,
        0x36535a934d4ab851,
        0x14b3bbd75c5e1c0c,
        0x9705561dd6631883,
        0x18f2b50c5d00fb3f,
        0xb460d636c965a5f8,
        0x112f7d837f8f9c36,
    ];
    assert!(ecdsa_verify_secp256r1(&pk721, &z721, &r721, &s721));

    // 718] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #158: edge case for u2
    let z722 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r722 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s722 = [0xde009e526adf21f2, 0x3ab3cccd0459b201, 0x6de86d42ad8a13da, 0xb62f26b5f2a2b26f];
    let pk722 = [
        0x56935671ae9305bf,
        0x4a9bafa2f14a5903,
        0x6d6f950a8e08ade0,
        0xd823533c04cd8edc,
        0x59d7797303123775,
        0xb58312907b195acb,
        0x96924c265f0ddb75,
        0x43178d1f88b6a57a,
    ];
    assert!(ecdsa_verify_secp256r1(&pk722, &z722, &r722, &s722));

    // 719] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #159: edge case for u2
    let z723 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r723 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s723 = [0x0686aa7b4c90851e, 0xd57b38bb61403d70, 0xd02bbbe749bd351c, 0xbb1d9ac949dd748c];
    let pk723 = [
        0xd209b92e654bab69,
        0xec108c105575c2f3,
        0x030624c6328e8ce3,
        0xdb2b3408b3167d91,
        0xc3d6be82836fa258,
        0x800df7c996d5d7b7,
        0x2c6e612f0fd3189d,
        0xc34318139c50b080,
    ];
    assert!(ecdsa_verify_secp256r1(&pk723, &z723, &r723, &s723));

    // 720] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #160: edge case for u2
    let z724 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r724 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s724 = [0x818f725b4f60aaf2, 0xe52545dac11f816e, 0x1c732513ca0234ec, 0x66755a00638cdaec];
    let pk724 = [
        0x1dd7ab6063852742,
        0x78c24837dfae26bc,
        0x2216453b2ac1e9d1,
        0x09179ce7c5922539,
        0xdcc3b691f95a9255,
        0x45c2860a59f2be1d,
        0xb826b2db7a86d19d,
        0x5556b42e330289f3,
    ];
    assert!(ecdsa_verify_secp256r1(&pk724, &z724, &r724, &s724));

    // 721] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #161: edge case for u2
    let z725 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r725 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s725 = [0x8ca48e982beb3669, 0xe98ebe492fdf02e4, 0x32513ca0234ecfff, 0x55a00c9fcdaebb60];
    let pk725 = [
        0x6b1eccebd6568d7e,
        0xd0c2fb29d70ff19b,
        0x467b7e4b214ea4c2,
        0x01959fb8deda56e5,
        0x211c39cc3a413398,
        0x25167db5a14d098a,
        0x970bff01e1343f69,
        0xd9dbd77a918297fd,
    ];
    assert!(ecdsa_verify_secp256r1(&pk725, &z725, &r725, &s725));

    // 722] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #162: edge case for u2
    let z726 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r726 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s726 = [0x19491d3057d66cd2, 0xd31d7c925fbe05c9, 0x64a27940469d9fff, 0xab40193f9b5d76c0];
    let pk726 = [
        0x5938245dd6bcab3a,
        0x947e1c5dd7ccc61a,
        0xc852b4e8f8ba9d6d,
        0x567f1fdc387e5350,
        0xb1f3eb1011130a11,
        0x2857970e26662267,
        0x9535c22eaaf0b581,
        0x9960bebaf919514f,
    ];
    assert!(ecdsa_verify_secp256r1(&pk726, &z726, &r726, &s726));

    // 723] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #163: edge case for u2
    let z727 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r727 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s727 = [0xa26b4408d0dc8600, 0xcb0dadbbc7f549f8, 0xca0234ecffffffff, 0xca0234ebb5fdcb13];
    let pk727 = [
        0x60b36d46d3e4bec2,
        0x2f9dd6dd28552626,
        0xb2f51682fd5f5176,
        0x3499f974ff4ca6bb,
        0x4546630f0d5c5e81,
        0xc64d4fa46ddce85c,
        0x20119152f0122476,
        0xf498fae2487807e2,
    ];
    assert!(ecdsa_verify_secp256r1(&pk727, &z727, &r727, &s727));

    // 724] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #164: edge case for u2
    let z728 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r728 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s728 = [0x8711c77298815ad3, 0x19933a9e65b28559, 0x082b9310572620ae, 0xbfffffff3ea3677e];
    let pk728 = [
        0x51b0f27094473426,
        0xcf30d0f3ec4b9f03,
        0x29596257db13b26e,
        0x2c5c01662cf00c19,
        0x1d9fdafa484e4ac7,
        0x7a0154b57f7a69c5,
        0xee822ddd2fc74424,
        0xe986a086060d086e,
    ];
    assert!(ecdsa_verify_secp256r1(&pk728, &z728, &r728, &s728));

    // 725] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #165: edge case for u2
    let z729 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r729 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s729 = [0x8f055d86e5cc41f4, 0x5b37902e023fab7c, 0xe666666666666666, 0x266666663bbbbbbb];
    let pk729 = [
        0xd9f3cf010b160501,
        0x15774183be7ba5b2,
        0xdbae94c23be6f52c,
        0x91d4cba813a04d86,
        0xe4b1874b02fd544a,
        0x541bf4b952b0ad7b,
        0x9a9ac080d516025a,
        0x900b8adfea649101,
    ];
    assert!(ecdsa_verify_secp256r1(&pk729, &z729, &r729, &s729));

    // 726] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #166: edge case for u2
    let z730 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r730 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s730 = [0x08a443e258970b09, 0x146c573f4c6dfc8d, 0xa492492492492492, 0xbfffffff36db6db7];
    let pk730 = [
        0x0c614b948e8aa124,
        0x02af36960831d021,
        0x8330ecad41e1a3b3,
        0xef7fd0a3a3638663,
        0x34116e35a8c7d098,
        0xd8cab5ab59c730eb,
        0xd3c1be0fdeaf11fc,
        0xef0d6d800e4047d6,
    ];
    assert!(ecdsa_verify_secp256r1(&pk730, &z730, &r730, &s730));

    // 727] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #167: edge case for u2
    let z731 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r731 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s731 = [0xcb1ad3a27cfd49c4, 0xc815d0e60b3e596e, 0x7fffffffffffffff, 0xbfffffff2aaaaaab];
    let pk731 = [
        0x8cea92eafe93df2a,
        0x6c55cc3ca5dbeb86,
        0x8ca77035a607fea0,
        0xa521dab13cc9152d,
        0xa36500418a2f43de,
        0x6ce1111bdb9c2e0c,
        0x5e6a5ccaa2826a40,
        0x7bfb9b2853199663,
    ];
    assert!(ecdsa_verify_secp256r1(&pk731, &z731, &r731, &s731));

    // 728] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #168: edge case for u2
    let z732 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r732 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s732 = [0xa27bdc81fd976e37, 0xd344a71e6f651458, 0xffffffffffffffff, 0x7fffffff55555555];
    let pk732 = [
        0xc01d1237a81a1097,
        0xe7a2683a12f38b4f,
        0x565f2187fe11d4e8,
        0x474d58a4eec16e0d,
        0xfde3a517a6ded4cd,
        0x9df2b67920fb5945,
        0xbdb67ef77f6fd296,
        0x6e55f73bb7cdda46,
    ];
    assert!(ecdsa_verify_secp256r1(&pk732, &z732, &r732, &s732));

    // 729] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #169: edge case for u2
    let z733 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r733 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s733 = [0x79dce5617e3192aa, 0xde737d56d38bcf42, 0x7fffffffffffffff, 0x3fffffff80000000];
    let pk733 = [
        0xf47d223a5b23a621,
        0x879f7b57208cdabb,
        0xe5cb525c37da8fa0,
        0x692da5cd4309d9a6,
        0x7d2902e9125e6ab4,
        0x7fc5fc3e6a5ed339,
        0xa7389aaed61738b1,
        0x40e0daa78cfdd207,
    ];
    assert!(ecdsa_verify_secp256r1(&pk733, &z733, &r733, &s733));

    // 730] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #170: edge case for u2
    let z734 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r734 = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    let s734 = [0x0343553da648428f, 0x6abd9c5db0a01eb8, 0x6815ddf3a4de9a8e, 0x5d8ecd64a4eeba46];
    let pk734 = [
        0xe9b8805c570a0670,
        0xfcd4d1f1679274f4,
        0x8a90279f14a8082c,
        0x85689b3e0775c771,
        0x70d3f240ebe705b1,
        0x15b9b7ca661ec7ff,
        0x09afa3640f4a034e,
        0x167fcc5ca734552e,
    ];
    assert!(ecdsa_verify_secp256r1(&pk734, &z734, &r734, &s734));

    // 731] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #171: point duplication during verification
    let z735 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r735 = [0x05ea6c42c4934569, 0x8c4aacafdfb6bcbe, 0x8fe0555ac3bc9904, 0x6f2347cab7dd7685];
    let s735 = [0x3a41de562aa18ed8, 0x3f54ddf7383e4402, 0xc4fa1f4703c1e50d, 0xf21d907e3890916d];
    let pk735 = [
        0x455edaef42cf237e,
        0x3cb2ef63b2ba2c0d,
        0x97a90d4ca8887e02,
        0x0158137755b901f7,
        0xe17bd1ba5677edcd,
        0xa7c7b9fd2b41d6e0,
        0x92b8b61aafa7a4aa,
        0x2a964fc00d377a85,
    ];
    assert!(ecdsa_verify_secp256r1(&pk735, &z735, &r735, &s735));

    // 732] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #172: duplication bug
    let z736 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r736 = [0x05ea6c42c4934569, 0x8c4aacafdfb6bcbe, 0x8fe0555ac3bc9904, 0x6f2347cab7dd7685];
    let s736 = [0x3a41de562aa18ed8, 0x3f54ddf7383e4402, 0xc4fa1f4703c1e50d, 0xf21d907e3890916d];
    let pk736 = [
        0x455edaef42cf237e,
        0x3cb2ef63b2ba2c0d,
        0x97a90d4ca8887e02,
        0x0158137755b901f7,
        0x1e842e45a9881232,
        0x58384603d4be291f,
        0x6d4749e550585b55,
        0xd569b03ef2c8857b,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk736, &z736, &r736, &s736));

    // 733] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #173: point with x-coordinate 0
    let z737 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r737 = [0x0000000000000001, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];
    let s737 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let pk737 = [
        0xb6bb2347adabc69c,
        0xd4ab283b2aa50f13,
        0x8204be2abca9fb8a,
        0x38a084ffccc4ae2f,
        0x653dddf7389365e2,
        0x31986e958e1f5cf5,
        0xad271e88b899c129,
        0xa699799b77b1cc6d,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk737, &z737, &r737, &s737));

    // 734] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #175: comparison with point at infinity
    let z738 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r738 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let s738 = [0x63f1f55a327a3aa9, 0x25c7cbbc549e52e7, 0x3333333333333333, 0x3333333300000000];
    let pk738 = [
        0xadb6f2c10aac831e,
        0xb36b7cdc54e33b84,
        0x8bdb2e61201b4549,
        0x664ce273320d918d,
        0xc7c915c736cef1f4,
        0xe1cceed2dd862e2d,
        0x73ac3d76bfbc8c5e,
        0x49e68831f18bda29,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk738, &z738, &r738, &s738));

    // 735] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #176: extreme value for k and edgecase s
    let z739 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r739 = [0xa60b48fc47669978, 0xc08969e277f21b35, 0x8a52380304b51ac3, 0x7cf27b188d034f7e];
    let s739 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let pk739 = [
        0x1add395efff1e0fe,
        0xc27d7089faeb3ddd,
        0x301dbbad4d86247e,
        0x961691a5e960d07a,
        0x231bd260a9e78aeb,
        0x37d1f1519817f09a,
        0xdf990d2c5377790e,
        0x7254622cc371866c,
    ];
    assert!(ecdsa_verify_secp256r1(&pk739, &z739, &r739, &s739));

    // 736] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #177: extreme value for k and s^-1
    let z740 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r740 = [0xa60b48fc47669978, 0xc08969e277f21b35, 0x8a52380304b51ac3, 0x7cf27b188d034f7e];
    let s740 = [0x1bcdd9f8fd6b63cc, 0x625bd7a09bec4ca8, 0x4924924924924924, 0xb6db6db624924925];
    let pk740 = [
        0x250e71e2aca63e9c,
        0xf1074793274e2928,
        0xa868e3b0fb33e6b4,
        0x5d283e13ce8ca60d,
        0xe0f44505a84886ce,
        0xbfd6d0c8bb6591d3,
        0x4d9e506d418ed9a1,
        0x214dc74fa25371fb,
    ];
    assert!(ecdsa_verify_secp256r1(&pk740, &z740, &r740, &s740));

    // 737] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #178: extreme value for k and s^-1
    let z741 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r741 = [0xa60b48fc47669978, 0xc08969e277f21b35, 0x8a52380304b51ac3, 0x7cf27b188d034f7e];
    let s741 = [0x8fc7d568c9e8eaa7, 0x971f2ef152794b9d, 0xcccccccccccccccc, 0xcccccccc00000000];
    let pk741 = [
        0xa9d297f792eef6a3,
        0xf9f8216551d9315a,
        0x3bd1d86514ae0462,
        0x0fc351da038ae080,
        0x5fc339d634019c73,
        0x753f00d6077a1e9e,
        0xda35360ca7aa925e,
        0x41c74eed786f2d33,
    ];
    assert!(ecdsa_verify_secp256r1(&pk741, &z741, &r741, &s741));

    // 738] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #179: extreme value for k and s^-1
    let z742 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r742 = [0xa60b48fc47669978, 0xc08969e277f21b35, 0x8a52380304b51ac3, 0x7cf27b188d034f7e];
    let s742 = [0x63f1f55a327a3aaa, 0x25c7cbbc549e52e7, 0x3333333333333333, 0x3333333300000000];
    let pk742 = [
        0x322bba9430ce4b60,
        0xfd4de7550065f638,
        0x3fee55c080547c2b,
        0xa1e34c8f16d13867,
        0x20b7d8a6b81ac936,
        0xc5d44a7bdf424366,
        0x4d7df8ab3f3b4181,
        0x662be9bb512663aa,
    ];
    assert!(ecdsa_verify_secp256r1(&pk742, &z742, &r742, &s742));

    // 739] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #180: extreme value for k and s^-1
    let z743 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r743 = [0xa60b48fc47669978, 0xc08969e277f21b35, 0x8a52380304b51ac3, 0x7cf27b188d034f7e];
    let s743 = [0xd7ebf0c9fef7c185, 0x5a8b230d0b2b51dc, 0xb6db6db6db6db6db, 0x49249248db6db6db];
    let pk743 = [
        0x7cda6e52817c1bdf,
        0xa87a23c7186150ed,
        0xf41d322a302d2078,
        0x7e1a8a8338d7fd8c,
        0x8d59ee34c615377f,
        0x254d748272b2d4eb,
        0x21e29014b2898349,
        0xd0a9135a89d21ce8,
    ];
    assert!(ecdsa_verify_secp256r1(&pk743, &z743, &r743, &s743));

    // 740] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #181: extreme value for k
    let z744 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r744 = [0xa60b48fc47669978, 0xc08969e277f21b35, 0x8a52380304b51ac3, 0x7cf27b188d034f7e];
    let s744 = [0x6e0478c34416e3bb, 0x1584d13e18411e2f, 0xc82cbc9d1edd8c98, 0x16a4502e2781e11a];
    let pk744 = [
        0xf76a686f64be078b,
        0x1b2c6f663ea33583,
        0x5c61ee7a018cc957,
        0x5c19fe227a61abc6,
        0x06c0541d17b24ddb,
        0xcf78492490a5cc56,
        0xd52bc48673b457c2,
        0x7b4a0d734940f613,
    ];
    assert!(ecdsa_verify_secp256r1(&pk744, &z744, &r744, &s744));

    // 741] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #182: extreme value for k and edgecase s
    let z745 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r745 = [0xf4a13945d898c296, 0x77037d812deb33a0, 0xf8bce6e563a440f2, 0x6b17d1f2e12c4247];
    let s745 = [0xa693439654210c70, 0x3ef7a8e48d07df81, 0x5555555555555555, 0x5555555500000000];
    let pk745 = [
        0x5db63abeb2739666,
        0x208eed08c2d4189a,
        0x9d9ef9e47419dba3,
        0xdb02d1f3421d600e,
        0x1715e6b24125512a,
        0xd210d5fd8ec628e3,
        0xd7ffe480827f90a0,
        0xe0ed26967b9ada9e,
    ];
    assert!(ecdsa_verify_secp256r1(&pk745, &z745, &r745, &s745));

    // 742] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #183: extreme value for k and s^-1
    let z746 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r746 = [0xf4a13945d898c296, 0x77037d812deb33a0, 0xf8bce6e563a440f2, 0x6b17d1f2e12c4247];
    let s746 = [0x1bcdd9f8fd6b63cc, 0x625bd7a09bec4ca8, 0x4924924924924924, 0xb6db6db624924925];
    let pk746 = [
        0xcfab338b88229c4b,
        0x5711bd3ed5a0ef72,
        0x93c29e441395b6c0,
        0x6222d19626555018,
        0xf1c54311d8e2fd23,
        0xac2626423b0bf81a,
        0x70362aaa520ee24c,
        0xaaae079cb44a1af0,
    ];
    assert!(ecdsa_verify_secp256r1(&pk746, &z746, &r746, &s746));

    // 743] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #184: extreme value for k and s^-1
    let z747 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r747 = [0xf4a13945d898c296, 0x77037d812deb33a0, 0xf8bce6e563a440f2, 0x6b17d1f2e12c4247];
    let s747 = [0x8fc7d568c9e8eaa7, 0x971f2ef152794b9d, 0xcccccccccccccccc, 0xcccccccc00000000];
    let pk747 = [
        0x361da184b04cdca5,
        0x9c0952ba599f4c03,
        0xfa81bc99c70bb041,
        0x4ccfa24c67f3def7,
        0xf6ca7a0a82153bfa,
        0x9728df870800be8c,
        0x729a2219478a7e62,
        0xdb76b797f7f41d9c,
    ];
    assert!(ecdsa_verify_secp256r1(&pk747, &z747, &r747, &s747));

    // 744] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #185: extreme value for k and s^-1
    let z748 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r748 = [0xf4a13945d898c296, 0x77037d812deb33a0, 0xf8bce6e563a440f2, 0x6b17d1f2e12c4247];
    let s748 = [0x63f1f55a327a3aaa, 0x25c7cbbc549e52e7, 0x3333333333333333, 0x3333333300000000];
    let pk748 = [
        0x61e99fefff9d84da,
        0xf3dbde7a99dc5740,
        0xac71402b6e9ecc4a,
        0xea1c72c91034036b,
        0x2bc2ea918c18cb63,
        0x29d5d055408c90d0,
        0xf56e34eb048f0a9d,
        0xb7dd057e75b78ac6,
    ];
    assert!(ecdsa_verify_secp256r1(&pk748, &z748, &r748, &s748));

    // 745] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #186: extreme value for k and s^-1
    let z749 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r749 = [0xf4a13945d898c296, 0x77037d812deb33a0, 0xf8bce6e563a440f2, 0x6b17d1f2e12c4247];
    let s749 = [0xd7ebf0c9fef7c185, 0x5a8b230d0b2b51dc, 0xb6db6db6db6db6db, 0x49249248db6db6db];
    let pk749 = [
        0x0d936988e90e79bc,
        0x38924f7817d1cd35,
        0x820b7795da2da62b,
        0xc2879a66d86cb20b,
        0x3aaaa11fa3b6a083,
        0xcb0177216db6fd1f,
        0x7a759de024eff90b,
        0x5431a7268ff6931c,
    ];
    assert!(ecdsa_verify_secp256r1(&pk749, &z749, &r749, &s749));

    // 746] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #187: extreme value for k
    let z750 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r750 = [0xf4a13945d898c296, 0x77037d812deb33a0, 0xf8bce6e563a440f2, 0x6b17d1f2e12c4247];
    let s750 = [0x6e0478c34416e3bb, 0x1584d13e18411e2f, 0xc82cbc9d1edd8c98, 0x16a4502e2781e11a];
    let pk750 = [
        0x58f455079aee0ba3,
        0x54c26df27711b065,
        0xb848c75006f2ef3c,
        0xab1c0f273f74abc2,
        0xe4d6c37fa48b47f2,
        0x6c179f0a13af1771,
        0x5997c776f14ad645,
        0xdf510f2ecef6d9a0,
    ];
    assert!(ecdsa_verify_secp256r1(&pk750, &z750, &r750, &s750));

    // 747] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #188: testing point duplication
    let z751 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r751 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let s751 = [0x6bf5f864ff7be0c2, 0xad4591868595a8ee, 0xdb6db6db6db6db6d, 0x249249246db6db6d];
    let pk751 = [
        0xf4a13945d898c296,
        0x77037d812deb33a0,
        0xf8bce6e563a440f2,
        0x6b17d1f2e12c4247,
        0xcbb6406837bf51f5,
        0x2bce33576b315ece,
        0x8ee7eb4a7c0f9e16,
        0x4fe342e2fe1a7f9b,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk751, &z751, &r751, &s751));

    // 748] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #189: testing point duplication
    let z752 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r752 = [0x9eac5054ed2ec72c, 0x9c400e9c69af7beb, 0x4089464733ff7cd3, 0xacd155416a8b77f3];
    let s752 = [0x6bf5f864ff7be0c2, 0xad4591868595a8ee, 0xdb6db6db6db6db6d, 0x249249246db6db6d];
    let pk752 = [
        0xf4a13945d898c296,
        0x77037d812deb33a0,
        0xf8bce6e563a440f2,
        0x6b17d1f2e12c4247,
        0xcbb6406837bf51f5,
        0x2bce33576b315ece,
        0x8ee7eb4a7c0f9e16,
        0x4fe342e2fe1a7f9b,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk752, &z752, &r752, &s752));

    // 749] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #190: testing point duplication
    let z753 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r753 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let s753 = [0x6bf5f864ff7be0c2, 0xad4591868595a8ee, 0xdb6db6db6db6db6d, 0x249249246db6db6d];
    let pk753 = [
        0xf4a13945d898c296,
        0x77037d812deb33a0,
        0xf8bce6e563a440f2,
        0x6b17d1f2e12c4247,
        0x3449bf97c840ae0a,
        0xd431cca994cea131,
        0x711814b583f061e9,
        0xb01cbd1c01e58065,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk753, &z753, &r753, &s753));

    // 750] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #191: testing point duplication
    let z754 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r754 = [0x9eac5054ed2ec72c, 0x9c400e9c69af7beb, 0x4089464733ff7cd3, 0xacd155416a8b77f3];
    let s754 = [0x6bf5f864ff7be0c2, 0xad4591868595a8ee, 0xdb6db6db6db6db6d, 0x249249246db6db6d];
    let pk754 = [
        0xf4a13945d898c296,
        0x77037d812deb33a0,
        0xf8bce6e563a440f2,
        0x6b17d1f2e12c4247,
        0x3449bf97c840ae0a,
        0xd431cca994cea131,
        0x711814b583f061e9,
        0xb01cbd1c01e58065,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk754, &z754, &r754, &s754));

    // 751] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #269: pseudorandom signature
    let z755 = [0x07a419feca605023, 0x0036e7c32b270c88, 0xed4361f59422a1e3, 0xbb5a52f42f9c9261];
    let r755 = [0x73d7904519e51388, 0x2711f9917060406a, 0x381c4c1f1da8e9de, 0xa8ea150cb80125d7];
    let s755 = [0x7288293285449b86, 0x0c22c9d76ec21725, 0xa73b2d40480c2ba5, 0xf3ab9fa68bd47973];
    let pk755 = [
        0x5b522eba7240fad5,
        0x32e41495a944d004,
        0x13fb8a9e64da3b86,
        0x04aaec73635726f2,
        0x1aa546e8365d525d,
        0xaaf7b4e09fc81d6d,
        0xba01775787ced05e,
        0x87d9315798aaa3a5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk755, &z755, &r755, &s755));

    // 752] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #270: pseudorandom signature
    let z756 = [0x550d7a6e0f345e25, 0x20a6ec113d682299, 0xbf76b9b8cc00832c, 0x532eaabd9574880d];
    let r756 = [0x88268822c253bcce, 0x15d8c43a1365713c, 0x065a051bc7adc206, 0x30e782f964b2e2ff];
    let s756 = [0x0428d2d3f4e08ed5, 0x2e84cacfa7c6eec3, 0xdc8b46c515f9604e, 0x5b16df652aa1ecb2];
    let pk756 = [
        0x5b522eba7240fad5,
        0x32e41495a944d004,
        0x13fb8a9e64da3b86,
        0x04aaec73635726f2,
        0x1aa546e8365d525d,
        0xaaf7b4e09fc81d6d,
        0xba01775787ced05e,
        0x87d9315798aaa3a5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk756, &z756, &r756, &s756));

    // 753] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #271: pseudorandom signature
    let z757 = [0xa495991b7852b855, 0x27ae41e4649b934c, 0x9afbf4c8996fb924, 0xe3b0c44298fc1c14];
    let r757 = [0x8e76e09d8770b34a, 0x42d16e47f219f9e9, 0x7a305c951c0dcbcc, 0xb292a619339f6e56];
    let s757 = [0xab2abebdf89a62e2, 0xe59ec2a17ce5bd2d, 0x2f76f07bfe3661bd, 0x0177e60492c5a824];
    let pk757 = [
        0x5b522eba7240fad5,
        0x32e41495a944d004,
        0x13fb8a9e64da3b86,
        0x04aaec73635726f2,
        0x1aa546e8365d525d,
        0xaaf7b4e09fc81d6d,
        0xba01775787ced05e,
        0x87d9315798aaa3a5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk757, &z757, &r757, &s757));

    // 754] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #272: pseudorandom signature
    let z758 = [0x7f1b40c4cbd36f90, 0x93262cf06340c4fa, 0xdbb5f2c353e632c3, 0xde47c9b27eb8d300];
    let r758 = [0xc69178490d57fb71, 0x39aaf63f00a91f29, 0xe5aada139f52b705, 0x986e65933ef2ed4e];
    let s758 = [0x0f701aaa7a694b9c, 0xdabf0c0217d1c0ff, 0x372308cbf1489bbb, 0x3dafedfb8da6189d];
    let pk758 = [
        0x5b522eba7240fad5,
        0x32e41495a944d004,
        0x13fb8a9e64da3b86,
        0x04aaec73635726f2,
        0x1aa546e8365d525d,
        0xaaf7b4e09fc81d6d,
        0xba01775787ced05e,
        0x87d9315798aaa3a5,
    ];
    assert!(ecdsa_verify_secp256r1(&pk758, &z758, &r758, &s758));

    // 755] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #288: x-coordinate of the public key has many trailing 0's
    let z759 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r759 = [0x7f29745eff3569f1, 0x50dd0fd5defa013c, 0x81e353a3565e4825, 0xd434e262a49eab77];
    let s759 = [0x844218305c6ba17a, 0x98953195d7bc10de, 0x52fd8077be769c2b, 0x9b0c0a93f267fb60];
    let pk759 = [
        0x9fbd816900000000,
        0xf3807eca11738023,
        0x05e4f1600ae2849d,
        0x4f337ccfd67726a8,
        0xd5df7ea60666d685,
        0x7eb504af43a3146c,
        0x416411e988c30f42,
        0xed9dea124cc8c396,
    ];
    assert!(ecdsa_verify_secp256r1(&pk759, &z759, &r759, &s759));

    // 756] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #289: x-coordinate of the public key has many trailing 0's
    let z760 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r760 = [0xadd0be9b1979110b, 0x1463489221bf0a33, 0xf76d79fd7a772e42, 0x0fe774355c04d060];
    let s760 = [0xac6181175df55737, 0x4ca8b91a1f325f3f, 0x43fa4f57f743ce12, 0x500dcba1c69a8fbd];
    let pk760 = [
        0x9fbd816900000000,
        0xf3807eca11738023,
        0x05e4f1600ae2849d,
        0x4f337ccfd67726a8,
        0xd5df7ea60666d685,
        0x7eb504af43a3146c,
        0x416411e988c30f42,
        0xed9dea124cc8c396,
    ];
    assert!(ecdsa_verify_secp256r1(&pk760, &z760, &r760, &s760));

    // 757] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #290: x-coordinate of the public key has many trailing 0's
    let z761 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r761 = [0xbfd06595ee1135e3, 0x8e3b2cd79693f125, 0x950c7d39f03d36dc, 0xbb40bf217bed3fb3];
    let s761 = [0xfa4780745bb55677, 0xc89a1e291ac692b3, 0x32710bdb6a1bf1bf, 0x541bf3532351ebb0];
    let pk761 = [
        0x9fbd816900000000,
        0xf3807eca11738023,
        0x05e4f1600ae2849d,
        0x4f337ccfd67726a8,
        0xd5df7ea60666d685,
        0x7eb504af43a3146c,
        0x416411e988c30f42,
        0xed9dea124cc8c396,
    ];
    assert!(ecdsa_verify_secp256r1(&pk761, &z761, &r761, &s761));

    // 758] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #291: y-coordinate of the public key has many trailing 0's
    let z762 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r762 = [0x556d3e75a233e73a, 0x05badd5ca99231ff, 0xdf3c86ea31389a54, 0x664eb7ee6db84a34];
    let s762 = [0x2e51a2901426a1bd, 0xe0badc678754b8f7, 0x137642490a51560c, 0x59f3c752e52eca46];
    let pk762 = [
        0x5c3f497265004935,
        0x618f06b8ff87e801,
        0xd499a07873fac281,
        0x3cf03d614d8939cf,
        0xe59cdbde80000000,
        0x7c7a1338a82f85a9,
        0x2ce3880a8960dd2a,
        0x84fa174d791c72bf,
    ];
    assert!(ecdsa_verify_secp256r1(&pk762, &z762, &r762, &s762));

    // 759] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #292: y-coordinate of the public key has many trailing 0's
    let z763 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r763 = [0x01985a79d1fd8b43, 0x9c3e42e2d1631fd0, 0x009d6fcd843d4ce3, 0x4cd0429bbabd2827];
    let s763 = [0xe466189d2acdabe3, 0xb7bca77a1a2b869a, 0xbe7ef1d0e0d98f08, 0x9638bf12dd682f60];
    let pk763 = [
        0x5c3f497265004935,
        0x618f06b8ff87e801,
        0xd499a07873fac281,
        0x3cf03d614d8939cf,
        0xe59cdbde80000000,
        0x7c7a1338a82f85a9,
        0x2ce3880a8960dd2a,
        0x84fa174d791c72bf,
    ];
    assert!(ecdsa_verify_secp256r1(&pk763, &z763, &r763, &s763));

    // 760] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #293: y-coordinate of the public key has many trailing 0's
    let z764 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r764 = [0x1e8added97c56c04, 0x60e3ce9aed5e5fd4, 0x1c44d8b6cb62b9f4, 0xe56c6ea2d1b01709];
    let s764 = [0x7fc1378180f89b55, 0x4fcf2b8025807820, 0xbe20b457e463440b, 0xa308ec31f281e955];
    let pk764 = [
        0x5c3f497265004935,
        0x618f06b8ff87e801,
        0xd499a07873fac281,
        0x3cf03d614d8939cf,
        0xe59cdbde80000000,
        0x7c7a1338a82f85a9,
        0x2ce3880a8960dd2a,
        0x84fa174d791c72bf,
    ];
    assert!(ecdsa_verify_secp256r1(&pk764, &z764, &r764, &s764));

    // 761] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #294: y-coordinate of the public key has many trailing 1's
    let z765 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r765 = [0x011f8fbbf3466830, 0x57c176356a2624fb, 0xcabed3346d891eee, 0x1158a08d291500b4];
    let s765 = [0xa46798c18f285519, 0xc91f378b75d487dd, 0xe082325b85290c5b, 0x228a8c486a736006];
    let pk765 = [
        0x5c3f497265004935,
        0x618f06b8ff87e801,
        0xd499a07873fac281,
        0x3cf03d614d8939cf,
        0x1a6324217fffffff,
        0x8385ecc857d07a56,
        0xd31c77f5769f22d5,
        0x7b05e8b186e38d41,
    ];
    assert!(ecdsa_verify_secp256r1(&pk765, &z765, &r765, &s765));

    // 762] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #295: y-coordinate of the public key has many trailing 1's
    let z766 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r766 = [0x3e0dde56d309fa9d, 0x2687b29176939dd2, 0x0ea36b0c0fc8d6aa, 0xb1db9289649f5941];
    let s766 = [0x4e1c3f48a1251336, 0x3a6d1af5c23c7d58, 0x5b0dbd987366dcf4, 0x3e1535e428055901];
    let pk766 = [
        0x5c3f497265004935,
        0x618f06b8ff87e801,
        0xd499a07873fac281,
        0x3cf03d614d8939cf,
        0x1a6324217fffffff,
        0x8385ecc857d07a56,
        0xd31c77f5769f22d5,
        0x7b05e8b186e38d41,
    ];
    assert!(ecdsa_verify_secp256r1(&pk766, &z766, &r766, &s766));

    // 763] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #296: y-coordinate of the public key has many trailing 1's
    let z767 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r767 = [0x0ac6f0ca4e24ed86, 0x0a341a79f2dd1a22, 0x446aa8d4e6e7578b, 0xb7b16e762286cb96];
    let s767 = [0x5e55234ecb8f12bc, 0x1780146df799ccf5, 0x661c547d07bbb072, 0xddc60a700a139b04];
    let pk767 = [
        0x5c3f497265004935,
        0x618f06b8ff87e801,
        0xd499a07873fac281,
        0x3cf03d614d8939cf,
        0x1a6324217fffffff,
        0x8385ecc857d07a56,
        0xd31c77f5769f22d5,
        0x7b05e8b186e38d41,
    ];
    assert!(ecdsa_verify_secp256r1(&pk767, &z767, &r767, &s767));

    // 764] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #297: x-coordinate of the public key has many trailing 1's
    let z768 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r768 = [0xd1c91c670d9105b4, 0xd796edad36bc6e6b, 0xc8e00d8df963ff35, 0xd82a7c2717261187];
    let s768 = [0x680d07debd139929, 0x351ecd5988efb23f, 0xf4603e7cbac0f3c0, 0x3dcabddaf8fcaa61];
    let pk768 = [
        0xdfa5ff8effffffff,
        0x45956ebcfe8ad0f6,
        0x344ed94bca3fcd05,
        0x2829c31faa2e400e,
        0xeb3bd37ebeb9222e,
        0x4113099052df57e7,
        0x5855afa7676ade28,
        0xa01aafaf000e5258,
    ];
    assert!(ecdsa_verify_secp256r1(&pk768, &z768, &r768, &s768));

    // 765] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #298: x-coordinate of the public key has many trailing 1's
    let z769 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r769 = [0x6a5cba063254af78, 0x7787802baff30ce9, 0x3d5befe719f462d7, 0x5eb9c8845de68eb1];
    let s769 = [0x2b87ddbe2ef66fb5, 0x44972186228ee9a6, 0x7ca0ff9bbd92fb6e, 0x2c026ae9be2e2a5e];
    let pk769 = [
        0xdfa5ff8effffffff,
        0x45956ebcfe8ad0f6,
        0x344ed94bca3fcd05,
        0x2829c31faa2e400e,
        0xeb3bd37ebeb9222e,
        0x4113099052df57e7,
        0x5855afa7676ade28,
        0xa01aafaf000e5258,
    ];
    assert!(ecdsa_verify_secp256r1(&pk769, &z769, &r769, &s769));

    // 766] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #299: x-coordinate of the public key has many trailing 1's
    let z770 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r770 = [0x404a8e4e36230c28, 0xf277921becc117d0, 0xf3b782b170239f90, 0x96843dd03c22abd2];
    let s770 = [0x19e1ede123dd991d, 0x9a31214eb4d7e6db, 0x43f67165976de9ed, 0xf2be378f526f74a5];
    let pk770 = [
        0xdfa5ff8effffffff,
        0x45956ebcfe8ad0f6,
        0x344ed94bca3fcd05,
        0x2829c31faa2e400e,
        0xeb3bd37ebeb9222e,
        0x4113099052df57e7,
        0x5855afa7676ade28,
        0xa01aafaf000e5258,
    ];
    assert!(ecdsa_verify_secp256r1(&pk770, &z770, &r770, &s770));

    // 767] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #300: x-coordinate of the public key is large
    let z771 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r771 = [0x3b760297067421f6, 0x4d27e9d98edc2d0e, 0x6f9996af72933946, 0x766456dce1857c90];
    let s771 = [0x3646bfbbf19d0b41, 0x4e55376eced699e9, 0x81dccaf5d19037ec, 0x402385ecadae0d80];
    let pk771 = [
        0x08318c4ca9a7a4f5,
        0x65ff9059ad6aac07,
        0x0458dd8f9e738f26,
        0xfffffff948081e6a,
        0x78e6529a1663bd73,
        0xe0c0fb89557ad0bf,
        0x311ee54149b973ca,
        0x5a8abcba2dda8474,
    ];
    assert!(ecdsa_verify_secp256r1(&pk771, &z771, &r771, &s771));

    // 768] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #301: x-coordinate of the public key is large
    let z772 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r772 = [0x9c34f777de7b9fd9, 0xb97ed8b07cced0b1, 0x19e6518a11b2dbc2, 0xc605c4b2edeab204];
    let s772 = [0xff5e159d47326dba, 0xb2cde2eda700fb1c, 0xc719647bc8af1b29, 0xedf0f612c5f46e03];
    let pk772 = [
        0x08318c4ca9a7a4f5,
        0x65ff9059ad6aac07,
        0x0458dd8f9e738f26,
        0xfffffff948081e6a,
        0x78e6529a1663bd73,
        0xe0c0fb89557ad0bf,
        0x311ee54149b973ca,
        0x5a8abcba2dda8474,
    ];
    assert!(ecdsa_verify_secp256r1(&pk772, &z772, &r772, &s772));

    // 769] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #302: x-coordinate of the public key is large
    let z773 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r773 = [0xb732bfe3b7eb8a84, 0x10e64485d9929ad7, 0xf6141c9ac54141f2, 0xd48b68e6cabfe03c];
    let s773 = [0x08f0772315b6c941, 0x4508c389109ad2f2, 0x19dc26f9b7e2265e, 0xfeedae50c61bd00e];
    let pk773 = [
        0x08318c4ca9a7a4f5,
        0x65ff9059ad6aac07,
        0x0458dd8f9e738f26,
        0xfffffff948081e6a,
        0x78e6529a1663bd73,
        0xe0c0fb89557ad0bf,
        0x311ee54149b973ca,
        0x5a8abcba2dda8474,
    ];
    assert!(ecdsa_verify_secp256r1(&pk773, &z773, &r773, &s773));

    // 770] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #303: x-coordinate of the public key is small
    let z774 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r774 = [0xc35a93d12a5dd4c7, 0x710ad7f6595d5874, 0x65957098569f0479, 0xb7c81457d4aeb6aa];
    let s774 = [0x4b9e3a05c0a1cdb3, 0x1a9199f2ca574dad, 0xd568069a432ca18a, 0xb7961a0b652878c2];
    let pk774 = [
        0xbff1173937ba748e,
        0x6f9e0015eeb23aeb,
        0x949d5f03a6f5c7f8,
        0x00000003fa15f963,
        0x548ca48af2ba7e71,
        0xfadcfcb0023ea889,
        0x555fa13659cca5d7,
        0x1099872070e8e87c,
    ];
    assert!(ecdsa_verify_secp256r1(&pk774, &z774, &r774, &s774));

    // 771] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #304: x-coordinate of the public key is small
    let z775 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r775 = [0xde5e9652e76ff3f7, 0xe3cf97e263e669f8, 0xa30a1321d5858e1e, 0x6b01332ddb6edfa9];
    let s775 = [0xcc58f9e69e96cd5a, 0x139c8f7d86b02cb1, 0x9a6a04ace2bd0f70, 0x5939545fced45730];
    let pk775 = [
        0xbff1173937ba748e,
        0x6f9e0015eeb23aeb,
        0x949d5f03a6f5c7f8,
        0x00000003fa15f963,
        0x548ca48af2ba7e71,
        0xfadcfcb0023ea889,
        0x555fa13659cca5d7,
        0x1099872070e8e87c,
    ];
    assert!(ecdsa_verify_secp256r1(&pk775, &z775, &r775, &s775));

    // 772] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #305: x-coordinate of the public key is small
    let z776 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r776 = [0x0e6a4fb93f106361, 0x4101cd2fd8436b7d, 0x349f9fc356b6c034, 0xefdb884720eaeadc];
    let s776 = [0xe48cb60d8113385d, 0xcba9e77de7d69b6c, 0x613975473aadf3aa, 0xf24bee6ad5dc05f7];
    let pk776 = [
        0xbff1173937ba748e,
        0x6f9e0015eeb23aeb,
        0x949d5f03a6f5c7f8,
        0x00000003fa15f963,
        0x548ca48af2ba7e71,
        0xfadcfcb0023ea889,
        0x555fa13659cca5d7,
        0x1099872070e8e87c,
    ];
    assert!(ecdsa_verify_secp256r1(&pk776, &z776, &r776, &s776));

    // 773] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #306: y-coordinate of the public key is small
    let z777 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r777 = [0x8014c87b8b20eb07, 0x9b23a23dd973dcbe, 0xb88fb5a646836aea, 0x31230428405560dc];
    let s777 = [0x8bd7ae3d9bd0beff, 0xaf97374e19f3c5fb, 0x6646747694a41b0a, 0x0f9344d6e812ce16];
    let pk777 = [
        0x25e9f851c18af015,
        0xbe5d2d6796707d81,
        0xaa6ecbbc612816b3,
        0xbcbb2914c79f045e,
        0xf300a698a7193bc2,
        0xdd684ade5a1127bc,
        0x0fa2ea4cceb9ab63,
        0x000000001352bb4a,
    ];
    assert!(ecdsa_verify_secp256r1(&pk777, &z777, &r777, &s777));

    // 774] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #307: y-coordinate of the public key is small
    let z778 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r778 = [0x9174db34c4855743, 0x94359c7db9841d67, 0x0d5c470cda0b36b2, 0xcaa797da65b320ab];
    let s778 = [0x3de6d9b36242e5a0, 0x123d2685ee3b941d, 0x45391aaf7505f345, 0xcf543a62f23e2127];
    let pk778 = [
        0x25e9f851c18af015,
        0xbe5d2d6796707d81,
        0xaa6ecbbc612816b3,
        0xbcbb2914c79f045e,
        0xf300a698a7193bc2,
        0xdd684ade5a1127bc,
        0x0fa2ea4cceb9ab63,
        0x000000001352bb4a,
    ];
    assert!(ecdsa_verify_secp256r1(&pk778, &z778, &r778, &s778));

    // 775] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #308: y-coordinate of the public key is small
    let z779 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r779 = [0x1c336ed800185945, 0x19bc54084536e7d2, 0xd7867657e5d6d365, 0x7e5f0ab5d900d3d3];
    let s779 = [0xe727ff0b19b646aa, 0x6688294aad35aa72, 0x4b82dfb322e5ac67, 0x9450c07f201faec9];
    let pk779 = [
        0x25e9f851c18af015,
        0xbe5d2d6796707d81,
        0xaa6ecbbc612816b3,
        0xbcbb2914c79f045e,
        0xf300a698a7193bc2,
        0xdd684ade5a1127bc,
        0x0fa2ea4cceb9ab63,
        0x000000001352bb4a,
    ];
    assert!(ecdsa_verify_secp256r1(&pk779, &z779, &r779, &s779));

    // 776] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #309: y-coordinate of the public key is large
    let z780 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r780 = [0x03aa69f0ca25b356, 0x3f8a1e4a2136fe4b, 0x6dc6a480bf037ae2, 0xd7d70c581ae9e3f6];
    let s780 = [0xaf41d9127cc47224, 0x13e85658e62a59e2, 0xba962c8a3ee833a4, 0x89c460f8a5a5c2bb];
    let pk780 = [
        0x25e9f851c18af015,
        0xbe5d2d6796707d81,
        0xaa6ecbbc612816b3,
        0xbcbb2914c79f045e,
        0x0cff596758e6c43d,
        0x2297b522a5eed843,
        0xf05d15b33146549c,
        0xfffffffeecad44b6,
    ];
    assert!(ecdsa_verify_secp256r1(&pk780, &z780, &r780, &s780));

    // 777] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #310: y-coordinate of the public key is large
    let z781 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r781 = [0xeee34bb396266b34, 0xb7aa20c625975e5e, 0xe0dfa0bf68bcdf4b, 0x341c1b9ff3c83dd5];
    let s781 = [0x902a67099e0a4469, 0x49c634e77765a017, 0x121b22b11366fad5, 0x72b69f061b750fd5];
    let pk781 = [
        0x25e9f851c18af015,
        0xbe5d2d6796707d81,
        0xaa6ecbbc612816b3,
        0xbcbb2914c79f045e,
        0x0cff596758e6c43d,
        0x2297b522a5eed843,
        0xf05d15b33146549c,
        0xfffffffeecad44b6,
    ];
    assert!(ecdsa_verify_secp256r1(&pk781, &z781, &r781, &s781));

    // 778] wycheproof/ecdsa_webcrypto_test.json EcdsaP1363Verify SHA-256 #311: y-coordinate of the public key is large
    let z782 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r782 = [0x7628d313a3814f67, 0x9bd1781a59180994, 0x72a42f0d87387935, 0x70bebe684cdcb5ca];
    let s782 = [0x2dcde6b2094798a9, 0xc0e464b1c3577f4c, 0xd535fa31027bbe9c, 0xaec03aca8f5587a4];
    let pk782 = [
        0x25e9f851c18af015,
        0xbe5d2d6796707d81,
        0xaa6ecbbc612816b3,
        0xbcbb2914c79f045e,
        0x0cff596758e6c43d,
        0x2297b522a5eed843,
        0xf05d15b33146549c,
        0xfffffffeecad44b6,
    ];
    assert!(ecdsa_verify_secp256r1(&pk782, &z782, &r782, &s782));

    // 779] invalid public key x param errors
    let z783 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r783 = [0x7628d313a3814f67, 0x9bd1781a59180994, 0x72a42f0d87387935, 0x70bebe684cdcb5ca];
    let s783 = [0x2dcde6b2094798a9, 0xc0e464b1c3577f4c, 0xd535fa31027bbe9c, 0xaec03aca8f5587a4];
    let pk783 = [
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0cff596758e6c43d,
        0x2297b522a5eed843,
        0xf05d15b33146549c,
        0xfffffffeecad44b6,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk783, &z783, &r783, &s783));

    let z784 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r784 = [0x7628d313a3814f67, 0x9bd1781a59180994, 0x72a42f0d87387935, 0x70bebe684cdcb5ca];
    let s784 = [0x2dcde6b2094798a9, 0xc0e464b1c3577f4c, 0xd535fa31027bbe9c, 0xaec03aca8f5587a4];
    let pk784 = [
        0xffffffffffffffff,
        0x00000000ffffffff,
        0x0000000000000000,
        0xffffffff00000001,
        0x0cff596758e6c43d,
        0x2297b522a5eed843,
        0xf05d15b33146549c,
        0xfffffffeecad44b6,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk784, &z784, &r784, &s784));

    // 780] invalid public key y param errors
    let z785 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r785 = [0x7628d313a3814f67, 0x9bd1781a59180994, 0x72a42f0d87387935, 0x70bebe684cdcb5ca];
    let s785 = [0x2dcde6b2094798a9, 0xc0e464b1c3577f4c, 0xd535fa31027bbe9c, 0xaec03aca8f5587a4];
    let pk785 = [
        0x25e9f851c18af015,
        0xbe5d2d6796707d81,
        0xaa6ecbbc612816b3,
        0xbcbb2914c79f045e,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk785, &z785, &r785, &s785));

    let z786 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r786 = [0x7628d313a3814f67, 0x9bd1781a59180994, 0x72a42f0d87387935, 0x70bebe684cdcb5ca];
    let s786 = [0x2dcde6b2094798a9, 0xc0e464b1c3577f4c, 0xd535fa31027bbe9c, 0xaec03aca8f5587a4];
    let pk786 = [
        0x25e9f851c18af015,
        0xbe5d2d6796707d81,
        0xaa6ecbbc612816b3,
        0xbcbb2914c79f045e,
        0xffffffffffffffff,
        0x00000000ffffffff,
        0x0000000000000000,
        0xffffffff00000001,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk786, &z786, &r786, &s786));

    // 781] reference point errors
    let z787 = [0xf6dcafa5132b2f91, 0x94c6ed9236e4a773, 0x848b9eeb4a7145ca, 0x2f77668a9dfbf8d5];
    let r787 = [0x7628d313a3814f67, 0x9bd1781a59180994, 0x72a42f0d87387935, 0x70bebe684cdcb5ca];
    let s787 = [0x2dcde6b2094798a9, 0xc0e464b1c3577f4c, 0xd535fa31027bbe9c, 0xaec03aca8f5587a4];
    let pk787 = [
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
        0x0000000000000000,
    ];
    assert!(!ecdsa_verify_secp256r1(&pk787, &z787, &r787, &s787));
}
