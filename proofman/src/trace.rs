use std::cell::UnsafeCell;

pub struct Ptr {
    pub ptr: UnsafeCell<*mut u8>,
}

impl Ptr {
    pub fn new(ptr: *mut u8) -> Self {
        Ptr { ptr: UnsafeCell::new(ptr) }
    }

    pub fn add<T>(&self) -> *mut u8 {
        let ptr = unsafe { *self.ptr.get() };
        unsafe { *self.ptr.get() = ptr.add(std::mem::size_of::<T>()) };
        ptr
    }
}

/// Macro for defining trace structures with specified fields.
#[macro_export]
macro_rules! trace {
    (
        $my_struct:ident { $($field_name:ident : $field_type:tt $(,)?)* }
    ) => {
        trace!($my_struct { $($field_name : $field_type),* }, offset: 0, stride: Self::ROW_SIZE);
    };

    ($my_struct:ident { $($field_name:ident : $field_type:tt $(,)?)* }, offset: $offset:expr, stride: $stride:expr) => {
        #[derive(Debug)]
        #[allow(dead_code)]
        pub struct $my_struct {
            pub buffer: Option<Vec<u8>>,
            pub ptr: *mut u8,
            num_rows: usize,
            $(pub $field_name: $crate::trace_field!($field_type),)*
        }

        #[allow(dead_code)]
        impl $my_struct {
            const ROW_SIZE: usize = $crate::trace_row_size!($($field_name : $field_type),*);

            /// Creates a new instance of $my_struct with a new buffer of size num_rows * ROW_SIZE.
            ///
            /// # Arguments
            ///
            /// * `num_rows` - The number of rows in all the TraceCol fields defined.
            pub fn new(num_rows: usize) -> Self {
                // PRECONDITIONS
                // num_rows must be greater than or equal to 2
                assert!(num_rows >= 2);
                // num_rows must be a power of 2
                assert!(num_rows & (num_rows - 1) == 0);

                let mut buffer = vec![0u8; num_rows * Self::ROW_SIZE];

                let ptr = buffer.as_mut_ptr();
                let ptr_x = $crate::trace::Ptr::new(ptr);

                $my_struct {
                    buffer: Some(buffer),
                    ptr: unsafe { std::slice::from_raw_parts_mut(ptr, num_rows * Self::ROW_SIZE).as_mut_ptr() },
                    num_rows,
                    $($field_name: $crate::trace_default_value!($field_type, ptr_x, num_rows, Self::ROW_SIZE),)*
                }
            }

            /// Create a new instance of $my_struct using an outside buffer.
            ///
            /// # Arguments
            ///
            /// * `buffer` - A mutable raw pointer to the starting memory location.
            /// * `offset` - The offset (in bytes) to the first element.
            /// * `stride` - The stride (in bytes) between consecutive elements.
            /// * `num_rows` - The number of rows in all the TraceCol fields defined.
            pub fn from_buffer(buffer: &[u8], num_rows: usize, offset: usize) -> Self {
                Self::from_ptr(buffer.as_ptr(), num_rows, offset)
            }

            /// Create a new instance of $my_struct using an outside buffer.
            ///
            /// # Arguments
            ///
            /// * `ptr` - A mutable raw pointer to the starting memory location.
            /// * `offset` - The offset (in bytes) to the first element.
            /// * `stride` - The stride (in bytes) between consecutive elements.
            /// * `num_rows` - The number of rows in all the TraceCol fields defined.
            pub fn from_ptr(ptr: *const u8, num_rows: usize, offset: usize) -> Self {
                // PRECONDITIONS
                // num_rows must be greater than or equal to 2
                assert!(num_rows >= 2);
                // num_rows must be a power of 2
                assert!(num_rows & (num_rows - 1) == 0);

                let ptr = unsafe { ptr.add($offset).add(offset) as *mut u8 };

                let ptr_x = $crate::trace::Ptr::new(ptr);

                $my_struct {
                    buffer: None,
                    ptr: unsafe { std::slice::from_raw_parts_mut(ptr, num_rows * $stride).as_mut_ptr() },
                    num_rows,
                    $($field_name: $crate::trace_default_value!($field_type, ptr_x, num_rows, $stride),)*
                }
            }

            pub fn row_size(&self) -> usize {
                Self::ROW_SIZE
            }

            pub fn num_rows(&self) -> usize {
                self.num_rows
            }

            pub fn buffer_size(&self) -> usize {
                self.buffer.as_ref().unwrap().len()
            }
        }
    };
}

