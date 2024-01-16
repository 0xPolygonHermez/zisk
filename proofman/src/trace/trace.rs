use std::cell::UnsafeCell;

pub struct Ptr {
    pub ptr: UnsafeCell<*mut u8>,
}

impl Ptr {
    pub fn new(ptr: *mut u8) -> Self {
        Ptr {
            ptr: UnsafeCell::new(ptr),
        }
    }

    pub fn add<T>(&self) -> *mut u8 {
        let ptr = unsafe { *self.ptr.get() };
        unsafe { *self.ptr.get() = ptr.add(std::mem::size_of::<T>() as usize) };
        ptr
    }
}

/// A trait representing a trace within a proof.
pub trait Trace: Send + Sync + std::fmt::Debug {
    fn num_rows(&self) -> usize;
    // TODO! uncomment fn split(&self, num_segments: usize) -> Vec<Self> where Self: Sized;
}

/// Macro for defining trace structures with specified fields.
#[macro_export]
macro_rules! trace {
    ($my_struct:ident { $($field_name:ident : $field_type:tt $(,)?)* }) => {
        #[derive(Debug)]
        #[allow(dead_code)]
        pub struct $my_struct<'a> {
            buffer: Option<Vec<u8>>,
            ptr: &'a [u8],
            num_rows: usize,
            $(pub $field_name: $crate::trace_field!($field_type),)*
        }

        #[allow(dead_code)]
        impl<'a> $my_struct<'a> {
            const ROW_SIZE: usize = $crate::trace_row_size!($($field_name : $field_type),*);

            /// Creates a new instance of $my_struct with a new buffer of size num_rows * ROW_SIZE.
            ///
            /// # Arguments
            ///
            /// * `num_rows` - The number of rows in all the TraceCol fields defined.
            ///
            /// # Preconditions
            ///
            /// * `num_rows` must be greater than or equal to 2 and a power of 2.
            pub fn new(num_rows: usize) -> Box<Self> {
                // PRECONDITIONS
                // num_rows must be greater than or equal to 2
                assert!(num_rows >= 2);
                // num_rows must be a power of 2
                assert!(num_rows & (num_rows - 1) == 0);

                let mut buffer = vec![0u8; num_rows * Self::ROW_SIZE];

                let ptr = buffer.as_mut_ptr();
                // TODO! check stride
                let stride = Self::ROW_SIZE;
                let ptr_x = $crate::trace::trace::Ptr::new(ptr);

                Box::new($my_struct {
                    buffer: Some(buffer),
                    ptr: unsafe { std::slice::from_raw_parts_mut(ptr, num_rows * stride) },
                    num_rows,
                    $($field_name: $crate::trace_default_value!($field_type, ptr_x, num_rows, Self::ROW_SIZE),)*
                })
            }

            /// Create a new instance of $my_struct using an outside buffer.
            /// TODO! Explain it better
            ///
            /// # Arguments
            ///
            /// * `ptr` - A mutable raw pointer to the starting memory location.
            /// * `offset` - The offset (in bytes) to the first element.
            /// * `stride` - The stride (in bytes) between consecutive elements.
            /// * `num_rows` - The number of rows in all the TraceCol fields defined.
            ///
            /// # Preconditions
            ///
            /// * `num_rows` must be greater than or equal to 2 and a power of 2.
            pub fn from_ptr(ptr: *mut std::ffi::c_void, offset:usize, stride: usize, num_rows: usize) -> Box<Self> {
                // PRECONDITIONS
                // num_rows must be greater than or equal to 2
                assert!(num_rows >= 2);
                // num_rows must be a power of 2
                assert!(num_rows & (num_rows - 1) == 0);
                
                let mut ptr = ptr as *mut u8;

                ptr = unsafe { ptr.add(offset) };
                let ptr_x = $crate::trace::trace::Ptr::new(ptr);

                Box::new($my_struct {
                    buffer: None,
                    ptr: unsafe { std::slice::from_raw_parts_mut(ptr, num_rows * stride) },
                    num_rows,
                    $($field_name: $crate::trace_default_value!($field_type, ptr_x, num_rows, stride),)*
                })
            }

            // TODO! uncomment
            /// Splits the TraceCol into multiple segments.
            ///
            /// # Arguments
            ///
            /// * `num_segments` - The number of segments to split the TraceCol into.
            ///
            /// # Preconditions
            ///
            /// * `num_segments` must be greater than 0.
            /// * `num_segments` must be less than or equal to the length of the TraceCol.
            ///
            /// # Returns
            ///
            /// Returns a vector of TraceCols, each representing a segment of the original TraceCol.
            // pub fn split(&self, num_segments: usize) -> Vec<Vec<TraceCol>> {
            //     // PRECONDITIONS
            //     // · num_segments must be greater than 0
            //     // · num_segments must be less than or equal to the length of the trace
            //     assert!(num_segments > 0 && num_segments <= self.num_rows());

            //     let segments = Vec::with_capacity(num_segments);
            //     let segment_size = self.num_rows() / num_segments;

            //     let mut start = 0;
            //     for _ in 0..num_segments {
            //         segments.push(Self {
            //             buffer: self.buffer[start * Self::ROW_SIZE..(start + segment_size) * Self::ROW_SIZE].to_vec(),
            //             num_rows: segment_size,
            //             $($field_name: $crate::trace_field!($field_type, $crate::trace::trace::Ptr::new(self.buffer.as_mut_ptr().add(start * Self::ROW_SIZE)), segment_size)),*
            //         });
            //     }
            //     segments
            // }

            pub fn num_rows(&self) -> usize {
                self.num_rows
            }
        }

        impl<'a> $crate::trace::trace::Trace for $my_struct<'a> {
            fn num_rows(&self) -> usize {
                self.num_rows()
            }

            // TODO! uncomment
            // fn split(&self, num_segments: usize) -> Vec<Self> {
            //     self.split(num_segments)
            // }
        }
    };
}

