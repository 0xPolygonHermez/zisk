//! zisk-patch-blst: the blst C library replaced by Rust bindings over the
//! bls12_381 zisklib functions (curve/twist ops, subgroup checks, pairing).

#![no_main]
ziskos::entrypoint!(main);

use blst::{
    blst_fp12, blst_hash_to_g1, blst_p1, blst_p1_add_or_double, blst_p1_affine, blst_p1_affine_in_g1,
    blst_p1_compress, blst_p1_double, blst_p1_from_affine, blst_p1_generator, blst_p1_is_equal,
    blst_p1_is_inf, blst_p1_mult, blst_p1_serialize, blst_p1_to_affine, blst_p1_uncompress, blst_p2,
    blst_p2_affine, blst_p2_compress, blst_p2_generator, blst_p2_mult, blst_p2_to_affine,
    min_pk::{AggregateSignature, PublicKey, SecretKey, Signature},
    BLST_ERROR,
};
use patch_test_common::{hex_array, Suite};

const ETH_DST: &[u8] = b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_POP_";

/// Group order r, little-endian, as blst scalars are.
fn order_le() -> [u8; 32] {
    let mut r = hex_array::<32>("73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001");
    r.reverse();
    r
}

fn le_scalar(v: u64) -> [u8; 32] {
    let mut s = [0u8; 32];
    s[..8].copy_from_slice(&v.to_le_bytes());
    s
}

fn p1_mul(p: &blst_p1, k: &[u8; 32]) -> blst_p1 {
    let mut out = blst_p1::default();
    unsafe { blst_p1_mult(&mut out, p, k.as_ptr(), 256) };
    out
}

fn p2_mul(p: &blst_p2, k: &[u8; 32]) -> blst_p2 {
    let mut out = blst_p2::default();
    unsafe { blst_p2_mult(&mut out, p, k.as_ptr(), 256) };
    out
}

fn p1_aff(p: &blst_p1) -> blst_p1_affine {
    let mut a = blst_p1_affine::default();
    unsafe { blst_p1_to_affine(&mut a, p) };
    a
}

fn p2_aff(p: &blst_p2) -> blst_p2_affine {
    let mut a = blst_p2_affine::default();
    unsafe { blst_p2_to_affine(&mut a, p) };
    a
}

fn pair(p: &blst_p1, q: &blst_p2) -> blst_fp12 {
    blst_fp12::miller_loop(&p2_aff(q), &p1_aff(p)).final_exp()
}

