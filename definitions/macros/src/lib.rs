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
use quote::{format_ident, quote};
use syn::{
    meta::ParseNestedMeta, parse_macro_input, spanned::Spanned, Attribute, Expr, ExprLit, Item,
    ItemConst, ItemMod, Lit, LitInt, LitStr, Meta, Type, Visibility,
};

/// The targets `to(..)`/`skip(..)` accept: the keyword and its `meta::Targets` const.
/// Inside the macro a target set is a `u8` with bit `1 << index`; the emitted code names
/// the consts, so their values stay defined only in `meta::Targets`.
const TARGETS: [(&str, &str); 4] = [("rust", "RUST"), ("c", "C"), ("pil", "PIL"), ("asm", "ASM")];

#[derive(Clone, Copy, Default)]
enum Radix {
    #[default]
    Hex,
    Dec,
}

/// Module-level defaults parsed from `#[constants(..)]`.
#[derive(Default)]
struct Container {
    group: Option<String>,
    /// Targets the group emits to. `None` until `to(..)` is parsed; `to(..)` is required
    /// (a group without it is a hard error in `expand`), so there is no implicit default.
    targets: Option<u8>,
    radix: Radix,
    fits: Option<u8>,
    c_prefix: String,
    pil_prefix: String,
    asm_prefix: String,
    c_file: Option<String>,
    pil_file: Option<String>,
    asm_file: Option<String>,
}

