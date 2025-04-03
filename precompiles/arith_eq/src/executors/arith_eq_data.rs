#[derive(Debug, Default)]
pub struct ArithEqData {
    pub x1: [i64; 16],
    pub y1: [i64; 16],
    pub x2: [i64; 16],
    pub y2: [i64; 16],
    pub x3: [i64; 16],
    pub y3: [i64; 16],
    pub s: [i64; 16],
    pub q0: [i64; 16],
    pub q1: [i64; 16],
    pub q2: [i64; 16],
    pub eq: [[i64; 3]; 32],
    pub cout: [[i64; 3]; 31],
}

impl ArithEqData {
    #[cfg(feature = "test_data")]
    pub fn check_ranges(&self) {
        const MAX_16_BITS: i64 = 0xFFFF;
        const MAX_OVERLOAD_LAST_CHUNK: i64 = 0xF_FFFF;
        const MIN_CARRY: i64 = -(0x3F_FFFF);
        const MAX_CARRY: i64 = 0x40_0000;
        for i in 0..16 {
            let max_range = if i == 15 { MAX_OVERLOAD_LAST_CHUNK } else { MAX_16_BITS };
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
            for j in 0..2 {
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
