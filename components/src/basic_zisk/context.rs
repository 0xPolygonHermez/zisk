use std::{cell::RefCell, rc::Rc};

use crate::BasicProcessorRegisters;

pub struct Context<'a, T> {
    n: Rc<RefCell<usize>>,
    pub row: Rc<RefCell<usize>>,
    pub step: Rc<RefCell<usize>>,
    pub registers: Rc<RefCell<BasicProcessorRegisters<'a, T>>>,
    pub source_ref: String,
}

impl<'a, T> Context<'a, T> {
    pub fn new(
        n: Rc<RefCell<usize>>,
        row: Rc<RefCell<usize>>,
        step: Rc<RefCell<usize>>,
        registers: Rc<RefCell<BasicProcessorRegisters<'a, T>>>,
    ) -> Self {
        Context { n, row, step, registers, source_ref: "".to_string() }
    }

    pub fn get_n(&self) -> usize {
        *self.n.borrow()
    }
}