use std::{cell::RefCell, fmt, rc::Rc};

use goldilocks::{AbstractField, DeserializeField};
use proofman::trace::trace_pol::TracePol;

use crate::RomProgramLine;

use super::Registerable;

pub struct RegisterN2<'a, T, const N: usize> {
    name: String,
    value: [T; N],
    value_col: &'a mut TracePol<[T; N]>,
    in_col: &'a mut TracePol<T>,
    set_col: Option<&'a mut TracePol<T>>,
    in_rom: String,
    set_rom: Option<String>,
}

impl<'a, T: AbstractField + Copy, const N: usize> RegisterN2<'a, T, N> {
    pub fn new(
        name: &str,
        value_col: &'a mut TracePol<[T; N]>,
        in_col: &'a mut TracePol<T>,
        set_col: &'a mut TracePol<T>,
        in_rom: &str,
        set_rom: &str,
    ) -> Self {
        RegisterN2 {
            name: name.into(),
            value: [T::zero(); N],
            value_col,
            in_col,
            set_col: Some(set_col),
            in_rom: in_rom.into(),
            set_rom: Some(set_rom.to_owned()),
        }
    }
}

pub struct RegisterN<T, const N: usize> {
    name: String,
    value: [T; N],
    value_col: Rc<RefCell<TracePol<[T; N]>>>,
    in_col: Rc<RefCell<TracePol<T>>>,
    set_col: Option<Rc<RefCell<TracePol<T>>>>,
    in_rom: String,
    set_rom: Option<String>,
}

impl<T: AbstractField + Copy, const N: usize> RegisterN<T, N> {
    pub fn new(
        name: &str,
        value_col: Rc<RefCell<TracePol<[T; N]>>>,
        in_col: Rc<RefCell<TracePol<T>>>,
        set_col: Rc<RefCell<TracePol<T>>>,
        in_rom: &str,
        set_rom: &str,
    ) -> Self {
        RegisterN {
            name: name.into(),
            value: [T::zero(); N],
            value_col,
            in_col,
            set_col: Some(set_col),
            in_rom: in_rom.into(),
            set_rom: Some(set_rom.to_owned()),
        }
    }

    pub fn new_ro(
        name: &str,
        value_col: Rc<RefCell<TracePol<[T; N]>>>,
        in_col: Rc<RefCell<TracePol<T>>>,
        in_rom: &str,
    ) -> Self {
        RegisterN {
            name: name.into(),
            value: [T::zero(); N],
            value_col,
            in_col,
            set_col: None,
            in_rom: in_rom.into(),
            set_rom: None,
        }
    }

    pub fn rotate_left(&self) -> [T; N] {
        let mut result = [T::zero(); N];
        result[0] = self.value[N - 1];
        for i in 1..N {
            result[i] = self.value[i - 1];
        }
        result
    }
}

impl<T: AbstractField + DeserializeField + Copy, const N: usize> Registerable<[T; N], T> for RegisterN<T, N> {
    fn reset(&mut self, row: usize) {
        self.reset_value();
        self.update_cols(row);
    }

    fn get_in_value(&mut self, row: usize, rom_line: &RomProgramLine) -> Option<[T; N]> {
        if !rom_line.program_line.contains_key(&self.in_rom) {
            self.in_col.borrow_mut()[row] = T::zero();
            return None;
        }

        let in_col_value = T::from_string(rom_line.program_line[&self.in_rom].as_str()?, 10);
        self.in_col.borrow_mut()[row] = in_col_value;

        Some(self.apply_in_to_value(in_col_value))
    }

    fn apply_set_value(&mut self, row: usize, rom_line: &RomProgramLine, value: [T; N]) {
        if self.set_rom.is_none() || !rom_line.program_line.contains_key(self.set_rom.as_ref().unwrap()) {
            if self.set_col.is_some() {
                self.set_col.as_ref().unwrap().borrow_mut()[row] = T::zero();
            }
            self.update_cols(row);
            return;
        }

        assert!(self.set_col.is_some(), "Couldn't set value for register {}", self.name);
        self.set_col.as_ref().unwrap().borrow_mut()[row] = T::one();
        self.update_value(row, value);
    }

    fn apply_in_to_value(&self, in_col_value: T) -> [T; N] {
        let mut result = [T::zero(); N];
        for i in 0..N {
            result[i] = in_col_value * self.value[i];
        }

        result
    }

    fn update_cols(&mut self, row: usize) {
        for i in 0..N {
            self.value_col.borrow_mut()[row][i] = self.value[i];
        }
    }

    fn update_value(&mut self, row: usize, value: [T; N]) {
        //TODO! review this
        self.value = value;
        self.update_cols(row);
    }

    fn set_value(&mut self, value: [T; N]) {
        self.value = value;
    }

    fn reset_value(&mut self) {
        self.value = [T::zero(); N];
    }

    fn get_value(&self) -> [T; N] {
        self.value.clone()
    }

    fn get_col_value(&self, row: usize) -> [T; N] {
        self.value_col.borrow_mut()[row].clone()
    }
}

