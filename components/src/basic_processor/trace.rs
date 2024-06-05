use std::{cell::RefCell, rc::Rc};

use proofman::trace::{trace::Ptr, trace_pol::TracePol};

#[allow(non_snake_case)]
pub struct BasicProcessorTrace<T> {
    pub buffer: Option<Vec<u8>>,
    pub ptr: *mut u8,
    num_rows: usize,

    pub zk_pc: Rc<RefCell<TracePol<T>>>,

    pub A: Rc<RefCell<TracePol<[T; 8]>>>,
    pub B: Rc<RefCell<TracePol<[T; 8]>>>,
    pub C: Rc<RefCell<TracePol<[T; 8]>>>,
    pub D: Rc<RefCell<TracePol<[T; 8]>>>,
    pub E: Rc<RefCell<TracePol<[T; 8]>>>,
    pub SR: Rc<RefCell<TracePol<[T; 8]>>>,
    pub FREE: Rc<RefCell<TracePol<[T; 8]>>>,
    pub SP: Rc<RefCell<TracePol<T>>>,
    pub PC: Rc<RefCell<TracePol<T>>>,
    pub RR: Rc<RefCell<TracePol<T>>>,
    pub CTX: Rc<RefCell<TracePol<T>>>,
    pub RCX: Rc<RefCell<TracePol<T>>>,
    pub in_A: Rc<RefCell<TracePol<T>>>,
    pub in_B: Rc<RefCell<TracePol<T>>>,
    pub in_C: Rc<RefCell<TracePol<T>>>,
    pub in_D: Rc<RefCell<TracePol<T>>>,
    pub in_E: Rc<RefCell<TracePol<T>>>,
    pub in_SR: Rc<RefCell<TracePol<T>>>,
    pub in_FREE: Rc<RefCell<TracePol<T>>>,
    pub in_SP: Rc<RefCell<TracePol<T>>>,
    pub in_PC: Rc<RefCell<TracePol<T>>>,
    pub in_RR: Rc<RefCell<TracePol<T>>>,
    pub in_CTX: Rc<RefCell<TracePol<T>>>,
    pub in_RCX: Rc<RefCell<TracePol<T>>>,
    pub in_STEP: Rc<RefCell<TracePol<T>>>,
    pub in_FREE0: Rc<RefCell<TracePol<T>>>,
    pub in_ROTL_C: Rc<RefCell<TracePol<T>>>,
    pub set_A: Rc<RefCell<TracePol<T>>>,
    pub set_B: Rc<RefCell<TracePol<T>>>,
    pub set_C: Rc<RefCell<TracePol<T>>>,
    pub set_D: Rc<RefCell<TracePol<T>>>,
    pub set_E: Rc<RefCell<TracePol<T>>>,
    pub set_SR: Rc<RefCell<TracePol<T>>>,
    pub set_SP: Rc<RefCell<TracePol<T>>>,
    pub set_PC: Rc<RefCell<TracePol<T>>>,
    pub set_RR: Rc<RefCell<TracePol<T>>>,
    pub set_CTX: Rc<RefCell<TracePol<T>>>,
    pub set_RCX: Rc<RefCell<TracePol<T>>>,

    pub is_stack: Rc<RefCell<TracePol<T>>>,
    pub is_mem: Rc<RefCell<TracePol<T>>>,
    pub m_op: Rc<RefCell<TracePol<T>>>,
    pub m_wr: Rc<RefCell<TracePol<T>>>,
    pub mem_use_addr_rel: Rc<RefCell<TracePol<T>>>,
    pub use_ctx: Rc<RefCell<TracePol<T>>>,

    pub inc_stack: Rc<RefCell<TracePol<T>>>,
    pub ind: Rc<RefCell<TracePol<T>>>,
    pub ind_rr: Rc<RefCell<TracePol<T>>>,
    pub offset: Rc<RefCell<TracePol<T>>>,

    pub do_assert: Rc<RefCell<TracePol<T>>>,
    pub assume_free: Rc<RefCell<TracePol<T>>>,

    pub jmp: Rc<RefCell<TracePol<T>>>,
    pub jmpn: Rc<RefCell<TracePol<T>>>,
    pub jmpz: Rc<RefCell<TracePol<T>>>,
    pub call: Rc<RefCell<TracePol<T>>>,
    pub return_jmp: Rc<RefCell<TracePol<T>>>,

    pub jmp_use_addr_rel: Rc<RefCell<TracePol<T>>>,
    pub else_use_addr_rel: Rc<RefCell<TracePol<T>>>,
    pub repeat: Rc<RefCell<TracePol<T>>>,

    pub cond_const: Rc<RefCell<TracePol<T>>>,
    pub jmp_addr: Rc<RefCell<TracePol<T>>>,
    pub else_addr: Rc<RefCell<TracePol<T>>>,
}

