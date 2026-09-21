// BabyJubJub (twisted Edwards) point addition, compatible with iden3/circom (circomlib).
//
// Curve: a·x² + y² = 1 + d·x²·y² over the BN254 scalar field Fr, with
//   a = 168700, d = 168696.
// The addition law is complete (the same formula doubles a point):
//   τ  = x1·x2·y1·y2
//   x3 = (x1·y2 + y1·x2) / (1 + d·τ)
//   y3 = (y1·y2 − a·x1·x2) / (1 − d·τ)
//
// Points are affine (x, y); each coordinate is a 256-bit little-endian limb array `[u64; 4]`,
// and a point is `[u64; 8]` = x ‖ y.

use ark_bn254::Fr as BabyJubJubField;
use ark_ff::{BigInt, PrimeField};

#[inline(always)]
fn babyjubjub_a() -> BabyJubJubField {
    BabyJubJubField::from(168700u64)
}

#[inline(always)]
fn babyjubjub_d() -> BabyJubJubField {
    BabyJubJubField::from(168696u64)
}

#[inline(always)]
pub fn babyjubjub_add(p1: &[u64; 8], p2: &[u64; 8], p: &mut [u64; 8]) {
    let x1 = BabyJubJubField::from(BigInt::<4>(p1[0..4].try_into().unwrap()));
    let y1 = BabyJubJubField::from(BigInt::<4>(p1[4..8].try_into().unwrap()));
    let x2 = BabyJubJubField::from(BigInt::<4>(p2[0..4].try_into().unwrap()));
    let y2 = BabyJubJubField::from(BigInt::<4>(p2[4..8].try_into().unwrap()));

    let a = babyjubjub_a();
    let d = babyjubjub_d();

    let tau = x1 * x2 * y1 * y2;
    let x3 = (x1 * y2 + y1 * x2) / (BabyJubJubField::from(1u64) + d * tau);
    let y3 = (y1 * y2 - a * x1 * x2) / (BabyJubJubField::from(1u64) - d * tau);

    p[..4].copy_from_slice(&x3.into_bigint().0);
    p[4..].copy_from_slice(&y3.into_bigint().0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::str::FromStr;

    fn fr_limbs(decimal: &str) -> [u64; 4] {
        BabyJubJubField::from_str(decimal).expect("invalid field element").into_bigint().0
    }

    fn point(x: &str, y: &str) -> [u64; 8] {
        let mut p = [0u64; 8];
        p[0..4].copy_from_slice(&fr_limbs(x));
        p[4..8].copy_from_slice(&fr_limbs(y));
        p
    }

    fn on_curve(p: &[u64; 8]) -> bool {
        let x = BabyJubJubField::from(BigInt::<4>(p[0..4].try_into().unwrap()));
        let y = BabyJubJubField::from(BigInt::<4>(p[4..8].try_into().unwrap()));
        let one = BabyJubJubField::from(1u64);
        babyjubjub_a() * x * x + y * y == one + babyjubjub_d() * x * x * y * y
    }

    // Ground-truth vector from circomlib test/babyjub.js ("Should add two points").
    #[test]
    fn add_matches_circomlib_vector() {
        let p1 = point(
            "17777552123799933955779906779655732241715742912184938656739573121738514868268",
            "2626589144620713026669568689430873010625803728049924121243784502389097019475",
        );
        let p2 = point(
            "16540640123574156134436876038791482806971768689494387082833631921987005038935",
            "20819045374670962167435360035096875258406992893633759881276124905556507972311",
        );
        let expected = point(
            "7916061937171219682591368294088513039687205273691143098332585753343424131937",
            "14035240266687799601661095864649209771790948434046947201833777492504781204499",
        );
        let mut out = [0u64; 8];
        babyjubjub_add(&p1, &p2, &mut out);
        assert_eq!(out, expected);
    }

    // (0, 1) is the neutral element of the twisted-Edwards group.
    #[test]
    fn identity_is_neutral() {
        let id = point("0", "1");
        let p = point(
            "17777552123799933955779906779655732241715742912184938656739573121738514868268",
            "2626589144620713026669568689430873010625803728049924121243784502389097019475",
        );
        let mut out = [0u64; 8];
        babyjubjub_add(&p, &id, &mut out);
        assert_eq!(out, p);

        let mut id_dbl = [0u64; 8];
        babyjubjub_add(&id, &id, &mut id_dbl);
        assert_eq!(id_dbl, id);
    }

    // The complete addition law doubles a point when p1 == p2; result must stay on-curve.
    #[test]
    fn doubling_stays_on_curve() {
        let p = point(
            "17777552123799933955779906779655732241715742912184938656739573121738514868268",
            "2626589144620713026669568689430873010625803728049924121243784502389097019475",
        );
        assert!(on_curve(&p));
        let mut dbl = [0u64; 8];
        babyjubjub_add(&p, &p, &mut dbl);
        assert!(on_curve(&dbl));
    }
}
