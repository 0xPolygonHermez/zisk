#[macro_export]
macro_rules! trace {
    // Case with at least one field and optional trailing comma
    ($my_struct:ident { $first_field:ident : $first_field_type:ty $(, $rest:ident : $rest_type:ty)* $(,)? }) => {
        #[derive(Debug)]
        pub struct $my_struct {
            pub $first_field: $crate::trace_col::TraceCol<$first_field_type>,
            $(pub $rest: $crate::trace_col::TraceCol<$rest_type>,)*
        }

        #[allow(dead_code)]
        impl $my_struct {
            pub fn new(num_rows: usize) -> Self {
                // PRECONDITIONS
                // Size must be greater than 0
                assert!(num_rows >= 2);
                
                Self {
                    $first_field: $crate::trace_col::TraceCol::new(num_rows),
                    $($rest: $crate::trace_col::TraceCol::new(num_rows),)*
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
                        $($rest: $crate::trace_col::TraceCol { col: self.$rest.col[start..end].to_vec() },)*
                    });
                    start = end;
                }
                segments
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
        let mut simple = Simple::new(1024);

        for i in 0..num_rows { simple.field1[i] = i; }
        
        for i in 0..num_rows { assert_eq!(simple.field1[i], i); }
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
        trace!(Fibonacci16 { a: BaseElement, b: BaseElement });
        let mut fibonacci = Fibonacci16::new(num_rows);

        fibonacci.a[0] = BaseElement::new(1);
        fibonacci.b[0] = BaseElement::new(1);

        for i in 1..num_rows {
            fibonacci.a[i] = fibonacci.b[i - 1];
            fibonacci.b[i] = fibonacci.a[i - 1] + fibonacci.b[i - 1];
        }

        // ASSERTIONS
        for i in 1..num_rows { assert_eq!(fibonacci.a[i - 1] + fibonacci.b[i - 1], fibonacci.b[i]); }
    }
}