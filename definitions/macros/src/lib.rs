//! Procedural macros backing `zisk-definitions`' multi-target constant codegen.
//!
//! `#[constants(..)]` on an inline module keeps every `pub const` exactly as written
//! (so `rustc` evaluates the DAG) and *additionally* emits, in the same module, a
//! `GROUP: GroupMeta` and an `EXPORTS: &[Export]` table. The generator crate
//! (`zisk-definitions-generator`) reads that table to write the Rust, C-header, PIL
//! and asm forms.
//!
//! `#[emit(..)]` on a single const overrides only the fields it names; everything
//! else inherits the module-level defaults (serde-style cascade). A const marked
//! `#[emit(internal)]` stays in the DAG but is emitted to no target.
//!
//! The emitted code references `zisk_definitions_generator::meta::*` directly, so a
//! consuming crate only needs `zisk-definitions-generator` as a dependency — it does
//! not have to re-export the schema as `crate::meta`.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    meta::ParseNestedMeta, parse_macro_input, spanned::Spanned, Attribute, Expr, ExprLit, Item,
    ItemConst, ItemMod, Lit, LitInt, LitStr, Meta, Type,
};

const T_RUST: u8 = 1;
const T_C: u8 = 2;
const T_PIL: u8 = 4;
const T_ASM: u8 = 8;

#[derive(Clone, Copy)]
enum Radix {
    Hex,
    Dec,
}

/// Module-level defaults parsed from `#[constants(..)]`.
struct Container {
    group: Option<String>,
    targets: u8,
    radix: Radix,
    fits: Option<u8>,
    c_prefix: String,
    pil_prefix: String,
    asm_prefix: String,
    c_file: Option<String>,
    pil_file: Option<String>,
    asm_file: Option<String>,
}

impl Default for Container {
    fn default() -> Self {
        // asm is opt-in (not in the default target set): only the hand-written
        // emulator asm needs a handful of constants, so groups add it explicitly.
        Container {
            group: None,
            targets: T_RUST | T_C | T_PIL,
            radix: Radix::Hex,
            fits: None,
            c_prefix: String::new(),
            pil_prefix: String::new(),
            asm_prefix: String::new(),
            c_file: None,
            pil_file: None,
            asm_file: None,
        }
    }
}

impl Container {
    fn parse_meta(&mut self, meta: ParseNestedMeta) -> syn::Result<()> {
        if meta.path.is_ident("group") {
            self.group = Some(meta.value()?.parse::<LitStr>()?.value());
        } else if meta.path.is_ident("to") {
            self.targets = 0;
            meta.parse_nested_meta(|m| add_target(&mut self.targets, &m))?;
        } else if meta.path.is_ident("hex") {
            self.radix = Radix::Hex;
        } else if meta.path.is_ident("dec") {
            self.radix = Radix::Dec;
        } else if meta.path.is_ident("fits") {
            self.fits = Some(meta.value()?.parse::<LitInt>()?.base10_parse()?);
        } else if meta.path.is_ident("c_prefix") {
            self.c_prefix = meta.value()?.parse::<LitStr>()?.value();
        } else if meta.path.is_ident("pil_prefix") {
            self.pil_prefix = meta.value()?.parse::<LitStr>()?.value();
        } else if meta.path.is_ident("asm_prefix") {
            self.asm_prefix = meta.value()?.parse::<LitStr>()?.value();
        } else if meta.path.is_ident("c_file") {
            self.c_file = Some(meta.value()?.parse::<LitStr>()?.value());
        } else if meta.path.is_ident("pil_file") {
            self.pil_file = Some(meta.value()?.parse::<LitStr>()?.value());
        } else if meta.path.is_ident("asm_file") {
            self.asm_file = Some(meta.value()?.parse::<LitStr>()?.value());
        } else {
            return Err(meta.error("unknown #[constants] argument"));
        }
        Ok(())
    }
}

/// Per-const overrides parsed from `#[emit(..)]`. `None` fields inherit the container.
#[derive(Default)]
struct Emit {
    internal: bool,
    targets: Option<u8>,
    skip: u8,
    radix: Option<Radix>,
    /// `None` = unspecified; `Some(None)` = `no_fits`; `Some(Some(n))` = `fits = n`.
    fits: Option<Option<u8>>,
    c_name: Option<String>,
    pil_name: Option<String>,
    asm_name: Option<String>,
}

