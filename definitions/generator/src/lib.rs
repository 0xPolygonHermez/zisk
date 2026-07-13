//! Reusable constants codegen engine.
//!
//! Renders `#[constants]`-style tables ([`meta::Export`] grouped by
//! [`meta::GroupMeta`]) to Rust, C-header, and PIL text. It is deterministic and
//! knows nothing about any particular project: a caller supplies the groups
//! (typically a crate's `CONSTANT_GROUPS`) and a `regen_cmd` string for the
//! "do not edit" banner.
//!
//! [`render`] validates (each value fits its width; no duplicate names within a
//! file) and returns the files in memory, each tagged with its [`Target`];
//! [`write`] / [`check`] reconcile them against one output directory per target.
//! Output is stable so a build step can gate drift.

use std::collections::HashSet;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

pub mod meta {
    //! The schema this engine renders: the types a `#[constants]`/`#[emit]` macro
    //! fills in and the renderer consumes.

    /// Targets a constant is emitted to, as bit flags.
    #[derive(Clone, Copy)]
    pub struct Targets(pub u8);

    impl Targets {
        pub const RUST: Targets = Targets(1);
        pub const C: Targets = Targets(2);
        pub const PIL: Targets = Targets(4);

        /// True if `self` includes target `t`.
        #[inline]
        pub const fn contains(self, t: Targets) -> bool {
            self.0 & t.0 != 0
        }
    }

    /// Number base used when rendering a value to text.
    #[derive(Clone, Copy)]
    pub enum Radix {
        Hex,
        Dec,
    }

    /// The evaluated value of a constant, widened into one carrier so a single
    /// `Export` can hold `u8`..=`u128`, `i8`..=`i128`, and `&str` constants.
    #[derive(Clone, Copy)]
    pub enum Value {
        U(u128),
        I(i128),
        Str(&'static str),
    }

    /// Module-level emission policy shared by every export in a group.
    pub struct GroupMeta {
        pub name: &'static str,
        pub c_prefix: &'static str,
        pub pil_prefix: &'static str,
        /// Base name of the output C header in the C dir (no subdirectories);
        /// `None` = `<group>.h`.
        pub c_file: Option<&'static str>,
        /// Base name of the output PIL file in the PIL dir (no subdirectories);
        /// `None` = `<group>.pil`.
        pub pil_file: Option<&'static str>,
    }

    /// One constant to emit, plus everything needed to render and validate it.
    pub struct Export {
        pub name: &'static str,
        pub value: Value,
        /// Storage width in bits (8/16/32/64/128); 0 for `&str`. Signedness is
        /// read from the `Value` variant (`I` vs `U`), so it isn't stored here.
        pub ty_bits: u8,
        pub targets: Targets,
        pub radix: Radix,
        /// Explicit domain bound in bits; `None` means "use `ty_bits`".
        pub fits: Option<u8>,
        /// Disable the fit check entirely (e.g. a full-width mask).
        pub no_fit: bool,
        /// Per-target name used in place of the ident for C/PIL; the group prefix is
        /// still prepended. `None` = the ident. (The Rust form always uses the ident.)
        pub c_name: Option<&'static str>,
        pub pil_name: Option<&'static str>,
        /// Source expression, carried as a provenance comment when derived; empty
        /// for bare literals.
        pub expr: &'static str,
        pub doc: &'static str,
    }
}

use meta::{Export, GroupMeta, Radix, Targets, Value};

/// Which language a generated file targets — used to route it to an output dir.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Rust,
    C,
    Pil,
}

/// A rendered output file: its target, base name, and full contents.
pub struct GenFile {
    pub target: Target,
    pub name: String,
    pub contents: String,
}

/// One output directory per target. Each dir must hold ONLY that target's
/// generated files — `write`/`check` delete stale files there by extension.
pub struct Dirs<'a> {
    pub rust: &'a Path,
    pub c: &'a Path,
    pub pil: &'a Path,
}

/// Render every group to its Rust/C/PIL files in memory (no I/O). Fails on a
/// fit-check violation, a duplicate name within a C/PIL file, or a value not
/// renderable for a target. `regen_cmd` is stamped into each file's banner.
pub fn render(groups: &[(&GroupMeta, &[Export])], regen_cmd: &str) -> Result<Vec<GenFile>, String> {
    for &(_, exports) in groups {
        for e in exports {
            fit_check(e)?;
        }
    }
    let mut files = render_c_pil(groups, regen_cmd)?;
    files.extend(render_rust(groups, regen_cmd));
    Ok(files)
}

