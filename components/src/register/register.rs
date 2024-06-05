use std::{cell::RefCell, fmt, rc::Rc};

use goldilocks::{AbstractField, DeserializeField};
use proofman::trace::trace_pol::TracePol;

use crate::RomProgramLine;

use super::Registerable;

pub struct Register<T> {
    name: String,
    value: T,
    value_col: Rc<RefCell<TracePol<T>>>,
    in_col: Rc<RefCell<TracePol<T>>>,
    set_col: Option<Rc<RefCell<TracePol<T>>>>,
    in_rom: String,
    set_rom: Option<String>,
}

impl<T: AbstractField + Copy> Register<T> {
    pub fn new(
        name: &str,
        value_col: Rc<RefCell<TracePol<T>>>,
        in_col: Rc<RefCell<TracePol<T>>>,
        set_col: Rc<RefCell<TracePol<T>>>,
        in_rom: &str,
        set_rom: &str,
    ) -> Self {
        Register {
            name: name.into(),
            value: T::zero(),
            value_col,
            in_col,
            set_col: Some(set_col),
            in_rom: in_rom.into(),
            set_rom: Some(set_rom.into()),
        }
    }

    pub fn new_ro(
        name: String,
        value_col: Rc<RefCell<TracePol<T>>>,
        in_col: Rc<RefCell<TracePol<T>>>,
        in_rom: String,
    ) -> Self {
        Register { name, value: T::zero(), value_col, in_col, set_col: None, in_rom, set_rom: None }
    }
}

impl<T: AbstractField + DeserializeField + Copy> Registerable<T, T> for Register<T> {
    fn reset(&mut self, row: usize) {
        self.reset_value();
        self.update_cols(row);
    }

    fn get_in_value(&mut self, row: usize, rom_line: &RomProgramLine) -> Option<T> {
        if !rom_line.program_line.contains_key(&self.in_rom) {
            self.in_col.borrow_mut()[row] = T::zero();
            return None;
        }

        let in_col_value = T::from_string(rom_line.program_line[&self.in_rom].as_str()?, 10);
        self.in_col.borrow_mut()[row] = in_col_value;

        Some(self.apply_in_to_value(in_col_value))
    }

    fn apply_set_value(&mut self, row: usize, rom_line: &RomProgramLine, value: T) {
        if self.set_rom.is_none() || !rom_line.program_line.contains_key(self.set_rom.as_ref().unwrap()) {
            if self.set_col.is_some() {
                self.set_col.as_mut().unwrap().borrow_mut()[row] = T::zero();
            }
            self.update_cols(row);
            return;
        }

        assert!(self.set_col.is_some(), "Couldn't set value for register {}", self.name);
        self.set_col.as_mut().unwrap().borrow_mut()[row] = T::one();
        self.update_value(row, value);
    }

    fn apply_in_to_value(&self, in_col_value: T) -> T {
        in_col_value * self.value
    }

    fn update_cols(&mut self, row: usize) {
        self.value_col.borrow_mut()[row] = self.value;
    }

    fn update_value(&mut self, row: usize, value: T) {
        //TODO! review this
        self.value = value;
        self.update_cols(row);
    }

    fn set_value(&mut self, value: T) {
        self.value = value;
    }

    fn reset_value(&mut self) {
        self.value = T::zero();
    }

    fn get_value(&self) -> T {
        self.value.clone()
    }

    fn get_col_value(&self, row: usize) -> T {
        self.value_col.borrow_mut()[row].clone()
    }
}

