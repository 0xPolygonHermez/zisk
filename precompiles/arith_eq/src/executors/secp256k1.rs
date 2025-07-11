use super::ArithEqData;
use lazy_static::lazy_static;
use num_bigint::BigInt;
use num_traits::Zero;
use precompiles_helpers::{bigint2_to_8_u64, bigint_from_field, bigint_to_16_chunks};

use crate::equations;
use ark_secp256k1::Fq as Secp256k1Field;

const COLS: u8 = 32;

lazy_static! {
    pub static ref SECP256K1_PRIME: BigInt = BigInt::parse_bytes(
        b"fffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f",
        16
    )
    .unwrap();
    pub static ref SECP256K1_ADD_Q0_OFFSET: BigInt = BigInt::from(1) << 257;
    pub static ref SECP256K1_DBL_Q0_OFFSET: BigInt = BigInt::from(1) << 258;
    pub static ref SECP256K1_Q1_OFFSET: BigInt = BigInt::from(4);
    pub static ref SECP256K1_Q2_OFFSET: BigInt = BigInt::from(1) << 257;
}

pub struct Secp256k1 {}

impl Secp256k1 {
    #[allow(dead_code)]
    pub fn calculate_add(p1: &[u64; 8], p2: &[u64; 8], p3: &mut [u64; 8]) {
        Self::prepare(false, p1, p2, Some(p3));
    }
    #[allow(dead_code)]
    pub fn calculate_dbl(p1: &[u64; 8], p3: &mut [u64; 8]) {
        Self::prepare(true, p1, p1, Some(p3));
    }

    fn point_from_8x64(p: &[u64; 8]) -> (Secp256k1Field, Secp256k1Field) {
        (
            Secp256k1Field::from(ark_ff::BigInt::<4>(p[0..4].try_into().unwrap())),
            Secp256k1Field::from(ark_ff::BigInt::<4>(p[4..8].try_into().unwrap())),
        )
    }
    fn prepare(
        is_dbl: bool,
        p1: &[u64; 8],
        p2: &[u64; 8],
        p3: Option<&mut [u64; 8]>,
    ) -> Option<ArithEqData> {
        let (x1, y1) = Self::point_from_8x64(p1);
        let (x2, y2) = if is_dbl { (x1, y1) } else { Self::point_from_8x64(p2) };

        let s = if is_dbl {
            (Secp256k1Field::from(3u64) * x1 * x1) / (y1 + y1)
        } else {
            (y2 - y1) / (x2 - x1)
        };
        let x3 = s * s - (x1 + x2);
        let y3 = s * (x1 - x3) - y1;

        let s = bigint_from_field(&s);
        let x1 = bigint_from_field(&x1);
        let y1 = bigint_from_field(&y1);
        let x2 = bigint_from_field(&x2);
        let y2 = bigint_from_field(&y2);
        let x3 = bigint_from_field(&x3);
        let y3 = bigint_from_field(&y3);

        let q0 = if is_dbl {
            let _q0: BigInt = 2 * &s * &y1 - 3 * &x1 * &x1;
            assert!((&_q0 % &*SECP256K1_PRIME).is_zero());
            &*SECP256K1_DBL_Q0_OFFSET - (&_q0 / &*SECP256K1_PRIME)
        } else {
            let _q0: BigInt = &s * (&x2 - &x1) - &y2 + &y1;
            assert!((&_q0 % &*SECP256K1_PRIME).is_zero());
            (&_q0 / &*SECP256K1_PRIME) + &*SECP256K1_ADD_Q0_OFFSET
        };

        let _q1 = &s * &s - &x1 - &x2 - &x3;
        assert!((&_q1 % &*SECP256K1_PRIME).is_zero());
        let q1 = (&_q1 / &*SECP256K1_PRIME) + &*SECP256K1_Q1_OFFSET;

        let _q2 = &s * &x1 - &s * &x3 - &y1 - &y3;
        assert!((&_q2 % &*SECP256K1_PRIME).is_zero());
        let q2 = &*SECP256K1_Q2_OFFSET - (&_q2 / &*SECP256K1_PRIME);

        if let Some(p3) = p3 {
            bigint2_to_8_u64(&x3, &y3, p3);
            return None;
        }

        let mut data = ArithEqData::default();
        bigint_to_16_chunks(&q0, &mut data.q0);
        bigint_to_16_chunks(&q1, &mut data.q1);
        bigint_to_16_chunks(&q2, &mut data.q2);
        bigint_to_16_chunks(&s, &mut data.s);
        bigint_to_16_chunks(&x1, &mut data.x1);
        bigint_to_16_chunks(&y1, &mut data.y1);
        bigint_to_16_chunks(&x2, &mut data.x2);
        bigint_to_16_chunks(&y2, &mut data.y2);
        bigint_to_16_chunks(&x3, &mut data.x3);
        bigint_to_16_chunks(&y3, &mut data.y3);
        Some(data)
    }
    #[inline(always)]
    #[allow(dead_code)]
    pub fn execute_add(p1: &[u64; 8], p2: &[u64; 8]) -> ArithEqData {
        Self::execute_add_dbl(false, p1, p2)
    }

