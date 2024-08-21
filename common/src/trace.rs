pub trait Trace: Send {
    fn num_rows(&self) -> usize;
    fn get_buffer_ptr(&mut self) -> *mut u8;
}

#[macro_export]
macro_rules! trace {
    ($row_struct_name:ident, $trace_struct_name:ident<$generic:ident> {
        $( $field_name:ident : $field_type:ty ),* $(,)?
    }) => {
        // Define the row structure (Main0RowTrace)
        #[allow(dead_code)]
        #[derive(Debug, Clone, Copy, Default)]
        pub struct $row_struct_name<$generic> {
            $( pub $field_name: $field_type ),*
        }

        impl<$generic: Copy> $row_struct_name<$generic> {
            // The size of each row in terms of the number of fields
             pub const ROW_SIZE: usize = 0 $(+ trace!(@count_elements $field_type))*;
        }

        // Define the trace structure (Main0Trace) that manages the row structure
        pub struct $trace_struct_name<'a, $generic> {
            pub buffer: Option<Vec<$generic>>,
            pub slice_buffer: &'a mut [$generic],
            pub slice_trace: &'a mut [$row_struct_name<$generic>],
            num_rows: usize,
        }

        impl<'a, $generic: Default + Clone + Copy> $trace_struct_name<'a, $generic> {
            // Constructor for creating a new buffer
            pub fn new(num_rows: usize) -> Self {
                // PRECONDITIONS
                // num_rows must be greater than or equal to 2
                assert!(num_rows >= 2);
                // num_rows must be a power of 2
                assert!(num_rows & (num_rows - 1) == 0);

                let buffer = vec![$generic::default(); num_rows * $row_struct_name::<$generic>::ROW_SIZE];

                let slice_trace = unsafe {
                    std::slice::from_raw_parts_mut(buffer.as_ptr() as *mut $row_struct_name<$generic>, num_rows)
                };

                let slice_buffer = unsafe {
                    std::slice::from_raw_parts_mut(buffer.as_ptr() as *mut $generic, num_rows * $row_struct_name::<$generic>::ROW_SIZE)
                };

                $trace_struct_name { buffer: Some(buffer), slice_buffer, slice_trace, num_rows }
            }

            // Constructor to map over an external buffer
            pub fn map_buffer(external_buffer: &'a mut [$generic], num_rows: usize, offset: usize) -> Result<Self, Box<dyn std::error::Error>> {
                // PRECONDITIONS
                // num_rows must be greater than or equal to 2
                assert!(num_rows >= 2);
                // num_rows must be a power of 2
                assert!(num_rows & (num_rows - 1) == 0);

                let start = offset;
                let end = start + num_rows * $row_struct_name::<$generic>::ROW_SIZE;

                if end > external_buffer.len() {
                    return Err("Buffer is too small to fit the trace".into());
                }

                // let slice_buffer = unsafe {
                //     let ptr = external_buffer.as_ptr() as *mut $generic;
                //     std::slice::from_raw_parts_mut(ptr, external_buffer.len())
                // };

                let slice_trace = unsafe {
                    std::slice::from_raw_parts_mut(
                        external_buffer[start..end].as_ptr() as *mut $row_struct_name<$generic>,
                        num_rows,
                    )
                };

                Ok($trace_struct_name {
                    buffer: None,
                    slice_buffer: external_buffer,
                    slice_trace,
                    num_rows,
                })
            }

            // Constructor to map over an external buffer
            pub fn map_row_vec(external_buffer: Vec<$row_struct_name<$generic>>) -> Result<Self, Box<dyn std::error::Error>> {
                let num_rows = external_buffer.len().next_power_of_two();

                // PRECONDITIONS
                // num_rows must be greater than or equal to 2
                assert!(num_rows >= 2);
                // num_rows must be a power of 2
                assert!(num_rows & (num_rows - 1) == 0);

                let slice_buffer = unsafe {
                    let ptr = external_buffer.as_ptr() as *mut $generic;
                    std::slice::from_raw_parts_mut(ptr, num_rows * $row_struct_name::<$generic>::ROW_SIZE)
                };

                let slice_trace = unsafe {
                    let ptr = external_buffer.as_ptr() as *mut $row_struct_name<$generic>;
                    std::slice::from_raw_parts_mut(ptr,
                        num_rows,
                    )
                };

                let buffer_F = unsafe {
                    Vec::from_raw_parts(external_buffer.as_ptr() as *mut $generic, num_rows * $row_struct_name::<$generic>::ROW_SIZE, num_rows * $row_struct_name::<$generic>::ROW_SIZE)
                };

                std::mem::forget(external_buffer);

                Ok($trace_struct_name {
                    buffer: Some(buffer_F),
                    slice_buffer,
                    slice_trace,
                    num_rows,
                })
            }

            pub fn num_rows(&self) -> usize {
                self.num_rows
            }
        }

        // Implement Index trait for immutable access
        impl<'a, $generic> std::ops::Index<usize> for $trace_struct_name<'a, $generic> {
            type Output = $row_struct_name<$generic>;

            fn index(&self, index: usize) -> &Self::Output {
                &self.slice_trace[index]
            }
        }

        // Implement IndexMut trait for mutable access
        impl<'a, $generic> std::ops::IndexMut<usize> for $trace_struct_name<'a, $generic> {
            fn index_mut(&mut self, index: usize) -> &mut Self::Output {
                &mut self.slice_trace[index]
            }
        }

                // Implement the Trace trait
        impl<'a, $generic: Send > $crate::trace::Trace for $trace_struct_name<'a, $generic> {
            fn num_rows(&self) -> usize {
                self.num_rows
            }

            fn get_buffer_ptr(&mut self) -> *mut u8 {
                let buffer = self.buffer.as_mut().expect("Buffer is not available");
                buffer.as_mut_ptr() as *mut u8
            }
        }
    };

    (@count_elements [$elem_type:ty; $len:expr]) => {
        $len
    };

    (@count_elements $elem_type:ty) => {
        1
    };

}