/// Render and write each target's files to its dir, skipping files whose content
/// is unchanged (avoids mtime churn) and removing generated files the groups no
/// longer produce.
pub fn write(
    groups: &[(&GroupMeta, &[Export])],
    dirs: &Dirs,
    regen_cmd: &str,
) -> Result<(), String> {
    let files = render(groups, regen_cmd)?;
    write_dir(&files, Target::Rust, dirs.rust, "rs")?;
    write_dir(&files, Target::C, dirs.c, "h")?;
    write_dir(&files, Target::Pil, dirs.pil, "pil")?;
    Ok(())
}

/// Render and compare against the files on disk; error lists any drift.
pub fn check(
    groups: &[(&GroupMeta, &[Export])],
    dirs: &Dirs,
    regen_cmd: &str,
) -> Result<(), String> {
    let files = render(groups, regen_cmd)?;
    let mut problems: Vec<String> = Vec::new();
    check_dir(&files, Target::Rust, dirs.rust, "rs", &mut problems);
    check_dir(&files, Target::C, dirs.c, "h", &mut problems);
    check_dir(&files, Target::Pil, dirs.pil, "pil", &mut problems);

    if problems.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{} generated file(s) drifted; re-run `{regen_cmd}`:\n  {}",
            problems.len(),
            problems.join("\n  ")
        ))
    }
}

// ---- writing / checking one target's dir ----------------------------------

fn write_dir(files: &[GenFile], target: Target, dir: &Path, ext: &str) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;
    let mine: Vec<&GenFile> = files.iter().filter(|f| f.target == target).collect();
    let expected: HashSet<&str> = mine.iter().map(|f| f.name.as_str()).collect();

    // Remove generated files no longer produced (e.g. after a group rename/drop).
    for name in files_on_disk(dir, ext) {
        if !expected.contains(name.as_str()) {
            let path = dir.join(&name);
            fs::remove_file(&path).map_err(|e| format!("removing {}: {e}", path.display()))?;
        }
    }
    for f in mine {
        let path = dir.join(&f.name);
        if fs::read_to_string(&path).is_ok_and(|on_disk| on_disk == f.contents) {
            continue;
        }
        fs::write(&path, &f.contents).map_err(|e| format!("writing {}: {e}", path.display()))?;
    }
    Ok(())
}

fn check_dir(files: &[GenFile], target: Target, dir: &Path, ext: &str, problems: &mut Vec<String>) {
    let mine: Vec<&GenFile> = files.iter().filter(|f| f.target == target).collect();
    let expected: HashSet<&str> = mine.iter().map(|f| f.name.as_str()).collect();

    for f in &mine {
        let path = dir.join(&f.name);
        match fs::read_to_string(&path) {
            Ok(on_disk) if on_disk == f.contents => {}
            Ok(_) => problems.push(format!("out of date: {}", path.display())),
            Err(_) => problems.push(format!("missing:     {}", path.display())),
        }
    }
    for name in files_on_disk(dir, ext) {
        if !expected.contains(name.as_str()) {
            problems.push(format!("orphaned:    {}", dir.join(&name).display()));
        }
    }
}

/// Base names of `*.{ext}` files currently in `dir`.
fn files_on_disk(dir: &Path, ext: &str) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            if path.extension().is_some_and(|x| x == ext) {
                path.file_name().and_then(|n| n.to_str()).map(String::from)
            } else {
                None
            }
        })
        .collect()
}

// ---- Rust rendering -------------------------------------------------------

/// One `<group>.rs` per group that opts into Rust, plus a `mod.rs` declaring them.
/// The Rust form always uses the source ident and no prefix, so consumers see the
/// same names the author wrote. Skipped entirely if no group targets Rust.
fn render_rust(groups: &[(&GroupMeta, &[Export])], regen_cmd: &str) -> Vec<GenFile> {
    let mut files = Vec::new();
    let mut modules: Vec<&str> = Vec::new();

    for &(meta, exports) in groups {
        let rust: Vec<&Export> =
            exports.iter().filter(|e| e.targets.contains(Targets::RUST)).collect();
        if rust.is_empty() {
            continue;
        }
        let mut body = format!("// @generated by `{regen_cmd}` \u{2014} do not edit.\n\n");
        for e in rust {
            if !e.doc.is_empty() {
                let _ = writeln!(body, "/// {}", e.doc);
            }
            let _ = write!(body, "pub const {}: {} = {};", e.name, rust_type(e), fmt_value_rust(e));
            if !e.expr.is_empty() {
                let _ = write!(body, " // {}", e.expr);
            }
            body.push('\n');
        }
        files.push(GenFile {
            target: Target::Rust,
            name: format!("{}.rs", meta.name),
            contents: body,
        });
        modules.push(meta.name);
    }

    if !modules.is_empty() {
        // `rustfmt` (reorder_modules) sorts `mod` declarations, so emit them sorted
        // to keep the committed file fmt-clean.
        modules.sort_unstable();
        let mut mod_rs = format!("// @generated by `{regen_cmd}` \u{2014} do not edit.\n\n");
        for m in &modules {
            let _ = writeln!(mod_rs, "pub mod {m};");
        }
        files.push(GenFile { target: Target::Rust, name: "mod.rs".to_string(), contents: mod_rs });
    }
    files
}

