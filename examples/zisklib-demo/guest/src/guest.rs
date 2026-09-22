//! Demo guest: calls a function implemented in the ZisK library (`ziskasm/zisklib/`)
//! rather than in Rust. The `ziskos_add` stub (see `ziskos.rs`) is redirected by
//! the transpiler to the hand-written `zisklib_add` routine, which runs as ZisK
//! instructions in the guest's place. The committed result (7) proves the
//! redirect happened — the stub's own body would return 0xBAD.

#![no_main]

ziskos::entrypoint!(main);

use core::hint::black_box;
use zisklib::{
    add_mod256, blake2b_compress, bls12_381_hash_to_curve_g2, bls12_381_map_to_curve_g1,
    bls12_381_map_to_curve_g2, bls12_381_pairing_check, bls12_381_verify,
    bls12_381_verify_kzg_proof, bn254_pairing_check, checked_add256, checked_div256,
    checked_mul256, checked_pow256, checked_square256, checked_sub256, div_ceil256, div_rem256,
    inv_mod256, inv256, keccak256, modexp_u64, mul_mod256, overflowing_add256, overflowing_mul256,
    overflowing_pow256, pow_mod256, reduce_mod256, saturating_pow256, saturating_sub256,
    secp256k1_ecdsa_recover, secp256k1_ecdsa_verify, secp256k1_schnorr_verify,
    secp256r1_ecdsa_verify, sha256, square_mod256, wrapping_add256, wrapping_mul256,
    wrapping_neg256, wrapping_pow256, wrapping_rem256, wrapping_square256, wrapping_sub256,
    ziskos_add,
};

// ---- BN254 (alt_bn128) ecPairing vectors: e(G1,G2)·e(-G1,G2) == 1, so the
// 2-pair check accepts (0) while the single pair e(G1,G2) != 1 rejects (1). ----
const BN_PC_G1: [[u64; 8]; 2] = [
    [0x0000000000000001, 0, 0, 0, 0x0000000000000002, 0, 0, 0],
    [
        0x0000000000000001,
        0,
        0,
        0,
        0x3c208c16d87cfd45,
        0x97816a916871ca8d,
        0xb85045b68181585d,
        0x30644e72e131a029,
    ],
];
const BN_PC_G2: [[u64; 16]; 2] = [
    [
        0x46debd5cd992f6ed,
        0x674322d4f75edadd,
        0x426a00665e5c4479,
        0x1800deef121f1e76,
        0x97e485b7aef312c2,
        0xf1aa493335a9e712,
        0x7260bfb731fb5d25,
        0x198e9393920d483a,
        0x4ce6cc0166fa7daa,
        0xe3d1e7690c43d37b,
        0x4aab71808dcb408f,
        0x12c85ea5db8c6deb,
        0x55acdadcd122975b,
        0xbc4b313370b38ef3,
        0xec9e99ad690c3395,
        0x090689d0585ff075,
    ],
    [
        0x46debd5cd992f6ed,
        0x674322d4f75edadd,
        0x426a00665e5c4479,
        0x1800deef121f1e76,
        0x97e485b7aef312c2,
        0xf1aa493335a9e712,
        0x7260bfb731fb5d25,
        0x198e9393920d483a,
        0x4ce6cc0166fa7daa,
        0xe3d1e7690c43d37b,
        0x4aab71808dcb408f,
        0x12c85ea5db8c6deb,
        0x55acdadcd122975b,
        0xbc4b313370b38ef3,
        0xec9e99ad690c3395,
        0x090689d0585ff075,
    ],
];

