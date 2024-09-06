use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, format_ident, ToTokens};
use syn::{
    parse2,
    parse::{Parse, ParseStream},
    Ident, Generics, FieldsNamed, Result, Field, Token,
};

#[proc_macro]
pub fn trace(input: TokenStream) -> TokenStream {
    match trace_impl(input.into()) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn trace_impl(input: TokenStream2) -> Result<TokenStream2> {
    let parsed_input: ParsedTraceInput = parse2(input)?;

    let row_struct_name = parsed_input.row_struct_name;
    let trace_struct_name = parsed_input.struct_name;
    let generics = parsed_input.generics.params;
    let fields = parsed_input.fields;

    // Calculate ROW_SIZE
    let row_size = fields.named.iter().map(|field| calculate_field_size_literal(&field.ty)).sum::<usize>();

    // Generate row struct
    let field_definitions = fields.named.iter().map(|field| {
        let Field { ident, ty, .. } = field;
        quote! { pub #ident: #ty, }
    });

    let row_struct = quote! {
        #[derive(Debug, Clone, Copy, Default)]
        pub struct #row_struct_name<#generics> {
            #(#field_definitions)*
        }
    };

    // Generate trace struct with lifetime
    let trace_struct = quote! {
        pub struct #trace_struct_name<'a, #generics> {
            pub buffer: Option<Vec<#generics>>,
            pub slice_trace: &'a mut [#row_struct_name<#generics>],
            num_rows: usize,
        }
    };

    let impl_block = quote! {
        impl<#generics: Default + Clone + Copy> #trace_struct_name<'_, #generics> {
            pub const ROW_SIZE: usize = #row_size;

            pub fn new(num_rows: usize) -> Self {
                assert!(num_rows >= 2);
                assert!(num_rows & (num_rows - 1) == 0);
                let buffer = vec![#generics::default(); num_rows * Self::ROW_SIZE];
                let slice_trace = unsafe {
                    std::slice::from_raw_parts_mut(buffer.as_ptr() as *mut #row_struct_name<#generics>, num_rows)
                };
                Self { buffer: Some(buffer), slice_trace, num_rows }
            }

            pub fn num_rows(&self) -> usize {
                self.num_rows
            }
        }

        impl<'a, #generics> std::ops::Index<usize> for #trace_struct_name<'a, #generics> {
            type Output = #row_struct_name<#generics>;

            fn index(&self, index: usize) -> &Self::Output {
                &self.slice_trace[index]
            }
        }

        impl<'a, #generics> std::ops::IndexMut<usize> for #trace_struct_name<'a, #generics> {
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

// A struct to handle parsing the input and all the syntactic variations
struct ParsedTraceInput {
    row_struct_name: Ident,
    struct_name: Ident,
    generics: Generics,
    fields: FieldsNamed,
}

impl Parse for ParsedTraceInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        let row_struct_name;

        // Handle explicit or implicit row struct names
        if lookahead.peek(Ident) && input.peek2(Token![,]) {
            row_struct_name = Some(input.parse::<Ident>()?);
            input.parse::<Token![,]>()?; // Skip comma after explicit row name
        } else {
            row_struct_name = None;
        }

        let struct_name = input.parse::<Ident>()?;
        let row_struct_name = row_struct_name.unwrap_or_else(|| format_ident!("{}Row", struct_name));

        let generics: Generics = input.parse()?;
        let fields: FieldsNamed = input.parse()?;

        Ok(ParsedTraceInput { row_struct_name, struct_name, generics, fields })
    }
}

// Calculate the size of a field based on its type and return it as a usize literal
fn calculate_field_size_literal(field_type: &syn::Type) -> usize {
    match field_type {
        // Handle arrays with multiple dimensions
        syn::Type::Array(type_array) => {
            let len = type_array.len.to_token_stream().to_string().parse::<usize>().unwrap();
            let elem_size = calculate_field_size_literal(&type_array.elem);
            len * elem_size
        }
        // For simple types, the size is 1
        _ => 1,
    }
}