#[macro_export]
macro_rules! trace_field {
    ([$field_type:ty; $num:expr]) => {
        [$crate::trace::trace_pol::TracePol<$field_type>; $num]
    };
    ($field_type:ty) => {
        $crate::trace::trace_pol::TracePol<$field_type>
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
            *elem = $crate::trace::trace_pol::TracePol::new(
                $ptr.add::<$field_type>(),
                $stride,
                $num_rows,
            );
        }
        array
    }};
    ($field_type:ty, $ptr:expr, $num_rows:expr, $stride: expr) => {
        $crate::trace::trace_pol::TracePol::new(
            $ptr.add::<$field_type>(),
            $stride,
            $num_rows,
        )
    };
}

#[cfg(test)]
mod tests {
    use std::ffi::c_void;

    use math::fields::f64::BaseElement;
    use rand::Rng;

    #[test]
    fn check() {
        trace!(Check {
            a:u8
        });

        let offset = 2;
        let stride = 5;
        let num_rows = 8;

        let mut buffer = vec![0u8; num_rows * stride];
        let ptr = buffer.as_mut_ptr() as *mut c_void;
        let mut check = Check::from_ptr(ptr, offset, stride, num_rows);

        for i in 0..num_rows {
            check.a[i] = i as u8;
        }

        for i in 0..num_rows {
            assert_eq!(check.a[i], i as u8);
        }
    }

    #[test]
    fn test_simple_trace_creation() {
        let num_rows = 256;

        // We simulate a buffer containing more data where row_size is 15 bytes and out data start at byte 3
        let offset = 3;
        let stride = 15;
        
        trace!(Simple { field1: usize });
        let mut buffer = vec![0u8; num_rows * stride];
        let ptr = buffer.as_mut_ptr() as *mut c_void;
        let mut simple = Simple::from_ptr(ptr, offset, stride, num_rows);

        let mut simple2 = Simple::new(num_rows);

        assert_eq!(simple.field1.num_rows(), num_rows);
        assert_eq!(simple2.field1.num_rows(), num_rows);

        for i in 0..num_rows {
            simple.field1[i] = i;
            simple2.field1[i] = i;
        }

        for i in 0..num_rows {
            assert_eq!(simple.field1[i], i);
            assert_eq!(simple2.field1[i], i);
        }

        assert_eq!(simple.num_rows(), num_rows);
        assert_eq!(simple2.num_rows(), num_rows);
    }

    #[test]
    #[should_panic]
    fn test_errors_are_launched_when_num_rows_is_invalid_1() {
        trace!(Simple { field1: usize });
        let _ = Simple::from_ptr(std::ptr::null_mut(), 0, 0, 1);
    }

