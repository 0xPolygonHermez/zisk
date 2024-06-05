use std::{cell::RefCell, rc::Rc};

use proofman::trace::trace_pol::TracePol;

pub struct RomLink<T> {
    col: Rc<RefCell<TracePol<T>>>,
    binary: bool,
}

impl<T> RomLink<T> {
    pub fn new(col: Rc<RefCell<TracePol<T>>>, binary: bool) -> Self {
        RomLink { col, binary }
    }
}