const BLS_PC_G1: [[u64; 12]; 2] = [
    [
        0xfb3af00adb22c6bb,
        0x6c55e83ff97a1aef,
        0xa14e3a3f171bac58,
        0xc3688c4f9774b905,
        0x2695638c4fa9ac0f,
        0x17f1d3a73197d794,
        0x0caa232946c5e7e1,
        0xd03cc744a2888ae4,
        0x00db18cb2c04b3ed,
        0xfcf5e095d5d00af6,
        0xa09e30ed741d8ae4,
        0x08b3f481e3aaa0f1,
    ],
    [
        0xfb3af00adb22c6bb,
        0x6c55e83ff97a1aef,
        0xa14e3a3f171bac58,
        0xc3688c4f9774b905,
        0x2695638c4fa9ac0f,
        0x17f1d3a73197d794,
        0xad54dcd6b939c2ca,
        0x4e6f38ba0ecb751b,
        0x6655b9d5caac4236,
        0x67816aef1db507c9,
        0xaa7d76c8cf2e21f2,
        0x114d1d6855d545a8,
    ],
];
const BLS_PC_G2: [[u64; 24]; 2] = [
    [
        0xd48056c8c121bdb8,
        0x0bac0326a805bbef,
        0xb4510b647ae3d177,
        0xc6e47ad4fa403b02,
        0x260805272dc51051,
        0x024aa2b2f08f0a91,
        0xe5ac7d055d042b7e,
        0x334cf11213945d57,
        0xb5da61bbdc7f5049,
        0x596bd0d09920b61a,
        0x7dacd3a088274f65,
        0x13e02b6052719f60,
        0xe193548608b82801,
        0x923ac9cc3baca289,
        0x6d429a695160d12c,
        0xadfd9baa8cbdd3a7,
        0x8cc9cdc6da2e351a,
        0x0ce5d527727d6e11,
        0xaaa9075ff05f79be,
        0x3f370d275cec1da1,
        0x267492ab572e99ab,
        0xcb3e287e85a763af,
        0x32acd2b02bc28b99,
        0x0606c4a02ea734cc,
    ],
    [
        0xd48056c8c121bdb8,
        0x0bac0326a805bbef,
        0xb4510b647ae3d177,
        0xc6e47ad4fa403b02,
        0x260805272dc51051,
        0x024aa2b2f08f0a91,
        0xe5ac7d055d042b7e,
        0x334cf11213945d57,
        0xb5da61bbdc7f5049,
        0x596bd0d09920b61a,
        0x7dacd3a088274f65,
        0x13e02b6052719f60,
        0xe193548608b82801,
        0x923ac9cc3baca289,
        0x6d429a695160d12c,
        0xadfd9baa8cbdd3a7,
        0x8cc9cdc6da2e351a,
        0x0ce5d527727d6e11,
        0xaaa9075ff05f79be,
        0x3f370d275cec1da1,
        0x267492ab572e99ab,
        0xcb3e287e85a763af,
        0x32acd2b02bc28b99,
        0x0606c4a02ea734cc,
    ],
];

const BLS_MAP_U1: [u64; 6] = [
    0x000000000000002a,
    0x0000000000000000,
    0x0000000000000000,
    0x0000000000000000,
    0x0000000000000000,
    0x0000000000000000,
];
const BLS_MAP_G1: [u64; 12] = [
    0x7a9e1d8543273a30,
    0xc566ab55d828dfd0,
    0xdfb472ff2be55501,
    0x2004453b4435bbf1,
    0x283e06ec543b6ebc,
    0x149c845640b62922,
    0xae34ebe0b048dfa6,
    0xcf887e0ee0638d57,
    0xa4917f4bbc9bd2f2,
    0xf9c4454adb818220,
    0xded4192633c38864,
    0x0e868eb185f23102,
];
const BLS_MAP_U2: [u64; 12] = [
    0x000000000000002a,
    0x0000000000000000,
    0x0000000000000000,
    0x0000000000000000,
    0x0000000000000000,
    0x0000000000000000,
    0x000000000000002b,
    0x0000000000000000,
    0x0000000000000000,
    0x0000000000000000,
    0x0000000000000000,
    0x0000000000000000,
];
const BLS_MAP_G2: [u64; 24] = [
    0x40f41de54925cf4f,
    0x3b807ddc1628c68d,
    0xed8806f295c7815b,
    0x657c645ccf514547,
    0x00fc5a63d535803c,
    0x0c9c9c6129ac3470,
    0x43cd782600ef9685,
    0xaf4b54391659dabf,
    0xf21a8c7f8b8a5098,
    0xcf0e22984e23fe98,
    0x8f76f5776084dd24,
    0x01e3689c3b5768e9,
    0x0fb941f07cf8398d,
    0x68cef9e267f9ecbe,
    0xf73284f519e48deb,
    0xa344f592f29b0726,
    0x2e07d41eb79eb8be,
    0x0f6f344b91984752,
    0xc4e3e743136a1671,
    0x768da8d06cc394c3,
    0x6533573f6b96b846,
    0xea5aacb5a473bebf,
    0x16f16ffb33eb5dbc,
    0x157f85373609d68b,
];