impl<T> BasicProcessorTrace<T> {
    const ROW_SIZE: usize = 7 * std::mem::size_of::<[T; 8]>() + 31 * std::mem::size_of::<T>();
    pub fn new(num_rows: usize) -> Self {
        // PRECONDITIONS
        // num_rows must be greater than or equal to 2
        assert!(num_rows >= 2);
        // num_rows must be a power of 2
        assert!(num_rows & (num_rows - 1) == 0);

        let mut buffer = vec![0u8; num_rows * Self::ROW_SIZE];

        let ptr = buffer.as_mut_ptr();
        let ptr_x = Ptr::new(ptr);

        BasicProcessorTrace {
            buffer: Some(buffer),
            ptr: unsafe { std::slice::from_raw_parts_mut(ptr, num_rows * Self::ROW_SIZE).as_mut_ptr() },
            num_rows,

            zk_pc: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),

            A: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<[T; 8]>(), Self::ROW_SIZE, num_rows))),
            B: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<[T; 8]>(), Self::ROW_SIZE, num_rows))),
            C: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<[T; 8]>(), Self::ROW_SIZE, num_rows))),
            D: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<[T; 8]>(), Self::ROW_SIZE, num_rows))),
            E: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<[T; 8]>(), Self::ROW_SIZE, num_rows))),
            SR: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<[T; 8]>(), Self::ROW_SIZE, num_rows))),
            FREE: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<[T; 8]>(), Self::ROW_SIZE, num_rows))),
            SP: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            PC: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            RR: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            CTX: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            RCX: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            in_A: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            in_B: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            in_C: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            in_D: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            in_E: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            in_SR: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            in_FREE: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            in_SP: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            in_PC: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            in_RR: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            in_CTX: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            in_RCX: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            in_STEP: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            in_FREE0: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            in_ROTL_C: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            set_A: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            set_B: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            set_C: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            set_D: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            set_E: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            set_SR: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            set_SP: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            set_PC: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            set_RR: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            set_CTX: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            set_RCX: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),

            is_stack: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            is_mem: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            m_op: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            m_wr: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            mem_use_addr_rel: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            use_ctx: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),

            inc_stack: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            ind: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            ind_rr: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            offset: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),

            do_assert: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            assume_free: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),

            jmp: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            jmpn: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            jmpz: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            call: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            return_jmp: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),

            jmp_use_addr_rel: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            else_use_addr_rel: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            repeat: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),

            cond_const: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            jmp_addr: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            else_addr: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
        }
    }

    pub fn from_ptr(ptr: *mut std::ffi::c_void, num_rows: usize, offset: usize, stride: usize) -> Self {
        // PRECONDITIONS
        // num_rows must be greater than or equal to 2
        assert!(num_rows >= 2);
        // num_rows must be a power of 2
        assert!(num_rows & (num_rows - 1) == 0);

        let mut ptr = ptr as *mut u8;

        ptr = unsafe { ptr.add(offset) };
        let ptr_x = Ptr::new(ptr);

        BasicProcessorTrace {
            buffer: None,
            ptr: unsafe { std::slice::from_raw_parts_mut(ptr, num_rows * stride).as_mut_ptr() },
            num_rows,

            zk_pc: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),

            A: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<[T; 8]>(), stride, num_rows))),
            B: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<[T; 8]>(), stride, num_rows))),
            C: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<[T; 8]>(), stride, num_rows))),
            D: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<[T; 8]>(), stride, num_rows))),
            E: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<[T; 8]>(), stride, num_rows))),
            SR: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<[T; 8]>(), stride, num_rows))),
            FREE: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<[T; 8]>(), stride, num_rows))),
            SP: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            PC: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            RR: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            CTX: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            RCX: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            in_A: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            in_B: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            in_C: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            in_D: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            in_E: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            in_SR: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            in_FREE: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            in_SP: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            in_PC: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            in_RR: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            in_CTX: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            in_RCX: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            in_STEP: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            in_FREE0: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            in_ROTL_C: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            set_A: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            set_B: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            set_C: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            set_D: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            set_E: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            set_SR: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            set_SP: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            set_PC: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            set_RR: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            set_CTX: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            set_RCX: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),

            is_stack: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            is_mem: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            m_op: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            m_wr: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            mem_use_addr_rel: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            use_ctx: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),

            inc_stack: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            ind: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            ind_rr: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            offset: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),

            do_assert: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            assume_free: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),

            jmp: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            jmpn: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            jmpz: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            call: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            return_jmp: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),

            jmp_use_addr_rel: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            else_use_addr_rel: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            repeat: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),

            cond_const: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            jmp_addr: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            else_addr: Rc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
        }
    }

    pub fn num_rows(&self) -> usize {
        self.num_rows
    }

    pub fn row_size(&self) -> usize {
        Self::ROW_SIZE
    }

    pub fn buffer_size(&self) -> usize {
        self.buffer.as_ref().unwrap().len()
    }
}