impl<T: fmt::Debug + fmt::Display, const N: usize> fmt::Display for RegisterN<T, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}: {:?}", self.name, self.value)
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use goldilocks::{AbstractField, Goldilocks};
    use serde_json::{Map, Value};

    use crate::register::Registerable;

    use super::RegisterN;
    use proofman::trace::trace_pol::TracePol;

    #[test]
    fn register_n_working() {
        const NUM_ROWS: usize = 16;
        const N: usize = 8;

        let mut trace_col: TracePol<[Goldilocks; N]> = TracePol::new(NUM_ROWS);
        let in_col: TracePol<Goldilocks> = TracePol::new(NUM_ROWS);
        let set_col: TracePol<Goldilocks> = TracePol::new(NUM_ROWS);

        // Initialize the trace
        for i in 0..NUM_ROWS {
            for j in 0..N {
                trace_col[i][j] = Goldilocks::from_canonical_usize(i * NUM_ROWS + j);
            }
        }

        let trace_col = Rc::new(RefCell::new(trace_col));
        let in_col = Rc::new(RefCell::new(in_col));
        let set_col = Rc::new(RefCell::new(set_col));

        let mut register = RegisterN::<Goldilocks, N>::new("A", trace_col, in_col, set_col, "inA", "setA");

        let mut val = [Goldilocks::from_canonical_u64(64); N];
        for i in 0..N {
            val[i] = Goldilocks::from_canonical_usize(i * 100);
        }
        register.set_value(val);
        assert_eq!(register.get_value(), val);
        register.reset_value();
        assert_eq!(register.get_value(), [Goldilocks::zero(); N]);

        let row = 2;
        let val = register.get_col_value(row);
        let mut to_compare = [Goldilocks::from_canonical_u64(64); N];
        for i in 0..N {
            to_compare[i] = Goldilocks::from_canonical_usize(row * NUM_ROWS + i);
        }

        assert_eq!(val, to_compare);

        let val22 = [Goldilocks::from_canonical_usize(22); N];
        register.update_value(2, val22);
        let val = register.get_col_value(2);
        assert_eq!(val, val22);

        let mut program_line = Map::new();
        program_line.insert("inA".to_owned(), Value::String("123".to_owned()));
        let rom_line = crate::RomProgramLine {
            line: 5,
            file_name: "rom.zkasm".to_owned(),
            line_str: "        STEP => A".to_owned(),
            program_line,
        };
        let in_value = register.get_in_value(5, &rom_line);
        assert_eq!(in_value, Some([val22[0] * Goldilocks::from_canonical_u64(123); N]));
    }

    #[test]
    fn register_ro_n_working() {
        const NUM_ROWS: usize = 16;
        const N: usize = 8;

        let mut trace_col: TracePol<[Goldilocks; N]> = TracePol::new(NUM_ROWS);
        let in_col: TracePol<Goldilocks> = TracePol::new(NUM_ROWS);

        // Initialize the trace
        for i in 0..NUM_ROWS {
            for j in 0..N {
                trace_col[i][j] = Goldilocks::from_canonical_usize(i * NUM_ROWS + j);
            }
        }

        let trace_col = Rc::new(RefCell::new(trace_col));
        let in_col = Rc::new(RefCell::new(in_col));

        let mut register = RegisterN::<Goldilocks, N>::new_ro("A", trace_col, in_col, "inA");

        let mut val = [Goldilocks::from_canonical_u64(64); N];
        for i in 0..N {
            val[i] = Goldilocks::from_canonical_usize(i * 100);
        }
        register.set_value(val);
        assert_eq!(register.get_value(), val);
        register.reset_value();
        assert_eq!(register.get_value(), [Goldilocks::zero(); N]);

        let row = 2;
        let val = register.get_col_value(row);
        let mut to_compare = [Goldilocks::from_canonical_u64(64); N];
        for i in 0..N {
            to_compare[i] = Goldilocks::from_canonical_usize(row * NUM_ROWS + i);
        }

        assert_eq!(val, to_compare);

        let val22 = [Goldilocks::from_canonical_usize(22); N];
        register.update_value(2, val22);
        let val = register.get_col_value(2);
        assert_eq!(val, val22);

        let mut program_line = Map::new();
        program_line.insert("inA".to_owned(), Value::String("123".to_owned()));
        let rom_line = crate::RomProgramLine {
            line: 5,
            file_name: "rom.zkasm".to_owned(),
            line_str: "        STEP => A".to_owned(),
            program_line,
        };
        let in_value = register.get_in_value(5, &rom_line);
        assert_eq!(in_value, Some([val22[0] * Goldilocks::from_canonical_u64(123); N]));
    }

    #[test]
    fn register_edge_cases() {
        const NUM_ROWS: usize = 16;

        let trace_col: TracePol<[Goldilocks; 1]> = TracePol::new(NUM_ROWS);
        let in_col: TracePol<Goldilocks> = TracePol::new(NUM_ROWS);
        let set_col: TracePol<Goldilocks> = TracePol::new(NUM_ROWS);

        let trace_col = Rc::new(RefCell::new(trace_col));
        let in_col = Rc::new(RefCell::new(in_col));
        let set_col = Rc::new(RefCell::new(set_col));

        let mut register = RegisterN::<Goldilocks, 1>::new("A", trace_col, in_col, set_col, "inA", "setA");

        // Test empty program line
        let rom_line = crate::RomProgramLine {
            line: 0,
            file_name: "empty".to_owned(),
            line_str: "".to_owned(),
            program_line: Map::new(),
        };

        assert_eq!(register.get_in_value(0, &rom_line), None);

        // Test with a program line not containing `inA`
        let mut program_line = Map::new();
        program_line.insert("other".to_owned(), Value::String("123".to_owned()));
        let rom_line = crate::RomProgramLine {
            line: 0,
            file_name: "rom.zkasm".to_owned(),
            line_str: "        STEP => A".to_owned(),
            program_line,
        };

        assert_eq!(register.get_in_value(0, &rom_line), None);
    }
}
