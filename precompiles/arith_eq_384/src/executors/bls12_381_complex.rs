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
    pub static ref BLS12_381_COMPLEX_PRIME: BigInt = BigInt::parse_bytes(
        b"1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab",
        16
    )
    .unwrap();
    pub static ref BLS12_381_COMPLEX_ADD_Q1_OFFSET: BigInt = BigInt::from(1);
    pub static ref BLS12_381_COMPLEX_ADD_Q2_OFFSET: BigInt = BigInt::from(1);
    pub static ref BLS12_381_COMPLEX_SUB_Q1_OFFSET: BigInt = BigInt::from(1);
    pub static ref BLS12_381_COMPLEX_SUB_Q2_OFFSET: BigInt = BigInt::from(1);
    pub static ref BLS12_381_COMPLEX_MUL_Q1_OFFSET: BigInt = BigInt::from(1) << 382;
    pub static ref BLS12_381_COMPLEX_MUL_Q2_OFFSET: BigInt = BigInt::from(1);
}

#[derive(Clone)]
pub(crate) enum OpType {
    Add,
    Sub,
    Mul,
}

pub struct Bls12_381Complex {}

impl Bls12_381Complex {
    #[allow(dead_code)]
    pub fn calculate_add(
        f1: &[u64; ARITH_EQ_384_U64S_DOUBLE],
        f2: &[u64; ARITH_EQ_384_U64S_DOUBLE],
        f3: &mut [u64; ARITH_EQ_384_U64S_DOUBLE],
    ) {
        Self::prepare(OpType::Add, f1, f2, Some(f3));
    }

    #[allow(dead_code)]
    pub fn calculate_sub(
        f1: &[u64; ARITH_EQ_384_U64S_DOUBLE],
        f2: &[u64; ARITH_EQ_384_U64S_DOUBLE],
        f3: &mut [u64; ARITH_EQ_384_U64S_DOUBLE],
    ) {
        Self::prepare(OpType::Sub, f1, f2, Some(f3));
    }

    #[allow(dead_code)]
    pub fn calculate_mul(
        f1: &[u64; ARITH_EQ_384_U64S_DOUBLE],
        f2: &[u64; ARITH_EQ_384_U64S_DOUBLE],
        f3: &mut [u64; ARITH_EQ_384_U64S_DOUBLE],
    ) {
        Self::prepare(OpType::Mul, f1, f2, Some(f3));
    }

    fn field_from_64(f: &[u64; ARITH_EQ_384_U64S_DOUBLE]) -> (Bls12_381Field, Bls12_381Field) {
        (
            Bls12_381Field::from(ark_ff::BigInt::<6>(f[0..6].try_into().unwrap())),
            Bls12_381Field::from(ark_ff::BigInt::<6>(f[6..12].try_into().unwrap())),
        )
    }