const BLS_HTC_MSG: [u8; 3] = [97, 98, 99];
const BLS_HTC_DST: [u8; 12] = [66, 76, 83, 95, 84, 69, 83, 84, 95, 68, 83, 84];
const BLS_HTC_EXP: [u64; 24] = [
    0xd49b9fa0ab72d14e,
    0x398b10f6999cc8a0,
    0x0cb15d5c7598bad7,
    0x392a5a344b088989,
    0x26083375405e0d59,
    0x0c657161524f9e91,
    0xdb9e9c7c081f770a,
    0x33e6329d094d42fd,
    0x7f72678ed54a58fb,
    0xcc59b6cbf17a1555,
    0x9bbe6791f4c39042,
    0x0c85113990e68b08,
    0xbda5a508a8690e11,
    0xd686da17caacd12f,
    0xdbb8256360b0518f,
    0x7aa2a9fff073a7d6,
    0xda97f6a4d4498387,
    0x1543ad0462f91be6,
    0x072c15da68a36e3e,
    0x92a1249fedff369e,
    0xd64579a5e8216fdb,
    0xf8f2a8407b06d9c6,
    0x3498a80c3b22756b,
    0x0add0ca265636311,
];
const BLS_SIG_PK: [u8; 48] = [
    185, 85, 48, 112, 180, 18, 163, 118, 116, 59, 0, 172, 214, 155, 235, 81, 72, 38, 205, 250, 43,
    149, 53, 0, 129, 133, 58, 138, 61, 113, 35, 163, 130, 138, 72, 118, 16, 7, 129, 117, 235, 124,
    62, 117, 202, 4, 233, 108,
];
const BLS_SIG_MSG: [u8; 3] = [97, 98, 99];
const BLS_SIG_SIG: [u8; 96] = [
    152, 151, 210, 163, 4, 65, 253, 53, 217, 112, 10, 95, 122, 109, 169, 209, 110, 144, 223, 232,
    178, 64, 16, 170, 199, 41, 213, 55, 201, 159, 134, 239, 153, 7, 36, 71, 152, 68, 172, 113, 110,
    92, 254, 28, 106, 238, 190, 98, 10, 110, 26, 11, 95, 250, 212, 99, 237, 111, 2, 175, 152, 215,
    16, 9, 60, 214, 3, 211, 89, 11, 104, 20, 67, 128, 180, 25, 54, 27, 151, 122, 120, 12, 32, 146,
    234, 87, 235, 72, 143, 152, 119, 23, 238, 249, 239, 88,
];
const BLS_KZG_Z: [u8; 32] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5,
];
const BLS_KZG_Y: [u8; 32] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5,
];
const BLS_KZG_COMM: [u8; 48] = [
    173, 62, 181, 1, 33, 19, 154, 163, 77, 177, 213, 69, 9, 58, 201, 55, 74, 183, 188, 162, 192,
    243, 191, 40, 226, 124, 141, 205, 143, 199, 203, 66, 210, 89, 38, 252, 12, 151, 179, 54, 233,
    240, 251, 53, 229, 160, 76, 129,
];
const BLS_KZG_PROOF: [u8; 48] = [
    151, 241, 211, 167, 49, 151, 215, 148, 38, 149, 99, 140, 79, 169, 172, 15, 195, 104, 140, 79,
    151, 116, 185, 5, 161, 78, 58, 63, 23, 27, 172, 88, 108, 85, 232, 63, 249, 122, 26, 239, 251,
    58, 240, 10, 219, 34, 198, 187,
];

const MODEXP_B0: [u64; 4] =
    [0xb9096a04e7d80068, 0xc963cfe0afae5a3b, 0xe1454c40c439f34a, 0x00000000000000e7];
const MODEXP_E0: [u64; 1] = [0x0000000000010001];
const MODEXP_M0: [u64; 4] =
    [0x42840d2b26b563b1, 0xa2beee31ac8be7d7, 0xe7aa8576d96e5adf, 0x037d0fbe19fcfc64];
const MODEXP_R0: [u64; 4] =
    [0x0f978f2b72b5a452, 0x3c8aafbe775fd13c, 0x7fd36e76228108e2, 0x01d117606712d600];
const MODEXP_B1: [u64; 11] = [
    0x92ac3d4253d23c0b,
    0x2b5c5cd1e7ca430e,
    0x6959935406e82a01,
    0xfe6c2b036820212c,
    0x1a6e72b91333bc1c,
    0x51b31a6c20050ed3,
    0xf335c3577972a36d,
    0x730bed9c94a67f00,
    0x356a41526977a41b,
    0x51209e8f332726d0,
    0x0e89c5bca0187b4d,
];
const MODEXP_E1: [u64; 2] = [0x55e7d67eae6ac4a9, 0x6d683cf8542861cd];
const MODEXP_M1: [u64; 8] = [
    0x8491cabea0afe357,
    0xd775f593ce3ad2b2,
    0x67a9b05c7dfb27e8,
    0x34d2ea1614daf467,
    0x3e1dcfb592bde31c,
    0x33aa391808fc2081,
    0x1570bc621832c9e2,
    0x303814a630b9f609,
];
const MODEXP_R1: [u64; 8] = [
    0xddf59ba42999f533,
    0xf4b29ac392e25f96,
    0xe1a72e94753406c4,
    0x57690de463fe5830,
    0xc5c58b41fafb685a,
    0x703f8f4034a63e98,
    0xe0e761f90d201018,
    0x0f64c2e2e27f9865,
];

