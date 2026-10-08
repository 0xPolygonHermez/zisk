//! Reusable constants codegen engine.
//!
//! Renders `#[constants]`-style tables ([`meta::Export`] grouped by
//! [`meta::GroupMeta`]) to Rust, C-header, and PIL text. It is deterministic and
//! knows nothing about any particular project: a caller supplies the groups
//! (typically a crate's `ZISK_CONSTANTS`) and a `regen_cmd` string for the
//! "do not edit" banner.
//!
//! [`render`] validates (each value fits its width; no duplicate names within a
//! file) and returns the files in memory, each tagged with its [`Target`];
//! [`write`] / [`check`] reconcile them against one output directory per target.
//! Output is stable so a build step can gate drift.

use std::collections::HashSet;
use std::fmt::Write as _;
use std::fs;
use std::path::{Component, Path, PathBuf};

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
        pub const ASM: Targets = Targets(8);

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
        pub asm_prefix: &'static str,
        /// Base name of the output C header in the C dir (no subdirectories);
        /// `None` = `<group>.gen.h`.
        pub c_file: Option<&'static str>,
        /// Base name of the output PIL file in the PIL dir (no subdirectories);
        /// `None` = `<group>.gen.pil`.
        pub pil_file: Option<&'static str>,
        /// Base name of the output asm include in the asm dir; `None` = `<group>.gen.inc`.
        pub asm_file: Option<&'static str>,
    }

    /// One constant to emit, plus everything needed to render and validate it.
    pub struct Export {
        pub name: &'static str,
        pub value: Value,
        /// Storage width in bits (8/16/32/64/128); 0 for `&str`. Signedness is
        /// read from the `Value` variant (`I` vs `U`), so it isn't stored here.
        pub ty_bits: u8,
        /// Declared as `usize`/`isize`: 64 bits everywhere (ZisK is a fixed 64-bit
        /// target), but the generated Rust keeps the pointer-sized type so consumers
        /// can still use it as a length or an index.
        pub pointer_sized: bool,
        pub targets: Targets,
        pub radix: Radix,
        /// Explicit domain bound in bits; `None` means "use `ty_bits`".
        pub fits: Option<u8>,
        /// Disable the fit check entirely (e.g. a full-width mask).
        pub no_fit: bool,
        /// Per-target name used in place of the ident for C/PIL/asm; the group prefix
        /// is still prepended. `None` = the ident. (The Rust form always uses the ident.)
        pub c_name: Option<&'static str>,
        pub pil_name: Option<&'static str>,
        pub asm_name: Option<&'static str>,
        /// Source expression, carried as a provenance comment when derived; empty
        /// for bare literals.
        pub expr: &'static str,
        pub doc: &'static str,
    }
}

use meta::{Export, GroupMeta, Radix, Targets, Value};

/// Which language a generated file targets — used to route it to an output dir.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Target {
    Rust,
    C,
    Pil,
    Asm,
}

/// A rendered output file: its target, base name, and full contents.
pub struct GenFile {
    pub target: Target,
    pub name: String,
    pub contents: String,
}

/// How an output dir may be reconciled — i.e. whether it holds only generated files.
#[derive(Clone, Copy)]
pub enum DirMode {
    /// Generated-only dir: files of this target's extension that the groups no longer
    /// produce are deleted (orphan cleanup). The default for a dedicated `generated/`.
    Exclusive,
    /// Dir shared with hand-written files of the same extension (e.g. emitting an asm
    /// include straight into a folder the emulator maintains by hand). Only files we
    /// produce are written, and a stale file is removed only when it carries the
    /// `@generated` banner — hand-written siblings are never touched.
    Shared,
}

/// One target's output: the dir its files go to, and how that dir is reconciled.
#[derive(Clone, Copy)]
pub struct Out<'a> {
    pub path: &'a Path,
    pub mode: DirMode,
}

impl Out<'_> {
    /// Whether a stale file at `path` (of this target's extension, no longer produced)
    /// is ours to reconcile — removed by `write`, flagged by `check`. An `Exclusive`
    /// dir reclaims every such file; a `Shared` dir only its own `@generated` files, so
    /// hand-written siblings are left untouched. `write` and `check` both route through
    /// this, so the removal and drift decisions can never drift apart.
    fn reconciles(&self, path: &Path) -> bool {
        match self.mode {
            DirMode::Exclusive => true,
            DirMode::Shared => is_generated(path),
        }
    }
}

