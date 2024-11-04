use std::ops::Add;

const ROWS: usize = 1 << 22;
const FULL: u8 = 0x00;
const POS: u8 = 0x01;
const NEG: u8 = 0x02;
pub struct ArithRangeTableHelpers;

const RANGES: [u8; 43] = [
    FULL, FULL, FULL, POS, POS, POS, NEG, NEG, NEG, FULL, FULL, FULL, FULL, FULL, FULL, FULL, FULL,
    FULL, POS, NEG, FULL, POS, NEG, FULL, POS, NEG, FULL, FULL, FULL, FULL, FULL, FULL, FULL, FULL,
    FULL, FULL, FULL, POS, POS, POS, NEG, NEG, NEG,
];
const OFFSETS: [usize; 43] = [
    0, 2, 4, 50, 51, 52, 59, 60, 61, 6, 8, 10, 12, 14, 16, 18, 20, 22, 53, 62, 24, 54, 63, 26, 55,
    64, 28, 30, 32, 34, 36, 38, 40, 42, 44, 46, 48, 56, 57, 58, 65, 66, 67,
];

impl ArithRangeTableHelpers {
    pub fn get_range_name(range_index: u8) -> &'static str {
        match range_index {
            0 => "F  F  F  F",
            1 => "F  F  +  F",
            2 => "F  F  -  F",
            3 => "+  F  F  F",
            4 => "+  F  +  F",
            5 => "+  F  -  F",
            6 => "-  F  F  F",
            7 => "-  F  +  F",
            8 => "-  F  -  F",
            9 => "F  F  F  +",
            10 => "F  F  F  -",
            11 => "F  +  F  F",
            12 => "F  +  F  +",
            13 => "F  +  F  -",
            14 => "F  -  F  F",
            15 => "F  -  F  +",
            16 => "F  -  F  -",
            _ => panic!("Invalid range index"),
        }
    }
    pub fn get_row_chunk_range_check(range_index: u8, value: u64) -> usize {
        // F F F + + + - - - F F F F F F F F F + - F + - F + - F F F F F F F F F F F + + + - - -
        let range_type = RANGES[range_index as usize];
        assert!(range_index < 43);
        assert!(value >= if range_type == NEG { 0x8000 } else { 0 });
        assert!(
            value
                <= match range_type {
                    FULL => 0xFFFF,
                    POS => 0x7FFF,
                    NEG => 0xFFFF,
                    _ => panic!("Invalid range type"),
                }
        );
        OFFSETS[range_index as usize] * 0x8000
            + if range_type == NEG { value - 0x8000 } else { value } as usize
    }
    pub fn get_row_carry_range_check(value: i64) -> usize {
        assert!(value >= -0xEFFFF);
        assert!(value <= 0xF0000);
        (0x220000 + 0xEFFFF + value) as usize
    }
}
pub struct ArithRangeTableInputs {
    multiplicity: [u64; ROWS],
}
impl ArithRangeTableInputs {
    pub fn new() -> Self {
        ArithRangeTableInputs { multiplicity: [0; ROWS] }
    }
    pub fn use_chunk_range_check(&mut self, range_id: u8, value: u64) {
        let row = ArithRangeTableHelpers::get_row_chunk_range_check(range_id, value);
        self.multiplicity[row as usize] += 1;
    }
    pub fn use_carry_range_check(&mut self, value: i64) {
        let row = ArithRangeTableHelpers::get_row_carry_range_check(value);
        self.multiplicity[row as usize] += 1;
    }
    pub fn multi_use_chunk_range_check(&mut self, times: usize, range_id: u8, value: u64) {
        let row = ArithRangeTableHelpers::get_row_chunk_range_check(range_id, value);
        self.multiplicity[row as usize] += times as u64;
    }
    pub fn multi_use_carry_range_check(&mut self, times: usize, value: i64) {
        let row = ArithRangeTableHelpers::get_row_carry_range_check(value);
        self.multiplicity[row as usize] += times as u64;
    }
    pub fn update_with(&mut self, other: &Self) {
        for i in 0..ROWS {
            self.multiplicity[i] += other.multiplicity[i];
        }
    }
}

impl Add for ArithRangeTableInputs {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let mut result = ArithRangeTableInputs::new();
        for i in 0..ROWS {
            result.multiplicity[i] = self.multiplicity[i] + other.multiplicity[i];
        }
        result
    }
}

#[cfg(feature = "generate_code_arith_range_table")]
fn generate_table() {
    let pattern = "FFF+++---FFFFFFFFF+-F+-F+-FFFFFFFFFFF+++---";
    // let mut ranges = [0u8; 43];
    let mut ranges = String::new();
    let mut offsets = [0usize; 43];
    let mut offset = 0;
    for range_loop in [FULL, POS, NEG] {
        let mut index = 0;
        for c in pattern.chars() {
            if c == ' ' || c == '_' {
                continue;
            }
            let range_id = match c {
                'F' => FULL,
                '+' => POS,
                '-' => NEG,
                _ => panic!("Invalid character in pattern"),
            };
            if range_loop == FULL {
                if index > 0 {
                    ranges.push_str(", ")
                }
                ranges.push_str(match range_id {
                    FULL => "FULL",
                    POS => "POS",
                    _ => "NEG",
                });
                // ranges[index] = range_id
            }
            if range_loop == range_id {
                offsets[index] = offset;
                offset = offset + if range_loop == FULL { 2 } else { 1 };
            }
            index += 1;
        }
    }
    println!("const RANGES: [u8; 43] = [{}];", ranges);
    println!("const OFFSETS: [usize; 43] = {:?};", offsets);
}