#[cfg(test)]
mod tests {
    // use rand::Rng;

    #[test]
    fn check() {
        const OFFSET: usize = 1;
        let num_rows = 8;

        trace!(TraceRow, MyTrace<F> { a: F, b: F });

        let mut buffer = vec![0usize; num_rows * TraceRow::<usize>::ROW_SIZE + OFFSET];
        let trace = MyTrace::map_buffer(&mut buffer, num_rows, OFFSET);
        let mut trace = trace.unwrap();

        // Set values
        for i in 0..num_rows {
            trace[i].a = i;
            trace[i].b = i * 10;
        }

        // Check values
        for i in 0..num_rows {
            assert_eq!(trace[i].a, i);
            assert_eq!(trace[i].b, i * 10);
        }
    }

    #[test]
    #[should_panic]
    fn test_errors_are_launched_when_num_rows_is_invalid_1() {
        let mut buffer = vec![0u8; 3];
        trace!(SimpleRow, Simple<F> { a: F });
        let _ = Simple::map_buffer(&mut buffer, 1, 0);
    }

    #[test]
    #[should_panic]
    fn test_errors_are_launched_when_num_rows_is_invalid_2() {
        let mut buffer = vec![0u8; 3];
        trace!(SimpleRow, Simple<F> { a: F });
        let _ = Simple::map_buffer(&mut buffer, 3, 0);
    }

    #[test]
    #[should_panic]
    fn test_errors_are_launched_when_num_rows_is_invalid_3() {
        trace!(SimpleRow, Simple<F> { a: F });
        let _ = Simple::<u8>::new(1);
    }

    #[test]
    #[should_panic]
    fn test_errors_are_launched_when_num_rows_is_invalid_4() {
        trace!(SimpleRow, Simple<F> { a: F });
        let _ = Simple::<u8>::new(3);
    }
}
