// code generated
//
// equation: d*T-DT-p*qdt+p*offset
//
// d: 168696
// p: 0x30644E72E131A029B85045B68181585D2833E84879B9709143E1F593F0000001
// offset: 0x10
// (p*offset): 0x30644E72E131A029B85045B68181585D2833E84879B9709143E1F593F00000010
//
// chunks:16
// chunk_bits:16
// terms_by_clock: 2

pub struct BabyJubJubDt {}

impl BabyJubJubDt {
    #[allow(clippy::too_many_arguments)]
    pub fn calculate(icol: u8, T: &[i64; 16], DT: &[i64; 16], qdt: &[i64; 16]) -> i64 {
        match icol {
            0 => 37624 * T[0] - DT[0] - qdt[0] + 0x10,
            1 => 2 * T[0] + 37624 * T[1] - DT[1] - 0xF000 * qdt[0] - qdt[1],
            2 => {
                2 * T[1] + 37624 * T[2] - DT[2] - 0xF593 * qdt[0] - 0xF000 * qdt[1] - qdt[2]
                    + 0x593F
            }
            3 => {
                2 * T[2] + 37624 * T[3]
                    - DT[3]
                    - 0x43E1 * qdt[0]
                    - 0xF593 * qdt[1]
                    - 0xF000 * qdt[2]
                    - qdt[3]
                    + 0x3E1F
            }
            4 => {
                2 * T[3] + 37624 * T[4]
                    - DT[4]
                    - 0x7091 * qdt[0]
                    - 0x43E1 * qdt[1]
                    - 0xF593 * qdt[2]
                    - 0xF000 * qdt[3]
                    - qdt[4]
                    + 0x914
            }
            5 => {
                2 * T[4] + 37624 * T[5]
                    - DT[5]
                    - 0x79B9 * qdt[0]
                    - 0x7091 * qdt[1]
                    - 0x43E1 * qdt[2]
                    - 0xF593 * qdt[3]
                    - 0xF000 * qdt[4]
                    - qdt[5]
                    + 0x9B97
            }
            6 => {
                2 * T[5] + 37624 * T[6]
                    - DT[6]
                    - 0xE848 * qdt[0]
                    - 0x79B9 * qdt[1]
                    - 0x7091 * qdt[2]
                    - 0x43E1 * qdt[3]
                    - 0xF593 * qdt[4]
                    - 0xF000 * qdt[5]
                    - qdt[6]
                    + 0x8487
            }
            7 => {
                2 * T[6] + 37624 * T[7]
                    - DT[7]
                    - 0x2833 * qdt[0]
                    - 0xE848 * qdt[1]
                    - 0x79B9 * qdt[2]
                    - 0x7091 * qdt[3]
                    - 0x43E1 * qdt[4]
                    - 0xF593 * qdt[5]
                    - 0xF000 * qdt[6]
                    - qdt[7]
                    + 0x833E
            }
            8 => {
                2 * T[7] + 37624 * T[8]
                    - DT[8]
                    - 0x585D * qdt[0]
                    - 0x2833 * qdt[1]
                    - 0xE848 * qdt[2]
                    - 0x79B9 * qdt[3]
                    - 0x7091 * qdt[4]
                    - 0x43E1 * qdt[5]
                    - 0xF593 * qdt[6]
                    - 0xF000 * qdt[7]
                    - qdt[8]
                    + 0x85D2
            }
            9 => {
                2 * T[8] + 37624 * T[9]
                    - DT[9]
                    - 0x8181 * qdt[0]
                    - 0x585D * qdt[1]
                    - 0x2833 * qdt[2]
                    - 0xE848 * qdt[3]
                    - 0x79B9 * qdt[4]
                    - 0x7091 * qdt[5]
                    - 0x43E1 * qdt[6]
                    - 0xF593 * qdt[7]
                    - 0xF000 * qdt[8]
                    - qdt[9]
                    + 0x1815
            }
            10 => {
                2 * T[9] + 37624 * T[10]
                    - DT[10]
                    - 0x45B6 * qdt[0]
                    - 0x8181 * qdt[1]
                    - 0x585D * qdt[2]
                    - 0x2833 * qdt[3]
                    - 0xE848 * qdt[4]
                    - 0x79B9 * qdt[5]
                    - 0x7091 * qdt[6]
                    - 0x43E1 * qdt[7]
                    - 0xF593 * qdt[8]
                    - 0xF000 * qdt[9]
                    - qdt[10]
                    + 0x5B68
            }
            11 => {
                2 * T[10] + 37624 * T[11]
                    - DT[11]
                    - 0xB850 * qdt[0]
                    - 0x45B6 * qdt[1]
                    - 0x8181 * qdt[2]
                    - 0x585D * qdt[3]
                    - 0x2833 * qdt[4]
                    - 0xE848 * qdt[5]
                    - 0x79B9 * qdt[6]
                    - 0x7091 * qdt[7]
                    - 0x43E1 * qdt[8]
                    - 0xF593 * qdt[9]
                    - 0xF000 * qdt[10]
                    - qdt[11]
                    + 0x8504
            }
            12 => {
                2 * T[11] + 37624 * T[12]
                    - DT[12]
                    - 0xA029 * qdt[0]
                    - 0xB850 * qdt[1]
                    - 0x45B6 * qdt[2]
                    - 0x8181 * qdt[3]
                    - 0x585D * qdt[4]
                    - 0x2833 * qdt[5]
                    - 0xE848 * qdt[6]
                    - 0x79B9 * qdt[7]
                    - 0x7091 * qdt[8]
                    - 0x43E1 * qdt[9]
                    - 0xF593 * qdt[10]
                    - 0xF000 * qdt[11]
                    - qdt[12]
                    + 0x29B
            }
            13 => {
                2 * T[12] + 37624 * T[13]
                    - DT[13]
                    - 0xE131 * qdt[0]
                    - 0xA029 * qdt[1]
                    - 0xB850 * qdt[2]
                    - 0x45B6 * qdt[3]
                    - 0x8181 * qdt[4]
                    - 0x585D * qdt[5]
                    - 0x2833 * qdt[6]
                    - 0xE848 * qdt[7]
                    - 0x79B9 * qdt[8]
                    - 0x7091 * qdt[9]
                    - 0x43E1 * qdt[10]
                    - 0xF593 * qdt[11]
                    - 0xF000 * qdt[12]
                    - qdt[13]
                    + 0x131A
            }
            14 => {
                2 * T[13] + 37624 * T[14]
                    - DT[14]
                    - 0x4E72 * qdt[0]
                    - 0xE131 * qdt[1]
                    - 0xA029 * qdt[2]
                    - 0xB850 * qdt[3]
                    - 0x45B6 * qdt[4]
                    - 0x8181 * qdt[5]
                    - 0x585D * qdt[6]
                    - 0x2833 * qdt[7]
                    - 0xE848 * qdt[8]
                    - 0x79B9 * qdt[9]
                    - 0x7091 * qdt[10]
                    - 0x43E1 * qdt[11]
                    - 0xF593 * qdt[12]
                    - 0xF000 * qdt[13]
                    - qdt[14]
                    + 0xE72E
            }
            15 => {
                2 * T[14] + 37624 * T[15]
                    - DT[15]
                    - 0x3064 * qdt[0]
                    - 0x4E72 * qdt[1]
                    - 0xE131 * qdt[2]
                    - 0xA029 * qdt[3]
                    - 0xB850 * qdt[4]
                    - 0x45B6 * qdt[5]
                    - 0x8181 * qdt[6]
                    - 0x585D * qdt[7]
                    - 0x2833 * qdt[8]
                    - 0xE848 * qdt[9]
                    - 0x79B9 * qdt[10]
                    - 0x7091 * qdt[11]
                    - 0x43E1 * qdt[12]
                    - 0xF593 * qdt[13]
                    - 0xF000 * qdt[14]
                    - qdt[15]
                    + 0x644
            }
            16 => {
                2 * T[15]
                    - 0x3064 * qdt[1]
                    - 0x4E72 * qdt[2]
                    - 0xE131 * qdt[3]
                    - 0xA029 * qdt[4]
                    - 0xB850 * qdt[5]
                    - 0x45B6 * qdt[6]
                    - 0x8181 * qdt[7]
                    - 0x585D * qdt[8]
                    - 0x2833 * qdt[9]
                    - 0xE848 * qdt[10]
                    - 0x79B9 * qdt[11]
                    - 0x7091 * qdt[12]
                    - 0x43E1 * qdt[13]
                    - 0xF593 * qdt[14]
                    - 0xF000 * qdt[15]
                    + 0x3
            }
            17 => {
                -0x3064 * qdt[2]
                    - 0x4E72 * qdt[3]
                    - 0xE131 * qdt[4]
                    - 0xA029 * qdt[5]
                    - 0xB850 * qdt[6]
                    - 0x45B6 * qdt[7]
                    - 0x8181 * qdt[8]
                    - 0x585D * qdt[9]
                    - 0x2833 * qdt[10]
                    - 0xE848 * qdt[11]
                    - 0x79B9 * qdt[12]
                    - 0x7091 * qdt[13]
                    - 0x43E1 * qdt[14]
                    - 0xF593 * qdt[15]
            }
            18 => {
                -0x3064 * qdt[3]
                    - 0x4E72 * qdt[4]
                    - 0xE131 * qdt[5]
                    - 0xA029 * qdt[6]
                    - 0xB850 * qdt[7]
                    - 0x45B6 * qdt[8]
                    - 0x8181 * qdt[9]
                    - 0x585D * qdt[10]
                    - 0x2833 * qdt[11]
                    - 0xE848 * qdt[12]
                    - 0x79B9 * qdt[13]
                    - 0x7091 * qdt[14]
                    - 0x43E1 * qdt[15]
            }
            19 => {
                -0x3064 * qdt[4]
                    - 0x4E72 * qdt[5]
                    - 0xE131 * qdt[6]
                    - 0xA029 * qdt[7]
                    - 0xB850 * qdt[8]
                    - 0x45B6 * qdt[9]
                    - 0x8181 * qdt[10]
                    - 0x585D * qdt[11]
                    - 0x2833 * qdt[12]
                    - 0xE848 * qdt[13]
                    - 0x79B9 * qdt[14]
                    - 0x7091 * qdt[15]
            }
            20 => {
                -0x3064 * qdt[5]
                    - 0x4E72 * qdt[6]
                    - 0xE131 * qdt[7]
                    - 0xA029 * qdt[8]
                    - 0xB850 * qdt[9]
                    - 0x45B6 * qdt[10]
                    - 0x8181 * qdt[11]
                    - 0x585D * qdt[12]
                    - 0x2833 * qdt[13]
                    - 0xE848 * qdt[14]
                    - 0x79B9 * qdt[15]
            }
            21 => {
                -0x3064 * qdt[6]
                    - 0x4E72 * qdt[7]
                    - 0xE131 * qdt[8]
                    - 0xA029 * qdt[9]
                    - 0xB850 * qdt[10]
                    - 0x45B6 * qdt[11]
                    - 0x8181 * qdt[12]
                    - 0x585D * qdt[13]
                    - 0x2833 * qdt[14]
                    - 0xE848 * qdt[15]
            }
            22 => {
                -0x3064 * qdt[7]
                    - 0x4E72 * qdt[8]
                    - 0xE131 * qdt[9]
                    - 0xA029 * qdt[10]
                    - 0xB850 * qdt[11]
                    - 0x45B6 * qdt[12]
                    - 0x8181 * qdt[13]
                    - 0x585D * qdt[14]
                    - 0x2833 * qdt[15]
            }
            23 => {
                -0x3064 * qdt[8]
                    - 0x4E72 * qdt[9]
                    - 0xE131 * qdt[10]
                    - 0xA029 * qdt[11]
                    - 0xB850 * qdt[12]
                    - 0x45B6 * qdt[13]
                    - 0x8181 * qdt[14]
                    - 0x585D * qdt[15]
            }
            24 => {
                -0x3064 * qdt[9]
                    - 0x4E72 * qdt[10]
                    - 0xE131 * qdt[11]
                    - 0xA029 * qdt[12]
                    - 0xB850 * qdt[13]
                    - 0x45B6 * qdt[14]
                    - 0x8181 * qdt[15]
            }
            25 => {
                -0x3064 * qdt[10]
                    - 0x4E72 * qdt[11]
                    - 0xE131 * qdt[12]
                    - 0xA029 * qdt[13]
                    - 0xB850 * qdt[14]
                    - 0x45B6 * qdt[15]
            }
            26 => {
                -0x3064 * qdt[11]
                    - 0x4E72 * qdt[12]
                    - 0xE131 * qdt[13]
                    - 0xA029 * qdt[14]
                    - 0xB850 * qdt[15]
            }
            27 => -0x3064 * qdt[12] - 0x4E72 * qdt[13] - 0xE131 * qdt[14] - 0xA029 * qdt[15],
            28 => -0x3064 * qdt[13] - 0x4E72 * qdt[14] - 0xE131 * qdt[15],
            29 => -0x3064 * qdt[14] - 0x4E72 * qdt[15],
            30 => -0x3064 * qdt[15],
            _ => 0,
        }
    }
}
