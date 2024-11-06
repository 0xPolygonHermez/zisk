use std::ops::Add;

const ROWS: usize = 95;
const FIRST_OP: u8 = 0xb0;

use log::info;
pub struct ArithTableHelpers;

impl ArithTableHelpers {
    pub fn get_row(op: u8, na: bool, nb: bool, np: bool, nr: bool, sext: bool) -> usize {
        static ARITH_TABLE_ROWS: [i16; 512] = [
            0, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
            -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
            -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
            -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
            -1, -1, -1, -1, -1, -1, -1, -1, 2, 3, -1, -1, -1, 4, -1, -1, -1, -1, -1, -1, -1, -1,
            -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 5, 6, 7, 8, -1,
            9, 10, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
            -1, -1, -1, -1, -1, 11, 12, 13, 14, -1, 15, 16, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
            -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 17, 18, 19, 20, -1, 21, 22,
            -1, -1, -1, -1, -1, -1, -1, -1, -1, 23, 24, 25, 26, -1, 27, 28, -1, -1, -1, -1, -1, -1,
            -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
            -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 29, -1, -1, -1, -1, -1, -1, -1, -1,
            -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
            -1, 30, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
            -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 31, 32, 33, 34, 35, 36, 37, -1, -1, -1, -1,
            -1, 38, 39, 40, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 41,
            42, 43, 44, 45, 46, 47, -1, -1, -1, -1, -1, 48, 49, 50, -1, -1, -1, -1, -1, -1, -1, -1,
            -1, -1, -1, -1, -1, -1, -1, -1, -1, 51, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
            -1, -1, -1, 52, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 53, -1, -1,
            -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 54, -1, -1, -1, -1, -1, -1, -1, -1,
            -1, -1, -1, -1, -1, -1, -1, 55, 56, 57, 58, 59, 60, 61, -1, -1, -1, -1, -1, 62, 63, 64,
            -1, 65, 66, 67, 68, 69, 70, 71, -1, -1, -1, -1, -1, 72, 73, 74, -1, 75, 76, 77, 78, 79,
            80, 81, -1, -1, -1, -1, -1, 82, 83, 84, -1, 85, 86, 87, 88, 89, 90, 91, -1, -1, -1, -1,
            -1, 92, 93, 94, -1,
        ];

        let index = (op - FIRST_OP) as u64 * 32
            + na as u64
            + nb as u64 * 2
            + np as u64 * 4
            + nr as u64 * 8
            + sext as u64 * 16;
        assert!(index < 512);
        let row = ARITH_TABLE_ROWS[index as usize];
        assert!(row >= 0);
        row as usize
    }
    pub fn get_max_row() -> usize {
        ROWS - 1
    }
}

pub struct ArithTableInputs {
    iter_row: u32,
    multiplicity: [u64; ROWS],
}

impl ArithTableInputs {
    pub fn new() -> Self {
        ArithTableInputs { iter_row: 0, multiplicity: [0; ROWS] }
    }
    pub fn add_use(&mut self, op: u8, na: bool, nb: bool, np: bool, nr: bool, sext: bool) {
        let row = ArithTableHelpers::get_row(op, na, nb, np, nr, sext);
        assert!(row < ROWS);
        self.multiplicity[row as usize] += 1;
        info!(
            "[ArithTableInputs]· add_use(op:{}, na:{}, nb:{}, np:{}, nr:{}, sext:{} row:{} multiplicity:{}",
            op, na, nb, np, nr, sext, row, self.multiplicity[row]
        );
    }
    pub fn multi_add_use(
        &mut self,
        times: usize,
        op: u8,
        na: bool,
        nb: bool,
        np: bool,
        nr: bool,
        sext: bool,
    ) {
        let row = ArithTableHelpers::get_row(op, na, nb, np, nr, sext);
        self.multiplicity[row as usize] += times as u64;
    }
    pub fn update_with(&mut self, other: &Self) {
        for i in 0..ROWS {
            self.multiplicity[i] += other.multiplicity[i];
        }
    }
    pub fn collect<F>(&self, call: F)
    where
        F: Fn(usize, u64),
    {
        for i in 0..ROWS {
            call(i, self.multiplicity[i] as u64);
        }
    }
}

pub struct ArithTableInputsIterator<'a> {
    iter_row: u32,
    inputs: &'a ArithTableInputs,
}

impl<'a> Iterator for ArithTableInputsIterator<'a> {
    type Item = (usize, u64);

    fn next(&mut self) -> Option<Self::Item> {
        while self.iter_row < ROWS as u32 && self.inputs.multiplicity[self.iter_row as usize] == 0 {
            self.iter_row += 1;
        }
        let row = self.iter_row as usize;
        if row < ROWS {
            self.iter_row += 1;
            return Some((row, self.inputs.multiplicity[row] as u64));
        }
        None
    }
}

impl<'a> IntoIterator for &'a ArithTableInputs {
    type Item = (usize, u64);
    type IntoIter = ArithTableInputsIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ArithTableInputsIterator { iter_row: 0, inputs: self }
    }
}