impl Emit {
    fn parse_meta(&mut self, meta: ParseNestedMeta) -> syn::Result<()> {
        if meta.path.is_ident("internal") {
            self.internal = true;
        } else if meta.path.is_ident("to") {
            let mut t = 0u8;
            meta.parse_nested_meta(|m| add_target(&mut t, &m))?;
            self.targets = Some(t);
        } else if meta.path.is_ident("skip") {
            meta.parse_nested_meta(|m| add_target(&mut self.skip, &m))?;
        } else if meta.path.is_ident("hex") {
            self.radix = Some(Radix::Hex);
        } else if meta.path.is_ident("dec") {
            self.radix = Some(Radix::Dec);
        } else if meta.path.is_ident("fits") {
            self.fits = Some(Some(meta.value()?.parse::<LitInt>()?.base10_parse()?));
        } else if meta.path.is_ident("no_fits") {
            self.fits = Some(None);
        } else if meta.path.is_ident("c_name") {
            self.c_name = Some(meta.value()?.parse::<LitStr>()?.value());
        } else if meta.path.is_ident("pil_name") {
            self.pil_name = Some(meta.value()?.parse::<LitStr>()?.value());
        } else if meta.path.is_ident("asm_name") {
            self.asm_name = Some(meta.value()?.parse::<LitStr>()?.value());
        } else {
            return Err(meta.error("unknown #[emit] argument"));
        }
        Ok(())
    }
}

fn add_target(bits: &mut u8, m: &ParseNestedMeta) -> syn::Result<()> {
    if m.path.is_ident("rust") {
        *bits |= T_RUST;
    } else if m.path.is_ident("c") {
        *bits |= T_C;
    } else if m.path.is_ident("pil") {
        *bits |= T_PIL;
    } else if m.path.is_ident("asm") {
        *bits |= T_ASM;
    } else {
        return Err(m.error("expected `rust`, `c`, `pil`, or `asm`"));
    }
    Ok(())
}