// ---- Elliptic-curve golden vectors (generated by pure-Python EC math; each
// vector self-checks: the corresponding verify/recover succeeds off-chain). All
// scalars/coordinates are 256-bit little-endian limbs; points are x‖y. ----

// secp256k1 ECDSA: pk, hash z, signature (r, s), and recovery id.
const K1_PK: [u64; 8] = [
    0x14407306bbb14036,
    0xd094a7f9e6c34f91,
    0x658bd41290f20684,
    0xdb34bc63d98280ff,
    0xc02c7405f8e56d15,
    0x4ae5b4e3c916a634,
    0x6bc37c8b5d663653,
    0x1f6b0812e5e07891,
];
const K1_Z: [u64; 4] =
    [0x7777777788888888, 0x5555555566666666, 0x3333333344444444, 0x1111111122222222];
const K1_R: [u64; 4] =
    [0xa295737bfd46a97b, 0xe8e2f2870abb4b29, 0xa34a6edabd55e4a7, 0x6e75db84b1c97480];
const K1_S: [u64; 4] =
    [0xad3dcf793c56c141, 0x67f3edb21bb4023b, 0x0b9b6277b82f45a5, 0xecc321902c0ee69d];
const K1_RECID: u64 = 1;

// secp256k1 BIP-340 Schnorr: x-only pubkey and signature (r, s) over msg = 0..31.
const SCH_PKX: [u64; 4] =
    [0xae0845d7f3604c14, 0x20ad82d71eaa28b3, 0x0c6fe03e6663f9c5, 0x66133fadb1debbb1];
const SCH_R: [u64; 4] =
    [0x28a45c7feed7a2bf, 0x5756107d5b30f269, 0x67f43f2e8da2922b, 0xd33f35d170add73e];
const SCH_S: [u64; 4] =
    [0x39bc5307d459879a, 0x16b8c15b09001067, 0x6eb49506d528af8b, 0x095299159785f61a];

// secp256r1 (NIST P-256) ECDSA: pk, hash z, signature (r, s).
const R1_PK: [u64; 8] = [
    0x044a5012125cdee9,
    0x1d0499409de15791,
    0x61e70a980bd21f7e,
    0x220a22939de323a8,
    0x9fa44d1c0c14beac,
    0xafbd5535548a2aa9,
    0xb8826a7811fddb8a,
    0x46e87866c68e3027,
];
const R1_Z: [u64; 4] =
    [0x7777777788888888, 0x5555555566666666, 0x3333333344444444, 0x1111111122222222];
const R1_R: [u64; 4] =
    [0x4c1311ba75ce25c5, 0x37b07698ecac901b, 0x29b85d379fce70c6, 0x22f8d8249d17151e];
const R1_S: [u64; 4] =
    [0xf870a80e44714eb2, 0xe3774a47b5a9d797, 0xb3f9051079640b29, 0x63c9f14497212880];

// Hardcoded expected keccak256 digests (independent of any keccak API), so the
// test is self-contained. keccak256("") is the canonical empty-string vector.
const KECCAK_EMPTY: [u8; 32] = [
    0xc5, 0xd2, 0x46, 0x01, 0x86, 0xf7, 0x23, 0x3c, 0x92, 0x7e, 0x7d, 0xb2, 0xdc, 0xc7, 0x03, 0xc0,
    0xe5, 0x00, 0xb6, 0x53, 0xca, 0x82, 0x27, 0x3b, 0x7b, 0xfa, 0xd8, 0x04, 0x5d, 0x85, 0xa4, 0x70,
];
// keccak256 of the 144-byte input built below (byte value i repeated 8 times, i = 0..17).
const KECCAK_BIG144: [u8; 32] = [
    0x52, 0xa0, 0x48, 0xee, 0x61, 0x22, 0x1c, 0x82, 0x92, 0xa1, 0x24, 0x59, 0x91, 0xe1, 0x22, 0x27,
    0x69, 0x41, 0x71, 0x29, 0x5a, 0xab, 0xc4, 0x03, 0xf0, 0x15, 0xe9, 0xc9, 0x57, 0x2c, 0x5e, 0xbd,
];
// keccak256 of the 13-byte input [0,1,..,12] — a non-multiple-of-8 length.
const KECCAK_13: [u8; 32] = [
    0xa1, 0xb3, 0x65, 0xd4, 0x5c, 0x3c, 0xde, 0x59, 0xc2, 0x47, 0xb8, 0x1f, 0xcf, 0xee, 0xb1, 0xc5,
    0x84, 0xcd, 0xad, 0x57, 0x3d, 0xfc, 0x2e, 0x3b, 0xb3, 0x2d, 0x42, 0x25, 0xd6, 0xbf, 0xe9, 0x89,
];

