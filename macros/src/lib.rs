use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, format_ident, ToTokens};
use syn::{parse2, DeriveInput, FieldsNamed, Ident, Type, Result, Field};

#[proc_macro]
pub fn trace(input: TokenStream) -> TokenStream {
    match trace_impl(input.into()) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn trace_impl(input: TokenStream2) -> Result<TokenStream2> {
    let derive_input = parse2::<DeriveInput>(input)?;

    // Extract the struct name and generic parameters
    let row_struct_name = &derive_input.ident;
    let trace_struct_name = format_ident!("{}Trace", row_struct_name);

    let generic_param = &derive_input.generics.params.first().unwrap(); // Assuming there's one generic param (like <F>)

    // Extract fields from the struct
    let fields_def = match &derive_input.data {
        syn::Data::Struct(data_struct) => match &data_struct.fields {
            syn::Fields::Named(FieldsNamed { named, .. }) => named,
            _ => return Err(syn::Error::new_spanned(derive_input, "Expected named fields")),
        },
        _ => return Err(syn::Error::new_spanned(derive_input, "Expected struct data")),
    };

    // Build up the integer literal for ROW_SIZE
    let row_size = fields_def.iter().map(|field| calculate_field_size_literal(&field.ty)).sum::<usize>();

    // Generate the row struct
    let field_definitions = fields_def.iter().map(|field| {
        let Field { ident, ty, .. } = field;
        quote! { pub #ident: #ty, }
    });

    let row_struct = quote! {
        #[derive(Debug, Clone, Copy, Default)]
        pub struct #row_struct_name<#generic_param> {
            #(#field_definitions)*
        }
    };

    // Generate the trace struct
    let trace_struct = quote! {
        pub struct #trace_struct_name<'a, #generic_param> {
            pub buffer: Option<Vec<#generic_param>>,
            pub slice_trace: &'a mut [#row_struct_name<#generic_param>],
            num_rows: usize,
        }
    };

    let impl_block = quote! {
        impl<#generic_param: Default + Clone + Copy> #trace_struct_name<'_, #generic_param> {
            pub const ROW_SIZE: usize = #row_size;

            pub fn new(num_rows: usize) -> Self {
                assert!(num_rows >= 2);
                assert!(num_rows & (num_rows - 1) == 0);
                let buffer = vec![#generic_param::default(); num_rows * Self::ROW_SIZE];
                let slice_trace = unsafe {
                    std::slice::from_raw_parts_mut(buffer.as_ptr() as *mut #row_struct_name<#generic_param>, num_rows)
                };
                Self { buffer: Some(buffer), slice_trace, num_rows }
            }

            pub fn num_rows(&self) -> usize {
                self.num_rows
            }
        }

        impl<'a, #generic_param> std::ops::Index<usize> for #trace_struct_name<'a, #generic_param> {
            type Output = #row_struct_name<#generic_param>;

            fn index(&self, index: usize) -> &Self::Output {
                &self.slice_trace[index]
            }
        }

        impl<'a, #generic_param> std::ops::IndexMut<usize> for #trace_struct_name<'a, #generic_param> {
            fn index_mut(&mut self, index: usize) -> &mut Self::Output {
                &mut self.slice_trace[index]
            }
        }
    };

    Ok(quote! {
        #row_struct
        #trace_struct
        #impl_block
    })
}

// Calculate the size of a field based on its type and return it as a usize literal
fn calculate_field_size_literal(field_type: &Type) -> usize {
    match field_type {
        // Handle arrays with multiple dimensions
        Type::Array(type_array) => {
            let len = type_array.len.to_token_stream().to_string().parse::<usize>().unwrap();
            let elem_size = calculate_field_size_literal(&type_array.elem);
            len * elem_size
        }
        // For simple types, the size is 1
        _ => 1,
    }
}

// Tests

#[test]
fn test_simple_struct() {
    let input = quote! {
        struct TraceRow1<F> { a: F, b: F, c: F }
    };

    let expected = quote! {
        pub struct TraceRow1<F> {
            pub a: F,
            pub b: F,
            pub c: F,
        }
        pub struct TraceRow1Trace<'a, F> {
            pub buffer: Option<Vec<F>>,
            pub slice_trace: &'a mut [TraceRow1<F>],
            num_rows: usize,
        }
        impl<F: Default + Clone + Copy> TraceRow1Trace<'_, F> {
            pub const ROW_SIZE: usize = 3usize;

            pub fn new(num_rows: usize) -> Self {
                assert!(num_rows >= 2);
                assert!(num_rows & (num_rows - 1) == 0);
                let buffer = vec![F::default(); num_rows * Self::ROW_SIZE];
                let slice_trace = unsafe {
                    std::slice::from_raw_parts_mut(buffer.as_ptr() as *mut TraceRow1<F>, num_rows)
                };
                Self { buffer: Some(buffer), slice_trace, num_rows }
            }

            pub fn num_rows(&self) -> usize {
                self.num_rows
            }
        }

        impl<'a, F> std::ops::Index<usize> for TraceRow1Trace<'a, F> {
            type Output = TraceRow1<F>;

            fn index(&self, index: usize) -> &Self::Output {
                &self.slice_trace[index]
            }
        }

        impl<'a, F> std::ops::IndexMut<usize> for TraceRow1Trace<'a, F> {
            fn index_mut(&mut self, index: usize) -> &mut Self::Output {
                &mut self.slice_trace[index]
            }
        }
    };

    let parsed_input = parse2::<DeriveInput>(input).unwrap();
    let generated = trace_impl(parsed_input.into_token_stream()).unwrap();

    assert_eq!(generated.to_string(), expected.to_string());
}

#[test]
fn test_three_dimensional_array() {
    let input = quote! {
        struct TraceRow3<F> { a: [[F; 3]; 2], b: F }
    };

    let expected = quote! {
        pub struct TraceRow3<F> {
            pub a: [[F; 3]; 2],
            pub b: F,
        }
        pub struct TraceRow3Trace<'a, F> {
            pub buffer: Option<Vec<F>>,
            pub slice_trace: &'a mut [TraceRow3<F>],
            num_rows: usize,
        }
        impl<F: Default + Clone + Copy> TraceRow3Trace<'_, F> {
            pub const ROW_SIZE: usize = 7usize;

            pub fn new(num_rows: usize) -> Self {
                assert!(num_rows >= 2);
                assert!(num_rows & (num_rows - 1) == 0);
                let buffer = vec![F::default(); num_rows * Self::ROW_SIZE];
                let slice_trace = unsafe {
                    std::slice::from_raw_parts_mut(buffer.as_ptr() as *mut TraceRow3<F>, num_rows)
                };
                Self { buffer: Some(buffer), slice_trace, num_rows }
            }

            pub fn num_rows(&self) -> usize {
                self.num_rows
            }
        }

        impl<'a, F> std::ops::Index<usize> for TraceRow3Trace<'a, F> {
            type Output = TraceRow3<F>;

            fn index(&self, index: usize) -> &Self::Output {
                &self.slice_trace[index]
            }
        }

        impl<'a, F> std::ops::IndexMut<usize> for TraceRow3Trace<'a, F> {
            fn index_mut(&mut self, index: usize) -> &mut Self::Output {
                &mut self.slice_trace[index]
            }
        }
    };

    let parsed_input = parse2::<DeriveInput>(input).unwrap();
    let generated = trace_impl(parsed_input.into_token_stream()).unwrap();

    assert_eq!(generated.to_string(), expected.to_string());
}
