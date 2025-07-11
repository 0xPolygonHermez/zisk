use super::{ArithEqData, P_256_MASK};
use crate::equations;
use precompiles_helpers::{bigint_from_u64s, bigint_to_16_chunks, bigint_to_4_u64};

const COLS: u8 = 32;

pub struct Arith256Mod {}

impl Arith256Mod {
    #[allow(dead_code)]
    pub fn calculate(
        a: &[u64; 4],
        b: &[u64; 4],
        c: &[u64; 4],
        module: &[u64; 4],
        d: &mut [u64; 4],
    ) {
        Self::prepare(a, b, c, module, Some(d));
    }
    fn prepare(
        a: &[u64; 4],
        b: &[u64; 4],
        c: &[u64; 4],
        module: &[u64; 4],
        d: Option<&mut [u64; 4]>,
    ) -> Option<ArithEqData> {
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
        let q0 = &q & &*P_256_MASK;
        let q1 = &q >> 256;

        if let Some(d) = d {
            bigint_to_4_u64(&res, d);
            return None;
        }
        // x3 = mod(x1*y1+x2, y2)
        // a:x1 b:y1 c:x2 d: x3 module: y2
        let mut data = ArithEqData::default();
        bigint_to_16_chunks(&a, &mut data.x1);
        bigint_to_16_chunks(&b, &mut data.y1);
        bigint_to_16_chunks(&c, &mut data.x2);
        bigint_to_16_chunks(&module, &mut data.y2);
        bigint_to_16_chunks(&res, &mut data.x3);
        bigint_to_16_chunks(&q0, &mut data.q0);
        bigint_to_16_chunks(&q1, &mut data.q1);
        Some(data)
    }
    pub fn execute(a: &[u64; 4], b: &[u64; 4], c: &[u64; 4], module: &[u64; 4]) -> ArithEqData {
        let mut data = Self::prepare(a, b, c, module, None).unwrap();
        for icol in 0..COLS {
            let index = icol as usize;
            data.eq[index][0] = equations::Arith256Mod::calculate(
                icol, &data.x1, &data.y1, &data.x2, &data.y2, &data.x3, &data.q0, &data.q1,
            );

            let cin = if index > 0 { data.cout[index - 1][0] } else { 0 };
            let value = data.eq[index][0] + cin;
            if icol != COLS - 1 {
                data.cout[index][0] = value / 0x10000;
            }
            debug_assert!(
                0 == if icol == COLS - 1 { value } else { value % 0x10000 },
                "Arith256Mod residue ({index}) #:{value} cin:{cin}"
            );
        }
        data
    }
    #[cfg(feature = "test_data")]
    #[allow(dead_code)]
    pub fn verify(a: &[u64; 4], b: &[u64; 4], c: &[u64; 4], module: &[u64; 4], d: &[u64; 4]) {
        let data = Self::execute(a, b, c, module);
        data.check_ranges();
        for (i, chunk_d) in d.iter().enumerate() {
            let offset = (i + 1) * 4 - 1;
            let mut x3 = data.x3[offset] as u64;
            for j in 1..4 {
                x3 <<= 16;
                x3 += data.x3[offset - j] as u64;
            }
            assert!(*chunk_d == x3, "Arith256Mod dh[{}]:{} not match with x3:{}", i, chunk_d, x3);
        }
    }
}
