mod register;
mod register_n;
mod virtual_register;
mod virtual_register_n;

pub use register::*;
pub use register_n::*;
pub use virtual_register::*;
pub use virtual_register_n::*;

use crate::RomProgramLine;

pub trait Registerable<T, C> {
    fn reset(&mut self, row: usize);

    fn apply_set_value(&mut self, row: usize, rom_line: &RomProgramLine, value: T);

    fn get_in_value(&mut self, row: usize, rom_line: &RomProgramLine) -> Option<T>;
    fn apply_in_to_value(&self, value: C) -> T;

    fn get_col_value(&self, row: usize) -> T;
    fn update_cols(&mut self, row: usize);
    fn update_value(&mut self, row: usize, value: T);

    fn set_value(&mut self, value: T);
    fn reset_value(&mut self);
    fn get_value(&self) -> T;
}
