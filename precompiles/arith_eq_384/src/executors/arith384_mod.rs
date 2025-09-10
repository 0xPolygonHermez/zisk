use lazy_static::lazy_static;
use num_bigint::BigInt;

use precompiles_helpers::{bigint_from_u64s, bigint_to_24_chunks, bigint_to_6_u64};

use super::ArithEq384Data;
use crate::{equations, ARITH_EQ_384_CHUNKS_DOUBLE, ARITH_EQ_384_U64S};

const COLS: u8 = ARITH_EQ_384_CHUNKS_DOUBLE as u8;

lazy_static! {
    pub static ref P_384_MASK: BigInt = BigInt::parse_bytes(
        b"FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        16
    )
    .unwrap();
}

pub struct Arith384Mod {}

impl Arith384Mod {
    #[allow(dead_code)]
    pub fn calculate(
        a: &[u64; ARITH_EQ_384_U64S],
        b: &[u64; ARITH_EQ_384_U64S],
        c: &[u64; ARITH_EQ_384_U64S],
        module: &[u64; ARITH_EQ_384_U64S],
        d: &mut [u64; ARITH_EQ_384_U64S],
    ) {
        Self::prepare(a, b, c, module, Some(d));
    }
    fn prepare(
        a: &[u64; ARITH_EQ_384_U64S],
        b: &[u64; ARITH_EQ_384_U64S],
        c: &[u64; ARITH_EQ_384_U64S],
        module: &[u64; ARITH_EQ_384_U64S],
        d: Option<&mut [u64; ARITH_EQ_384_U64S]>,
    ) -> Option<ArithEq384Data> {
        let a = bigint_from_u64s(a);
        let b = bigint_from_u64s(b);
        let c = bigint_from_u64s(c);
        let module = bigint_from_u64s(module);

        let res = &a * &b + &c;

        // a (0..2^256-1) * b (0..2^256-1) + c (0..2^256-1) = res (0..2^512-2^256)
        // if module is small, q is large, upto 512 bits, we need divide q into two
        // numbers of 256 bits.  q = q1 * 2^256 + q0

        let q = &res / &module;
        let res = &res % &module;
        let q0 = &q & &*P_384_MASK;
        let q1 = &q >> 384;

        if let Some(d) = d {
            bigint_to_6_u64(&res, d);
            return None;
        }
        // x3 = mod(x1*y1+x2, y2)
        // a:x1 b:y1 c:x2 d: x3 module: y2
        let mut data = ArithEq384Data::default();
        bigint_to_24_chunks(&a, &mut data.x1);
        bigint_to_24_chunks(&b, &mut data.y1);
        bigint_to_24_chunks(&c, &mut data.x2);
        bigint_to_24_chunks(&module, &mut data.y2);
        bigint_to_24_chunks(&res, &mut data.x3);
        bigint_to_24_chunks(&q0, &mut data.q0);
        bigint_to_24_chunks(&q1, &mut data.q1);
        Some(data)
    }

    pub fn execute(
        a: &[u64; ARITH_EQ_384_U64S],
        b: &[u64; ARITH_EQ_384_U64S],
        c: &[u64; ARITH_EQ_384_U64S],
        module: &[u64; ARITH_EQ_384_U64S],
    ) -> ArithEq384Data {
        let mut data = Self::prepare(a, b, c, module, None).unwrap();
        for icol in 0..COLS {
            let index = icol as usize;
            data.eq[index][0] = equations::Arith384Mod::calculate(
                icol, &data.x1, &data.y1, &data.x2, &data.y2, &data.x3, &data.q0, &data.q1,
            );

            let cin = if index > 0 { data.cout[index - 1][0] } else { 0 };
            let value = data.eq[index][0] + cin;
            if icol != COLS - 1 {
                data.cout[index][0] = value / 0x10000;
            }
            debug_assert!(
                0 == if icol == COLS - 1 { value } else { value % 0x10000 },
                "Arith384Mod residue ({index}) #:{value} cin:{cin}"
            );
        }
        data
    }

    #[cfg(feature = "test_data")]
    #[allow(dead_code)]
    pub fn verify(
        a: &[u64; ARITH_EQ_384_U64S],
        b: &[u64; ARITH_EQ_384_U64S],
        c: &[u64; ARITH_EQ_384_U64S],
        module: &[u64; ARITH_EQ_384_U64S],
        d: &[u64; ARITH_EQ_384_U64S],
    ) {
        let data = Self::execute(a, b, c, module);
        data.check_ranges();
        for (i, chunk_d) in d.iter().enumerate() {
            let offset = (i + 1) * 4 - 1;
            let mut x3 = data.x3[offset] as u64;
            for j in 1..4 {
                x3 <<= 16;
                x3 += data.x3[offset - j] as u64;
            }
            assert!(*chunk_d == x3, "Arith384Mod dh[{}]:{} not match with x3:{}", i, chunk_d, x3);
        }
    }
}
