use super::BabyJubJubData;
use crate::babyjubjub_constants::{BABYJUBJUB_A, BABYJUBJUB_D, BABYJUBJUB_PRIME};
use crate::equations;
use ark_bn254::Fr as BabyJubJubField;
use ark_ff::{BigInt as ArkBigInt, PrimeField};
use lazy_static::lazy_static;
use num_bigint::BigInt;
use num_traits::Zero;
use precompiles_helpers::{bigint2_to_8_u64, bigint_to_16_chunks};

const COLS: u8 = 32;

lazy_static! {
    pub static ref BABYJUBJUB_PRIME_BI: BigInt =
        BigInt::parse_bytes(BABYJUBJUB_PRIME.as_bytes(), 16).unwrap();
    pub static ref BABYJUBJUB_A_BI: BigInt = BigInt::from(BABYJUBJUB_A);
    pub static ref BABYJUBJUB_D_BI: BigInt = BigInt::from(BABYJUBJUB_D);
    /// Offset added to keep every plus-convention quotient non-negative.
    pub static ref OFFSET_SMALL: BigInt = BigInt::from(0x10);
    /// Offset for the minus-convention y3 quotient.
    pub static ref OFFSET_Y3: BigInt = BigInt::from(0x40000);
}

/// Convert a reduced `Fr` element to a `num_bigint::BigInt` in `[0, p)`.
fn fr_to_bigint(f: &BabyJubJubField) -> BigInt {
    let mut r = BigInt::zero();
    for &w in f.into_bigint().0.iter().rev() {
        r <<= 64;
        r += BigInt::from(w);
    }
    r
}

fn point_from_8x64(p: &[u64; 8]) -> (BabyJubJubField, BabyJubJubField) {
    (
        BabyJubJubField::from(ArkBigInt::<4>(p[0..4].try_into().unwrap())),
        BabyJubJubField::from(ArkBigInt::<4>(p[4..8].try_into().unwrap())),
    )
}

pub struct BabyJubJub {}

impl BabyJubJub {
    #[allow(dead_code)]
    pub fn calculate_add(p1: &[u64; 8], p2: &[u64; 8], p3: &mut [u64; 8]) {
        Self::prepare(p1, p2, Some(p3));
    }