/// One output per target. Each [`Out`] names a dir and how it may be reconciled; a
/// dir may be shared with hand-written files by using [`DirMode::Shared`].
pub struct Dirs<'a> {
    pub rust: Out<'a>,
    pub c: Out<'a>,
    pub pil: Out<'a>,
    pub asm: Out<'a>,
}

impl<'a> Dirs<'a> {
    /// Each target with its output and file extension — the single source of the
    /// target↔dir↔ext mapping that `write` and `check` iterate.
    fn each(&self) -> [(Target, Out<'a>, &'static str); 4] {
        [
            (Target::Rust, self.rust, "rs"),
            (Target::C, self.c, "h"),
            (Target::Pil, self.pil, "pil"),
            (Target::Asm, self.asm, "inc"),
        ]
    }
}

/// Marker that opens every generated file's banner. Single source of truth: the banner
/// is built from it (`generated_note`) and generated files are recognized by it
/// (`is_generated`), so the producer and the detector can never disagree.
const GENERATED_MARKER: &str = "@generated by";

/// The "@generated … do not edit." notice text (no comment markers — each caller
/// wraps it in the target's own comment syntax).
fn generated_note(regen_cmd: &str) -> String {
    format!("{GENERATED_MARKER} `{regen_cmd}` \u{2014} do not edit.")
}

/// Render every group to its Rust/C/PIL files in memory (no I/O). Fails on a
/// fit-check violation, a duplicate name within a C/PIL file, an invalid group or
/// output file name, or a value not renderable for a target. `regen_cmd` is stamped
/// into each file's banner.
pub fn render(groups: &[(&GroupMeta, &[Export])], regen_cmd: &str) -> Result<Vec<GenFile>, String> {
    for &(meta, exports) in groups {
        check_group_name(meta.name)?;
        for e in exports {
            fit_check(e)?;
        }
    }
    let mut files = render_flat(groups, regen_cmd)?;
    files.extend(render_rust(groups, regen_cmd)?);
    Ok(files)
}

/// A group name becomes the default output file names and, for Rust, a module name,
/// so it must be an identifier: anything else could leave the output dir or fail to
/// compile in the generated `mod.rs`.
fn check_group_name(name: &str) -> Result<(), String> {
    let mut chars = name.chars();
    let ident = chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
    if ident {
        Ok(())
    } else {
        Err(format!("group name `{name}` must be an identifier ([A-Za-z_][A-Za-z0-9_]*)"))
    }
}

/// Strict and reserved Rust keywords (edition 2021): not usable as a `pub mod` name.
const RUST_KEYWORDS: &[&str] = &[
    "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "crate",
    "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "if", "impl", "in",
    "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref",
    "return", "self", "Self", "static", "struct", "super", "trait", "true", "try", "type",
    "typeof", "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
];

/// Comment text on one line: a doc may hold newlines (`#[doc = "a\nb"]`), and a line
/// comment covers only its first line.
fn one_line(text: &str) -> String {
    text.replace(['\r', '\n'], " ")
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
    for (target, out, ext) in dirs.each() {
        write_dir(&files, target, out, ext)?;
    }
    Ok(())
}

/// Errors if two of `all` write the same target into the same dir. Each [`write`]
/// reconciles a dir against only its own groups' files, so a second writer would delete
/// the first one's outputs (and, for Rust, replace its `mod.rs`). Call it once over
/// every job's [`Dirs`] before writing any of them. Paths are compared lexically, with
/// `.` and `..` resolved.
pub fn ensure_disjoint(all: &[Dirs]) -> Result<(), String> {
    let mut seen: HashSet<(Target, PathBuf)> = HashSet::new();
    for dirs in all {
        for (target, out, ext) in dirs.each() {
            if !seen.insert((target, normalize(out.path))) {
                return Err(format!(
                    "two jobs write `.{ext}` files into {}; give each job its own output dir",
                    out.path.display()
                ));
            }
        }
    }
    Ok(())
}

/// `path` with `.` dropped and `..` applied, without touching the filesystem (the dirs
/// may not exist yet).
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push(part);
                }
            }
            other => out.push(other),
        }
    }
    out
}

