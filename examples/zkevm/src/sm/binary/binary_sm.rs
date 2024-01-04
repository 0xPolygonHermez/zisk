use proofman::trace_pol::{TracePol, Ptr};
use math::fields::f64::BaseElement as T;

#[derive(Debug)]
pub struct BinarySM<T> {
    pub opcode: TracePol<T>,
    pub a: [TracePol<T>; 8],
    pub b: [TracePol<T>; 8],
    pub c: [TracePol<T>; 8],
    pub freein_a: [TracePol<T>; 2],
    pub freein_b: [TracePol<T>; 2],
    pub freein_c: [TracePol<T>; 2],
    pub cin: TracePol<T>,
    pub cmiddle: TracePol<T>,
    pub cout: TracePol<T>,
    pub l_cout: TracePol<T>,
    pub l_opcode: TracePol<T>,
    pub previous_are_lt4: TracePol<T>,
    pub use_previous_are_lt4: TracePol<T>,
    pub reset4: TracePol<T>,
    pub use_carry: TracePol<T>,
    pub result_bin_op: TracePol<T>,
    pub result_valid_range: TracePol<T>,        

    buffer: Vec<u8>,
}

impl<T> BinarySM<T> {
    const ROW_SIZE: usize = std::mem::size_of::<T>() * 42;

    pub fn new(num_rows: usize) -> Self {
        let mut buffer = vec![0u8; num_rows * Self::ROW_SIZE];
        let mut ptr = Ptr::new(buffer.as_mut_ptr());

        BinarySM {
            opcode: TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
            a: [
                TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
                TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
                TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
                TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
                TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
                TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
                TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
                TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
            ],
            b: [
                TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
                TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
                TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
                TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
                TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
                TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
                TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
                TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
            ],
            c: [
                TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
                TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
                TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
                TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
                TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
                TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
                TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
                TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
            ],
            freein_a: [
                TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
                TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows)
            ],
            freein_b: [
                TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
                TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows)
            ],
            freein_c: [
                TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
                TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows)
            ],
            cin: TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
            cmiddle: TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
            cout: TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
            l_cout: TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
            l_opcode: TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
            previous_are_lt4: TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
            use_previous_are_lt4: TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
            reset4: TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
            use_carry: TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
            result_bin_op: TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
            result_valid_range: TracePol::new(ptr.add::<T>(), Self::ROW_SIZE, num_rows),
            
            buffer,
        }
    }
}