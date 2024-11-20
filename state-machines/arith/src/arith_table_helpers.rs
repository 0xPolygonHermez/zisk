pub struct ArithTableHelpers;

#[cfg(debug_assertions)]
use crate::ARITH_TABLE;

use crate::{ARITH_TABLE_ROWS, FIRST_OP, ROWS};

impl ArithTableHelpers {
    #[allow(clippy::too_many_arguments)]
    pub fn direct_get_row(
        op: u8,
        na: bool,
        nb: bool,
        np: bool,
        nr: bool,
        sext: bool,
        div_by_zero: bool,
        div_overflow: bool,
    ) -> usize {
        let index = (op - FIRST_OP) as u64 * 128
            + na as u64
            + nb as u64 * 2
            + np as u64 * 4
            + nr as u64 * 8
            + sext as u64 * 16
            + div_by_zero as u64 * 32
            + div_overflow as u64 * 64;
        assert!(index < ARITH_TABLE_ROWS.len() as u64);
        let row = ARITH_TABLE_ROWS[index as usize];
        assert!(
            row < 255,
            "INVALID ROW row:{} op:0x{:x} na:{} nb:{} np:{} nr:{} sext:{} div_by_zero:{} div_overflow:{} index:{}",
            row,
            op,
            na as u8,
            nb as u8,
            np as u8,
            nr as u8,
            sext as u8,
            div_by_zero as u8,
            div_overflow as u8,
            index
        );
        row as usize
    }
    #[cfg(not(debug_assertions))]
    pub fn get_row(
        op: u8,
        na: bool,
        nb: bool,
        np: bool,
        nr: bool,
        sext: bool,
        div_by_zero: bool,
        div_overflow: bool,
    ) -> usize {
        Self::direct_get_row(op, na, nb, np, nr, sext, div_by_zero, div_overflow)
    }
    #[cfg(debug_assertions)]
    #[allow(clippy::too_many_arguments)]
    pub fn get_row(
        op: u8,
        na: bool,
        nb: bool,
        np: bool,
        nr: bool,
        sext: bool,
        div_by_zero: bool,
        div_overflow: bool,
        m32: bool,
        div: bool,
        main_mul: bool,
        main_div: bool,
        signed: bool,
        range_ab: u16,
        range_cd: u16,
    ) -> usize {
        let flags = if m32 { 1 } else { 0 }
            + if div { 2 } else { 0 }
            + if na { 4 } else { 0 }
            + if nb { 8 } else { 0 }
            + if np { 16 } else { 0 }
            + if nr { 32 } else { 0 }
            + if sext { 64 } else { 0 }
            + if div_by_zero { 128 } else { 0 }
            + if div_overflow { 256 } else { 0 }
            + if main_mul { 512 } else { 0 }
            + if main_div { 1024 } else { 0 }
            + if signed { 2048 } else { 0 };
        let row = Self::direct_get_row(op, na, nb, np, nr, sext, div_by_zero, div_overflow);
        assert_eq!(
            op as u16, ARITH_TABLE[row][0],
            "at row {} not match op {} vs {}",
            row, op, ARITH_TABLE[row][0]
        );
        assert_eq!(
            flags, ARITH_TABLE[row][1],
            "at row {0} op:0x{1:x}({1}) not match flags {2:b}({2}) vs {3:b}({3})",
            row, op, flags, ARITH_TABLE[row][1]
        );
        assert_eq!(
            range_ab, ARITH_TABLE[row][2],
            "at row {} op:{} not match range_ab {} vs {}",
            row, op, flags, ARITH_TABLE[row][2]
        );
        assert_eq!(
            range_cd, ARITH_TABLE[row][3],
            "at row {} op:{} not match range_cd {} vs {}",
            row, op, flags, ARITH_TABLE[row][3]
        );
        row
    }

    pub fn flags_to_string(flags: u16) -> String {
        let mut result = String::new();
        if flags & 1 != 0 {
            result += " m32";
        }
        if flags & 2 != 0 {
            result += " div";
        }
        if flags & 4 != 0 {
            result += " na";
        }
        if flags & 8 != 0 {
            result += " nb";
        }
        if flags & 16 != 0 {
            result += " np";
        }
        if flags & 32 != 0 {
            result += " nr";
        }
        if flags & 64 != 0 {
            result += " sext";
        }
        if flags & 128 != 0 {
            result += " div_by_zero";
        }
        if flags & 256 != 0 {
            result += " div_overflow";
        }
        if flags & 512 != 0 {
            result += " main_mul";
        }
        if flags & 1024 != 0 {
            result += " main_div";
        }
        if flags & 2048 != 0 {
            result += " signed";
        }
        result
    }

    pub fn get_max_row() -> usize {
        ROWS - 1
    }
}

pub struct ArithTableInputs {
    multiplicity: [u64; ROWS],
}

impl Default for ArithTableInputs {
    fn default() -> Self {
        Self::new()
    }
}

impl ArithTableInputs {
    pub fn new() -> Self {
        ArithTableInputs { multiplicity: [0; ROWS] }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn add_use(
        &mut self,
        op: u8,
        na: bool,
        nb: bool,
        np: bool,
        nr: bool,
        sext: bool,
        div_by_zero: bool,
        div_overflow: bool,
    ) {
        let row =
            ArithTableHelpers::direct_get_row(op, na, nb, np, nr, sext, div_by_zero, div_overflow);
        assert!(row < ROWS);
        self.multiplicity[row] += 1;
    }
    #[allow(clippy::too_many_arguments)]
    pub fn multi_add_use(
        &mut self,
        times: usize,
        op: u8,
        na: bool,
        nb: bool,
        np: bool,
        nr: bool,
        sext: bool,
        div_by_zero: bool,
        div_overflow: bool,
    ) {
        let row =
            ArithTableHelpers::direct_get_row(op, na, nb, np, nr, sext, div_by_zero, div_overflow);
        self.multiplicity[row] += times as u64;
    }
    pub fn update_with(&mut self, other: &Self) {
        for i in 0..ROWS {
            self.multiplicity[i] += other.multiplicity[i];
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
            Some((row, self.inputs.multiplicity[row]))
        } else {
            None
        }
    }
}

impl<'a> IntoIterator for &'a ArithTableInputs {
    type Item = (usize, u64);
    type IntoIter = ArithTableInputsIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ArithTableInputsIterator { iter_row: 0, inputs: self }
    }
}