/// Render and compare against the files on disk; error lists any drift.
pub fn check(
    groups: &[(&GroupMeta, &[Export])],
    dirs: &Dirs,
    regen_cmd: &str,
) -> Result<(), String> {
    let files = render(groups, regen_cmd)?;
    let mut problems: Vec<String> = Vec::new();
    for (target, out, ext) in dirs.each() {
        check_dir(&files, target, out, ext, &mut problems);
    }

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

fn write_dir(files: &[GenFile], target: Target, out: Out, ext: &str) -> Result<(), String> {
    let dir = out.path;
    let mine: Vec<&GenFile> = files.iter().filter(|f| f.target == target).collect();
    // Nothing to write and no existing dir to clean → leave the filesystem untouched
    // (don't create empty target dirs for targets no group uses).
    if mine.is_empty() && !dir.exists() {
        return Ok(());
    }
    fs::create_dir_all(dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;
    let expected: HashSet<&str> = mine.iter().map(|f| f.name.as_str()).collect();

    // Remove files we no longer produce (e.g. after a group rename/drop). An Exclusive
    // dir is generated-only, so every stale file of this extension goes; a Shared dir
    // may hold hand-written files, so only stale *generated* ones (banner-marked) go.
    for name in files_on_disk(dir, ext) {
        if expected.contains(name.as_str()) {
            continue;
        }
        let path = dir.join(&name);
        if out.reconciles(&path) {
            fs::remove_file(&path).map_err(|e| format!("removing {}: {e}", path.display()))?;
        }
    }
    for f in mine {
        let path = dir.join(&f.name);
        if fs::read_to_string(&path).is_ok_and(|on_disk| on_disk == f.contents) {
            continue;
        }
        // Write beside the target and rename over it, so a concurrent reader (rustc
        // compiling the generated Rust in the same workspace build) sees the old file or
        // the new one, never a partly written one. The `.tmp` extension keeps the
        // scratch file out of the per-extension reconcile above.
        let tmp = dir.join(format!(".{}.tmp", f.name));
        fs::write(&tmp, &f.contents).map_err(|e| format!("writing {}: {e}", tmp.display()))?;
        fs::rename(&tmp, &path).map_err(|e| format!("replacing {}: {e}", path.display()))?;
    }
    Ok(())
}

fn check_dir(files: &[GenFile], target: Target, out: Out, ext: &str, problems: &mut Vec<String>) {
    let dir = out.path;
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
        if expected.contains(name.as_str()) {
            continue;
        }
        let path = dir.join(&name);
        if out.reconciles(&path) {
            problems.push(format!("orphaned:    {}", path.display()));
        }
    }
}

/// True if `path` is a file this engine generated, detected by the [`GENERATED_MARKER`]
/// banner that every rendered file carries on its first line. Lets a `Shared` dir tell
/// its generated files from hand-written ones. A missing/unreadable file is not ours.
///
/// The marker must open the line, right after the comment opener (`//`, `/*` or `#`):
/// a hand-written file whose first comment merely mentions it stays hand-written.
fn is_generated(path: &Path) -> bool {
    match fs::read_to_string(path) {
        Ok(s) => s.lines().next().is_some_and(|first| {
            first.trim_start_matches(['/', '*', '#', ' ']).starts_with(GENERATED_MARKER)
        }),
        Err(_) => false,
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
///
/// Unlike C/PIL/asm, groups can't share a Rust file: each is its own module, so two
/// Rust-emitting groups with one name (or a keyword name) are rejected.
fn render_rust(
    groups: &[(&GroupMeta, &[Export])],
    regen_cmd: &str,
) -> Result<Vec<GenFile>, String> {
    let mut files = Vec::new();
    let mut modules: Vec<&str> = Vec::new();

    for &(meta, exports) in groups {
        let rust: Vec<&Export> =
            exports.iter().filter(|e| e.targets.contains(Targets::RUST)).collect();
        if rust.is_empty() {
            continue;
        }
        if RUST_KEYWORDS.contains(&meta.name) {
            return Err(format!(
                "group `{}` emits Rust, but a keyword can't name a module",
                meta.name
            ));
        }
        if modules.contains(&meta.name) {
            return Err(format!(
                "two groups named `{}` emit Rust; each needs its own module name",
                meta.name
            ));
        }
        let mut body = format!("// {}\n\n", generated_note(regen_cmd));
        for e in rust {
            if !e.doc.is_empty() {
                let _ = writeln!(body, "/// {}", one_line(e.doc));
            }
            let _ = write!(body, "pub const {}: {} = {};", e.name, rust_type(e), fmt_value_rust(e));
            if !e.expr.is_empty() {
                let _ = write!(body, " // {}", one_line(e.expr));
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

    // Always emit the aggregator — empty when no group targets Rust — so `mod
    // generated;` in the consuming crate is valid after every regen. rustfmt
    // (reorder_modules) sorts `mod` declarations, so emit them sorted.
    modules.sort_unstable();
    let mut mod_rs = format!("// {}\n", generated_note(regen_cmd));
    if !modules.is_empty() {
        mod_rs.push('\n');
        for m in &modules {
            let _ = writeln!(mod_rs, "pub mod {m};");
        }
    }
    files.push(GenFile { target: Target::Rust, name: "mod.rs".to_string(), contents: mod_rs });
    Ok(files)
}

/// The Rust type of a value: signedness from the `Value` variant, width from `ty_bits`
/// (or `usize`/`isize` when the source declared a pointer-sized type).
fn rust_type(e: &Export) -> &'static str {
    match e.value {
        Value::Str(_) => "&str",
        Value::I(_) if e.pointer_sized => "isize",
        Value::U(_) if e.pointer_sized => "usize",
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    C,
    Pil,
    Asm,
}

impl Kind {
    fn target(self) -> Target {
        match self {
            Kind::C => Target::C,
            Kind::Pil => Target::Pil,
            Kind::Asm => Target::Asm,
        }
    }

    /// A comment in this target's syntax: `/* body */` for C, `// body` for PIL,
    /// `# body` for asm (GAS). `body` (a doc, often) is kept on one line, and a `*/`
    /// in it is split so it can't close the C comment early.
    fn comment(self, body: &str) -> String {
        let body = one_line(body);
        match self {
            Kind::C => format!("/* {} */", body.replace("*/", "* /")),
            Kind::Pil => format!("// {body}"),
            Kind::Asm => format!("# {body}"),
        }
    }

    /// A constant-definition line (no trailing newline). C/PIL left-align the name to
    /// `width` so values line up in a column; asm uses GAS `.equ name, value`.
    fn define(self, name: &str, value: &str, width: usize) -> String {
        match self {
            Kind::C => format!("#define {name:<width$} {value}"),
            Kind::Pil => format!("const int {name:<width$} = {value};"),
            Kind::Asm => format!(".equ {name}, {value}"),
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

fn render_flat(
    groups: &[(&GroupMeta, &[Export])],
    regen_cmd: &str,
) -> Result<Vec<GenFile>, String> {
    let mut files: Vec<FileBuf> = Vec::new();

    for &(meta, exports) in groups {
        let c_file = flat_file_name(meta, meta.c_file, "c_file", "h")?;
        let pil_file = flat_file_name(meta, meta.pil_file, "pil_file", "pil")?;
        let asm_file = flat_file_name(meta, meta.asm_file, "asm_file", "inc")?;
        for e in exports {
            if e.targets.contains(Targets::C) {
                push_entry(&mut files, &c_file, Kind::C, meta, e, fmt_value_c(e)?)?;
            }
            if e.targets.contains(Targets::PIL) {
                push_entry(
                    &mut files,
                    &pil_file,
                    Kind::Pil,
                    meta,
                    e,
                    fmt_numeric_upper(e, "PIL")?,
                )?;
            }
            if e.targets.contains(Targets::ASM) {
                check_64bit_literal(e, "asm")?;
                push_entry(
                    &mut files,
                    &asm_file,
                    Kind::Asm,
                    meta,
                    e,
                    fmt_numeric_upper(e, "asm")?,
                )?;
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

/// A group's output file name for one flat target: the `field` override, else
/// `<group>.gen.<ext>`. An override must be a plain file name ending in `.<ext>`:
/// reconciliation only scans the target dir for `*.<ext>`, so anything else (a missing
/// suffix, a subdir) would be written once and never cleaned up or drift-checked.
fn flat_file_name(
    meta: &GroupMeta,
    over: Option<&'static str>,
    field: &str,
    ext: &str,
) -> Result<String, String> {
    let Some(name) = over else {
        return Ok(format!("{}.gen.{ext}", meta.name));
    };
    let plain = !name.contains(['/', '\\']);
    let has_stem = name.strip_suffix(&format!(".{ext}")).is_some_and(|stem| !stem.is_empty());
    if plain && has_stem {
        Ok(name.to_string())
    } else {
        Err(format!(
            "group `{}`: {field} = \"{name}\" must be a plain file name ending in `.{ext}`",
            meta.name
        ))
    }
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
        Kind::Asm => format!("{}{}", meta.asm_prefix, e.asm_name.unwrap_or(e.name)),
    };

    // Keyed by target too: each target writes to its own dir, so C and PIL overrides
    // may share a name without their entries landing in one file.
    let idx = match files.iter().position(|f| f.file_name == fname && f.kind == kind) {
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
    let banner = kind.comment(&generated_note(regen_cmd));
    let guard = format!(
        "ZISK_GENERATED_{}",
        fb.file_name.to_uppercase().replace(|c: char| !c.is_ascii_alphanumeric(), "_")
    );

    let mut out = String::new();
    // C wraps the body in an include guard; PIL and asm just carry the banner.
    match kind {
        Kind::C => {
            let _ = write!(
                out,
                "{banner}\n#ifndef {guard}\n#define {guard}\n\n#include <stdint.h>\n\n"
            );
        }
        Kind::Pil | Kind::Asm => {
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

fn fmt_value_c(e: &Export) -> Result<String, String> {
    if let Value::Str(s) = e.value {
        return Ok(c_string_literal(s));
    }
    check_64bit_literal(e, "C")?;
    Ok(format!("(({}){})", c_type(e), fmt_number(&e.value, e.radix, false)))
}

/// A C string literal for `s`. Not Rust's `{:?}`: C has no `\u{..}` escapes. Every byte
/// outside printable ASCII is written as a 3-digit octal escape, which (unlike `\x`)
/// can't swallow a following hex-digit character.
fn c_string_literal(s: &str) -> String {
    let mut out = String::from("\"");
    for b in s.bytes() {
        match b {
            b'"' => out.push_str("\\\""),
            b'\\' => out.push_str("\\\\"),
            b'\n' => out.push_str("\\n"),
            b'\t' => out.push_str("\\t"),
            b'\r' => out.push_str("\\r"),
            0x20..=0x7e => out.push(b as char),
            _ => {
                let _ = write!(out, "\\{b:03o}");
            }
        }
    }
    out.push('"');
    out
}

/// C and GAS integer literals stop at 64 bits: a wider value can't be written as one
/// (a cast after the literal comes too late), so reject it instead of emitting a header
/// or include that fails to assemble or silently truncates.
fn check_64bit_literal(e: &Export, target: &str) -> Result<(), String> {
    let fits = match e.value {
        Value::U(v) => u64::try_from(v).is_ok(),
        Value::I(v) => i64::try_from(v).is_ok(),
        Value::Str(_) => true,
    };
    if fits {
        Ok(())
    } else {
        Err(format!("`{}`: value does not fit a 64-bit {target} literal", e.name))
    }
}

/// Uppercase bare numeric literal for the PIL and asm targets (both use `0x…`
/// uppercase and neither can hold a string). `target` names the caller for errors.
fn fmt_numeric_upper(e: &Export, target: &str) -> Result<String, String> {
    if let Value::Str(_) = e.value {
        return Err(format!("`{}`: string constants cannot be emitted to {target}", e.name));
    }
    Ok(fmt_number(&e.value, e.radix, true))
}

/// Assert the evaluated value fits the effective bit bound (explicit `fits`, else
/// the storage width). Skipped for strings, when `no_fit` is set, or at 128 bits.
fn fit_check(e: &Export) -> Result<(), String> {
    if e.no_fit || matches!(e.value, Value::Str(_)) {
        return Ok(());
    }
    let bound = e.fits.unwrap_or(e.ty_bits);
    if bound == 0 {
        return Err(format!("`{}`: a numeric fit bound must be at least 1 bit", e.name));
    }
    if bound >= 128 {
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
    use std::path::Path;

    use super::meta::{Export, GroupMeta, Radix, Targets, Value};
    use super::{check, render, write, DirMode, Dirs, Out};

    /// A dedicated, generated-only output dir (the common test case).
    fn exclusive(path: &Path) -> Out<'_> {
        Out { path, mode: DirMode::Exclusive }
    }

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
            pointer_sized: false,
            targets,
            radix: Radix::Hex,
            fits,
            no_fit: false,
            c_name: None,
            pil_name: None,
            asm_name: None,
            expr: "",
            doc: "",
        }
    }

    const fn group(name: &'static str, c_file: &'static str) -> GroupMeta {
        GroupMeta {
            name,
            c_prefix: "",
            pil_prefix: "",
            asm_prefix: "",
            c_file: Some(c_file),
            pil_file: None,
            asm_file: None,
        }
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
        let c: Vec<_> = files.iter().filter(|f| matches!(f.target, super::Target::C)).collect();
        assert_eq!(c.len(), 1, "both C groups merge into shared.h");
        let h = &c[0].contents;
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
    fn asm_target_emits_equ_include() {
        static G: GroupMeta = group("mem", "mem.h");
        static E: &[Export] =
            &[export("RAM", Value::U(0xa000_0000), Targets(Targets::ASM.0), None)];
        let files = render(&[(&G, E)], "test").expect("render");
        let inc = files.iter().find(|f| f.name == "mem.gen.inc").expect("mem.gen.inc missing");
        assert!(matches!(inc.target, super::Target::Asm));
        assert!(inc.contents.contains(".equ RAM, 0xA0000000"), "{}", inc.contents);
    }

    #[test]
    fn check_flags_and_write_removes_orphans() {
        static G: GroupMeta = group("mem", "mem.h");
        static E: &[Export] = &[export("ONE", Value::U(1), Targets::C, None)];
        let groups: &[(&GroupMeta, &[Export])] = &[(&G, E)];

        let base = std::env::temp_dir().join(format!("zisk-gen-orphan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let (r, c, p, a) = (base.join("rs"), base.join("c"), base.join("pil"), base.join("asm"));
        let dirs =
            Dirs { rust: exclusive(&r), c: exclusive(&c), pil: exclusive(&p), asm: exclusive(&a) };

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

    #[test]
    fn shared_dir_preserves_handwritten_and_cleans_generated_orphans() {
        static G: GroupMeta = group("mem", "mem.h");
        static E: &[Export] = &[export("ONE", Value::U(1), Targets::C, None)];
        let groups: &[(&GroupMeta, &[Export])] = &[(&G, E)];

        let base = std::env::temp_dir().join(format!("zisk-gen-shared-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();

        // A hand-written header of the SAME extension living alongside generated output.
        let hand = base.join("handwritten.h");
        std::fs::write(&hand, "/* hand-written, keep me */\n#define KEEP 1\n").unwrap();

        // Route C into the shared dir; unused targets get throwaway exclusive dirs.
        let (r, p, a) = (base.join("rs"), base.join("pil"), base.join("asm"));
        let dirs = Dirs {
            rust: exclusive(&r),
            c: Out { path: &base, mode: DirMode::Shared },
            pil: exclusive(&p),
            asm: exclusive(&a),
        };

        write(groups, &dirs, "test").expect("write");
        assert!(base.join("mem.h").exists(), "generated header written into shared dir");
        assert!(hand.exists(), "hand-written sibling preserved");
        assert!(check(groups, &dirs, "test").is_ok(), "hand-written file is not drift");

        // Drop the group: mem.h is now a stale *generated* file (has the banner) → gone;
        // the hand-written file (no banner) stays.
        let empty: &[(&GroupMeta, &[Export])] = &[];
        write(empty, &dirs, "test").expect("rewrite");
        assert!(!base.join("mem.h").exists(), "stale generated file removed in shared dir");
        assert!(hand.exists(), "hand-written file survives cleanup");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn override_names_must_be_plain_with_the_target_extension() {
        static E: &[Export] =
            &[export("X", Value::U(1), Targets(Targets::C.0 | Targets::PIL.0), None)];
        static NO_EXT: GroupMeta = group("g", "shared");
        static SUBDIR: GroupMeta = group("g", "sub/x.h");
        static BARE: GroupMeta = group("g", ".h");
        static WRONG_EXT: GroupMeta = GroupMeta { pil_file: Some("x.h"), ..group("g", "x.h") };
        for (bad, why) in [
            (&NO_EXT, "missing suffix"),
            (&SUBDIR, "subdir"),
            (&BARE, "no stem"),
            (&WRONG_EXT, "PIL override with a C suffix"),
        ] {
            assert!(render(&[(bad, E)], "test").is_err(), "{why} must be rejected");
        }
        static OK: GroupMeta = GroupMeta { pil_file: Some("x.pil"), ..group("g", "x.h") };
        assert!(render(&[(&OK, E)], "test").is_ok());
    }

    #[test]
    fn group_names_must_be_valid_and_unique_rust_modules() {
        static RUST: &[Export] = &[export("X", Value::U(1), Targets::RUST, None)];
        static C_ONLY: &[Export] = &[export("Y", Value::U(2), Targets::C, None)];
        static DASHED: GroupMeta = group("my-group", "a.h");
        static KEYWORD: GroupMeta = group("type", "type.h");
        static A: GroupMeta = group("same", "a.h");
        static B: GroupMeta = group("same", "b.h");

        assert!(render(&[(&DASHED, C_ONLY)], "test").is_err(), "not an identifier");
        assert!(render(&[(&KEYWORD, RUST)], "test").is_err(), "keyword module");
        assert!(render(&[(&KEYWORD, C_ONLY)], "test").is_ok(), "keywords are fine off Rust");
        assert!(render(&[(&A, RUST), (&B, RUST)], "test").is_err(), "two `same.rs` modules");
        assert!(render(&[(&A, C_ONLY), (&B, C_ONLY)], "test").is_ok(), "C groups may share a name");
    }

    #[test]
    fn comments_stay_on_one_line_and_c_comments_stay_closed() {
        static G: GroupMeta = group("d", "d.h");
        static E: &[Export] = &[Export {
            doc: "ends */ here\nsecond line",
            ..export(
                "X",
                Value::U(1),
                Targets(Targets::RUST.0 | Targets::C.0 | Targets::PIL.0),
                None,
            )
        }];
        let files = render(&[(&G, E)], "test").expect("render");
        let get = |n: &str| &files.iter().find(|f| f.name == n).expect(n).contents;
        let (h, pil, rs) = (get("d.h"), get("d.gen.pil"), get("d.rs"));
        assert!(h.contains("/* ends * / here second line */"), "{h}");
        assert!(pil.contains("// ends */ here second line"), "{pil}");
        assert!(rs.contains("/// ends */ here second line"), "{rs}");
        for out in [h, pil, rs] {
            assert!(!out.lines().any(|l| l.starts_with("second")), "uncommented doc line:\n{out}");
        }
    }

    #[test]
    fn banner_mention_is_not_a_banner() {
        static G: GroupMeta = group("mem", "mem.h");
        static E: &[Export] = &[export("ONE", Value::U(1), Targets::C, None)];
        let base = std::env::temp_dir().join(format!("zisk-gen-mention-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        // Hand-written, but its first comment mentions the marker.
        let hand = base.join("notes.h");
        std::fs::write(&hand, "/* not @generated by any tool, keep me */\n").unwrap();

        let (r, p, a) = (base.join("rs"), base.join("pil"), base.join("asm"));
        let dirs = Dirs {
            rust: exclusive(&r),
            c: Out { path: &base, mode: DirMode::Shared },
            pil: exclusive(&p),
            asm: exclusive(&a),
        };
        write(&[(&G, E)], &dirs, "test").expect("write");
        let empty: &[(&GroupMeta, &[Export])] = &[];
        write(empty, &dirs, "test").expect("rewrite");
        assert!(hand.exists(), "a mention of the marker is not a generated banner");
        assert!(!base.join("mem.h").exists(), "the real generated file is still cleaned up");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn write_leaves_no_scratch_files() {
        static G: GroupMeta = group("mem", "mem.h");
        static E: &[Export] =
            &[export("ONE", Value::U(1), Targets(Targets::RUST.0 | Targets::C.0), None)];
        let base = std::env::temp_dir().join(format!("zisk-gen-atomic-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let (r, c, p, a) = (base.join("rs"), base.join("c"), base.join("pil"), base.join("asm"));
        let dirs =
            Dirs { rust: exclusive(&r), c: exclusive(&c), pil: exclusive(&p), asm: exclusive(&a) };

        write(&[(&G, E)], &dirs, "test").expect("write");
        for dir in [&r, &c] {
            let names: Vec<String> = std::fs::read_dir(dir)
                .unwrap()
                .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect();
            assert!(names.iter().all(|n| !n.ends_with(".tmp")), "scratch file left: {names:?}");
        }
        assert!(check(&[(&G, E)], &dirs, "test").is_ok(), "renamed files hold the content");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn jobs_must_not_share_a_target_dir() {
        let (r1, c1, p1, a1) =
            (Path::new("g/rs"), Path::new("g/c"), Path::new("g/pil"), Path::new("g/asm"));
        let job1 =
            Dirs { rust: exclusive(r1), c: exclusive(c1), pil: exclusive(p1), asm: exclusive(a1) };

        // Same C dir spelled differently: rejected.
        let c2 = Path::new("g/x/../c");
        let (r2, p2, a2) = (Path::new("h/rs"), Path::new("h/pil"), Path::new("h/asm"));
        let clash =
            Dirs { rust: exclusive(r2), c: exclusive(c2), pil: exclusive(p2), asm: exclusive(a2) };
        let err = super::ensure_disjoint(&[job1, clash]).unwrap_err();
        assert!(err.contains(".h"), "{err}");

        // Different targets may share a dir (each reconciles only its own extension).
        let job1 =
            Dirs { rust: exclusive(r1), c: exclusive(c1), pil: exclusive(p1), asm: exclusive(a1) };
        let h_c = Path::new("h/c");
        let ok =
            Dirs { rust: exclusive(r2), c: exclusive(h_c), pil: exclusive(p2), asm: exclusive(c1) };
        assert!(super::ensure_disjoint(&[job1, ok]).is_ok());
    }

    #[test]
    fn same_override_name_stays_per_target() {
        static G: GroupMeta = GroupMeta {
            c_file: Some("shared.h"),
            pil_file: Some("shared.pil"),
            ..group("g", "shared.h")
        };
        static E: &[Export] =
            &[export("X", Value::U(1), Targets(Targets::C.0 | Targets::PIL.0), None)];
        let files = render(&[(&G, E)], "test").expect("render");
        let c = files.iter().find(|f| f.target == super::Target::C).expect("C file");
        let pil = files.iter().find(|f| f.target == super::Target::Pil).expect("PIL file");
        assert!(c.contents.contains("#define X"), "{}", c.contents);
        assert!(pil.contents.contains("const int X"), "{}", pil.contents);
        assert!(!c.contents.contains("const int"), "PIL entry leaked into the C file");
    }

    #[test]
    fn c_strings_use_c_escapes() {
        static G: GroupMeta = group("s", "s.h");
        static E: &[Export] = &[Export {
            ty_bits: 0,
            ..export("S", Value::Str("a\"b\\c\n\u{7f}\u{e9}1"), Targets::C, None)
        }];
        let files = render(&[(&G, E)], "test").expect("render");
        let h = &files.iter().find(|f| f.name == "s.h").expect("s.h").contents;
        // DEL and the UTF-8 bytes of 'é' as octal; never Rust's `\u{..}`.
        assert!(h.contains(r#""a\"b\\c\n\177\303\2511""#), "{h}");
        assert!(!h.contains("\\u{"), "{h}");
    }

    #[test]
    fn values_wider_than_64_bits_are_rejected_for_c_and_asm() {
        static G: GroupMeta = group("w", "w.h");
        static WIDE: Value = Value::U(1 << 64);
        static C: &[Export] = &[Export { ty_bits: 128, ..export("W", WIDE, Targets::C, None) }];
        static ASM: &[Export] = &[Export { ty_bits: 128, ..export("W", WIDE, Targets::ASM, None) }];
        static PIL: &[Export] = &[Export { ty_bits: 128, ..export("W", WIDE, Targets::PIL, None) }];
        static NEG: &[Export] = &[Export {
            ty_bits: 128,
            ..export("N", Value::I(i64::MIN as i128 - 1), Targets::C, None)
        }];
        assert!(render(&[(&G, C)], "test").is_err(), "C cannot hold a >64-bit literal");
        assert!(render(&[(&G, ASM)], "test").is_err(), "GAS cannot hold a >64-bit literal");
        assert!(render(&[(&G, NEG)], "test").is_err(), "below i64::MIN");
        assert!(render(&[(&G, PIL)], "test").is_ok(), "PIL has big integers");

        // A 128-bit type holding a 64-bit value is still a valid (cast) C literal.
        static SMALL: &[Export] =
            &[Export { ty_bits: 128, ..export("S", Value::U(5), Targets::C, None) }];
        let files = render(&[(&G, SMALL)], "test").expect("render");
        assert!(files[0].contents.contains("((unsigned __int128)0x5)"), "{}", files[0].contents);
    }

    #[test]
    fn pointer_sized_keeps_its_rust_type() {
        static G: GroupMeta = group("p", "p.h");
        static E: &[Export] = &[Export {
            pointer_sized: true,
            ..export("LEN", Value::U(4), Targets(Targets::RUST.0 | Targets::C.0), None)
        }];
        let files = render(&[(&G, E)], "test").expect("render");
        let rs = files.iter().find(|f| f.name == "p.rs").expect("p.rs");
        assert!(rs.contents.contains("pub const LEN: usize = 0x4;"), "{}", rs.contents);
        let h = files.iter().find(|f| f.name == "p.h").expect("p.h");
        assert!(h.contents.contains("((uint64_t)0x4)"), "{}", h.contents);
    }

    #[test]
    fn zero_fit_bound_rejects_numbers_not_strings() {
        static G: GroupMeta = group("z", "z.h");
        static NUM: &[Export] = &[export("N", Value::U(1), Targets::C, Some(0))];
        assert!(render(&[(&G, NUM)], "test").is_err(), "fits = 0 must not accept a number");

        static STR: &[Export] =
            &[Export { ty_bits: 0, ..export("S", Value::Str("x"), Targets::C, None) }];
        assert!(render(&[(&G, STR)], "test").is_ok(), "strings carry no width");
    }
}