// SHA-256 digests (FIPS 180-4 test vectors): the empty string, "abc", and 56
// bytes of 'a' (which forces the two-block final-padding path).
const SHA256_EMPTY: [u8; 32] = [
    0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9, 0x24,
    0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52, 0xb8, 0x55,
];
const SHA256_ABC: [u8; 32] = [
    0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae, 0x22, 0x23,
    0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61, 0xf2, 0x00, 0x15, 0xad,
];
const SHA256_56A: [u8; 32] = [
    0xb3, 0x54, 0x39, 0xa4, 0xac, 0x6f, 0x09, 0x48, 0xb6, 0xd6, 0xf9, 0xe3, 0xc6, 0xaf, 0x0f, 0x5f,
    0x59, 0x0c, 0xe2, 0x0f, 0x1b, 0xde, 0x70, 0x90, 0xef, 0x79, 0x70, 0x68, 0x6e, 0xc6, 0x73, 0x8a,
];

/// keccak256 via the ziskasm-backed wrapper (`zisklib::keccak256` → redirected
/// `zisklib_keccak`), checked against a hardcoded expected digest.
fn keccak_matches(input: &[u8], expected: &[u8; 32]) -> bool {
    &keccak256(input) == expected
}

/// SHA-256 via `zisklib::sha256` (→ redirected `zisklib_sha256`), checked against
/// a hardcoded expected digest.
fn sha256_matches(input: &[u8], expected: &[u8; 32]) -> bool {
    &sha256(input) == expected
}

