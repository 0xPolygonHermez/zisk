use crate::{ARITH_EQ_384_CHUNKS, ARITH_EQ_384_CHUNKS_DOUBLE, ARITH_EQ_384_MAX_CEQS};

#[derive(Debug)]
pub struct ArithEq384Data {
    pub x1: [i64; ARITH_EQ_384_CHUNKS],
    pub y1: [i64; ARITH_EQ_384_CHUNKS],
    pub x2: [i64; ARITH_EQ_384_CHUNKS],
    pub y2: [i64; ARITH_EQ_384_CHUNKS],
    pub x3: [i64; ARITH_EQ_384_CHUNKS],
    pub y3: [i64; ARITH_EQ_384_CHUNKS],
    pub s: [i64; ARITH_EQ_384_CHUNKS],
    pub q0: [i64; ARITH_EQ_384_CHUNKS],
    pub q1: [i64; ARITH_EQ_384_CHUNKS],
    pub q2: [i64; ARITH_EQ_384_CHUNKS],
    pub eq: [[i64; ARITH_EQ_384_MAX_CEQS]; ARITH_EQ_384_CHUNKS_DOUBLE],
    pub cout: [[i64; ARITH_EQ_384_MAX_CEQS]; ARITH_EQ_384_CHUNKS_DOUBLE - 1],
}

impl Default for ArithEq384Data {
    fn default() -> Self {
        Self {
            x1: [0; ARITH_EQ_384_CHUNKS],
            y1: [0; ARITH_EQ_384_CHUNKS],
            x2: [0; ARITH_EQ_384_CHUNKS],
            y2: [0; ARITH_EQ_384_CHUNKS],
            x3: [0; ARITH_EQ_384_CHUNKS],
            y3: [0; ARITH_EQ_384_CHUNKS],
            s: [0; ARITH_EQ_384_CHUNKS],
            q0: [0; ARITH_EQ_384_CHUNKS],
            q1: [0; ARITH_EQ_384_CHUNKS],
            q2: [0; ARITH_EQ_384_CHUNKS],
            eq: [[0; ARITH_EQ_384_MAX_CEQS]; ARITH_EQ_384_CHUNKS_DOUBLE],
            cout: [[0; ARITH_EQ_384_MAX_CEQS]; ARITH_EQ_384_CHUNKS_DOUBLE - 1],
        }
    }
}

// TODO: carry min and max!!
impl ArithEq384Data {
    #[cfg(feature = "test_data")]
    pub fn check_ranges(&self) {
        const MAX_16_BITS: i64 = 0xFFFF;
        const MAX_OVERLOAD_LAST_CHUNK: i64 = 0xF_FFFF;
        const MIN_CARRY: i64 = -(0x3F_FFFF);
        const MAX_CARRY: i64 = 0x40_0000;
        for i in 0..ARITH_EQ_384_CHUNKS {
            let max_range =
                if i == (ARITH_EQ_384_CHUNKS - 1) { MAX_OVERLOAD_LAST_CHUNK } else { MAX_16_BITS };
            assert!(self.x1[i] >= 0 && self.x1[i] <= MAX_16_BITS);
            assert!(self.y1[i] >= 0 && self.y1[i] <= MAX_16_BITS);
            assert!(self.x2[i] >= 0 && self.x2[i] <= MAX_16_BITS);
            assert!(self.y2[i] >= 0 && self.y2[i] <= MAX_16_BITS);
            assert!(self.x3[i] >= 0 && self.x3[i] <= MAX_16_BITS);
            assert!(self.y3[i] >= 0 && self.y3[i] <= MAX_16_BITS);
            assert!(self.x1[i] >= 0 && self.x1[i] <= MAX_16_BITS);
            assert!(self.x1[i] >= 0 && self.x1[i] <= MAX_16_BITS);
            assert!(self.s[i] >= 0 && self.s[i] <= max_range);
            assert!(self.q0[i] >= 0 && self.q0[i] <= max_range);
            assert!(self.q1[i] >= 0 && self.q1[i] <= max_range);
            assert!(self.q2[i] >= 0 && self.q2[i] <= max_range);
            for j in 0..ARITH_EQ_384_MAX_CEQS {
                assert!(
                    self.cout[i][j] >= MIN_CARRY && self.cout[i][j] <= MAX_CARRY,
                    "cout[{}][{}]:{} not in [{},{}]",
                    i,
                    j,
                    self.cout[i][j],
                    MIN_CARRY,
                    MAX_CARRY
                );
            }
        }
    }
}