fn main() {
    let mut t = Suite::new("blst");

    let g1 = unsafe { *blst_p1_generator() };
    let g2 = unsafe { *blst_p2_generator() };

    // --- Encodings ------------------------------------------------------------
    let mut c1 = [0u8; 48];
    unsafe { blst_p1_compress(c1.as_mut_ptr(), &g1) };
    t.eq_hex(
        "G1 compressed",
        &c1,
        "97f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb",
    );
    let mut c2 = [0u8; 96];
    unsafe { blst_p2_compress(c2.as_mut_ptr(), &g2) };
    t.eq_hex(
        "G2 compressed",
        &c2,
        "93e02b6052719f607dacd3a088274f65596bd0d09920b61ab5da61bbdc7f5049334cf11213945d57e5ac7d055d042b7e\
         024aa2b2f08f0a91260805272dc51051c6e47ad4fa403b02b4510b647ae3d1770bac0326a805bbefd48056c8c121bdb8",
    );
    let mut g1_back = blst_p1_affine::default();
    t.check("uncompress(G1)", unsafe { blst_p1_uncompress(&mut g1_back, c1.as_ptr()) } == BLST_ERROR::BLST_SUCCESS);
    let mut g1_proj = blst_p1::default();
    unsafe { blst_p1_from_affine(&mut g1_proj, &g1_back) };
    t.check("uncompress(compress(G1)) == G1", unsafe { blst_p1_is_equal(&g1_proj, &g1) });

    // --- Hash to curve: RFC 9380, BLS12381G1_XMD:SHA-256_SSWU_RO_, msg = "" ------
    let dst = b"QUUX-V01-CS02-with-BLS12381G1_XMD:SHA-256_SSWU_RO_";
    let mut h = blst_p1::default();
    unsafe { blst_hash_to_g1(&mut h, [].as_ptr(), 0, dst.as_ptr(), dst.len(), [].as_ptr(), 0) };
    let mut ser = [0u8; 96];
    unsafe { blst_p1_serialize(ser.as_mut_ptr(), &h) };
    t.eq_hex(
        "hash_to_g1('')",
        &ser,
        "052926add2207b76ca4fa57a8734416c8dc95e24501772c814278700eed6d1e4e8cf62d9c09db0fac349612b759e79a1\
         08ba738453bfed09cb546dbb0783dbb3a5f1f566ed67bb6be0e8c67e2e81a4cc68ee29813bb7994998f3eae0c9c6a265",
    );

    // --- Group law ------------------------------------------------------------
    let mut sum = blst_p1::default();
    unsafe { blst_p1_add_or_double(&mut sum, &g1, &g1) };
    let mut dbl = blst_p1::default();
    unsafe { blst_p1_double(&mut dbl, &g1) };
    t.check("G1 + G1 == 2*G1 (double)", unsafe { blst_p1_is_equal(&sum, &dbl) });
    t.check("G1 + G1 == 2*G1 (mult)", unsafe { blst_p1_is_equal(&sum, &p1_mul(&g1, &le_scalar(2))) });
    t.check("r*G1 == O", unsafe { blst_p1_is_inf(&p1_mul(&g1, &order_le())) });
    t.check("G1 in G1", unsafe { blst_p1_affine_in_g1(&p1_aff(&g1)) });
    // (0, 2) is on E(Fp) but outside the prime-order subgroup.
    let mut off = [0u8; 48];
    off[0] = 0x80;
    let mut off_aff = blst_p1_affine::default();
    let decoded = unsafe { blst_p1_uncompress(&mut off_aff, off.as_ptr()) };
    t.check(
        "(0, 2) rejected or outside G1",
        decoded != BLST_ERROR::BLST_SUCCESS || !unsafe { blst_p1_affine_in_g1(&off_aff) },
    );
    t.check("key_validate rejects (0, 2)", PublicKey::key_validate(&off).is_err());

    // --- Pairing ------------------------------------------------------------
    let a = le_scalar(0x1234_5678_9abc_def1);
    let b = le_scalar(0x0fed_cba9_8765_4321);
    let e = pair(&g1, &g2);
    t.check("e(G1, G2) in GT", e.in_group());
    t.check("e(a*G1, G2) == e(G1, a*G2)", pair(&p1_mul(&g1, &a), &g2) == pair(&g1, &p2_mul(&g2, &a)));
    t.check("e(a*G1, b*G2) == e(b*G1, a*G2)", pair(&p1_mul(&g1, &a), &p2_mul(&g2, &b)) == pair(&p1_mul(&g1, &b), &p2_mul(&g2, &a)));
    t.check("e(a*G1, G2) != e(b*G1, G2)", pair(&p1_mul(&g1, &a), &g2) != pair(&p1_mul(&g1, &b), &g2));

    // --- BLS signatures (min_pk, Ethereum DST) ------------------------------
    let sk = SecretKey::key_gen(&[1u8; 32], &[]).unwrap();
    let pk = sk.sk_to_pk();
    t.eq_hex(
        "pk(key_gen([1; 32]))",
        &pk.compress(),
        "95a254501b7733239ed3cec4d56737977bd09ede881d8a234560e83e5525017add3b1dcc3eabfb85e12a4131b19c253b",
    );
    let sig = sk.sign(b"zisk patch testing", ETH_DST, &[]);
    t.eq_hex(
        "sign(...)",
        &sig.compress(),
        "846701edce42d0098e4cb97f91e5d5aaba30e1553d805798f6d03b55251edfcb2300059829f299ab3ffea556571f9035198b04cb919e508c8ead60cf50cd63923e4746e982db92f8a3c48e62f420b8704dffa55df85bad2fd292a2a95921eb78",
    );
    t.eq("verify valid", sig.verify(true, b"zisk patch testing", ETH_DST, &[], &pk, true), BLST_ERROR::BLST_SUCCESS);
    t.check("reject wrong message", sig.verify(true, b"zisk patch testinG", ETH_DST, &[], &pk, true) != BLST_ERROR::BLST_SUCCESS);
    t.check(
        "signature round-trips",
        Signature::uncompress(&sig.compress()).map(|s| s == sig).unwrap_or(false),
    );
    t.check(
        "public key round-trips",
        PublicKey::uncompress(&pk.compress()).map(|p| p == pk).unwrap_or(false),
    );

    let sks: Vec<SecretKey> = (2u8..5).map(|i| SecretKey::key_gen(&[i; 32], &[]).unwrap()).collect();
    let pks: Vec<PublicKey> = sks.iter().map(|s| s.sk_to_pk()).collect();
    let msg = b"same message";
    let sigs: Vec<Signature> = sks.iter().map(|s| s.sign(msg, ETH_DST, &[])).collect();
    let agg = AggregateSignature::aggregate(&sigs.iter().collect::<Vec<_>>(), true).unwrap().to_signature();
    t.eq(
        "fast_aggregate_verify",
        agg.fast_aggregate_verify(true, msg, ETH_DST, &pks.iter().collect::<Vec<_>>()),
        BLST_ERROR::BLST_SUCCESS,
    );
    t.check(
        "fast_aggregate_verify rejects missing key",
        agg.fast_aggregate_verify(true, msg, ETH_DST, &pks[..2].iter().collect::<Vec<_>>()) != BLST_ERROR::BLST_SUCCESS,
    );
    let msgs: [&[u8]; 3] = [b"m0", b"m1", b"m2"];
    let sigs: Vec<Signature> = sks.iter().zip(msgs).map(|(s, m)| s.sign(m, ETH_DST, &[])).collect();
    let agg = AggregateSignature::aggregate(&sigs.iter().collect::<Vec<_>>(), true).unwrap().to_signature();
    t.eq(
        "aggregate_verify",
        agg.aggregate_verify(true, &msgs, ETH_DST, &pks.iter().collect::<Vec<_>>(), true),
        BLST_ERROR::BLST_SUCCESS,
    );

    t.finish();
}