#[macro_export]
macro_rules! trace_field {
    ([$field_type:ty; $num:expr]) => {
        [$crate::trace::trace_pol::TracePol<$field_type>; $num]
    };
    ($field_type:ty) => {
        $crate::trace_pol::TracePol<$field_type>
    };
}

#[macro_export]
macro_rules! trace_row_size {
    ($($field_name:ident : $field_type:tt),*) => {
        {
            $(std::mem::size_of::<$field_type>() +)* 0
        }
    };
}

#[macro_export]
macro_rules! trace_default_value {
    ([$field_type:ty; $num:expr], $ptr:expr, $num_rows:expr, $stride: expr) => {{
        let mut array: [$crate::trace::trace_pol::TracePol<$field_type>; $num] = Default::default();
        for elem in array.iter_mut() {
            *elem = $crate::trace::trace_pol::TracePol::from_ptr($ptr.add::<$field_type>(), $stride, $num_rows);
        }
        array
    }};
    ($field_type:ty, $ptr:expr, $num_rows:expr, $stride: expr) => {
        $crate::trace_pol::TracePol::from_ptr($ptr.add::<$field_type>(), $stride, $num_rows)
    };
}

#[cfg(test)]
mod tests {
    // use rand::Rng;

    #[test]
    fn check() {
        const OFFSET: usize = 2;
        let offset = 1;
        const STRIDE: usize = 16;
        let num_rows = 8;

        trace!(Trace { a: usize, b: usize }, offset: OFFSET, stride: STRIDE);

        let buffer = vec![0u8; num_rows * 2 * std::mem::size_of::<usize>() + OFFSET + offset];
        let mut trace = Trace::from_buffer(&buffer, num_rows, offset);

        for i in 0..num_rows {
            trace.a[i] = i;
            trace.b[i] = i * 10;
        }

        for i in 0..num_rows {
            assert_eq!(trace.a[i], i);
            assert_eq!(trace.b[i], i * 10);
        }

        for i in 0..num_rows {
            let value_a = unsafe { &*(buffer.as_ptr().add(OFFSET).add(offset).add(i * STRIDE) as *const usize) };
            let value_b = unsafe {
                &*(buffer.as_ptr().add(OFFSET).add(offset).add(i * STRIDE + std::mem::size_of::<usize>())
                    as *const usize)
            };

            assert_eq!(*value_a, i);
            assert_eq!(*value_b, i * 10);
        }
    }

    #[test]
    #[should_panic]
    fn test_errors_are_launched_when_num_rows_is_invalid_1() {
        let buffer = vec![0u8; 3];
        trace!(Simple { field1: usize });
        let _ = Simple::from_buffer(&buffer, 1, 0);
    }

    #[test]
    #[should_panic]
    fn test_errors_are_launched_when_num_rows_is_invalid_2() {
        let buffer = vec![0u8; 3];
        trace!(Simple { field1: usize });
        let _ = Simple::from_buffer(&buffer, 3, 0);
    }

    #[test]
    #[should_panic]
    fn test_errors_are_launched_when_num_rows_is_invalid_3() {
        trace!(Simple { field1: usize });
        let _ = Simple::new(1);
    }

    #[test]
    #[should_panic]
    fn test_errors_are_launched_when_num_rows_is_invalid_4() {
        trace!(Simple { field1: usize });
        let _ = Simple::new(3);
    }
}
