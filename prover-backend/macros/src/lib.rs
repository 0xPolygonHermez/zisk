//! The `load_program!` proc macro: embed a ZisK guest ELF as a `const GuestProgram`.
//!
//! Two forms, both expanding to a plain `const` (no `LazyLock`, no runtime hashing):
//!
//! * `load_program!("name")` — embeds the ELF built by `build_program("...")`, reading its path
//!   and precomputed blake3 hash from the `ZISK_ELF_<name>` / `ZISK_ELF_HASH_<name>` env vars the
//!   build script emits.
//! * `load_program!("name", "path")` — embeds a prebuilt ELF at `path` (relative to the invoking
//!   crate's root). The file is read and hashed **at macro-expansion time**, so the hash is baked
//!   into the `const`. `include_bytes!` embeds the bytes and ties recompilation to the file.

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;
use std::path::PathBuf;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, LitStr, Token,
};

/// `load_program!("name")` or `load_program!("name", "path")`.
struct Args {
    name: LitStr,
    path: Option<LitStr>,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        let path = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            Some(input.parse()?)
        } else {
            None
        };
        Ok(Args { name, path })
    }
}

/// Load a guest program at compile time as a `const GuestProgram`. See the crate-level docs for
/// the two supported forms.
#[proc_macro]
pub fn load_program(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as Args);
    expand(args).unwrap_or_else(syn::Error::into_compile_error).into()
}

fn expand(Args { name, path }: Args) -> syn::Result<proc_macro2::TokenStream> {
    let krate = resolve_runtime_crate();
    let name_str = name.value();

    match path {
        // Form 1: built by `build_program`; path + hash come from build-script env vars.
        None => {
            let hash_env = format!("ZISK_ELF_HASH_{name_str}");
            let path_env = format!("ZISK_ELF_{name_str}");
            Ok(quote! {{
                #[cfg(zisk_skip_guest_build)]
                { #krate::GuestProgram::from_static(#name_str, "", &[]) }
                #[cfg(not(zisk_skip_guest_build))]
                {
                    #krate::GuestProgram::from_static(
                        #name_str,
                        env!(#hash_env),
                        include_bytes!(env!(#path_env)),
                    )
                }
            }})
        }

        // Form 2: prebuilt ELF at `path`, read and hashed at expansion time.
        Some(path) => {
            let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").map_err(|_| {
                syn::Error::new(Span::call_site(), "load_program!: CARGO_MANIFEST_DIR is not set")
            })?;
            let abs_path = PathBuf::from(&manifest_dir).join(path.value());
            let elf_bytes = std::fs::read(&abs_path).map_err(|err| {
                syn::Error::new(
                    path.span(),
                    format!("load_program!: cannot read ELF `{}`: {err}", abs_path.display()),
                )
            })?;
            let hash = blake3::hash(&elf_bytes).to_hex().to_string();
            let abs_str = abs_path.to_string_lossy().into_owned();

            Ok(quote! {
                #krate::GuestProgram::from_static(#name_str, #hash, include_bytes!(#abs_str))
            })
        }
    }
}

/// Resolve the path to the crate that exports `GuestProgram` (prover-backend directly, or via the
/// `zisk-sdk` re-export), honoring any rename the caller applied.
fn resolve_runtime_crate() -> proc_macro2::TokenStream {
    for dep in ["zisk-prover-backend", "zisk-sdk"] {
        match crate_name(dep) {
            Ok(FoundCrate::Itself) => return quote!(crate),
            Ok(FoundCrate::Name(name)) => {
                let ident = Ident::new(&name, Span::call_site());
                return quote!(::#ident);
            }
            Err(_) => continue,
        }
    }
    quote!(::zisk_sdk)
}