#[test]
fn test_simple_struct_without_struct_keyword() {
    let input = quote! {
        Simple<F> { a: F, b: F, c: F }
    };

    let expected = quote! {
        #[derive(Debug, Clone, Copy, Default)]
        pub struct SimpleRow<F> {
            pub a: F,
            pub b: F,
            pub c: F,
        }
        pub struct Simple<'a, F> {
            pub buffer: Option<Vec<F>>,
            pub slice_trace: &'a mut [SimpleRow<F>],
            num_rows: usize,
        }
        impl<F: Default + Clone + Copy> Simple<'_, F> {
            pub const ROW_SIZE: usize = 3usize;

            pub fn new(num_rows: usize) -> Self {
                assert!(num_rows >= 2);
                assert!(num_rows & (num_rows - 1) == 0);
                let buffer = vec![F::default(); num_rows * Self::ROW_SIZE];
                let slice_trace = unsafe {
                    std::slice::from_raw_parts_mut(buffer.as_ptr() as *mut SimpleRow<F>, num_rows)
                };
                Self { buffer: Some(buffer), slice_trace, num_rows }
            }

            pub fn num_rows(&self) -> usize {
                self.num_rows
            }
        }

        impl<'a, F> std::ops::Index<usize> for Simple<'a, F> {
            type Output = SimpleRow<F>;

            fn index(&self, index: usize) -> &Self::Output {
                &self.slice_trace[index]
            }
        }

        impl<'a, F> std::ops::IndexMut<usize> for Simple<'a, F> {
            fn index_mut(&mut self, index: usize) -> &mut Self::Output {
                &mut self.slice_trace[index]
            }
        }
    };

    let generated = trace_impl(input.into()).unwrap();
    assert_eq!(generated.to_string(), expected.into_token_stream().to_string());
}

#[test]
fn test_explicit_row_and_trace_struct() {
    let input = quote! {
        SimpleRow, Simple<F> { a: F, b: F }
    };

    let expected = quote! {
        #[derive(Debug, Clone, Copy, Default)]
        pub struct SimpleRow<F> {
            pub a: F,
            pub b: F,
        }
        pub struct Simple<'a, F> {
            pub buffer: Option<Vec<F>>,
            pub slice_trace: &'a mut [SimpleRow<F>],
            num_rows: usize,
        }
        impl<F: Default + Clone + Copy> Simple<'_, F> {
            pub const ROW_SIZE: usize = 2usize;

            pub fn new(num_rows: usize) -> Self {
                assert!(num_rows >= 2);
                assert!(num_rows & (num_rows - 1) == 0);
                let buffer = vec![F::default(); num_rows * Self::ROW_SIZE];
                let slice_trace = unsafe {
                    std::slice::from_raw_parts_mut(buffer.as_ptr() as *mut SimpleRow<F>, num_rows)
                };
                Self { buffer: Some(buffer), slice_trace, num_rows }
            }

            pub fn num_rows(&self) -> usize {
                self.num_rows
            }
        }

        impl<'a, F> std::ops::Index<usize> for Simple<'a, F> {
            type Output = SimpleRow<F>;

            fn index(&self, index: usize) -> &Self::Output {
                &self.slice_trace[index]
            }
        }

        impl<'a, F> std::ops::IndexMut<usize> for Simple<'a, F> {
            fn index_mut(&mut self, index: usize) -> &mut Self::Output {
                &mut self.slice_trace[index]
            }
        }
    };

    let generated = trace_impl(input.into()).unwrap();
    assert_eq!(generated.to_string(), expected.into_token_stream().to_string());
}

#[test]
fn test_parsing_01() {
    let input = quote! {
        TraceRow, MyTrace<F> { a: F, b: F }
    };
    let parsed: ParsedTraceInput = parse2(input).unwrap();
    assert_eq!(parsed.row_struct_name, "TraceRow");
    assert_eq!(parsed.struct_name, "MyTrace");
}

#[test]
fn test_parsing_02() {
    let input = quote! {
        SimpleRow, Simple<F> { a: F }
    };
    let parsed: ParsedTraceInput = parse2(input).unwrap();
    assert_eq!(parsed.row_struct_name, "SimpleRow");
    assert_eq!(parsed.struct_name, "Simple");
}

#[test]
fn test_parsing_03() {
    let input = quote! {
        Simple<F> { a: F }
    };
    let parsed: ParsedTraceInput = parse2(input).unwrap();
    assert_eq!(parsed.row_struct_name, "SimpleRow");
    assert_eq!(parsed.struct_name, "Simple");
}
