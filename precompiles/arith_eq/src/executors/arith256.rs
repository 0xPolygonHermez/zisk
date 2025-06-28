use super::ArithEqData;
use crate::equations;
use lazy_static::lazy_static;
use num_bigint::BigInt;
use precompiles_helpers::{bigint_from_u64s, bigint_to_16_chunks, bigint_to_4_u64};

const COLS: u8 = 32;

lazy_static! {
    pub static ref P_256_MASK: BigInt = BigInt::parse_bytes(
        b"FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        16
    )
    .unwrap();
}

pub struct Arith256 {}
impl Arith256 {
    #[allow(dead_code)]
    pub fn calculate(
        a: &[u64; 4],
        b: &[u64; 4],
        c: &[u64; 4],
        dl: &mut [u64; 4],
        dh: &mut [u64; 4],
    ) {
        Self::prepare(a, b, c, Some(dl), Some(dh));
    }
    fn prepare(
        a: &[u64; 4],
        b: &[u64; 4],
        c: &[u64; 4],
        dl: Option<&mut [u64; 4]>,
        dh: Option<&mut [u64; 4]>,
    ) -> Option<ArithEqData> {
        let a = bigint_from_u64s(a);
        let b = bigint_from_u64s(b);
        let c = bigint_from_u64s(c);

        let res = &a * &b + &c;
        let res_dh = &res >> 256;
        let res_dl = &res & &*P_256_MASK;

        // y3|x3 = x1*y1+x2
        // a:x1 b:y1 c:x2 dl: x3 dh: y3
        if let Some(dh) = dh {
            bigint_to_4_u64(&res_dh, dh);
            bigint_to_4_u64(&res_dl, dl.unwrap());
            return None;
        }

        let mut data = ArithEqData::default();
        bigint_to_16_chunks(&a, &mut data.x1);
        bigint_to_16_chunks(&b, &mut data.y1);
        bigint_to_16_chunks(&c, &mut data.x2);
        bigint_to_16_chunks(&res_dh, &mut data.y3);
        bigint_to_16_chunks(&res_dl, &mut data.x3);
        Some(data)
    }
    pub fn execute(a: &[u64; 4], b: &[u64; 4], c: &[u64; 4]) -> ArithEqData {
        let mut data = Self::prepare(a, b, c, None, None).unwrap();
        for icol in 0..COLS {
            let index = icol as usize;
            data.eq[index][0] = equations::Arith256::calculate(
                icol, &data.x1, &data.y1, &data.x2, &data.x3, &data.y3,
            );

            let cin = if index > 0 { data.cout[index - 1][0] } else { 0 };
            let value = data.eq[index][0] + cin;
            if icol != COLS - 1 {
                data.cout[index][0] = value / 0x10000;
            }
            debug_assert!(
                0 == if icol == COLS - 1 { value } else { value % 0x10000 },
                "Arith256 residue ({index}) #:{value} cin:{cin}"
            );
        }
        data
    }
    #[cfg(feature = "test_data")]
    #[allow(dead_code)]
    pub fn verify(a: &[u64; 4], b: &[u64; 4], c: &[u64; 4], dl: &[u64; 4], dh: &[u64; 4]) {
        let data = Self::execute(a, b, c);
        data.check_ranges();
        for i in 0..2 {
            let offset = (i + 1) * 4 - 1;
            let mut _x3 = data.x3[offset] as u64;
            let mut _y3 = data.y3[offset] as u64;
            for j in 1..4 {
                _x3 <<= 16;
                _y3 <<= 16;
                _x3 += data.x3[offset - j] as u64;
                _y3 += data.y3[offset - j] as u64;
            }
            assert!(dl[i] == _x3, "Arith256 dl not match dh[{}]:{} != x3:{}", i, dl[i], _x3);
            assert!(dh[i] == _y3, "Arith256 dl not match dh[{}]:{} != y3:{}", i, dl[i], _y3);
        }
    }
}
