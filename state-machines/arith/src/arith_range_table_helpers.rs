use std::collections::HashMap;

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
            value <=
                match range_type {
                    FULL => 0xFFFF,
                    POS => 0x7FFF,
                    NEG => 0xFFFF,
                    _ => panic!("Invalid range type"),
                }
        );
        OFFSETS[range_index as usize] * 0x8000 +
            if range_type == NEG { value - 0x8000 } else { value } as usize
    }
    pub fn get_row_carry_range_check(value: i64) -> usize {
        assert!(value >= -0xEFFFF);
        assert!(value <= 0xF0000);
        (0x220000 + 0xEFFFF + value) as usize
    }
}
pub struct ArithRangeTableInputs {
    // TODO: check improvement of multiplicity[64] to reserv only chunks used
    // with this 16 bits version, this table has aprox 8MB.
    updated: u64,
    multiplicity_overflow: HashMap<u32, u32>,
    multiplicity: Vec<u16>,
}

impl Default for ArithRangeTableInputs {
    fn default() -> Self {
        Self::new()
    }
}

impl ArithRangeTableInputs {
    pub fn new() -> Self {
        ArithRangeTableInputs {
            updated: 0,
            multiplicity_overflow: HashMap::new(),
            multiplicity: vec![0u16; ROWS],
        }
    }
    fn incr_row_one(&mut self, row: usize) {
        if self.multiplicity[row] > u16::MAX - 1 {
            let count = self.multiplicity_overflow.entry(row as u32).or_insert(0);
            *count += 1;
            self.multiplicity[row] = 0;
        } else {
            self.multiplicity[row] += 1;
        }
        self.updated &= 1 << (row >> (22 - 6));
    }
    fn incr_row(&mut self, row: usize, times: usize) {
        self.incr_row_without_update(row, times);
        self.updated &= 1 << (row >> (22 - 6));
    }
    fn incr_row_without_update(&mut self, row: usize, times: usize) {
        if (u16::MAX - self.multiplicity[row]) as usize <= times {
            let count = self.multiplicity_overflow.entry(row as u32).or_insert(0);
            let new_count = self.multiplicity[row] as u64 + times as u64;
            *count += (new_count >> 16) as u32;
            self.multiplicity[row] = (new_count & 0xFFFF) as u16;
        } else {
            self.multiplicity[row] += times as u16;
        }
    }
    pub fn use_chunk_range_check(&mut self, range_id: u8, value: u64) {
        let row = ArithRangeTableHelpers::get_row_chunk_range_check(range_id, value);
        self.incr_row_one(row);
    }
    pub fn use_carry_range_check(&mut self, value: i64) {
        let row = ArithRangeTableHelpers::get_row_carry_range_check(value);
        self.incr_row_one(row);
    }
    pub fn multi_use_chunk_range_check(&mut self, times: usize, range_id: u8, value: u64) {
        let row = ArithRangeTableHelpers::get_row_chunk_range_check(range_id, value);
        self.incr_row(row, times);
    }
    pub fn multi_use_carry_range_check(&mut self, times: usize, value: i64) {
        let row = ArithRangeTableHelpers::get_row_carry_range_check(value);
        self.incr_row(row, times);
    }
    pub fn update_with(&mut self, other: &Self) {
        let chunk_size = 1 << (22 - 6);
        for i_chunk in 0..64 {
            if (other.updated & (1 << i_chunk)) == 0 {
                continue;
            }
            let from = chunk_size * i_chunk;
            let to = from + chunk_size;
            for row in from..to {
                let count = other.multiplicity[row];
                if count > 0 {
                    self.incr_row_without_update(row, count as usize);
                }
            }
        }
        for (row, value) in other.multiplicity_overflow.iter() {
            let count = self.multiplicity_overflow.entry(*row).or_insert(0);
            *count += (*value) << 16;
        }
        self.updated |= other.updated;
    }
}

pub struct ArithRangeTableInputsIterator<'a> {
    iter_row: u32,
    iter_hash: bool,
    inputs: &'a ArithRangeTableInputs,
}

impl<'a> Iterator for ArithRangeTableInputsIterator<'a> {
    type Item = (usize, u64);

    fn next(&mut self) -> Option<Self::Item> {
        if !self.iter_hash {
            while self.iter_row < ROWS as u32 &&
                self.inputs.multiplicity[self.iter_row as usize] == 0
            {
                self.iter_row += 1;
            }
            let row = self.iter_row as usize;
            if row < ROWS {
                self.iter_row += 1;
                return Some((row, self.inputs.multiplicity[row] as u64));
            }
            self.iter_hash = true;
            self.iter_row = 0;
        }
        let res = self.inputs.multiplicity_overflow.iter().nth(self.iter_row as usize);
        match res {
            Some((row, value)) => {
                self.iter_row += 1;
                Some((*row as usize, (*value as u64) << 16))
            }
            None => None,
        }
    }
}

impl<'a> IntoIterator for &'a ArithRangeTableInputs {
    type Item = (usize, u64);
    type IntoIter = ArithRangeTableInputsIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ArithRangeTableInputsIterator { iter_row: 0, iter_hash: false, inputs: self }
    }
}

#[cfg(feature = "generate_code_arith_range_table")]
#[allow(dead_code)]
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
                offset += if range_loop == FULL { 2 } else { 1 };
            }
            index += 1;
        }
    }
    println!("const RANGES: [u8; 43] = [{}];", ranges);
    println!("const OFFSETS: [usize; 43] = {:?};", offsets);
}