    fn prepare(p1: &[u64; 8], p2: &[u64; 8], p3: Option<&mut [u64; 8]>) -> Option<BabyJubJubData> {
        let (x1f, y1f) = point_from_8x64(p1);
        let (x2f, y2f) = point_from_8x64(p2);

        let a = BabyJubJubField::from(BABYJUBJUB_A);
        let d = BabyJubJubField::from(BABYJUBJUB_D);
        let one = BabyJubJubField::from(1u64);

        let af = x1f * x2f; // A  = x1*x2
        let bf = y1f * y2f; // B  = y1*y2
        let nf = x1f * y2f + y1f * x2f; // N  = x1*y2 + y1*x2
        let tf = af * bf; // T  = A*B = tau
        let dtf = d * tf; // DT = d*T
        let x3f = nf / (one + dtf);
        let y3f = (bf - a * af) / (one - dtf);

        let x1 = fr_to_bigint(&x1f);
        let y1 = fr_to_bigint(&y1f);
        let x2 = fr_to_bigint(&x2f);
        let y2 = fr_to_bigint(&y2f);
        let av = fr_to_bigint(&af);
        let bv = fr_to_bigint(&bf);
        let nv = fr_to_bigint(&nf);
        let tv = fr_to_bigint(&tf);
        let dtv = fr_to_bigint(&dtf);
        let x3 = fr_to_bigint(&x3f);
        let y3 = fr_to_bigint(&y3f);

        let p = &*BABYJUBJUB_PRIME_BI;

        let _qa: BigInt = &x1 * &x2 - &av;
        assert!((&_qa % p).is_zero());
        let qa = (&_qa / p) + &*OFFSET_SMALL;

        let _qb: BigInt = &y1 * &y2 - &bv;
        assert!((&_qb % p).is_zero());
        let qb = (&_qb / p) + &*OFFSET_SMALL;

        let _qn: BigInt = &x1 * &y2 + &y1 * &x2 - &nv;
        assert!((&_qn % p).is_zero());
        let qn = (&_qn / p) + &*OFFSET_SMALL;

        let _qt: BigInt = &av * &bv - &tv;
        assert!((&_qt % p).is_zero());
        let qt = (&_qt / p) + &*OFFSET_SMALL;

        let _qdt: BigInt = &*BABYJUBJUB_D_BI * &tv - &dtv;
        assert!((&_qdt % p).is_zero());
        let qdt = (&_qdt / p) + &*OFFSET_SMALL;

        let _qx: BigInt = &x3 + &x3 * &dtv - &nv;
        assert!((&_qx % p).is_zero());
        let qx = (&_qx / p) + &*OFFSET_SMALL;

        let _qy: BigInt = &y3 - &y3 * &dtv - &bv + &*BABYJUBJUB_A_BI * &av;
        assert!((&_qy % p).is_zero());
        let qy = &*OFFSET_Y3 - (&_qy / p);

        if let Some(p3) = p3 {
            bigint2_to_8_u64(&x3, &y3, p3);
            return None;
        }

        let mut data = BabyJubJubData::default();
        bigint_to_16_chunks(&x1, &mut data.x1);
        bigint_to_16_chunks(&y1, &mut data.y1);
        bigint_to_16_chunks(&x2, &mut data.x2);
        bigint_to_16_chunks(&y2, &mut data.y2);
        bigint_to_16_chunks(&x3, &mut data.x3);
        bigint_to_16_chunks(&y3, &mut data.y3);
        bigint_to_16_chunks(&av, &mut data.a);
        bigint_to_16_chunks(&bv, &mut data.b);
        bigint_to_16_chunks(&nv, &mut data.n);
        bigint_to_16_chunks(&tv, &mut data.t);
        bigint_to_16_chunks(&dtv, &mut data.dt);
        bigint_to_16_chunks(&qa, &mut data.qa);
        bigint_to_16_chunks(&qb, &mut data.qb);
        bigint_to_16_chunks(&qn, &mut data.qn);
        bigint_to_16_chunks(&qt, &mut data.qt);
        bigint_to_16_chunks(&qdt, &mut data.qdt);
        bigint_to_16_chunks(&qx, &mut data.qx);
        bigint_to_16_chunks(&qy, &mut data.qy);
        Some(data)
    }

    #[allow(dead_code)]
    pub fn execute_add(p1: &[u64; 8], p2: &[u64; 8]) -> BabyJubJubData {
        let mut data = Self::prepare(p1, p2, None).unwrap();
        for icol in 0..COLS {
            let index = icol as usize;
            data.eq[index] = [
                equations::BabyJubJubA::calculate(icol, &data.x1, &data.x2, &data.a, &data.qa),
                equations::BabyJubJubB::calculate(icol, &data.y1, &data.y2, &data.b, &data.qb),
                equations::BabyJubJubN::calculate(
                    icol, &data.x1, &data.y2, &data.y1, &data.x2, &data.n, &data.qn,
                ),
                equations::BabyJubJubT::calculate(icol, &data.a, &data.b, &data.t, &data.qt),
                equations::BabyJubJubDt::calculate(icol, &data.t, &data.dt, &data.qdt),
                equations::BabyJubJubX3::calculate(icol, &data.x3, &data.dt, &data.n, &data.qx),
                equations::BabyJubJubY3::calculate(
                    icol, &data.y3, &data.dt, &data.b, &data.a, &data.qy,
                ),
            ];
            for ieq in 0..7 {
                let cin = if index > 0 { data.cout[index - 1][ieq] } else { 0 };
                let value = data.eq[index][ieq] + cin;
                if icol != COLS - 1 {
                    data.cout[index][ieq] = value / 0x10000;
                }
                debug_assert!(
                    0 == if icol == COLS - 1 { value } else { value % 0x10000 },
                    "BabyJubJub residue eq{ieq} ({index}) #:{value} cin:{cin}"
                );
            }
        }
        data
    }

