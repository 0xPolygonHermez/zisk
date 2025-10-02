use ark_bls12_381::Fq as Bls12_381Field;
use lazy_static::lazy_static;
use num_bigint::BigInt;
use num_traits::Zero;

use precompiles_helpers::{bigint2_to_12_u64, bigint_from_field, bigint_to_24_chunks};

use super::ArithEq384Data;
use crate::{equations, ARITH_EQ_384_CHUNKS_DOUBLE, ARITH_EQ_384_U64S_DOUBLE};

#[cfg(feature = "test_data")]
use crate::ARITH_EQ_384_U64S;

const COLS: u8 = ARITH_EQ_384_CHUNKS_DOUBLE as u8;

lazy_static! {
    pub static ref BLS12_381_CURVE_PRIME: BigInt = BigInt::parse_bytes(
        b"1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab",
        16
    )
    .unwrap();
    pub static ref BLS12_381_CURVE_ADD_Q0_OFFSET: BigInt = BigInt::from(1) << 382; //BigInt::from(1) << 388;
    pub static ref BLS12_381_CURVE_DBL_Q0_OFFSET: BigInt = BigInt::from(1) << 383;
    pub static ref BLS12_381_CURVE_Q1_OFFSET: BigInt = BigInt::from(1) << 2;
    pub static ref BLS12_381_CURVE_Q2_OFFSET: BigInt = BigInt::from(1) << 382;
}

pub struct Bls12_381Curve {}

impl Bls12_381Curve {
    #[allow(dead_code)]
    pub fn calculate_add(
        p1: &[u64; ARITH_EQ_384_U64S_DOUBLE],
        p2: &[u64; ARITH_EQ_384_U64S_DOUBLE],
        p3: &mut [u64; ARITH_EQ_384_U64S_DOUBLE],
    ) {
        Self::prepare(false, p1, p2, Some(p3));
    }

    #[allow(dead_code)]
    pub fn calculate_dbl(
        p1: &[u64; ARITH_EQ_384_U64S_DOUBLE],
        p3: &mut [u64; ARITH_EQ_384_U64S_DOUBLE],
    ) {
        Self::prepare(true, p1, p1, Some(p3));
    }

    fn point_from_u64s(p: &[u64; ARITH_EQ_384_U64S_DOUBLE]) -> (Bls12_381Field, Bls12_381Field) {
        (
            Bls12_381Field::from(ark_ff::BigInt::<6>(p[0..6].try_into().unwrap())),
            Bls12_381Field::from(ark_ff::BigInt::<6>(p[6..12].try_into().unwrap())),
        )
    }

    fn prepare(
        is_dbl: bool,
        p1: &[u64; ARITH_EQ_384_U64S_DOUBLE],
        p2: &[u64; ARITH_EQ_384_U64S_DOUBLE],
        p3: Option<&mut [u64; ARITH_EQ_384_U64S_DOUBLE]>,
    ) -> Option<ArithEq384Data> {
        let (x1, y1) = Self::point_from_u64s(p1);
        let (x2, y2) = if is_dbl { (x1, y1) } else { Self::point_from_u64s(p2) };

        let s = if is_dbl {
            (Bls12_381Field::from(3u64) * x1 * x1) / (y1 + y1)
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
            assert!((&_q0 % &*BLS12_381_CURVE_PRIME).is_zero());
            &*BLS12_381_CURVE_DBL_Q0_OFFSET - (&_q0 / &*BLS12_381_CURVE_PRIME)
        } else {
            let _q0: BigInt = &s * (&x2 - &x1) - &y2 + &y1;
            assert!((&_q0 % &*BLS12_381_CURVE_PRIME).is_zero());
            (&_q0 / &*BLS12_381_CURVE_PRIME) + &*BLS12_381_CURVE_ADD_Q0_OFFSET
        };

        let _q1 = &s * &s - &x1 - &x2 - &x3;
        assert!((&_q1 % &*BLS12_381_CURVE_PRIME).is_zero());
        let q1 = (&_q1 / &*BLS12_381_CURVE_PRIME) + &*BLS12_381_CURVE_Q1_OFFSET;

        let _q2 = &s * &x1 - &s * &x3 - &y1 - &y3;
        assert!((&_q2 % &*BLS12_381_CURVE_PRIME).is_zero());
        let q2 = &*BLS12_381_CURVE_Q2_OFFSET - (&_q2 / &*BLS12_381_CURVE_PRIME);

        if let Some(p3) = p3 {
            bigint2_to_12_u64(&x3, &y3, p3);
            return None;
        }

        let mut data = ArithEq384Data::default();
        bigint_to_24_chunks(&q0, &mut data.q0);
        bigint_to_24_chunks(&q1, &mut data.q1);
        bigint_to_24_chunks(&q2, &mut data.q2);
        bigint_to_24_chunks(&s, &mut data.s);
        bigint_to_24_chunks(&x1, &mut data.x1);
        bigint_to_24_chunks(&y1, &mut data.y1);
        bigint_to_24_chunks(&x2, &mut data.x2);
        bigint_to_24_chunks(&y2, &mut data.y2);
        bigint_to_24_chunks(&x3, &mut data.x3);
        bigint_to_24_chunks(&y3, &mut data.y3);
        Some(data)
    }

