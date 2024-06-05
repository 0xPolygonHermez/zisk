use std::{cell::RefCell, fmt, rc::Rc};

use goldilocks::{AbstractField, DeserializeField};
use proofman::trace::trace_pol::TracePol;

use crate::RomProgramLine;

use super::Registerable;

pub struct VirtualRegister<'a, T> {
    name: String,
    value: T,
    function_col: Box<dyn Fn() -> T + 'a>,
    in_col: Rc<RefCell<TracePol<T>>>,
    in_rom: String,
}

impl<'a, T: AbstractField + Copy> VirtualRegister<'a, T> {
    pub fn new(
        name: &str,
        function_col: Box<dyn Fn() -> T + 'a>,
        in_col: Rc<RefCell<TracePol<T>>>,
        in_rom: &str,
    ) -> Self {
        VirtualRegister { name: name.into(), value: T::zero(), function_col, in_col, in_rom: in_rom.into() }
    }
}

impl<'a, T: AbstractField + DeserializeField + Copy> Registerable<T, T> for VirtualRegister<'a, T> {
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

    fn apply_set_value(&mut self, row: usize, _rom_line: &RomProgramLine, _value: T) {
        self.update_cols(row);
    }

    fn apply_in_to_value(&self, in_col_value: T) -> T {
        in_col_value * self.value
    }

    fn update_cols(&mut self, _row: usize) {}

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
        self.value
    }

    fn get_col_value(&self, _row: usize) -> T {
        (self.function_col)()
    }
}

impl<'a, T: fmt::Debug + fmt::Display> fmt::Display for VirtualRegister<'a, T> {
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

    use super::VirtualRegister;
    use proofman::trace::trace_pol::TracePol;

    #[test]
    fn virtual_register_1_working() {
        const NUM_ROWS: usize = 16;

        let mut trace_col: TracePol<Goldilocks> = TracePol::new(NUM_ROWS);
        let in_col: TracePol<Goldilocks> = TracePol::new(NUM_ROWS);

        // Initialize the trace
        for i in 0..NUM_ROWS {
            trace_col[i] = Goldilocks::from_canonical_usize(i);
        }

        let in_col = Rc::new(RefCell::new(in_col));

        let mut register =
            VirtualRegister::<Goldilocks>::new("A", Box::new(|| Goldilocks::zero()), in_col.clone(), "inA");

        // Test set_value and get_value
        let val = Goldilocks::from_canonical_u64(64);
        register.set_value(val);
        assert_eq!(register.get_value(), val);

        // Test reset_value
        register.reset_value();
        assert_eq!(register.get_value(), Goldilocks::zero());

        // Test get_col_value
        let val = register.get_col_value(2);
        assert_eq!(val, Goldilocks::from_canonical_usize(0));

        // Test update_value
        let val22 = Goldilocks::from_canonical_usize(22);
        register.update_value(2, val22);
        let val = register.get_col_value(2);
        assert_eq!(val, Goldilocks::from_canonical_usize(0));

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

        let in_col: TracePol<Goldilocks> = TracePol::new(NUM_ROWS);
        let in_col = Rc::new(RefCell::new(in_col));

        let mut register =
            VirtualRegister::<Goldilocks>::new("A", Box::new(|| Goldilocks::zero()), in_col.clone(), "inA");

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