#[proc_macro_attribute]
pub fn constants(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut container = Container::default();
    {
        let parser = syn::meta::parser(|meta| container.parse_meta(meta));
        parse_macro_input!(attr with parser);
    }

    let item_mod = parse_macro_input!(item as ItemMod);
    match expand(container, item_mod) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand(container: Container, item_mod: ItemMod) -> syn::Result<TokenStream2> {
    let ItemMod { vis, ident, attrs: mod_attrs, content, .. } = item_mod;
    let ident_span = ident.span();

    let content = match content {
        Some((_, items)) => items,
        None => {
            return Err(syn::Error::new(
                ident_span,
                "#[constants] requires an inline module body: `mod name { .. }`",
            ))
        }
    };

    let group_name = container.group.clone().unwrap_or_else(|| ident.to_string());

    let mut out_items: Vec<TokenStream2> = Vec::new();
    let mut exports: Vec<TokenStream2> = Vec::new();

    for it in content {
        match it {
            Item::Const(mut c) => {
                let mut emit = Emit::default();
                let mut kept_attrs: Vec<Attribute> = Vec::new();
                for a in std::mem::take(&mut c.attrs) {
                    if a.path().is_ident("emit") {
                        a.parse_nested_meta(|m| emit.parse_meta(m))?;
                    } else {
                        kept_attrs.push(a);
                    }
                }
                c.attrs = kept_attrs;

                // Keep the const verbatim (rustc evaluates the DAG; doc attrs stay).
                out_items.push(quote!(#c));

                if !emit.internal {
                    exports.push(build_export(&container, &emit, &c)?);
                }
            }
            other => out_items.push(quote!(#other)),
        }
    }

    let c_prefix = &container.c_prefix;
    let pil_prefix = &container.pil_prefix;
    let asm_prefix = &container.asm_prefix;
    let c_file = opt(&container.c_file);
    let pil_file = opt(&container.pil_file);
    let asm_file = opt(&container.asm_file);

    Ok(quote! {
        #(#mod_attrs)*
        #vis mod #ident {
            #(#out_items)*

            pub const GROUP: zisk_definitions_generator::meta::GroupMeta = zisk_definitions_generator::meta::GroupMeta {
                name: #group_name,
                c_prefix: #c_prefix,
                pil_prefix: #pil_prefix,
                asm_prefix: #asm_prefix,
                c_file: #c_file,
                pil_file: #pil_file,
                asm_file: #asm_file,
            };

            pub const EXPORTS: &[zisk_definitions_generator::meta::Export] = &[ #(#exports),* ];
        }
    })
}

fn build_export(container: &Container, emit: &Emit, c: &ItemConst) -> syn::Result<TokenStream2> {
    let id = &c.ident;
    let name = id.to_string();

    let (bits, kind) = classify_type(&c.ty)?;
    let value = match kind {
        Kind::Uint => quote!( zisk_definitions_generator::meta::Value::U(#id as u128) ),
        Kind::Int => quote!( zisk_definitions_generator::meta::Value::I(#id as i128) ),
        Kind::Str => quote!( zisk_definitions_generator::meta::Value::Str(#id) ),
    };

    let mut targets = emit.targets.unwrap_or(container.targets);
    targets &= !emit.skip;
    // Emit the target set symbolically so `meta::Targets` remains the single
    // source of truth for the bit values (no duplicated 1/2/4 across crates).
    let mut target_parts: Vec<TokenStream2> = Vec::new();
    if targets & T_RUST != 0 {
        target_parts.push(quote!(zisk_definitions_generator::meta::Targets::RUST.0));
    }
    if targets & T_C != 0 {
        target_parts.push(quote!(zisk_definitions_generator::meta::Targets::C.0));
    }
    if targets & T_PIL != 0 {
        target_parts.push(quote!(zisk_definitions_generator::meta::Targets::PIL.0));
    }
    if targets & T_ASM != 0 {
        target_parts.push(quote!(zisk_definitions_generator::meta::Targets::ASM.0));
    }
    let targets_tok = if target_parts.is_empty() {
        quote!(zisk_definitions_generator::meta::Targets(0))
    } else {
        quote!(zisk_definitions_generator::meta::Targets( #(#target_parts)|* ))
    };

    let radix = emit.radix.unwrap_or(container.radix);
    let radix_tok = match radix {
        Radix::Hex => quote!(Hex),
        Radix::Dec => quote!(Dec),
    };

    let (fits, no_fit) = match emit.fits {
        Some(None) => (None, true),
        Some(Some(n)) => (Some(n), false),
        None => (container.fits, false),
    };
    let fits_tok = opt(&fits);

    let c_name = opt(&emit.c_name);
    let pil_name = opt(&emit.pil_name);
    let asm_name = opt(&emit.asm_name);

    // Provenance: only carry the expression when it is derived (not a bare literal).
    let expr = &c.expr;
    let expr_str =
        if matches!(&**expr, Expr::Lit(_)) { String::new() } else { quote!(#expr).to_string() };
    let doc = doc_of(&c.attrs);

    Ok(quote! {
        zisk_definitions_generator::meta::Export {
            name: #name,
            value: #value,
            ty_bits: #bits,
            targets: #targets_tok,
            radix: zisk_definitions_generator::meta::Radix::#radix_tok,
            fits: #fits_tok,
            no_fit: #no_fit,
            c_name: #c_name,
            pil_name: #pil_name,
            asm_name: #asm_name,
            expr: #expr_str,
            doc: #doc,
        }
    })
}

enum Kind {
    Uint,
    Int,
    Str,
}

/// Maps a supported const type to `(storage_bits, kind)`.
///
/// `usize`/`isize` map to 64 bits: ZisK targets a fixed 64-bit word, so this is an
/// intentional target assumption, not host-dependent.
fn classify_type(ty: &Type) -> syn::Result<(u8, Kind)> {
    match ty {
        Type::Path(tp) => {
            let id = tp.path.get_ident().map(|i| i.to_string());
            match id.as_deref() {
                Some("u8") => Ok((8, Kind::Uint)),
                Some("u16") => Ok((16, Kind::Uint)),
                Some("u32") => Ok((32, Kind::Uint)),
                Some("u64") => Ok((64, Kind::Uint)),
                Some("u128") => Ok((128, Kind::Uint)),
                Some("usize") => Ok((64, Kind::Uint)),
                Some("i8") => Ok((8, Kind::Int)),
                Some("i16") => Ok((16, Kind::Int)),
                Some("i32") => Ok((32, Kind::Int)),
                Some("i64") => Ok((64, Kind::Int)),
                Some("i128") => Ok((128, Kind::Int)),
                Some("isize") => Ok((64, Kind::Int)),
                _ => Err(syn::Error::new(
                    ty.span(),
                    "unsupported const type for #[constants]; use u8..=u128 / i8..=i128 / usize / &str",
                )),
            }
        }
        Type::Reference(r) => {
            if let Type::Path(tp) = &*r.elem {
                if tp.path.is_ident("str") {
                    return Ok((0, Kind::Str));
                }
            }
            Err(syn::Error::new(ty.span(), "unsupported reference type; only `&str` is allowed"))
        }
        _ => Err(syn::Error::new(ty.span(), "unsupported const type for #[constants]")),
    }
}

/// Emit `Some(<v>)` or `None` for an optional macro argument — works for any
/// `ToTokens` inner type (`String`, `u8`, …).
fn opt<T: quote::ToTokens>(o: &Option<T>) -> TokenStream2 {
    match o {
        Some(v) => quote!(Some(#v)),
        None => quote!(None),
    }
}

fn doc_of(attrs: &[Attribute]) -> String {
    let mut parts: Vec<String> = Vec::new();
    for a in attrs {
        if a.path().is_ident("doc") {
            if let Meta::NameValue(nv) = &a.meta {
                if let Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) = &nv.value {
                    parts.push(s.value().trim().to_string());
                }
            }
        }
    }
    parts.join(" ")
}