    fn prepare(
        op: OpType,
        f1: &[u64; ARITH_EQ_384_U64S_DOUBLE],
        f2: &[u64; ARITH_EQ_384_U64S_DOUBLE],
        f3: Option<&mut [u64; ARITH_EQ_384_U64S_DOUBLE]>,
    ) -> Option<ArithEq384Data> {
        let (x1, y1) = Self::field_from_64(f1);
        let (x2, y2) = Self::field_from_64(f2);

        let (x3, y3) = match op {
            OpType::Add => (x1 + x2, y1 + y2),
            OpType::Sub => (x1 - x2, y1 - y2),
            OpType::Mul => (x1 * x2 - y1 * y2, y1 * x2 + x1 * y2),
        };

        let x1 = bigint_from_field(&x1);
        let y1 = bigint_from_field(&y1);
        let x2 = bigint_from_field(&x2);
        let y2 = bigint_from_field(&y2);
        let x3 = bigint_from_field(&x3);
        let y3 = bigint_from_field(&y3);

        let (q1, q2) = match op {
            OpType::Add => {
                let _q1 = &x1 + &x2 - &x3;
                assert!((&_q1 % &*BLS12_381_COMPLEX_PRIME).is_zero());
                let q1 = (&_q1 / &*BLS12_381_COMPLEX_PRIME) + &*BLS12_381_COMPLEX_ADD_Q1_OFFSET;

                let _q2 = &y1 + &y2 - &y3;
                assert!((&_q2 % &*BLS12_381_COMPLEX_PRIME).is_zero());
                let q2 = (&_q2 / &*BLS12_381_COMPLEX_PRIME) + &*BLS12_381_COMPLEX_ADD_Q2_OFFSET;
                (q1, q2)
            }
            OpType::Sub => {
                let _q1 = &x1 - &x2 - &x3;
                assert!((&_q1 % &*BLS12_381_COMPLEX_PRIME).is_zero());
                let q1 = &*BLS12_381_COMPLEX_SUB_Q1_OFFSET - (&_q1 / &*BLS12_381_COMPLEX_PRIME);

                let _q2 = &y1 - &y2 - &y3;
                assert!((&_q2 % &*BLS12_381_COMPLEX_PRIME).is_zero());
                let q2 = &*BLS12_381_COMPLEX_SUB_Q2_OFFSET - (&_q2 / &*BLS12_381_COMPLEX_PRIME);
                (q1, q2)
            }
            OpType::Mul => {
                let _q1 = &x1 * &x2 - &y1 * &y2 - &x3;
                assert!((&_q1 % &*BLS12_381_COMPLEX_PRIME).is_zero());
                let q1 = &*BLS12_381_COMPLEX_MUL_Q1_OFFSET - (&_q1 / &*BLS12_381_COMPLEX_PRIME);

                let _q2 = &y1 * &x2 + &x1 * &y2 - &y3;
                assert!((&_q2 % &*BLS12_381_COMPLEX_PRIME).is_zero());
                let q2 = (&_q2 / &*BLS12_381_COMPLEX_PRIME) + &*BLS12_381_COMPLEX_MUL_Q2_OFFSET;
                (q1, q2)
            }
        };

        if let Some(f3) = f3 {
            bigint2_to_12_u64(&x3, &y3, f3);
            return None;
        }

        let mut data = ArithEq384Data::default();
        bigint_to_24_chunks(&q1, &mut data.q1);
        bigint_to_24_chunks(&q2, &mut data.q2);
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
        f1: &[u64; ARITH_EQ_384_U64S_DOUBLE],
        f2: &[u64; ARITH_EQ_384_U64S_DOUBLE],
    ) -> ArithEq384Data {
        Self::execute(OpType::Add, f1, f2)
    }

    #[inline(always)]
    #[allow(dead_code)]
    pub fn execute_sub(
        f1: &[u64; ARITH_EQ_384_U64S_DOUBLE],
        f2: &[u64; ARITH_EQ_384_U64S_DOUBLE],
    ) -> ArithEq384Data {
        Self::execute(OpType::Sub, f1, f2)
    }

    #[inline(always)]
    #[allow(dead_code)]
    pub fn execute_mul(
        f1: &[u64; ARITH_EQ_384_U64S_DOUBLE],
        f2: &[u64; ARITH_EQ_384_U64S_DOUBLE],
    ) -> ArithEq384Data {
        Self::execute(OpType::Mul, f1, f2)
    }