/// The Rust type of a value: signedness from the `Value` variant, width from `ty_bits`.
fn rust_type(e: &Export) -> &'static str {
    match e.value {
        Value::Str(_) => "&str",
        Value::I(_) => match e.ty_bits {
            8 => "i8",
            16 => "i16",
            32 => "i32",
            128 => "i128",
            _ => "i64",
        },
        Value::U(_) => match e.ty_bits {
            8 => "u8",
            16 => "u16",
            32 => "u32",
            128 => "u128",
            _ => "u64",
        },
    }
}

fn fmt_value_rust(e: &Export) -> String {
    match e.value {
        Value::Str(s) => format!("{s:?}"),
        _ => fmt_number(&e.value, e.radix, false),
    }
}

// ---- C / PIL rendering ----------------------------------------------------

#[derive(Clone, Copy)]
enum Kind {
    C,
    Pil,
}

impl Kind {
    fn target(self) -> Target {
        match self {
            Kind::C => Target::C,
            Kind::Pil => Target::Pil,
        }
    }

    /// A comment in this target's syntax: `/* body */` for C, `// body` for PIL.
    fn comment(self, body: &str) -> String {
        match self {
            Kind::C => format!("/* {body} */"),
            Kind::Pil => format!("// {body}"),
        }
    }

    /// A constant-definition line (no trailing newline); the name is left-aligned to
    /// `width` so the values line up in a column.
    fn define(self, name: &str, value: &str, width: usize) -> String {
        match self {
            Kind::C => format!("#define {name:<width$} {value}"),
            Kind::Pil => format!("const int {name:<width$} = {value};"),
        }
    }
}

struct Entry {
    group: &'static str,
    doc: &'static str,
    name: String,
    value: String,
    provenance: &'static str,
}

struct FileBuf {
    file_name: String,
    kind: Kind,
    entries: Vec<Entry>,
}

fn render_c_pil(
    groups: &[(&GroupMeta, &[Export])],
    regen_cmd: &str,
) -> Result<Vec<GenFile>, String> {
    let mut files: Vec<FileBuf> = Vec::new();

    for &(meta, exports) in groups {
        let c_file = meta.c_file.map(String::from).unwrap_or_else(|| format!("{}.h", meta.name));
        let pil_file =
            meta.pil_file.map(String::from).unwrap_or_else(|| format!("{}.pil", meta.name));
        for e in exports {
            if e.targets.contains(Targets::C) {
                push_entry(&mut files, &c_file, Kind::C, meta, e, fmt_value_c(e))?;
            }
            if e.targets.contains(Targets::PIL) {
                push_entry(&mut files, &pil_file, Kind::Pil, meta, e, fmt_value_pil(e)?)?;
            }
        }
    }

    Ok(files
        .into_iter()
        .map(|fb| GenFile {
            target: fb.kind.target(),
            contents: render_file(&fb, regen_cmd),
            name: fb.file_name,
        })
        .collect())
}

fn push_entry(
    files: &mut Vec<FileBuf>,
    fname: &str,
    kind: Kind,
    meta: &GroupMeta,
    e: &Export,
    value: String,
) -> Result<(), String> {
    let name = match kind {
        Kind::C => format!("{}{}", meta.c_prefix, e.c_name.unwrap_or(e.name)),
        Kind::Pil => format!("{}{}", meta.pil_prefix, e.pil_name.unwrap_or(e.name)),
    };

    let idx = match files.iter().position(|f| f.file_name == fname) {
        Some(i) => i,
        None => {
            files.push(FileBuf { file_name: fname.to_string(), kind, entries: Vec::new() });
            files.len() - 1
        }
    };
    let fb = &mut files[idx];

    if fb.entries.iter().any(|en| en.name == name) {
        return Err(format!("duplicate name `{}` in generated file `{}`", name, fb.file_name));
    }

    fb.entries.push(Entry { group: meta.name, doc: e.doc, name, value, provenance: e.expr });
    Ok(())
}