    #[inline(always)]
    #[allow(dead_code)]
    pub fn execute_add(
        p1: &[u64; ARITH_EQ_384_U64S_DOUBLE],
        p2: &[u64; ARITH_EQ_384_U64S_DOUBLE],
    ) -> ArithEq384Data {
        Self::execute_add_dbl(false, p1, p2)
    }

    #[inline(always)]
    #[allow(dead_code)]
    pub fn execute_dbl(p1: &[u64; ARITH_EQ_384_U64S_DOUBLE]) -> ArithEq384Data {
        Self::execute_add_dbl(true, p1, p1)
    }

    pub fn execute_add_dbl(
        is_dbl: bool,
        p1: &[u64; ARITH_EQ_384_U64S_DOUBLE],
        p2: &[u64; ARITH_EQ_384_U64S_DOUBLE],
    ) -> ArithEq384Data {
        let mut data = Self::prepare(is_dbl, p1, p2, None).unwrap();
        for icol in 0..COLS {
            let index = icol as usize;
            data.eq[index] = [
                if is_dbl {
                    equations::Bls12_381CurveDbl::calculate(
                        icol, &data.x1, &data.y1, &data.s, &data.q0,
                    )
                } else {
                    equations::Bls12_381CurveAdd::calculate(
                        icol, &data.x1, &data.y1, &data.x2, &data.y2, &data.s, &data.q0,
                    )
                },
                equations::Bls12_381CurveX3::calculate(
                    icol, &data.x1, &data.x2, &data.x3, &data.s, &data.q1,
                ),
                equations::Bls12_381CurveY3::calculate(
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
                    "Bls12_381Curve residue eq{ieq} (#{index}) eq={} cin={cin} value={value}",
                    data.eq[index][ieq]
                );
            }
        }
        data
    }

    #[cfg(feature = "test_data")]
    #[allow(dead_code)]
    pub fn verify_add_dbl(
        is_dbl: bool,
        p1: &[u64; ARITH_EQ_384_U64S_DOUBLE],
        p2: &[u64; ARITH_EQ_384_U64S_DOUBLE],
        p: &[u64; ARITH_EQ_384_U64S_DOUBLE],
    ) {
        let data = Self::execute_add_dbl(is_dbl, p1, p2);
        data.check_ranges();
        let op = if is_dbl { "Bls12_381CurveDbl" } else { "Bls12_381CurveAdd" };
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
            assert!(
                p[i + ARITH_EQ_384_U64S] == y3,
                "{} p[{}]:{} not match with y3:{}",
                op,
                i + ARITH_EQ_384_U64S,
                p[i + ARITH_EQ_384_U64S],
                y3
            );
        }
    }

    #[cfg(feature = "test_data")]
    #[allow(dead_code)]
    pub fn verify_add(
        p1: &[u64; ARITH_EQ_384_U64S_DOUBLE],
        p2: &[u64; ARITH_EQ_384_U64S_DOUBLE],
        p: &[u64; ARITH_EQ_384_U64S_DOUBLE],
    ) {
        Self::verify_add_dbl(false, p1, p2, p);
    }

    #[cfg(feature = "test_data")]
    #[allow(dead_code)]
    pub fn verify_dbl(p1: &[u64; ARITH_EQ_384_U64S_DOUBLE], p: &[u64; ARITH_EQ_384_U64S_DOUBLE]) {
        Self::verify_add_dbl(true, p1, p1, p);
    }
}
