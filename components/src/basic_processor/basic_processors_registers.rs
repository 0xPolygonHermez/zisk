use std::{cell::RefCell, rc::Rc};

use crate::register::{Register, RegisterN, VirtualRegister, VirtualRegisterN};
use super::CHUNKS;

pub struct BasicProcessorRegisters<'a, T> {
    pub reg_a: RegisterN<T, CHUNKS>,
    pub reg_b: RegisterN<T, CHUNKS>,
    pub reg_c: Rc<RefCell<RegisterN<T, CHUNKS>>>,
    pub reg_d: RegisterN<T, CHUNKS>,
    pub reg_e: RegisterN<T, CHUNKS>,
    pub reg_sr: RegisterN<T, CHUNKS>,
    pub reg_free: Rc<RefCell<RegisterN<T, CHUNKS>>>,
    pub reg_sp: Register<T>,
    pub reg_pc: Register<T>,
    pub reg_rr: Register<T>,
    pub reg_ctx: Register<T>,
    pub reg_rcx: Register<T>,
    pub reg_step: VirtualRegister<'a, T>,
    pub reg_free0: VirtualRegister<'a, T>,
    pub reg_rotl_c: VirtualRegisterN<'a, T, CHUNKS>,
}