fn render_file(fb: &FileBuf, regen_cmd: &str) -> String {
    let width = fb.entries.iter().map(|e| e.name.len()).max().unwrap_or(0);
    // Per-group separator lines only help when several groups share one file.
    let multi = fb.entries.first().is_some_and(|f0| fb.entries.iter().any(|e| e.group != f0.group));

    let kind = fb.kind;
    let banner = kind.comment(&format!("@generated by `{regen_cmd}` \u{2014} do not edit."));
    let guard = format!(
        "ZISK_GENERATED_{}",
        fb.file_name.to_uppercase().replace(|c: char| !c.is_ascii_alphanumeric(), "_")
    );

    let mut out = String::new();
    // C wraps the body in an include guard; PIL just carries the banner.
    match kind {
        Kind::C => {
            let _ = write!(
                out,
                "{banner}\n#ifndef {guard}\n#define {guard}\n\n#include <stdint.h>\n\n"
            );
        }
        Kind::Pil => {
            let _ = write!(out, "{banner}\n\n");
        }
    }

    let mut cur = "";
    for e in &fb.entries {
        if multi && e.group != cur {
            let _ = writeln!(out, "{}", kind.comment(&format!("--- {} ---", e.group)));
            cur = e.group;
        }
        if !e.doc.is_empty() {
            let _ = writeln!(out, "{}", kind.comment(e.doc));
        }
        out.push_str(&kind.define(&e.name, &e.value, width));
        if !e.provenance.is_empty() {
            let _ = write!(out, "  {}", kind.comment(e.provenance));
        }
        out.push('\n');
    }

    if let Kind::C = kind {
        let _ = write!(out, "\n#endif /* {guard} */\n");
    }
    out
}

/// C type name for a value: signedness comes from the `Value` variant, width from
/// `ty_bits`.
fn c_type(e: &Export) -> String {
    let signed = matches!(e.value, Value::I(_));
    if e.ty_bits == 128 {
        return format!("{}__int128", if signed { "" } else { "unsigned " });
    }
    format!("{}int{}_t", if signed { "" } else { "u" }, e.ty_bits)
}

/// Render a numeric value as a bare literal, honoring radix and hex case. Only
/// called for `U`/`I`; `Str` is handled by the callers.
fn fmt_number(value: &Value, radix: Radix, upper: bool) -> String {
    match *value {
        Value::U(v) => match radix {
            Radix::Hex if upper => format!("{v:#X}"),
            Radix::Hex => format!("{v:#x}"),
            Radix::Dec => format!("{v}"),
        },
        Value::I(v) if matches!(radix, Radix::Hex) && v >= 0 => {
            if upper {
                format!("{v:#X}")
            } else {
                format!("{v:#x}")
            }
        }
        Value::I(v) => format!("{v}"),
        Value::Str(_) => unreachable!("fmt_number is numeric-only; callers handle Str"),
    }
}

fn fmt_value_c(e: &Export) -> String {
    if let Value::Str(s) = e.value {
        return format!("{s:?}");
    }
    format!("(({}){})", c_type(e), fmt_number(&e.value, e.radix, false))
}

fn fmt_value_pil(e: &Export) -> Result<String, String> {
    if let Value::Str(_) = e.value {
        return Err(format!("`{}`: string constants cannot be emitted to PIL", e.name));
    }
    Ok(fmt_number(&e.value, e.radix, true))
}

/// Assert the evaluated value fits the effective bit bound (explicit `fits`, else
/// the storage width). Skipped when `no_fit` is set or the width is 128/str.
fn fit_check(e: &Export) -> Result<(), String> {
    if e.no_fit {
        return Ok(());
    }
    let bound = e.fits.unwrap_or(e.ty_bits);
    if bound == 0 || bound >= 128 {
        return Ok(());
    }
    match e.value {
        Value::U(v) => {
            if v >= (1u128 << bound) {
                return Err(format!("`{}` = {v} does not fit in {bound} bits", e.name));
            }
        }
        Value::I(v) => {
            let lo = -(1i128 << (bound - 1));
            let hi = (1i128 << (bound - 1)) - 1;
            if v < lo || v > hi {
                return Err(format!("`{}` = {v} does not fit in signed {bound} bits", e.name));
            }
        }
        Value::Str(_) => {}
    }
    Ok(())
}

