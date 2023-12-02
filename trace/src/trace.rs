#[macro_export]
macro_rules! trace {
    // Case with at least one field and optional trailing comma
    (struct $my_struct:ident { $first_field:ident : $first_field_type:ty $(, $rest:ident : $rest_type:ty)* $(,)? }) => {
        #[derive(Debug)]
        pub struct $my_struct {
            pub $first_field: $crate::trace_col::TraceCol<$first_field_type>,
            $(pub $rest: $crate::trace_col::TraceCol<$rest_type>,)*
        }

        impl $my_struct {
            pub fn new(size: usize) -> Self {
                Self {
                    $first_field: $crate::trace_col::TraceCol::with_capacity(size),
                    $($rest: $crate::trace_col::TraceCol::with_capacity(size),)*
                }
            }
        }
    };
}