    #[inline(always)]
    #[allow(dead_code)]
    pub fn execute(
        op: OpType,
        f1: &[u64; ARITH_EQ_384_U64S_DOUBLE],
        f2: &[u64; ARITH_EQ_384_U64S_DOUBLE],
    ) -> ArithEq384Data {
        let mut data = Self::prepare(op.clone(), f1, f2, None).unwrap();
        for icol in 0..COLS {
            let index = icol as usize;
            // data.eq[index][0] = 0;
            data.eq[index][1] = match op {
                OpType::Add => equations::Bls12_381ComplexAddX3::calculate(
                    icol, &data.x1, &data.x2, &data.x3, &data.q1,
                ),
                OpType::Sub => equations::Bls12_381ComplexSubX3::calculate(
                    icol, &data.x1, &data.x2, &data.x3, &data.q1,
                ),
                OpType::Mul => equations::Bls12_381ComplexMulX3::calculate(
                    icol, &data.x1, &data.y1, &data.x2, &data.y2, &data.x3, &data.q1,
                ),
            };
            data.eq[index][2] = match op {
                OpType::Add => equations::Bls12_381ComplexAddY3::calculate(
                    icol, &data.y1, &data.y2, &data.y3, &data.q2,
                ),
                OpType::Sub => equations::Bls12_381ComplexSubY3::calculate(
                    icol, &data.y1, &data.y2, &data.y3, &data.q2,
                ),
                OpType::Mul => equations::Bls12_381ComplexMulY3::calculate(
                    icol, &data.x1, &data.y1, &data.x2, &data.y2, &data.y3, &data.q2,
                ),
            };
            for ieq in 1..3 {
                let cin = if index > 0 { data.cout[index - 1][ieq] } else { 0 };
                let value = data.eq[index][ieq] + cin;
                if icol != COLS - 1 {
                    data.cout[index][ieq] = value / 0x10000;
                }
                debug_assert!(
                    0 == if icol == COLS - 1 { value } else { value % 0x10000 },
                    "Bls12_381Complex residue eq{ieq} ({index}) #:{value} cin:{cin}"
                );
            }
        }
        data
    }

    #[cfg(feature = "test_data")]
    #[allow(dead_code)]
    pub fn verify(
        op: OpType,
        f1: &[u64; ARITH_EQ_384_U64S_DOUBLE],
        f2: &[u64; ARITH_EQ_384_U64S_DOUBLE],
        f: &[u64; ARITH_EQ_384_U64S_DOUBLE],
    ) {
        let data = Self::execute(op.clone(), f1, f2);
        // data.check_ranges();
        let op = match op {
            OpType::Add => "Bls12_381ComplexAdd",
            OpType::Sub => "Bls12_381ComplexSub",
            OpType::Mul => "Bls12_381ComplexMul",
        };
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
            assert!(f[i] == x3, "{} p[{}]:{} not match with x3:{}", op, i, f[i], x3);
            assert!(
                f[i + ARITH_EQ_384_U64S] == y3,
                "{} p[{}]:{} not match with y3:{}",
                op,
                i + ARITH_EQ_384_U64S,
                f[i + ARITH_EQ_384_U64S],
                y3
            );
        }
    }

    #[cfg(feature = "test_data")]
    #[allow(dead_code)]
    pub fn verify_add(
        f1: &[u64; ARITH_EQ_384_U64S_DOUBLE],
        f2: &[u64; ARITH_EQ_384_U64S_DOUBLE],
        f: &[u64; ARITH_EQ_384_U64S_DOUBLE],
    ) {
        Self::verify(OpType::Add, f1, f2, f);
    }

    #[cfg(feature = "test_data")]
    #[allow(dead_code)]
    pub fn verify_sub(
        f1: &[u64; ARITH_EQ_384_U64S_DOUBLE],
        f2: &[u64; ARITH_EQ_384_U64S_DOUBLE],
        f: &[u64; ARITH_EQ_384_U64S_DOUBLE],
    ) {
        Self::verify(OpType::Sub, f1, f2, f);
    }

    #[cfg(feature = "test_data")]
    #[allow(dead_code)]
    pub fn verify_mul(
        f1: &[u64; ARITH_EQ_384_U64S_DOUBLE],
        f2: &[u64; ARITH_EQ_384_U64S_DOUBLE],
        f: &[u64; ARITH_EQ_384_U64S_DOUBLE],
    ) {
        Self::verify(OpType::Mul, f1, f2, f);
    }
}