// ---- tests ----------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::meta::{Export, GroupMeta, Radix, Targets, Value};
    use super::{check, render, write, Dirs};

    // Minimal hand-built fixtures for the engine paths (no external definitions).
    const fn export(
        name: &'static str,
        value: Value,
        targets: Targets,
        fits: Option<u8>,
    ) -> Export {
        Export {
            name,
            value,
            ty_bits: 64,
            targets,
            radix: Radix::Hex,
            fits,
            no_fit: false,
            c_name: None,
            pil_name: None,
            expr: "",
            doc: "",
        }
    }

    const fn group(name: &'static str, c_file: &'static str) -> GroupMeta {
        GroupMeta { name, c_prefix: "", pil_prefix: "", c_file: Some(c_file), pil_file: None }
    }

    #[test]
    fn fit_check_rejects_overflow() {
        static G: GroupMeta = group("t", "t.h");
        static E: &[Export] = &[export("X", Value::U(0x1_0000_0000), Targets::C, Some(32))]; // 33 bits
        assert!(render(&[(&G, E)], "test").is_err(), "fits=32 must reject a 33-bit value");
    }

    #[test]
    fn merges_two_groups_into_one_file() {
        static A: GroupMeta = group("a", "shared.h");
        static B: GroupMeta = group("b", "shared.h");
        static EA: &[Export] = &[export("A_ONE", Value::U(1), Targets::C, None)];
        static EB: &[Export] = &[export("B_ONE", Value::U(2), Targets::C, None)];
        let files = render(&[(&A, EA), (&B, EB)], "test").expect("render");
        assert_eq!(files.len(), 1, "both C groups merge into shared.h; no Rust/PIL");
        let h = &files[0].contents;
        assert!(h.contains("/* --- a --- */") && h.contains("/* --- b --- */"));
        assert!(h.contains("A_ONE") && h.contains("B_ONE"));
    }

    #[test]
    fn rejects_duplicate_name_in_file() {
        static A: GroupMeta = group("a", "dup.h");
        static B: GroupMeta = group("b", "dup.h");
        static E: &[Export] = &[export("DUP", Value::U(1), Targets::C, None)];
        assert!(render(&[(&A, E), (&B, E)], "test").is_err(), "same name in one file must error");
    }

    #[test]
    fn rust_target_emits_typed_consts_and_mod() {
        static G: GroupMeta = group("mem", "mem.h");
        static E: &[Export] =
            &[export("RAM", Value::U(0xa000_0000), Targets(Targets::RUST.0 | Targets::C.0), None)];
        let files = render(&[(&G, E)], "test").expect("render");

        let rs = files.iter().find(|f| f.name == "mem.rs").expect("mem.rs missing");
        assert!(rs.contents.contains("pub const RAM: u64 = 0xa0000000;"));

        let mod_rs = files.iter().find(|f| f.name == "mod.rs").expect("mod.rs missing");
        assert!(mod_rs.contents.contains("pub mod mem;"));

        // The same constant also reaches the C header (different target/name).
        assert!(files.iter().any(|f| f.name == "mem.h"));
    }

    #[test]
    fn check_flags_and_write_removes_orphans() {
        static G: GroupMeta = group("mem", "mem.h");
        static E: &[Export] = &[export("ONE", Value::U(1), Targets::C, None)];
        let groups: &[(&GroupMeta, &[Export])] = &[(&G, E)];

        let base = std::env::temp_dir().join(format!("zisk-gen-orphan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let (r, c, p) = (base.join("rs"), base.join("c"), base.join("pil"));
        let dirs = Dirs { rust: &r, c: &c, pil: &p };

        write(groups, &dirs, "test").expect("write");
        assert!(check(groups, &dirs, "test").is_ok(), "fresh write is up to date");

        // A stale C file the definitions no longer produce.
        std::fs::write(c.join("stale.h"), "orphan").unwrap();
        assert!(check(groups, &dirs, "test").is_err(), "check must flag an orphaned file");

        write(groups, &dirs, "test").expect("rewrite");
        assert!(!c.join("stale.h").exists(), "write must remove the orphan");
        assert!(check(groups, &dirs, "test").is_ok(), "clean again after rewrite");

        let _ = std::fs::remove_dir_all(&base);
    }
}
