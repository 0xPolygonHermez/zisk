#[macro_export]
macro_rules! trace {
    // Case with at least one field and optional trailing comma
    ($my_struct:ident { $first_field:ident : $first_field_type:ty $(, $rest:ident : $rest_type:ty)* $(,)? }) => {
        #[derive(Debug)]
        pub struct $my_struct {
            pub $first_field: $crate::trace_col::TraceCol<$first_field_type>,
            $(pub $rest: $crate::trace_col::TraceCol<$rest_type>),*
        }

        #[allow(dead_code)]
        impl $my_struct {
            pub fn new(num_rows: usize) -> Self {
                // PRECONDITIONS
                // num_rows must be greater than or equal to 2
                assert!(num_rows >= 2);
                // num_rows must be a power of 2
                assert!(num_rows & (num_rows - 1) == 0);

                Self {
                    $first_field: $crate::trace_col::TraceCol::new(num_rows),
                    $($rest: $crate::trace_col::TraceCol::new(num_rows)),*
                }
            }

            pub fn split(&self, num_segments: usize) -> Vec<Self> {
                // PRECONDITIONS
                // 1. num_segments must be greater than 0
                // 2. num_segments must be less than or equal to the length of the trace
                assert!(num_segments > 0 && num_segments <= self.$first_field.num_rows());

                let mut segments = Vec::with_capacity(num_segments);
                let segment_size = self.$first_field.num_rows() / num_segments;

                let mut start = 0;
                for _ in 0..num_segments {
                    let end = start + segment_size;
                    segments.push(Self {
                        $first_field: $crate::trace_col::TraceCol { col: self.$first_field.col[start..end].to_vec() },
                        $($rest: $crate::trace_col::TraceCol { col: self.$rest.col[start..end].to_vec() }),*
                    });
                    start = end;
                }
                segments
            }

            pub fn num_rows(&self) -> usize {
                self.$first_field.num_rows()
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::trace;
    use math::fields::f64::BaseElement;
    use rand::Rng;

    #[test]
    fn it_creates_a_simple_traces() {
        let num_rows = 256;

        trace!(Simple { field1: usize });
        let mut simple = Simple::new(num_rows);

        for i in 0..num_rows {
            simple.field1[i] = i;
        }

        for i in 0..num_rows {
            assert_eq!(simple.field1[i], i);
        }

        assert_eq!(simple.num_rows(), num_rows);
    }

    #[test]
    fn it_throws_an_error_when_new_trace_with_non_valid_size() {
        todo!();
    }

    #[test]
    fn it_creates_a_fibonacci_trace() {
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
            b: BaseElement
        });
        let mut fibonacci = Fibonacci::new(num_rows);

        fibonacci.a[0] = BaseElement::new(1);
        fibonacci.b[0] = BaseElement::new(1);

        for i in 1..num_rows {
            fibonacci.a[i] = fibonacci.b[i - 1];
            fibonacci.b[i] = fibonacci.a[i - 1] + fibonacci.b[i - 1];
        }

        for i in 1..num_rows {
            assert_eq!(fibonacci.a[i - 1] + fibonacci.b[i - 1], fibonacci.b[i]);
        }

        let num_segments = 2;
        let splitted = fibonacci.split(num_segments);

        assert_eq!(splitted[0].num_rows(), num_rows / num_segments);

        for i in 0..num_segments {
            for j in 1..num_rows / num_segments {
                assert_eq!(
                    splitted[i].a[j - 1] + splitted[i].b[j - 1],
                    splitted[i].b[j]
                );
            }
            if i != 0 {
                assert_eq!(
                    splitted[i - 1].a[splitted[i - 1].num_rows() - 1]
                        + splitted[i - 1].b[splitted[i - 1].num_rows() - 1],
                    splitted[i].b[0]
                );
            }
        }
    }
}
