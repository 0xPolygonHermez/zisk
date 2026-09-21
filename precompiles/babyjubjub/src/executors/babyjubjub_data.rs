/// Per-operation witness data for one BabyJubJub point addition.
///
/// Coordinates and scratch values are stored as 16 little-endian 16-bit chunks.
/// `eq[icol]` holds the 7 equation residues at column `icol`; `cout[icol]` holds
/// their carries to the next column. Equation order:
///   0: A = x1*x2, 1: B = y1*y2, 2: N = x1*y2 + y1*x2, 3: T = A*B (= tau),
///   4: DT = d*T, 5: x3, 6: y3.
#[derive(Debug, Default)]
pub struct BabyJubJubData {
    pub x1: [i64; 16],
    pub y1: [i64; 16],
    pub x2: [i64; 16],
    pub y2: [i64; 16],
    pub x3: [i64; 16],
    pub y3: [i64; 16],
    pub a: [i64; 16],
    pub b: [i64; 16],
    pub n: [i64; 16],
    pub t: [i64; 16],
    pub dt: [i64; 16],
    pub qa: [i64; 16],
    pub qb: [i64; 16],
    pub qn: [i64; 16],
    pub qt: [i64; 16],
    pub qdt: [i64; 16],
    pub qx: [i64; 16],
    pub qy: [i64; 16],
    pub eq: [[i64; 7]; 32],
    pub cout: [[i64; 7]; 31],
}

impl BabyJubJubData {
    #[cfg(any(test, feature = "test_data"))]
    pub fn check_ranges(&self) {
        const MAX_16_BITS: i64 = 0xFFFF;
        const MAX_OVERLOAD_LAST_CHUNK: i64 = 0xF_FFFF;
        const MIN_CARRY: i64 = -(0x3F_FFFF);
        const MAX_CARRY: i64 = 0x40_0000;
        for i in 0..16 {
            let max_range = if i == 15 { MAX_OVERLOAD_LAST_CHUNK } else { MAX_16_BITS };
            assert!(self.x1[i] >= 0 && self.x1[i] <= MAX_16_BITS, "x1[{i}]:{}", self.x1[i]);
            assert!(self.y1[i] >= 0 && self.y1[i] <= MAX_16_BITS, "y1[{i}]:{}", self.y1[i]);
            assert!(self.x2[i] >= 0 && self.x2[i] <= MAX_16_BITS, "x2[{i}]:{}", self.x2[i]);
            assert!(self.y2[i] >= 0 && self.y2[i] <= MAX_16_BITS, "y2[{i}]:{}", self.y2[i]);
            assert!(self.x3[i] >= 0 && self.x3[i] <= MAX_16_BITS, "x3[{i}]:{}", self.x3[i]);
            assert!(self.y3[i] >= 0 && self.y3[i] <= MAX_16_BITS, "y3[{i}]:{}", self.y3[i]);
            assert!(self.a[i] >= 0 && self.a[i] <= max_range, "a[{i}]:{}", self.a[i]);
            assert!(self.b[i] >= 0 && self.b[i] <= max_range, "b[{i}]:{}", self.b[i]);
            assert!(self.n[i] >= 0 && self.n[i] <= max_range, "n[{i}]:{}", self.n[i]);
            assert!(self.t[i] >= 0 && self.t[i] <= max_range, "t[{i}]:{}", self.t[i]);
            assert!(self.dt[i] >= 0 && self.dt[i] <= max_range, "dt[{i}]:{}", self.dt[i]);
            assert!(self.qa[i] >= 0 && self.qa[i] <= max_range, "qa[{i}]:{}", self.qa[i]);
            assert!(self.qb[i] >= 0 && self.qb[i] <= max_range, "qb[{i}]:{}", self.qb[i]);
            assert!(self.qn[i] >= 0 && self.qn[i] <= max_range, "qn[{i}]:{}", self.qn[i]);
            assert!(self.qt[i] >= 0 && self.qt[i] <= max_range, "qt[{i}]:{}", self.qt[i]);
            assert!(self.qdt[i] >= 0 && self.qdt[i] <= max_range, "qdt[{i}]:{}", self.qdt[i]);
            assert!(self.qx[i] >= 0 && self.qx[i] <= max_range, "qx[{i}]:{}", self.qx[i]);
            assert!(self.qy[i] >= 0 && self.qy[i] <= max_range, "qy[{i}]:{}", self.qy[i]);
        }
        for i in 0..31 {
            for j in 0..7 {
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