impl<T: fmt::Debug + fmt::Display> fmt::Display for Register<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}: {}", self.name, self.value)
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use goldilocks::{AbstractField, Goldilocks};
    use serde_json::{Map, Value};

    use crate::register::Registerable;

    use super::Register;
    use proofman::trace::trace_pol::TracePol;

    #[test]
    fn register_1_working() {
        const NUM_ROWS: usize = 16;

        let mut trace_col: TracePol<Goldilocks> = TracePol::new(NUM_ROWS);
        let in_col: TracePol<Goldilocks> = TracePol::new(NUM_ROWS);
        let set_col: TracePol<Goldilocks> = TracePol::new(NUM_ROWS);

        // Initialize the trace
        for i in 0..NUM_ROWS {
            trace_col[i] = Goldilocks::from_canonical_usize(i);
        }

        let trace_col = Rc::new(RefCell::new(trace_col));
        let in_col = Rc::new(RefCell::new(in_col));
        let set_col = Rc::new(RefCell::new(set_col));

        let mut register = Register::<Goldilocks>::new("A", trace_col, in_col, set_col, "inA", "setA");

        // Test set_value and get_value
        let val = Goldilocks::from_canonical_u64(64);
        register.set_value(val);
        assert_eq!(register.get_value(), val);

        // Test reset_value
        register.reset_value();
        assert_eq!(register.get_value(), Goldilocks::zero());

        // Test get_col_value
        let val = register.get_col_value(2);
        assert_eq!(val, Goldilocks::from_canonical_usize(2));

        // Test update_value
        let val22 = Goldilocks::from_canonical_usize(22);
        register.update_value(2, val22);
        let val = register.get_col_value(2);
        assert_eq!(val, val22);

        // Test get_in_value
        let mut program_line = Map::new();
        program_line.insert("inA".to_owned(), Value::String("123".to_owned()));
        let rom_line = crate::RomProgramLine {
            line: 5,
            file_name: "rom.zkasm".to_owned(),
            line_str: "        STEP => A".to_owned(),
            program_line,
        };
        let in_value = register.get_in_value(5, &rom_line);
        assert_eq!(in_value, Some(val22 * Goldilocks::from_canonical_u64(123)));
    }

    #[test]
    fn register_ro_1_working() {
        const NUM_ROWS: usize = 16;

        let mut trace_col: TracePol<Goldilocks> = TracePol::new(NUM_ROWS);
        let in_col: TracePol<Goldilocks> = TracePol::new(NUM_ROWS);

        // Initialize the trace
        for i in 0..NUM_ROWS {
            trace_col[i] = Goldilocks::from_canonical_usize(i);
        }

        let trace_col = Rc::new(RefCell::new(trace_col));
        let in_col = Rc::new(RefCell::new(in_col));

        let mut register = Register::<Goldilocks>::new_ro("A".to_owned(), trace_col, in_col, "inA".to_owned());

        // Test set_value and get_value
        let val = Goldilocks::from_canonical_u64(64);
        register.set_value(val);
        assert_eq!(register.get_value(), val);

        // Test reset_value
        register.reset_value();
        assert_eq!(register.get_value(), Goldilocks::zero());

        // Test get_col_value
        let val = register.get_col_value(2);
        assert_eq!(val, Goldilocks::from_canonical_usize(2));

        // Test update_value
        let val22 = Goldilocks::from_canonical_usize(22);
        register.update_value(2, val22);
        let val = register.get_col_value(2);
        assert_eq!(val, val22);

        // Test get_in_value
        let mut program_line = Map::new();
        program_line.insert("inA".to_owned(), Value::String("123".to_owned()));
        let rom_line = crate::RomProgramLine {
            line: 5,
            file_name: "rom.zkasm".to_owned(),
            line_str: "        STEP => A".to_owned(),
            program_line,
        };
        let in_value = register.get_in_value(5, &rom_line);
        assert_eq!(in_value, Some(val22 * Goldilocks::from_canonical_u64(123)));
    }

    #[test]
    fn register_edge_cases() {
        const NUM_ROWS: usize = 16;

        let trace_col: TracePol<Goldilocks> = TracePol::new(NUM_ROWS);
        let in_col: TracePol<Goldilocks> = TracePol::new(NUM_ROWS);
        let set_col: TracePol<Goldilocks> = TracePol::new(NUM_ROWS);

        let trace_col = Rc::new(RefCell::new(trace_col));
        let in_col = Rc::new(RefCell::new(in_col));
        let set_col = Rc::new(RefCell::new(set_col));

        let mut register = Register::<Goldilocks>::new("A", trace_col, in_col, set_col, "inA", "setA");

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