fn main() {
    // 1. Simple function: `black_box` keeps args opaque so the call is real.
    let a = black_box(3u64);
    let b = black_box(4u64);
    let sum = ziskos_add(a, b);

    // 2. keccak256, checked against the reference ziskos implementation. Inputs
    // are 8-byte aligned with len % 8 == 0 (the current zisklib_keccak constraint):
    // the empty message (padding-only, one permutation) and a 144-byte message
    // (one full rate block + a final block, exercising the absorb loop).
    let empty_ok = keccak_matches(&[], &KECCAK_EMPTY);
    let words: [u64; 18] = core::array::from_fn(|i| (i as u64).wrapping_mul(0x0101_0101_0101_0101));
    let big: &[u8] = unsafe { core::slice::from_raw_parts(words.as_ptr() as *const u8, 18 * 8) };
    let big_ok = keccak_matches(big, &KECCAK_BIG144);
    // Non-multiple-of-8 length (13 bytes): exercises the byte-level final block.
    let odd: [u8; 13] = core::array::from_fn(|i| i as u8);
    let odd_ok = keccak_matches(&odd, &KECCAK_13);

    // 2b. SHA-256: empty, "abc", and 56×'a' (the two-block padding boundary).
    let a56: [u8; 56] = [b'a'; 56];
    let sha_ok = sha256_matches(&[], &SHA256_EMPTY)
        && sha256_matches(b"abc", &SHA256_ABC)
        && sha256_matches(&a56, &SHA256_56A);

    // 2c. BLAKE2b compression: one final compression of the empty message
    // reproduces blake2b512("") = 786a02f7...9be2ce, checking the state ends up
    // equal to that digest's eight little-endian words.
    let mut h = black_box([
        0x6A09_E667_F2BD_C948, // IV[0] ^ 0x01010040 (nn=64, kk=0)
        0xBB67_AE85_84CA_A73B,
        0x3C6E_F372_FE94_F82B,
        0xA54F_F53A_5F1D_36F1,
        0x510E_527F_ADE6_82D1,
        0x9B05_688C_2B3E_6C1F,
        0x1F83_D9AB_FB41_BD6B,
        0x5BE0_CD19_137E_2179,
    ]);
    blake2b_compress(12, &mut h, &black_box([0u64; 16]), &black_box([0u64; 2]), true);
    let blake2b_ok = h
        == [
            0x0359_0142_F702_6A78,
            0x72D2_5225_85FD_C6C6,
            0x6147_58E1_4047_2F91,
            0x1954_1FF7_17E2_868A,
            0x5358_EEAF_3110_5ED2,
            0x4BB0_4E93_4464_8913,
            0x55B7_4814_5B68_3A90,
            0xCEE2_9BFE_1A70_6FD5,
        ];

    // 3. inv256: an odd input is invertible (the ziskasm routine verifies
    // a*inv ≡ 1 mod 2^256 before returning); an even input is not.
    let inv_expected = [
        0xaaaa_aaaa_aaaa_aaab,
        0xaaaa_aaaa_aaaa_aaaa,
        0xaaaa_aaaa_aaaa_aaaa,
        0xaaaa_aaaa_aaaa_aaaa,
    ];
    let inv_ok = inv256(&black_box([3u64, 0, 0, 0])) == Some(inv_expected);
    let noinv_ok = inv256(&black_box([2u64, 0, 0, 0])).is_none();

    // 4. 256-bit add/sub/neg (all derived from the two overflowing cores).
    let max = [u64::MAX; 4];
    let three = black_box([3u64, 0, 0, 0]);
    let five = black_box([5u64, 0, 0, 0]);
    let addsub_ok = checked_add256(&black_box(max), &black_box([1, 0, 0, 0])).is_none()      // overflow
        && wrapping_add256(&black_box(max), &black_box([1, 0, 0, 0])) == [0, 0, 0, 0]         // wraps to 0
        && overflowing_add256(&black_box([5, 0, 0, 0]), &three) == ([8, 0, 0, 0], false)      // 5+3=8
        && checked_sub256(&three, &five).is_none()                                            // underflow
        && wrapping_sub256(&three, &five) == [0xffff_ffff_ffff_fffe, u64::MAX, u64::MAX, u64::MAX] // 3-5
        && saturating_sub256(&three, &five) == [0, 0, 0, 0]                                   // saturates to 0
        && wrapping_neg256(&black_box([1, 0, 0, 0])) == max; // -1 = 2^256-1

    // 5. 256-bit mul/square (derived from the overflowing_mul256 core).
    let p128 = black_box([0u64, 0, 1, 0]); // 2^128
    let mul_ok = checked_mul256(&p128, &p128).is_none()                                      // 2^128·2^128 = 2^256 overflows
        && wrapping_mul256(&black_box([6, 0, 0, 0]), &black_box([7, 0, 0, 0])) == [42, 0, 0, 0]
        && overflowing_mul256(&p128, &p128) == ([0, 0, 0, 0], true)                          // low bits 0, overflow
        && wrapping_square256(&black_box([u64::MAX, 0, 0, 0])) == [1, 0xffff_ffff_ffff_fffe, 0, 0] // (2^64-1)^2
        && checked_square256(&p128).is_none();

    // 6. 256-bit div/rem (hint via fcall + arith256 verify + 256-bit compare).
    let hundred = black_box([100u64, 0, 0, 0]);
    let seven = black_box([7u64, 0, 0, 0]);
    let div_ok = div_rem256(&hundred, &seven) == ([14, 0, 0, 0], [2, 0, 0, 0])               // 100 = 7·14 + 2
        && checked_div256(&hundred, &black_box([0, 0, 0, 0])).is_none()                      // ÷0 -> None
        && wrapping_rem256(&hundred, &seven) == [2, 0, 0, 0]
        && div_ceil256(&hundred, &seven) == [15, 0, 0, 0]                                    // ceil(100/7) = 15
        && div_ceil256(&black_box([42, 0, 0, 0]), &seven) == [6, 0, 0, 0]                    // exact -> 6
        && div_rem256(&black_box([u64::MAX, u64::MAX, 0, 0]), &black_box([0, 1, 0, 0]))      // (2^128-1)/2^64
            == ([u64::MAX, 0, 0, 0], [u64::MAX, 0, 0, 0]);

    // 7. 256-bit modular arithmetic (arith256_mod precompile).
    let m7 = black_box([7u64, 0, 0, 0]);
    let p128 = black_box([0u64, 0, 1, 0]); // 2^128
    let p64 = black_box([0u64, 1, 0, 0]); // 2^64
    let mod_ok = reduce_mod256(&black_box([100, 0, 0, 0]), &m7) == [2, 0, 0, 0]              // 100 mod 7
        && reduce_mod256(&black_box([3, 0, 0, 0]), &m7) == [3, 0, 0, 0]                       // already reduced
        && reduce_mod256(&black_box([5, 0, 0, 0]), &black_box([0, 0, 0, 0])) == [0, 0, 0, 0]  // modulus 0 -> 0
        && add_mod256(&black_box([6, 0, 0, 0]), &black_box([6, 0, 0, 0]), &m7) == [5, 0, 0, 0] // (6+6) mod 7
        && mul_mod256(&black_box([5, 0, 0, 0]), &black_box([4, 0, 0, 0]), &m7) == [6, 0, 0, 0] // (5·4) mod 7
        && mul_mod256(&p128, &p128, &p64) == [0, 0, 0, 0]                                      // 2^256 mod 2^64
        && square_mod256(&black_box([5, 0, 0, 0]), &m7) == [4, 0, 0, 0]; // 25 mod 7

    // 8. 256-bit modular inverse (fcall hint + verify, or gcd witness for none).
    let invmod_ok = inv_mod256(&black_box([3, 0, 0, 0]), &m7) == Some([5, 0, 0, 0])           // 3·5 ≡ 1 (mod 7)
        && inv_mod256(&black_box([3, 0, 0, 0]), &black_box([11, 0, 0, 0])) == Some([4, 0, 0, 0]) // 3·4 ≡ 1 (mod 11)
        && inv_mod256(&black_box([4, 0, 0, 0]), &black_box([8, 0, 0, 0])).is_none()            // gcd(4,8)=4
        && inv_mod256(&black_box([6, 0, 0, 0]), &black_box([9, 0, 0, 0])).is_none(); // gcd(6,9)=3

    // 9. 256-bit exponentiation: modular (pow_mod) and mod-2^256 (pow) with overflow.
    let pow_ok = pow_mod256(&black_box([2, 0, 0, 0]), &black_box([100, 0, 0, 0]), &black_box([13, 0, 0, 0])) == [3, 0, 0, 0] // 2^100 mod 13
        && pow_mod256(&black_box([2, 0, 0, 0]), &black_box([10, 0, 0, 0]), &black_box([1000, 0, 0, 0])) == [24, 0, 0, 0]     // 2^10 mod 1000
        && pow_mod256(&black_box([7, 0, 0, 0]), &black_box([3, 0, 0, 0]), &black_box([1, 0, 0, 0])) == [0, 0, 0, 0]          // mod 1 -> 0
        && checked_pow256(&black_box([2, 0, 0, 0]), &black_box([10, 0, 0, 0])) == Some([1024, 0, 0, 0])
        && overflowing_pow256(&black_box([3, 0, 0, 0]), &black_box([5, 0, 0, 0])) == ([243, 0, 0, 0], false)
        && checked_pow256(&black_box([2, 0, 0, 0]), &black_box([256, 0, 0, 0])).is_none()                                    // 2^256 overflows
        && wrapping_pow256(&black_box([2, 0, 0, 0]), &black_box([256, 0, 0, 0])) == [0, 0, 0, 0]
        && saturating_pow256(&black_box([2, 0, 0, 0]), &black_box([256, 0, 0, 0])) == [u64::MAX; 4]
        && wrapping_pow256(&black_box([2, 0, 0, 0]), &black_box([255, 0, 0, 0])) == [0, 0, 0, 0x8000_0000_0000_0000]; // 2^255

    // 10. secp256k1 ECDSA: the valid signature verifies; flipping the low bit of
    // r must make it fail.
    let mut k1_rbad = black_box(K1_R);
    k1_rbad[0] ^= 1;
    let k1_ecdsa_ok = secp256k1_ecdsa_verify(
        &black_box(K1_PK),
        &black_box(K1_Z),
        &black_box(K1_R),
        &black_box(K1_S),
    ) && !secp256k1_ecdsa_verify(
        &black_box(K1_PK),
        &black_box(K1_Z),
        &k1_rbad,
        &black_box(K1_S),
    );

    // 11. secp256k1 public-key recovery: recovering from (r, s, z, recid) yields
    // exactly the signing public key; a wrong recid must not.
    let k1_recover_ok =
        secp256k1_ecdsa_recover(&black_box(K1_R), &black_box(K1_S), &black_box(K1_Z), K1_RECID)
            == Ok(K1_PK)
            && secp256k1_ecdsa_recover(
                &black_box(K1_R),
                &black_box(K1_S),
                &black_box(K1_Z),
                K1_RECID ^ 1,
            ) != Ok(K1_PK);

    // 12. secp256k1 BIP-340 Schnorr: the signature over msg = 0..31 verifies; the
    // same signature over a tampered message must fail.
    let sch_msg: [u8; 32] = core::array::from_fn(|i| i as u8);
    let mut sch_bad = sch_msg;
    sch_bad[0] ^= 1;
    let k1_schnorr_ok = secp256k1_schnorr_verify(
        &black_box(SCH_PKX),
        &black_box(SCH_R),
        &black_box(SCH_S),
        &sch_msg,
    ) && !secp256k1_schnorr_verify(
        &black_box(SCH_PKX),
        &black_box(SCH_R),
        &black_box(SCH_S),
        &sch_bad,
    );

    // 13. secp256r1 (NIST P-256) ECDSA: valid verifies, tampered r fails.
    let mut r1_rbad = black_box(R1_R);
    r1_rbad[0] ^= 1;
    let r1_ecdsa_ok = secp256r1_ecdsa_verify(
        &black_box(R1_PK),
        &black_box(R1_Z),
        &black_box(R1_R),
        &black_box(R1_S),
    ) && !secp256r1_ecdsa_verify(
        &black_box(R1_PK),
        &black_box(R1_Z),
        &r1_rbad,
        &black_box(R1_S),
    );

    // 14. BN254 ecPairing: e(G1,G2)·e(-G1,G2) == 1 accepts (status 0); the single
    // pair e(G1,G2) != 1 rejects (status 1).
    let bn_pair_ok = bn254_pairing_check(&BN_PC_G1, &BN_PC_G2) == 0
        && bn254_pairing_check(&BN_PC_G1[..1], &BN_PC_G2[..1]) == 1;

    // 15. BLS12-381 pairing (EIP-2537): e(G1,G2)·e(-G1,G2) == 1 accepts (status 0);
    // the single pair e(G1,G2) != 1 rejects (status 1).
    let bls_pair_ok = bls12_381_pairing_check(&BLS_PC_G1, &BLS_PC_G2) == 0
        && bls12_381_pairing_check(&BLS_PC_G1[..1], &BLS_PC_G2[..1]) == 1;

    // 16. BLS12-381 map-to-curve (EIP-2537 MAP_FP_TO_G1 / MAP_FP2_TO_G2): mapped
    // points match the off-chain reference.
    let bls_map_ok = bls12_381_map_to_curve_g1(&black_box(BLS_MAP_U1)) == Ok(BLS_MAP_G1)
        && bls12_381_map_to_curve_g2(&black_box(BLS_MAP_U2)) == Ok(BLS_MAP_G2);

    // 17. BLS12-381 hash-to-curve (RFC 9380): matches the off-chain reference.
    let bls_htc_ok = bls12_381_hash_to_curve_g2(&BLS_HTC_MSG, &BLS_HTC_DST) == BLS_HTC_EXP;

    // 18. BLS signature verify: a valid signature verifies; a tampered one fails.
    let mut bad_sig = BLS_SIG_SIG;
    bad_sig[95] ^= 1;
    let bls_sig_ok = bls12_381_verify(&BLS_SIG_PK, &BLS_SIG_MSG, &BLS_SIG_SIG)
        && !bls12_381_verify(&BLS_SIG_PK, &BLS_SIG_MSG, &bad_sig);

    // 19. KZG proof verify (EIP-4844): a valid evaluation proof verifies; a wrong
    // claimed value fails.
    let mut bad_y = BLS_KZG_Y;
    bad_y[31] ^= 1;
    let bls_kzg_ok =
        bls12_381_verify_kzg_proof(&BLS_KZG_Z, &BLS_KZG_Y, &BLS_KZG_COMM, &BLS_KZG_PROOF)
            && !bls12_381_verify_kzg_proof(&BLS_KZG_Z, &bad_y, &BLS_KZG_COMM, &BLS_KZG_PROOF);

    // 20. EIP-198 modexp: short (256-bit modulus) and long (512-bit modulus).
    let mut me0 = [0u64; 4];
    let me0_n = modexp_u64(&MODEXP_B0, &MODEXP_E0, &MODEXP_M0, &mut me0);
    let mut me1 = [0u64; 8];
    let me1_n = modexp_u64(&MODEXP_B1, &MODEXP_E1, &MODEXP_M1, &mut me1);
    let modexp_ok = me0_n == 4 && me0 == MODEXP_R0 && me1_n == 8 && me1 == MODEXP_R1;

    let ok = sum == 7
        && empty_ok
        && big_ok
        && odd_ok
        && sha_ok
        && blake2b_ok
        && inv_ok
        && noinv_ok
        && addsub_ok
        && mul_ok
        && div_ok
        && mod_ok
        && invmod_ok
        && pow_ok
        && k1_ecdsa_ok
        && k1_recover_ok
        && k1_schnorr_ok
        && r1_ecdsa_ok
        && bn_pair_ok;
    ziskos::io::commit(&ok);
    println!(
        "add=0x{sum:x} keccak(empty)={empty_ok} keccak(144B)={big_ok} keccak(13B)={odd_ok} sha256={sha_ok} blake2b={blake2b_ok} inv256={inv_ok} noinv={noinv_ok} addsub={addsub_ok} mul={mul_ok} div={div_ok} mod={mod_ok} invmod={invmod_ok} pow={pow_ok} k1_ecdsa={k1_ecdsa_ok} k1_recover={k1_recover_ok} k1_schnorr={k1_schnorr_ok} r1_ecdsa={r1_ecdsa_ok} bn254_pairing={bn_pair_ok} bls_pairing={bls_pair_ok} bls_map={bls_map_ok} bls_htc={bls_htc_ok} bls_sig={bls_sig_ok} bls_kzg={bls_kzg_ok} modexp={modexp_ok} => ok={ok}"
    );
}
