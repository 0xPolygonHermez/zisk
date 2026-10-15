// Ed25519 (edwards25519, RFC 8032) twisted-Edwards point addition.
//
// Curve: a·x² + y² = 1 + d·x²·y² over Fp, p = 2^255 − 19, with
//   a = −1, d = −121665/121666 mod p.
// The addition law is complete (the same formula doubles a point):
//   τ  = x1·x2·y1·y2
//   x3 = (x1·y2 + y1·x2) / (1 + d·τ)
//   y3 = (y1·y2 + x1·x2) / (1 − d·τ)
//
// Points are affine (x, y); each coordinate is a 256-bit little-endian limb array `[u64; 4]`,
// and a point is `[u64; 8]` = x ‖ y. Input coordinates are reduced mod p (the AIR accepts any
// 256-bit input), the output is always canonical.

use ark_ff::fields::{Fp256, MontBackend, MontConfig};
use ark_ff::{BigInt, PrimeField};

#[derive(MontConfig)]
#[modulus = "57896044618658097711785492504343953926634992332820282019728792003956564819949"]
#[generator = "2"]
pub struct Ed25519FieldConfig;

/// Base field of Ed25519, Fp = 2^255 − 19.
pub type Ed25519Field = Fp256<MontBackend<Ed25519FieldConfig, 4>>;

/// d = −121665/121666 mod p
const ED25519_D: BigInt<4> =
    BigInt([0x75EB4DCA135978A3, 0x00700A4D4141D8AB, 0x8CC740797779E898, 0x52036CEE2B6FFE73]);

#[inline(always)]
pub fn ed25519_d() -> Ed25519Field {
    Ed25519Field::from_bigint(ED25519_D).unwrap()
}

/// Converts 4 little-endian limbs to a field element, reducing mod p.
#[inline(always)]
pub fn ed25519_field_from_limbs(limbs: &[u64]) -> Ed25519Field {
    let mut bytes = [0u8; 32];
    for (chunk, limb) in bytes.chunks_mut(8).zip(limbs) {
        chunk.copy_from_slice(&limb.to_le_bytes());
    }
    Ed25519Field::from_le_bytes_mod_order(&bytes)
}

#[inline(always)]
pub fn ed25519_add(p1: &[u64; 8], p2: &[u64; 8], p: &mut [u64; 8]) {
    let x1 = ed25519_field_from_limbs(&p1[0..4]);
    let y1 = ed25519_field_from_limbs(&p1[4..8]);
    let x2 = ed25519_field_from_limbs(&p2[0..4]);
    let y2 = ed25519_field_from_limbs(&p2[4..8]);

    let one = Ed25519Field::from(1u64);
    let dtau = ed25519_d() * x1 * x2 * y1 * y2;
    let x3 = (x1 * y2 + y1 * x2) / (one + dtau);
    let y3 = (y1 * y2 + x1 * x2) / (one - dtau);

    p[..4].copy_from_slice(&x3.into_bigint().0);
    p[4..].copy_from_slice(&y3.into_bigint().0);
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 8032 base point B and its multiples (computed independently, little-endian limbs).
    const B: [u64; 8] = [
        14507833142362363162,
        7578651490590762930,
        13881468655802702940,
        2407515759118799870,
        7378697629483820632,
        7378697629483820646,
        7378697629483820646,
        7378697629483820646,
    ];
    const B2: [u64; 8] = [
        9495172746435612174,
        580276354371003487,
        4396704544683452332,
        3939304200807777388,
        1035624154251437001,
        10896757615647805782,
        2135373527235305302,
        2477206238767819202,
    ];
    const B3: [u64; 8] = [
        12421570298656645724,
        7152727868506785926,
        2302657123787130938,
        7471080674075774793,
        199117469010736340,
        11463609486776992516,
        11612146322091974391,
        1326224128502294955,
    ];
    const IDENTITY: [u64; 8] = [0, 0, 0, 0, 1, 0, 0, 0];

    fn add(p1: &[u64; 8], p2: &[u64; 8]) -> [u64; 8] {
        let mut out = [0u64; 8];
        ed25519_add(p1, p2, &mut out);
        out
    }

    fn on_curve(p: &[u64; 8]) -> bool {
        let x = ed25519_field_from_limbs(&p[0..4]);
        let y = ed25519_field_from_limbs(&p[4..8]);
        let one = Ed25519Field::from(1u64);
        -(x * x) + y * y == one + ed25519_d() * x * x * y * y
    }

    #[test]
    fn d_matches_definition() {
        let d = -Ed25519Field::from(121665u64) / Ed25519Field::from(121666u64);
        assert_eq!(d, ed25519_d());
    }

    #[test]
    fn base_point_multiples() {
        assert!(on_curve(&B));
        assert_eq!(add(&B, &B), B2);
        assert_eq!(add(&B2, &B), B3);
        assert_eq!(add(&B, &B2), B3);
    }

    #[test]
    fn identity_and_inverse() {
        assert_eq!(add(&B, &IDENTITY), B);
        assert_eq!(add(&IDENTITY, &IDENTITY), IDENTITY);
        // -B = (p - x, y)
        let mut neg_b = B;
        neg_b[0..4].copy_from_slice(&[
            3938910931347188435,
            10868092583118788685,
            4565275417906848675,
            6815856277735975937,
        ]);
        assert_eq!(add(&B, &neg_b), IDENTITY);
    }

    #[test]
    fn non_canonical_inputs_are_reduced() {
        // x + p still fits in 256 bits; the result must equal the canonical one.
        let p = Ed25519FieldConfig::MODULUS;
        let mut b_alias = B;
        let mut carry = false;
        for (limb, p_limb) in b_alias[0..4].iter_mut().zip(p.0) {
            let (s1, c1) = limb.overflowing_add(p_limb);
            let (s2, c2) = s1.overflowing_add(carry as u64);
            *limb = s2;
            carry = c1 || c2;
        }
        assert!(!carry);
        assert_eq!(add(&b_alias, &B), B2);
    }
}
