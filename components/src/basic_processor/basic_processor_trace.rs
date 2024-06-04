use std::cell::RefCell;
use std::sync::Arc;

use proofman::trace::{trace::Ptr, trace_pol::TracePol};

#[allow(non_snake_case)]
pub struct BasicProcessorTrace<T> {
    pub buffer: Option<Vec<u8>>,
    pub ptr: *mut u8,
    num_rows: usize,

    pub A: Arc<RefCell<TracePol<[T; 8]>>>,
    pub B: Arc<RefCell<TracePol<[T; 8]>>>,
    pub C: Arc<RefCell<TracePol<[T; 8]>>>,
    pub D: Arc<RefCell<TracePol<[T; 8]>>>,
    pub E: Arc<RefCell<TracePol<[T; 8]>>>,
    pub SR: Arc<RefCell<TracePol<[T; 8]>>>,
    pub FREE: Arc<RefCell<TracePol<[T; 8]>>>,
    pub SP: Arc<RefCell<TracePol<T>>>,
    pub PC: Arc<RefCell<TracePol<T>>>,
    pub RR: Arc<RefCell<TracePol<T>>>,
    pub CTX: Arc<RefCell<TracePol<T>>>,
    pub RCX: Arc<RefCell<TracePol<T>>>,
    pub in_A: Arc<RefCell<TracePol<T>>>,
    pub in_B: Arc<RefCell<TracePol<T>>>,
    pub in_C: Arc<RefCell<TracePol<T>>>,
    pub in_D: Arc<RefCell<TracePol<T>>>,
    pub in_E: Arc<RefCell<TracePol<T>>>,
    pub in_SR: Arc<RefCell<TracePol<T>>>,
    pub in_FREE: Arc<RefCell<TracePol<T>>>,
    pub in_SP: Arc<RefCell<TracePol<T>>>,
    pub in_PC: Arc<RefCell<TracePol<T>>>,
    pub in_RR: Arc<RefCell<TracePol<T>>>,
    pub in_CTX: Arc<RefCell<TracePol<T>>>,
    pub in_RCX: Arc<RefCell<TracePol<T>>>,
    pub in_STEP: Arc<RefCell<TracePol<T>>>,
    pub in_FREE0: Arc<RefCell<TracePol<T>>>,
    pub in_ROTL_C: Arc<RefCell<TracePol<T>>>,
    pub set_A: Arc<RefCell<TracePol<T>>>,
    pub set_B: Arc<RefCell<TracePol<T>>>,
    pub set_C: Arc<RefCell<TracePol<T>>>,
    pub set_D: Arc<RefCell<TracePol<T>>>,
    pub set_E: Arc<RefCell<TracePol<T>>>,
    pub set_SR: Arc<RefCell<TracePol<T>>>,
    pub set_SP: Arc<RefCell<TracePol<T>>>,
    pub set_PC: Arc<RefCell<TracePol<T>>>,
    pub set_RR: Arc<RefCell<TracePol<T>>>,
    pub set_CTX: Arc<RefCell<TracePol<T>>>,
    pub set_RCX: Arc<RefCell<TracePol<T>>>,

    pub is_stack: Arc<RefCell<TracePol<T>>>,
    pub is_mem: Arc<RefCell<TracePol<T>>>,
    pub m_op: Arc<RefCell<TracePol<T>>>,
    pub m_wr: Arc<RefCell<TracePol<T>>>,
    pub mem_use_addr_rel: Arc<RefCell<TracePol<T>>>,
    pub use_ctx: Arc<RefCell<TracePol<T>>>,

    pub inc_stack: Arc<RefCell<TracePol<T>>>,
    pub ind: Arc<RefCell<TracePol<T>>>,
    pub ind_rr: Arc<RefCell<TracePol<T>>>,
    pub offset: Arc<RefCell<TracePol<T>>>,

    pub do_assert: Arc<RefCell<TracePol<T>>>,
    pub assume_free: Arc<RefCell<TracePol<T>>>,