    #[cfg(any(test, feature = "test_data"))]
    #[allow(dead_code)]
    pub fn verify_add(p1: &[u64; 8], p2: &[u64; 8], p: &[u64; 8]) {
        let data = Self::execute_add(p1, p2);
        data.check_ranges();
        for i in 0..2 {
            let offset = (i + 1) * 4 - 1;
            let mut x3 = data.x3[offset] as u64;
            let mut y3 = data.y3[offset] as u64;
            for j in 1..4 {
                x3 <<= 16;
                y3 <<= 16;
                x3 += data.x3[offset - j] as u64;
                y3 += data.y3[offset - j] as u64;
            }
            assert!(p[i] == x3, "BabyJubJub p[{}]:{} not match with x3:{}", i, p[i], x3);
            assert!(
                p[i + 4] == y3,
                "BabyJubJub p[{}]:{} not match with y3:{}",
                i + 4,
                p[i + 4],
                y3
            );
        }
    }
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
        BabyJubJub::verify_add(&p1, &p2, &expected);
    }

    // Random field inputs near the modulus stress every chunk/carry/quotient range; the
    // executor must reconstruct the host-helper result and stay within check_ranges bounds.
    #[test]
    fn add_ranges_hold_for_random_inputs() {
        // splitmix64 PRNG (no external dep).
        let mut state: u64 = 0x9E3779B97F4A7C15;
        let mut next = || {
            state = state.wrapping_add(0x9E3779B97F4A7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            z ^ (z >> 31)
        };
        let rand_fr = |next: &mut dyn FnMut() -> u64| -> [u64; 4] {
            let mut bytes = [0u8; 32];
            for chunk in bytes.chunks_mut(8) {
                chunk.copy_from_slice(&next().to_le_bytes());
            }
            BabyJubJubField::from_le_bytes_mod_order(&bytes).into_bigint().0
        };
        for _ in 0..5000 {
            let mut p1 = [0u64; 8];
            let mut p2 = [0u64; 8];
            p1[0..4].copy_from_slice(&rand_fr(&mut next));
            p1[4..8].copy_from_slice(&rand_fr(&mut next));
            p2[0..4].copy_from_slice(&rand_fr(&mut next));
            p2[4..8].copy_from_slice(&rand_fr(&mut next));
            let mut expected = [0u64; 8];
            precompiles_helpers::babyjubjub_add(&p1, &p2, &mut expected);
            BabyJubJub::verify_add(&p1, &p2, &expected);
        }
    }

    // Executor result must match the host helper bit-for-bit, including doubling.
    #[test]
    fn add_matches_host_helper() {
        let base = point(
            "17777552123799933955779906779655732241715742912184938656739573121738514868268",
            "2626589144620713026669568689430873010625803728049924121243784502389097019475",
        );
        let other = point(
            "16540640123574156134436876038791482806971768689494387082833631921987005038935",
            "20819045374670962167435360035096875258406992893633759881276124905556507972311",
        );
        let identity = point("0", "1");
        for (p1, p2) in [(base, other), (base, base), (base, identity), (identity, identity)] {
            let mut expected = [0u64; 8];
            precompiles_helpers::babyjubjub_add(&p1, &p2, &mut expected);
            BabyJubJub::verify_add(&p1, &p2, &expected);

            let mut got = [0u64; 8];
            BabyJubJub::calculate_add(&p1, &p2, &mut got);
            assert_eq!(got, expected, "calculate_add mismatch");
        }
    }
}