impl Container {
    fn parse_meta(&mut self, meta: ParseNestedMeta) -> syn::Result<()> {
        match ident_of(&meta).as_str() {
            "group" => self.group = Some(lit_str(&meta)?),
            "to" => self.targets = Some(parse_targets(&meta)?),
            "hex" => self.radix = Radix::Hex,
            "dec" => self.radix = Radix::Dec,
            "fits" => self.fits = Some(parse_fits(&meta)?),
            "c_prefix" => self.c_prefix = lit_str(&meta)?,
            "pil_prefix" => self.pil_prefix = lit_str(&meta)?,
            "asm_prefix" => self.asm_prefix = lit_str(&meta)?,
            "c_file" => self.c_file = Some(lit_str(&meta)?),
            "pil_file" => self.pil_file = Some(lit_str(&meta)?),
            "asm_file" => self.asm_file = Some(lit_str(&meta)?),
            _ => return Err(meta.error("unknown #[constants] argument")),
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
        match ident_of(&meta).as_str() {
            "internal" => self.internal = true,
            "to" => self.targets = Some(parse_targets(&meta)?),
            "skip" => self.skip |= parse_targets(&meta)?,
            "hex" => self.radix = Some(Radix::Hex),
            "dec" => self.radix = Some(Radix::Dec),
            "fits" => self.fits = Some(Some(parse_fits(&meta)?)),
            "no_fits" => self.fits = Some(None),
            "c_name" => self.c_name = Some(lit_str(&meta)?),
            "pil_name" => self.pil_name = Some(lit_str(&meta)?),
            "asm_name" => self.asm_name = Some(lit_str(&meta)?),
            _ => return Err(meta.error("unknown #[emit] argument")),
        }
        Ok(())
    }
}

/// Parses a `fits = N` bound. Zero would accept nothing meaningful (and the engine
/// reserves width 0 for strings), and nothing wider than 128 bits is representable.
fn parse_fits(meta: &ParseNestedMeta) -> syn::Result<u8> {
    let lit = meta.value()?.parse::<LitInt>()?;
    let bits: u8 = lit.base10_parse()?;
    if !(1..=128).contains(&bits) {
        return Err(syn::Error::new(
            lit.span(),
            "`fits` must be between 1 and 128 bits; use `no_fits` to disable the check",
        ));
    }
    Ok(bits)
}

/// The argument's name, or `""` for a multi-segment path (which no argument matches).
fn ident_of(meta: &ParseNestedMeta) -> String {
    meta.path.get_ident().map(ToString::to_string).unwrap_or_default()
}

/// The string literal of a `key = "value"` argument.
fn lit_str(meta: &ParseNestedMeta) -> syn::Result<String> {
    Ok(meta.value()?.parse::<LitStr>()?.value())
}

/// The target set of a `to(..)`/`skip(..)` list.
fn parse_targets(meta: &ParseNestedMeta) -> syn::Result<u8> {
    let mut bits = 0u8;
    meta.parse_nested_meta(|m| {
        let i = TARGETS
            .iter()
            .position(|(kw, _)| m.path.is_ident(kw))
            .ok_or_else(|| m.error("expected `rust`, `c`, `pil`, or `asm`"))?;
        bits |= 1 << i;
        Ok(())
    })?;
    Ok(bits)
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

    // `to(..)` is mandatory: a group must state every target it emits to, so its fan-out
    // is explicit at the definition site (no default set to memorize, no asm asymmetry).
    let Some(group_targets) = container.targets else {
        return Err(syn::Error::new(
            ident_span,
            "#[constants] requires `to(..)`: list every target the group emits to, \
             e.g. `to(rust, c, pil)`",
        ));
    };

    let group_name = container.group.clone().unwrap_or_else(|| ident.to_string());

    let mut out_items: Vec<TokenStream2> = Vec::new();
    let mut exports: Vec<TokenStream2> = Vec::new();

    for it in content {
        match it {
            Item::Const(mut c) => {
                // The generated files are committed and drift-checked, so they must not
                // depend on the build that renders them, which a conditional const (and
                // anything derived from it) would. It would also leave an EXPORTS entry
                // naming a const the cfg removed.
                if let Some(a) = c
                    .attrs
                    .iter()
                    .find(|a| a.path().is_ident("cfg") || a.path().is_ident("cfg_attr"))
                {
                    return Err(syn::Error::new(
                        a.span(),
                        "#[constants] does not allow `#[cfg]`/`#[cfg_attr]` on a const: \
                         generated output must not depend on the build configuration",
                    ));
                }

                let mut emit = Emit::default();
                for a in c.attrs.iter().filter(|a| a.path().is_ident("emit")) {
                    a.parse_nested_meta(|m| emit.parse_meta(m))?;
                }
                c.attrs.retain(|a| !a.path().is_ident("emit"));

                // Exported consts become `pub` in the generated Rust and visible to every
                // target, so only a `pub` const may be exported; a private helper must say
                // it stays out of the outputs.
                if !emit.internal && !matches!(c.vis, Visibility::Public(_)) {
                    return Err(syn::Error::new(
                        c.ident.span(),
                        "#[constants] exports this const: make it `pub`, or mark it \
                         `#[emit(internal)]` to keep it out of every target",
                    ));
                }

                // Keep the const verbatim (rustc evaluates the DAG; doc attrs stay).
                out_items.push(quote!(#c));

                if !emit.internal {
                    exports.push(build_export(&container, group_targets, &emit, &c)?);
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

fn build_export(
    container: &Container,
    group_targets: u8,
    emit: &Emit,
    c: &ItemConst,
) -> syn::Result<TokenStream2> {
    let id = &c.ident;
    let name = id.to_string();

    let (bits, kind) = classify_type(&c.ty)?;
    // 64 bits like u64/i64 for every other target, but the generated Rust must keep the
    // pointer-sized type, or a `usize` length/index const comes out as `u64`.
    let pointer_sized = matches!(&*c.ty, Type::Path(tp)
        if tp.path.is_ident("usize") || tp.path.is_ident("isize"));
    let value = match kind {
        Kind::Uint => quote!( zisk_definitions_generator::meta::Value::U(#id as u128) ),
        Kind::Int => quote!( zisk_definitions_generator::meta::Value::I(#id as i128) ),
        Kind::Str => quote!( zisk_definitions_generator::meta::Value::Str(#id) ),
    };

    let targets = emit.targets.unwrap_or(group_targets) & !emit.skip;
    // `skip(..)` can subtract every target; emitting nowhere has its own explicit
    // spelling, so an empty set here is a mistake, not a silent omission.
    if targets == 0 {
        return Err(syn::Error::new(
            c.ident.span(),
            "this const emits to no target after `skip(..)`: keep a target, or use \
             `#[emit(internal)]` to emit nowhere",
        ));
    }
    let target_parts = TARGETS.iter().enumerate().filter(|(i, _)| targets & (1 << i) != 0).map(
        |(_, (_, name))| {
            let name = format_ident!("{name}");
            quote!(zisk_definitions_generator::meta::Targets::#name.0)
        },
    );
    let targets_tok = quote!(zisk_definitions_generator::meta::Targets( #(#target_parts)|* ));

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
            pointer_sized: #pointer_sized,
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
/// intentional target assumption, not host-dependent. (The generated Rust still keeps
/// the pointer-sized type; see `pointer_sized` in `build_export`.)
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
                    "unsupported const type for #[constants]; use u8..=u128 / i8..=i128 / usize / isize / &str",
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

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream as TokenStream2;
    use quote::quote;
    use syn::parse::Parser;

    use super::{expand, Container};

    /// Runs the `#[constants(attr)]` expansion over `item`, as the attribute would.
    fn constants(attr: TokenStream2, item: TokenStream2) -> syn::Result<TokenStream2> {
        let mut container = Container::default();
        syn::meta::parser(|meta| container.parse_meta(meta)).parse2(attr)?;
        expand(container, syn::parse2(item)?)
    }

    /// `pub mod g` holding one `pub const X: u64`, with `attrs` on the const.
    fn module(attrs: TokenStream2) -> TokenStream2 {
        quote!(pub mod g { #attrs pub const X: u64 = 1; })
    }

    #[test]
    fn private_const_must_be_internal() {
        let private = quote!(
            pub mod g {
                const HELPER: u64 = 1;
            }
        );
        let err = constants(quote!(to(rust)), private).unwrap_err().to_string();
        assert!(err.contains("make it `pub`"), "{err}");

        let internal = quote!(
            pub mod g {
                #[emit(internal)]
                const HELPER: u64 = 1;
            }
        );
        assert!(constants(quote!(to(rust)), internal).is_ok());
    }

    #[test]
    fn conditional_consts_are_rejected() {
        for attr in [quote!(#[cfg(test)]), quote!(#[cfg_attr(test, emit(internal))])] {
            let err = constants(quote!(to(rust)), module(attr)).unwrap_err().to_string();
            assert!(err.contains("build configuration"), "{err}");
        }
        // Conditional non-const items (e.g. a test module) are left to the compiler.
        let item = quote!(
            pub mod g {
                pub const X: u64 = 1;
                #[cfg(test)]
                mod t {}
            }
        );
        assert!(constants(quote!(to(rust)), item).is_ok());
    }

    #[test]
    fn empty_target_lists_are_rejected() {
        // syn already rejects an empty nested list ("expected nested attribute"), so an
        // empty `to()` can neither pass the required-`to(..)` check nor silently emit a
        // const nowhere. Pinned here so a syn upgrade can't quietly change it.
        assert!(constants(quote!(to()), module(quote!())).is_err());
        assert!(constants(quote!(to(rust)), module(quote!(#[emit(to())]))).is_err());
    }

    #[test]
    fn skipping_every_target_is_rejected() {
        let skip_all = module(quote!(#[emit(skip(rust))]));
        let err = constants(quote!(to(rust)), skip_all).unwrap_err().to_string();
        assert!(err.contains("emit(internal)"), "{err}");

        // Explicitly emitting nowhere stays available.
        assert!(constants(quote!(to(rust)), module(quote!(#[emit(internal)]))).is_ok());
    }

    #[test]
    fn fits_must_be_1_to_128_bits() {
        for bad in [quote!(to(c), fits = 0), quote!(to(c), fits = 129)] {
            let err = constants(bad, module(quote!())).unwrap_err().to_string();
            assert!(err.contains("between 1 and 128"), "{err}");
        }
        assert!(constants(quote!(to(c), fits = 1), module(quote!())).is_ok());
        assert!(constants(quote!(to(c)), module(quote!(#[emit(fits = 0)]))).is_err());
    }
}
