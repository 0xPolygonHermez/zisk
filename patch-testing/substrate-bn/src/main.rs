//! zisk-patch-bn / substrate-bn: BN254 curve arithmetic, subgroup checks and
//! pairings routed to the bn254 zisklib functions.

#![no_main]
ziskos::entrypoint!(main);

use patch_test_common::{hex, to_hex, Suite};
use substrate_bn::{pairing, pairing_batch, AffineG1, AffineG2, Fq, Fq2, Fr, Group, Gt, G1, G2};

fn g1_affine_hex(p: G1) -> String {
    let a = AffineG1::from_jacobian(p).expect("point at infinity");
    let mut buf = [0u8; 64];
    a.x().to_big_endian(&mut buf[..32]).unwrap();
    a.y().to_big_endian(&mut buf[32..]).unwrap();
    to_hex(&buf)
}

fn main() {
    let mut t = Suite::new("substrate-bn");

    let g1 = G1::one();
    let g2 = G2::one();
    let a = Fr::from_str("123456789012345678901234567890").unwrap();
    let b = Fr::from_str("98765432109876543210987654321").unwrap();

    // --- G1 ------------------------------------------------------------------
    // 2·G1, from the EIP-196 ecadd/ecmul test vectors.
    t.eq(
        "2*G1",
        g1_affine_hex(g1 * Fr::from_str("2").unwrap()),
        "030644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd3\
         15ed738c0e0a7c92e7845f96b2ae9c0a68a6a449e3538fc7ff3ebf7a5a18a2c4"
            .to_string(),
    );
    t.check("G1 + G1 == 2*G1", g1 + g1 == g1 * Fr::from_str("2").unwrap());
    t.check("(r-1)*G1 + G1 == O", (g1 * (-Fr::one()) + g1).is_zero());
    t.check("(a+b)*G1 == a*G1 + b*G1", g1 * (a + b) == g1 * a + g1 * b);
    t.check(
        "AffineG1 accepts (1, 2)",
        AffineG1::new(Fq::from_str("1").unwrap(), Fq::from_str("2").unwrap()).is_ok(),
    );
    t.check(
        "AffineG1 rejects (1, 3)",
        AffineG1::new(Fq::from_str("1").unwrap(), Fq::from_str("3").unwrap()).is_err(),
    );
    t.check(
        "Fq::from_slice rejects p",
        Fq::from_slice(&hex("30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47")).is_err(),
    );

    // --- G2 (twist): on-curve and subgroup checks ---------------------------
    t.check("(a+b)*G2 == a*G2 + b*G2", g2 * (a + b) == g2 * a + g2 * b);
    t.check("(r-1)*G2 + G2 == O", (g2 * (-Fr::one()) + g2).is_zero());
    let g2_aff = AffineG2::from_jacobian(g2).unwrap();
    t.check("AffineG2 accepts generator", AffineG2::new(g2_aff.x(), g2_aff.y()).is_ok());
    t.check("AffineG2 rejects off-curve point", AffineG2::new(g2_aff.x(), g2_aff.y() + Fq2::one()).is_err());

    // --- Pairing ----------------------------------------------------------
    let e = pairing(g1, g2);
    t.check("e(G1, G2) != 1", e != Gt::one());
    t.check("e(a*G1, b*G2) == e(G1, G2)^(ab)", pairing(g1 * a, g2 * b) == e.pow(a * b));
    t.check("e(a*G1, G2) == e(G1, a*G2)", pairing(g1 * a, g2) == pairing(g1, g2 * a));
    t.check("e(P, Q) * e(-P, Q) == 1", pairing_batch(&[(g1, g2), (-g1, g2)]) == Gt::one());
    t.check(
        "e(aP, bQ) * e(-abP, Q) == 1",
        pairing_batch(&[(g1 * a, g2 * b), (-(g1 * (a * b)), g2)]) == Gt::one(),
    );
    t.check("e(aP, bQ) * e(-aP, Q) != 1", pairing_batch(&[(g1 * a, g2 * b), (-(g1 * a), g2)]) != Gt::one());
    t.check("pairing_batch skips infinity", pairing_batch(&[(G1::zero(), g2), (g1, G2::zero())]) == Gt::one());
    t.check("pairing_batch(&[]) == 1", pairing_batch(&[]) == Gt::one());
    t.check("e(G1, G2)^r == 1", e.pow(-Fr::one()) * e == Gt::one());

    // Products other than 1 and invalid inputs must still return the exact value (the ZisK
    // patch falls back to software there); compare against the separate `pairing` path.
    t.check("pairing_batch(&[(aP, Q)]) == pairing(aP, Q)", pairing_batch(&[(g1 * a, g2)]) == pairing(g1 * a, g2));
    t.check(
        "pairing_batch(&[(P, Q), (bP, aQ)]) == e(P, Q) * e(bP, aQ)",
        pairing_batch(&[(g1, g2), (g1 * b, g2 * a)]) == pairing(g1, g2) * pairing(g1 * b, g2 * a),
    );
    let off_curve = G2::new(g2.x(), g2.y() + Fq2::one(), g2.z());
    t.check("pairing_batch with off-curve G2 == pairing", pairing_batch(&[(g1, off_curve)]) == pairing(g1, off_curve));

    t.finish();
}