    pub jmp: Arc<RefCell<TracePol<T>>>,
    pub jmpn: Arc<RefCell<TracePol<T>>>,
    pub jmpz: Arc<RefCell<TracePol<T>>>,
    pub call: Arc<RefCell<TracePol<T>>>,
    pub return_jmp: Arc<RefCell<TracePol<T>>>,

    pub jmp_use_addr_rel: Arc<RefCell<TracePol<T>>>,
    pub else_use_addr_rel: Arc<RefCell<TracePol<T>>>,
    pub repeat: Arc<RefCell<TracePol<T>>>,

    pub cond_const: Arc<RefCell<TracePol<T>>>,
    pub jmp_addr: Arc<RefCell<TracePol<T>>>,
    pub else_addr: Arc<RefCell<TracePol<T>>>,
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

            A: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<[T; 8]>(), Self::ROW_SIZE, num_rows))),
            B: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<[T; 8]>(), Self::ROW_SIZE, num_rows))),
            C: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<[T; 8]>(), Self::ROW_SIZE, num_rows))),
            D: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<[T; 8]>(), Self::ROW_SIZE, num_rows))),
            E: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<[T; 8]>(), Self::ROW_SIZE, num_rows))),
            SR: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<[T; 8]>(), Self::ROW_SIZE, num_rows))),
            FREE: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<[T; 8]>(), Self::ROW_SIZE, num_rows))),
            SP: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            PC: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            RR: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            CTX: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            RCX: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            in_A: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            in_B: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            in_C: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            in_D: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            in_E: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            in_SR: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            in_FREE: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            in_SP: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            in_PC: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            in_RR: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            in_CTX: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            in_RCX: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            in_STEP: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            in_FREE0: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            in_ROTL_C: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            set_A: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            set_B: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            set_C: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            set_D: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            set_E: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            set_SR: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            set_SP: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            set_PC: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            set_RR: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            set_CTX: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            set_RCX: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),

            is_stack: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            is_mem: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            m_op: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            m_wr: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            mem_use_addr_rel: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            use_ctx: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),

            inc_stack: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            ind: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            ind_rr: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            offset: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),

            do_assert: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            assume_free: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),

            jmp: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            jmpn: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            jmpz: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            call: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            return_jmp: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),

            jmp_use_addr_rel: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            else_use_addr_rel: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            repeat: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),

            cond_const: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            jmp_addr: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
            else_addr: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, num_rows))),
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

            A: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<[T; 8]>(), stride, num_rows))),
            B: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<[T; 8]>(), stride, num_rows))),
            C: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<[T; 8]>(), stride, num_rows))),
            D: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<[T; 8]>(), stride, num_rows))),
            E: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<[T; 8]>(), stride, num_rows))),
            SR: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<[T; 8]>(), stride, num_rows))),
            FREE: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<[T; 8]>(), stride, num_rows))),
            SP: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            PC: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            RR: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            CTX: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            RCX: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            in_A: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            in_B: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            in_C: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            in_D: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            in_E: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            in_SR: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            in_FREE: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            in_SP: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            in_PC: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            in_RR: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            in_CTX: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            in_RCX: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            in_STEP: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            in_FREE0: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            in_ROTL_C: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            set_A: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            set_B: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            set_C: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            set_D: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            set_E: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            set_SR: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            set_SP: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            set_PC: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            set_RR: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            set_CTX: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            set_RCX: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),

            is_stack: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            is_mem: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            m_op: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            m_wr: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            mem_use_addr_rel: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            use_ctx: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),

            inc_stack: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            ind: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            ind_rr: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            offset: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),

            do_assert: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            assume_free: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),

            jmp: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            jmpn: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            jmpz: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            call: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            return_jmp: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),

            jmp_use_addr_rel: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            else_use_addr_rel: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            repeat: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),

            cond_const: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            jmp_addr: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
            else_addr: Arc::new(RefCell::new(TracePol::from_ptr(ptr_x.add::<T>(), stride, num_rows))),
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
