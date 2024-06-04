use std::{cell::RefCell, fmt, sync::Arc};

use goldilocks::{AbstractField, DeserializeField};
use proofman::trace::trace_pol::TracePol;

use crate::RomProgramLine;

use super::Registerable;

pub struct VirtualRegisterN<'a, T, const N: usize> {
    name: String,
    value: [T; N],
    value_col: Box<dyn Fn() -> [T; N] + 'a>,
    in_col: Arc<RefCell<TracePol<T>>>,
    in_rom: String,
}

impl<'a, T: AbstractField + Copy, const N: usize> VirtualRegisterN<'a, T, N> {
    pub fn new(
        name: &str,
        value_col: Box<dyn Fn() -> [T; N] + 'a>,
        in_col: Arc<RefCell<TracePol<T>>>,
        in_rom: &str,
    ) -> Self {
        VirtualRegisterN { name: name.into(), value: [T::zero(); N], value_col, in_col, in_rom: in_rom.into() }
    }
}

impl<'a, T: AbstractField + DeserializeField + Copy, const N: usize> Registerable<[T; N], T>
    for VirtualRegisterN<'a, T, N>
{
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

    fn apply_set_value(&mut self, row: usize, _rom_line: &RomProgramLine, _value: [T; N]) {
        self.update_cols(row);
    }

    fn apply_in_to_value(&self, in_col_value: T) -> [T; N] {
        let mut result = [T::zero(); N];
        for i in 0..N {
            result[i] = in_col_value * self.value[i];
        }

        result
    }

    fn update_cols(&mut self, _row: usize) {}

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

    fn get_col_value(&self, _row: usize) -> [T; N] {
        (self.value_col)()
    }
}

impl<'a, T: fmt::Debug + fmt::Display, const N: usize> fmt::Display for VirtualRegisterN<'a, T, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}: {:?}", self.name, self.value)
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, sync::Arc};

    use goldilocks::{AbstractField, Goldilocks};
    use serde_json::{Map, Value};

    use crate::register::Registerable;

    use super::VirtualRegisterN;
    use proofman::trace::trace_pol::TracePol;

    #[test]
    fn virtual_register_n_working() {
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

        let in_col = Arc::new(RefCell::new(in_col));

        let mut register = VirtualRegisterN::<Goldilocks, N>::new(
            "A",
            Box::new(|| [Goldilocks::from_canonical_u64(128); N]),
            in_col,
            "inA",
        );

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
        let to_compare = [Goldilocks::from_canonical_u64(128); N];

        assert_eq!(val, to_compare);

        let val22 = [Goldilocks::from_canonical_usize(22); N];
        register.update_value(2, val22);
        let val = register.get_col_value(2);
        assert_eq!(val, to_compare);

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

        let in_col: TracePol<Goldilocks> = TracePol::new(NUM_ROWS);
        let in_col = Arc::new(RefCell::new(in_col));

        let mut register =
            VirtualRegisterN::<Goldilocks, 1>::new("A", Box::new(|| [Goldilocks::zero(); 1]), in_col.clone(), "inA");

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