    #[inline(always)]
    #[allow(dead_code)]
    pub fn execute_dbl(p1: &[u64; 8]) -> ArithEqData {
        Self::execute_add_dbl(true, p1, p1)
    }
    pub fn execute_add_dbl(is_dbl: bool, p1: &[u64; 8], p2: &[u64; 8]) -> ArithEqData {
        let mut data = Self::prepare(is_dbl, p1, p2, None).unwrap();
        for icol in 0..COLS {
            let index = icol as usize;
            data.eq[index] = [
                if is_dbl {
                    equations::Secp256k1Dbl::calculate(icol, &data.x1, &data.y1, &data.s, &data.q0)
                } else {
                    equations::Secp256k1Add::calculate(
                        icol, &data.x1, &data.y1, &data.x2, &data.y2, &data.s, &data.q0,
                    )
                },
                equations::Secp256k1X3::calculate(
                    icol, &data.x1, &data.x2, &data.x3, &data.s, &data.q1,
                ),
                equations::Secp256k1Y3::calculate(
                    icol, &data.x1, &data.y1, &data.x3, &data.y3, &data.s, &data.q2,
                ),
            ];
            for ieq in 0..3 {
                let cin = if index > 0 { data.cout[index - 1][ieq] } else { 0 };
                let value = data.eq[index][ieq] + cin;
                if icol != COLS - 1 {
                    data.cout[index][ieq] = value / 0x10000;
                }
                debug_assert!(
                    0 == if icol == COLS - 1 { value } else { value % 0x10000 },
                    "EqSecp256k1 residue eq{ieq} ({index}) #:{value} cin:{cin}"
                );
            }
        }
        data
    }
    #[cfg(feature = "test_data")]
    #[allow(dead_code)]
    pub fn verify_add_dbl(is_dbl: bool, p1: &[u64; 8], p2: &[u64; 8], p: &[u64; 8]) {
        let data = Self::execute_add_dbl(is_dbl, p1, p2);
        data.check_ranges();
        let op = if is_dbl { "Secp256k1Dbl" } else { "Secp256k1Add" };
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
            assert!(p[i] == x3, "{} p[{}]:{} not match with x3:{}", op, i, p[i], x3);
            assert!(p[i + 4] == y3, "{} p[{}]:{} not match with y3:{}", op, i + 4, p[i + 4], y3);
        }
    }
    #[cfg(feature = "test_data")]
    #[allow(dead_code)]
    pub fn verify_add(p1: &[u64; 8], p2: &[u64; 8], p: &[u64; 8]) {
        Self::verify_add_dbl(false, p1, p2, p);
    }
    #[cfg(feature = "test_data")]
    #[allow(dead_code)]
    pub fn verify_dbl(p1: &[u64; 8], p: &[u64; 8]) {
        Self::verify_add_dbl(true, p1, p1, p);
    }
}
