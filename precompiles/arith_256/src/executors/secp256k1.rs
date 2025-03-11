use super::{bigint_from_field, bigint_to_16_chunks, Arith256Data};
use num_bigint::BigInt;
use num_traits::Zero;

use crate::{EqSecp256k1Add, EqSecp256k1Dbl, EqSecp256k1X3, EqSecp256k1Y3};
use ark_secp256k1::Fq as Secp256k1Field;

const COLS: u8 = 32;

pub struct Secp256k1 {
    pub prime: BigInt,
    pub q0_add_offset: BigInt,
    pub q0_dbl_offset: BigInt,
    pub q1_offset: BigInt,
    pub q2_offset: BigInt,
}

impl Secp256k1 {
    pub fn new() -> Self {
        let prime = BigInt::parse_bytes(
            b"fffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f",
            16,
        )
        .unwrap();
        let q0_add_offset = BigInt::from(1) << 257;
        let q0_dbl_offset = BigInt::from(1) << 258;

        let q1_offset = BigInt::from(4);
        let q2_offset = BigInt::from(1) << 257;
        Self { prime, q0_add_offset, q0_dbl_offset, q1_offset, q2_offset }
    }
    fn point_from_8x64(&self, p: &[u64; 8]) -> (Secp256k1Field, Secp256k1Field) {
        (
            Secp256k1Field::from(ark_ff::BigInt::<4>(p[0..4].try_into().unwrap())),
            Secp256k1Field::from(ark_ff::BigInt::<4>(p[4..8].try_into().unwrap())),
        )
    }
    fn prepare(&self, is_dbl: bool, p1: &[u64; 8], p2: &[u64; 8]) -> Arith256Data {
        let (x1, y1) = self.point_from_8x64(p1);
        let (x2, y2) = if is_dbl { (x1, y1) } else { self.point_from_8x64(p2) };

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
            assert!((&_q0 % &self.prime).is_zero());
            &self.q0_dbl_offset - (&_q0 / &self.prime)
        } else {
            let _q0: BigInt = &s * (&x2 - &x1) - &y2 + &y1;
            assert!((&_q0 % &self.prime).is_zero());
            (&_q0 / &self.prime) + &self.q0_add_offset
        };

        let _q1 = &s * &s - &x1 - &x2 - &x3;
        assert!((&_q1 % &self.prime).is_zero());
        let q1 = (&_q1 / &self.prime) + &self.q1_offset;

        let _q2 = &s * &x1 - &s * &x3 - &y1 - &y3;
        assert!((&_q2 % &self.prime).is_zero());
        let q2 = &self.q2_offset - (&_q2 / &self.prime);

        let mut data = Arith256Data::default();
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
        data
    }
    #[inline(always)]
    pub fn execute_add(&self, p1: &[u64; 8], p2: &[u64; 8]) -> Arith256Data {
        self.execute_add_dbl(false, p1, p2)
    }
    pub fn execute_dbl(&self, p1: &[u64; 8]) -> Arith256Data {
        self.execute_add_dbl(true, p1, p1)
    }
    pub fn execute_add_dbl(&self, is_dbl: bool, p1: &[u64; 8], p2: &[u64; 8]) -> Arith256Data {
        let mut data = self.prepare(is_dbl, p1, p2);
        for icol in 0..COLS {
            let index = icol as usize;
            data.eq[index] = [
                if is_dbl {
                    EqSecp256k1Dbl::calculate(icol, &data.x1, &data.y1, &data.s, &data.q0)
                } else {
                    EqSecp256k1Add::calculate(
                        icol, &data.x1, &data.y1, &data.x2, &data.y2, &data.s, &data.q0,
                    )
                },
                EqSecp256k1X3::calculate(icol, &data.x1, &data.x2, &data.x3, &data.s, &data.q1),
                EqSecp256k1Y3::calculate(
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
                    "EqSecp256k1 residue eq{} ({}) #:{} cin:{}",
                    ieq,
                    index,
                    value,
                    cin
                );
            }
        }
        data
    }
    pub fn verify_add_dbl(&self, is_dbl: bool, p1: &[u64; 8], p2: &[u64; 8], p: &[u64; 8]) {
        let data = self.execute_add_dbl(is_dbl, p1, p2);
        data.check_ranges();
        let op = if is_dbl { "Secp256k1Dbl" } else { "Secp256k1Add" };
        for i in 0..2 {
            let offset = (i + 1) * 4 - 1;
            let mut x3 = data.x3[offset] as u64;
            let mut y3 = data.y3[offset] as u64;
            for j in 1..4 {
                x3 = x3 << 16;
                y3 = y3 << 16;
                x3 += data.x3[offset - j] as u64;
                y3 += data.y3[offset - j] as u64;
            }
            assert!(p[i] == x3, "{} p[{}]:{} not match with x3:{}", op, i, p[i], x3);
            assert!(p[i + 4] == y3, "{} p[{}]:{} not match with y3:{}", op, i + 4, p[i + 4], y3);
        }
    }
    pub fn verify_add(&self, p1: &[u64; 8], p2: &[u64; 8], p: &[u64; 8]) {
        self.verify_add_dbl(false, p1, p2, p);
    }
    pub fn verify_dbl(&self, p1: &[u64; 8], p: &[u64; 8]) {
        self.verify_add_dbl(true, p1, p1, p);
    }
}