    #[test]
    #[should_panic]
    fn test_errors_are_launched_when_num_rows_is_invalid_2() {
        trace!(Simple { field1: usize });
        let _ = Simple::from_ptr(std::ptr::null_mut(), 0, 0, 3);
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

    #[test]
    fn test_fibonacci_trace_creation() {
        // NOTE: we are looking for a syntaxis like this:
        // fibonacci = trace!{ { a: BaseField, b: BaseField }::new(num_rows);
        // let fibs = fibonacci.split(8);
        // use fibonacci {
        //     a[0] = BaseElement::new(1);
        //     b[0] = BaseElement::new(1);
        //     for i in 1..num_rows {
        //         a[i] = b[i - 1];
        //         b[i] = a[i - 1] + b[i - 1];
        //     }
        // }
        let mut rng = rand::thread_rng();
        let num_rows = 2_u8.pow(rng.gen_range(2..7)) as usize;

        // QUESTION: why not this syntax? trace!(cols Fibonacci { a: BaseElement, b: BaseElement });
        // and why not this alternative syntax? trace!(buffer Fibonacci { a: BaseElement, b: BaseElement });
        trace!(Fibonacci {
            a: BaseElement,
            b: BaseElement,
            c: [u64; 2],
        });
    
        // We simulate a buffer containing more data where row_size is 15 bytes and out data start at byte 3
        let offset = 7;
        let stride = 45;

        let mut buffer = vec![0u8; num_rows * stride];
        let ptr = buffer.as_mut_ptr() as *mut c_void;
        let mut fibonacci = Fibonacci::from_ptr(ptr, offset, stride, num_rows);

        let mut fibonacci2 = Fibonacci::new(num_rows);

        fibonacci.a[0] = BaseElement::new(1);
        fibonacci.b[0] = BaseElement::new(1);
        fibonacci.c[0][0] = 2;
        fibonacci.c[1][0] = 3;

        fibonacci2.a[0] = BaseElement::new(1);
        fibonacci2.b[0] = BaseElement::new(1);
        fibonacci2.c[0][0] = 2;
        fibonacci2.c[1][0] = 3;

        for i in 1..num_rows {
            fibonacci.a[i] = fibonacci.b[i - 1];
            fibonacci.b[i] = fibonacci.a[i - 1] + fibonacci.b[i - 1];
            fibonacci.c[0][i] = fibonacci.c[0][i - 1];
            fibonacci.c[1][i] = fibonacci.c[0][i - 1] + fibonacci.c[1][i - 1];

            fibonacci2.a[i] = fibonacci2.b[i - 1];
            fibonacci2.b[i] = fibonacci2.a[i - 1] + fibonacci2.b[i - 1];
            fibonacci2.c[0][i] = fibonacci2.c[0][i - 1];
            fibonacci2.c[1][i] = fibonacci2.c[0][i - 1] + fibonacci2.c[1][i - 1];
        }

        for i in 1..num_rows {
            assert_eq!(fibonacci.a[i - 1] + fibonacci.b[i - 1], fibonacci.b[i]);
            assert_eq!(
                fibonacci.c[0][i - 1] + fibonacci.c[1][i - 1],
                fibonacci.c[1][i]
            );

            assert_eq!(fibonacci2.a[i - 1] + fibonacci2.b[i - 1], fibonacci2.b[i]);
            assert_eq!(
                fibonacci2.c[0][i - 1] + fibonacci2.c[1][i - 1],
                fibonacci2.c[1][i]
            );
        }

        // let num_segments = 2;
        // let splitted = fibonacci.split(num_segments);

        // assert_eq!(splitted[0].num_rows(), num_rows / num_segments);

        // for i in 0..num_segments {
        //     for j in 1..num_rows / num_segments {
        //         assert_eq!(
        //             splitted[i].a[j - 1] + splitted[i].b[j - 1],
        //             splitted[i].b[j]
        //         );
        //     }
        //     if i != 0 {
        //         assert_eq!(
        //             splitted[i - 1].a[splitted[i - 1].num_rows() - 1]
        //                 + splitted[i - 1].b[splitted[i - 1].num_rows() - 1],
        //             splitted[i].b[0]
        //         );
        //     }
        // }
    }
}
