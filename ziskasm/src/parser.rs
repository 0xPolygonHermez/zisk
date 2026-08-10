//! Parser for the ZisK assembly (`.zisk`) source format.
//!
//! See `ziskasm/ziskasm.md` for the full syntax. This is a line-oriented parser:
//! each definition, label and instruction lives on its own line. It produces a
//! flat list of [`Instruction`]s (labels attached to the instruction they
//! precede); [`crate::assembler`] turns that into a `ZiskRom`.

use std::collections::{HashMap, HashSet};

/// A parsed program: the instruction stream plus its data declarations. Both are
/// concatenated across source files; symbols (labels and data names) are resolved
/// globally by the assembler, so declaration order does not matter.
#[derive(Debug, Clone, Default)]
pub struct Program {
    pub instructions: Vec<Instruction>,
    pub data: Vec<DataDecl>,
}

/// A parsed instruction, with the label that precedes it (if any) and the
/// original source text (used as the ZisK instruction `verbose` comment).
#[derive(Debug, Clone)]
pub struct Instruction {
    pub label: Option<String>,
    pub kind: Kind,
    pub verbose: String,
    pub file: String,
    pub line: usize,
}

/// A `[const] TYPE NAME[SIZE] [= values]` data declaration. `const` data lives in
/// ROM (read-only), non-`const` data in RAM. Every element occupies one 8-byte
/// slot regardless of `ty` (the width only range-checks the initial values).
#[derive(Debug, Clone)]
pub struct DataDecl {
    pub name: String,
    pub ty: DataType,
    pub is_const: bool,
    /// Number of 8-byte slots (>= 1). A scalar is a 1-element array.
    pub count: usize,
    /// Initial values, one per slot; `len() <= count`, remaining slots are zero.
    pub values: Vec<u64>,
    pub file: String,
    pub line: usize,
}

/// Declared width of a data element. Every element is stored in one 8-byte slot;
/// the type only bounds the initial values (and documents intent).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    U8,
    U16,
    U32,
    U64,
}

impl DataType {
    /// The inclusive maximum initial value this type accepts.
    pub fn max_value(self) -> u64 {
        match self {
            DataType::U8 => u8::MAX as u64,
            DataType::U16 => u16::MAX as u64,
            DataType::U32 => u32::MAX as u64,
            DataType::U64 => u64::MAX,
        }
    }

    fn from_keyword(s: &str) -> Option<DataType> {
        match s {
            "u8" => Some(DataType::U8),
            "u16" => Some(DataType::U16),
            "u32" => Some(DataType::U32),
            "u64" => Some(DataType::U64),
            _ => None,
        }
    }
}

/// A number operand that is either a literal or a symbol (a label or data name)
/// resolved to its address by the assembler.
#[derive(Debug, Clone)]
pub enum Num {
    Lit(u64),
    Sym(String),
}

#[derive(Debug, Clone)]
pub enum Kind {
    /// A regular `operation(a, b) -> c, ...` instruction.
    Op(Op),
    /// `call LABEL` pseudo-instruction.
    Call(Target),
    /// `ret` pseudo-instruction.
    Ret,
    /// `jump(target)` pseudo-instruction: an unconditional *static* jump to a
    /// label or an absolute address (compiles to `copyb(0, addr), setpc(0)`).
    Jump(JumpTarget),
    /// `ret_to_bios` pseudo-instruction: a static jump to the BIOS
    /// output-finalization address (which the assembler derives), returning
    /// control to the BIOS so it reads the output and ends the program.
    RetToBios,
}

/// Target of a `jump(...)`: a label, or an absolute numeric address. Unlike a
/// `j(...)`/`call` [`Target`], a `jump` number is an absolute address, not a
/// pc-relative offset.
#[derive(Debug, Clone)]
pub enum JumpTarget {
    Addr(u64),
    Label(String),
}

#[derive(Debug, Clone)]
pub struct Op {
    pub op: String,
    pub a: ASource,
    pub b: BSource,
    pub store: Option<Store>,
    pub control: Control,
    pub end: bool,
}

/// `a_source` (no `ind`).
#[derive(Debug, Clone)]
pub enum ASource {
    C,
    Reg(u64),
    Mem(Num),
    Imm(Num),
    Step,
}

/// `b_source` (adds `ind`).
#[derive(Debug, Clone)]
pub enum BSource {
    C,
    Reg(u64),
    Mem(Num),
    Imm(Num),
    Ind { width: u64, offset: i64 },
}

#[derive(Debug, Clone)]
pub enum Store {
    Reg(u64),
    Mem(Num),
    Ind { width: u64, offset: i64 },
}

/// Control-flow field of an instruction.
#[derive(Debug, Clone)]
pub enum Control {
    /// No jump field: next pc = current pc + instruction size.
    Fallthrough,
    /// `j(jump1[, jump2])`. `jump2 = None` means "the next instruction".
    Jump(Target, Option<Target>),
    /// `setpc(offset)`.
    SetPc(i64),
}

/// A jump/call target: a relative offset or a label to be resolved later.
#[derive(Debug, Clone)]
pub enum Target {
    Offset(i64),
    Label(String),
}

/// One level of `ifdef`/`ifndef`/`else`/`endif` conditional compilation.
struct CondFrame {
    /// Whether lines at this level are currently included.
    active: bool,
    /// Whether any branch of this conditional has already been taken (for `else`).
    taken: bool,
    /// Whether the enclosing level was active when this one opened.
    parent_active: bool,
}

/// Handles a conditional-compilation directive line (`ifdef`/`ifndef`/`else`/`endif`).
fn handle_cond(
    code: &str,
    cond: &mut Vec<CondFrame>,
    defs: &HashMap<String, String>,
    predefined: &HashSet<String>,
) -> Result<(), String> {
    let parent_active = cond.last().map_or(true, |f| f.active);
    let is_defined = |name: &str| defs.contains_key(name) || predefined.contains(name);
    if let Some(name) = code.strip_prefix("ifdef ") {
        let taken = parent_active && is_defined(name.trim());
        cond.push(CondFrame { active: taken, taken, parent_active });
    } else if let Some(name) = code.strip_prefix("ifndef ") {
        let taken = parent_active && !is_defined(name.trim());
        cond.push(CondFrame { active: taken, taken, parent_active });
    } else if code == "else" {
        let f = cond.last_mut().ok_or("`else` without matching `ifdef`")?;
        f.active = f.parent_active && !f.taken;
        f.taken = true;
    } else if code == "endif" {
        cond.pop().ok_or("`endif` without matching `ifdef`")?;
    } else {
        return Err(format!("malformed conditional directive `{code}`"));
    }
    Ok(())
}

/// Parses one `.zisk` source file into a [`Program`] (instructions + data).
pub fn parse_program(src: &str, file: &str) -> Result<Program, String> {
    parse_program_with_defines(src, file, &HashSet::new())
}

/// Like [`parse_program`], but with a set of externally predefined symbols that
/// `ifdef`/`ifndef` can test (in addition to source-level `define`s). Used to
/// select a build target: e.g. the `ASM` symbol lets a program exclude ops that
/// the x86 assembly generator cannot emit.
pub fn parse_program_with_defines(
    src: &str,
    file: &str,
    predefined: &HashSet<String>,
) -> Result<Program, String> {
    parse_program_seeded(src, file, predefined, &HashMap::new())
}

/// Scans a source for `pub define NAME VALUE` directives and returns their
/// `(name, value)` pairs. A `pub define` is visible to every file of a multi-file
/// assembly (a library or a `-z` directory), whereas a plain `define` is
/// file-local. Multi-file assemblers collect these across all sources and pass
/// them as the `seed` to [`parse_program_seeded`] for every file, so a constant
/// can be declared once and used everywhere. (Comment-stripped, whole-line scan;
/// `pub define`s are meant to be unconditional top-level declarations.)
pub fn collect_public_defines(src: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for raw in src.lines() {
        let code = raw.split(';').next().unwrap_or("").trim();
        if let Some(rest) = code.strip_prefix("pub define ") {
            let mut it = rest.split_whitespace();
            if let (Some(name), Some(value)) = (it.next(), it.next()) {
                out.push((name.to_string(), value.to_string()));
            }
        }
    }
    out
}

/// Core program parser. `predefined` are names `ifdef`/`ifndef` can test (no
/// value); `seed` are value-carrying defines injected before parsing begins (used
/// to propagate `pub define`s from sibling files — see [`collect_public_defines`]).
pub fn parse_program_seeded(
    src: &str,
    file: &str,
    predefined: &HashSet<String>,
    seed: &HashMap<String, String>,
) -> Result<Program, String> {
    let mut defs: HashMap<String, String> = seed.clone();
    let mut pending_label: Option<String> = None;
    let mut program = Program::default();
    let mut cond: Vec<CondFrame> = Vec::new();

    let lines: Vec<&str> = src.lines().collect();
    let mut idx = 0;
    while idx < lines.len() {
        let line = idx + 1;
        // Strip the `;` comment; anything after it is ignored by the parser.
        let code = lines[idx].split(';').next().unwrap_or("").trim();
        idx += 1;
        if code.is_empty() {
            continue;
        }

        // Conditional-compilation directive. Handled even inside an inactive
        // branch so that nesting (ifdef/endif pairing) is tracked correctly.
        let first = code.split_whitespace().next().unwrap_or("");
        if matches!(first, "ifdef" | "ifndef" | "else" | "endif") {
            handle_cond(code, &mut cond, &defs, predefined).map_err(|e| err(file, line, &e))?;
            continue;
        }

        // Skip everything else while inside an inactive conditional branch.
        if cond.last().is_some_and(|f| !f.active) {
            continue;
        }

        // Definition: `define NAME VALUE`, or `pub define NAME VALUE` (public —
        // visible to sibling files in a multi-file assembly, collected by
        // `collect_public_defines` and seeded here; within this file it behaves
        // like a normal define).
        if let Some(rest) =
            code.strip_prefix("pub define ").or_else(|| code.strip_prefix("define "))
        {
            let mut it = rest.split_whitespace();
            let name = it.next().ok_or_else(|| err(file, line, "define without identifier"))?;
            let value = it.next().ok_or_else(|| err(file, line, "define without value"))?;
            defs.insert(name.to_string(), value.to_string());
            continue;
        }

        // Data declaration: `[const] TYPE NAME[SIZE] [= values]`. Detected by the
        // first token being `const` or a type keyword (no operation is so named).
        // The value list may span several lines: while the accumulated text ends
        // with `=` or `,`, following (comment-stripped) lines are appended.
        if is_data_decl(code) {
            if pending_label.is_some() {
                return Err(err(file, line, "a label cannot precede a data declaration"));
            }
            let mut full = code.to_string();
            while (full.ends_with('=') || full.ends_with(',')) && idx < lines.len() {
                let cont = lines[idx].split(';').next().unwrap_or("").trim();
                idx += 1;
                if cont.is_empty() {
                    continue;
                }
                full.push(' ');
                full.push_str(cont);
            }
            let substituted = substitute(&full, &defs);
            let decl = parse_data_decl(&substituted, file, line)
                .map_err(|e| err(file, line, &format!("{e} (in `{full}`)")))?;
            program.data.push(decl);
            continue;
        }

        // Label: a single identifier ending in `:`.
        if let Some(name) = code.strip_suffix(':') {
            let name = name.trim();
            if name.is_empty() || !is_identifier(name) {
                return Err(err(file, line, &format!("invalid label `{name}`")));
            }
            if pending_label.is_some() {
                return Err(err(file, line, "two labels without an instruction between them"));
            }
            pending_label = Some(name.to_string());
            continue;
        }

        // Instruction. Keep the original (pre-substitution) text as `verbose`.
        let verbose = code.to_string();
        let substituted = substitute(code, &defs);
        let kind = parse_instruction(&substituted)
            .map_err(|e| err(file, line, &format!("{e} (in `{code}`)")))?;
        program.instructions.push(Instruction {
            label: pending_label.take(),
            kind,
            verbose,
            file: file.to_string(),
            line,
        });
    }

    if !cond.is_empty() {
        return Err(format!("{file}: unterminated `ifdef`/`ifndef` (missing `endif`)"));
    }
    if let Some(l) = pending_label {
        return Err(format!("{file}: label `{l}:` at end of file has no instruction"));
    }
    Ok(program)
}

/// Whether a (comment-stripped, trimmed) line is a data declaration: its first
/// whitespace-delimited token is `const` or a type keyword. No operation name
/// collides with these, and instructions have no space before their `(`.
fn is_data_decl(code: &str) -> bool {
    let first = code.split_whitespace().next().unwrap_or("");
    first == "const" || DataType::from_keyword(first).is_some()
}

/// Parses `[const] TYPE NAME[SIZE] [= v0, v1, ...]`.
fn parse_data_decl(code: &str, file: &str, line: usize) -> Result<DataDecl, String> {
    let mut rest = code.trim();
    let is_const = match rest.strip_prefix("const ") {
        Some(r) => {
            rest = r.trim_start();
            true
        }
        None => false,
    };

    // TYPE NAME[SIZE] [= values]
    let (type_kw, after_type) =
        rest.split_once(char::is_whitespace).ok_or("data declaration missing a name")?;
    let ty =
        DataType::from_keyword(type_kw).ok_or_else(|| format!("unknown data type `{type_kw}`"))?;

    let (head, values_str) = match after_type.split_once('=') {
        Some((h, v)) => (h.trim(), Some(v.trim())),
        None => (after_type.trim(), None),
    };

    // head is `NAME` or `NAME[SIZE]`.
    let (name, explicit_size) = match head.find('[') {
        Some(open) => {
            if !head.ends_with(']') {
                return Err(format!("malformed array size in `{head}`"));
            }
            let name = head[..open].trim();
            let size = parse_u64(head[open + 1..head.len() - 1].trim())? as usize;
            (name, Some(size))
        }
        None => (head, None),
    };
    if !is_identifier(name) {
        return Err(format!("invalid data name `{name}`"));
    }

    let mut values = Vec::new();
    if let Some(v) = values_str {
        if !v.is_empty() {
            for part in v.split(',') {
                let val = parse_u64(part.trim())?;
                if val > ty.max_value() {
                    return Err(format!("value {val} does not fit in {type_kw}"));
                }
                values.push(val);
            }
        }
    }

    let count = match explicit_size {
        Some(size) => {
            if values.len() > size {
                return Err(format!("{} initializers for `{name}` of size {size}", values.len()));
            }
            size
        }
        // No `[SIZE]`: a value list defines an array of that length; otherwise a
        // scalar (one slot).
        None => values.len().max(1),
    };
    if count == 0 {
        return Err(format!("`{name}` has size 0; arrays need at least one element"));
    }

    Ok(DataDecl {
        name: name.to_string(),
        ty,
        is_const,
        count,
        values,
        file: file.to_string(),
        line,
    })
}

fn err(file: &str, line: usize, msg: &str) -> String {
    format!("{file}:{line}: {msg}")
}

fn is_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Replaces whole-identifier occurrences of defined names with their values.
fn substitute(code: &str, defs: &HashMap<String, String>) -> String {
    if defs.is_empty() {
        return code.to_string();
    }
    let bytes = code.as_bytes();
    let mut out = String::with_capacity(code.len());
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < bytes.len()
                && ((bytes[i] as char).is_ascii_alphanumeric() || bytes[i] as char == '_')
            {
                i += 1;
            }
            let ident = &code[start..i];
            match defs.get(ident) {
                Some(v) => out.push_str(v),
                None => out.push_str(ident),
            }
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

fn parse_instruction(s: &str) -> Result<Kind, String> {
    let s = s.trim();
    if s == "ret" {
        return Ok(Kind::Ret);
    }
    if s == "ret_to_bios" {
        return Ok(Kind::RetToBios);
    }
    if let Some(rest) = s.strip_prefix("call ") {
        return Ok(Kind::Call(parse_target(rest.trim())?));
    }
    if let Some(inner) = strip_call(s, "jump") {
        return Ok(Kind::Jump(parse_jump_target(inner.trim())?));
    }
    parse_op(s).map(Kind::Op)
}

/// Parses a `jump(...)` target: a label, or an absolute numeric address.
fn parse_jump_target(s: &str) -> Result<JumpTarget, String> {
    if s.is_empty() {
        return Err("empty jump target".into());
    }
    if s.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        Ok(JumpTarget::Addr(parse_u64(s)?))
    } else if is_identifier(s) {
        Ok(JumpTarget::Label(s.to_string()))
    } else {
        Err(format!("invalid jump target `{s}`"))
    }
}

fn parse_op(s: &str) -> Result<Op, String> {
    // Split at top-level commas: parts[0] = `op(a, b) [-> store]`, the rest are
    // the optional `j(...)`, `setpc(...)`, `sp`, `end` modifiers.
    let parts = split_top_level(s, ',');
    let head = parts.first().ok_or("empty instruction")?.trim();

    // Split the head into `op(a, b)` and an optional `-> store`.
    let (call_part, store_part) = match head.split_once("->") {
        Some((c, st)) => (c.trim(), Some(st.trim())),
        None => (head, None),
    };

    // `op(a, b)`
    let open = call_part.find('(').ok_or("missing `(` in operation")?;
    if !call_part.ends_with(')') {
        return Err("missing `)` in operation".into());
    }
    let op = call_part[..open].trim().to_string();
    if op.is_empty() {
        return Err("missing operation name".into());
    }
    let args = &call_part[open + 1..call_part.len() - 1];
    let arg_list = split_top_level(args, ',');
    if arg_list.len() != 2 {
        return Err(format!("operation takes exactly 2 sources, got {}", arg_list.len()));
    }
    let a = parse_a_source(arg_list[0].trim())?;
    let b = parse_b_source(arg_list[1].trim())?;

    let store = match store_part {
        Some(st) => Some(parse_store(st)?),
        None => None,
    };

    // Modifiers.
    let mut control = Control::Fallthrough;
    let mut end = false;
    for m in parts.iter().skip(1) {
        let m = m.trim();
        if m == "end" {
            end = true;
        } else if m == "sp" {
            return Err("the `sp` modifier is not supported yet".into());
        } else if let Some(inner) = strip_call(m, "j") {
            let targets = split_top_level(&inner, ',');
            match targets.len() {
                1 => control = Control::Jump(parse_target(targets[0].trim())?, None),
                2 => {
                    control = Control::Jump(
                        parse_target(targets[0].trim())?,
                        Some(parse_target(targets[1].trim())?),
                    )
                }
                n => return Err(format!("j() takes 1 or 2 targets, got {n}")),
            }
        } else if let Some(inner) = strip_call(m, "setpc") {
            control = Control::SetPc(parse_i64(inner.trim())?);
        } else {
            return Err(format!("unknown instruction modifier `{m}`"));
        }
    }

    Ok(Op { op, a, b, store, control, end })
}

/// If `s` is `name(...)`, returns the inner text.
fn strip_call(s: &str, name: &str) -> Option<String> {
    let s = s.trim();
    let prefix = format!("{name}(");
    if s.starts_with(&prefix) && s.ends_with(')') {
        Some(s[prefix.len()..s.len() - 1].to_string())
    } else {
        None
    }
}

fn parse_a_source(s: &str) -> Result<ASource, String> {
    Ok(match s {
        "c" => ASource::C,
        "step" => ASource::Step,
        _ if is_reg(s) => ASource::Reg(parse_reg(s)?),
        _ if s.starts_with('[') => ASource::Mem(parse_mem(s)?),
        _ if is_ind(s) => {
            return Err("`a` source cannot be an indirect (`W[a + N]`) operand".into())
        }
        _ => ASource::Imm(parse_num(s)?),
    })
}

fn parse_b_source(s: &str) -> Result<BSource, String> {
    Ok(match s {
        "c" => BSource::C,
        "step" => return Err("`step` is only valid as the `a` source".into()),
        _ if is_reg(s) => BSource::Reg(parse_reg(s)?),
        _ if s.starts_with('[') => BSource::Mem(parse_mem(s)?),
        _ if is_ind(s) => {
            let (width, offset) = parse_ind(s)?;
            BSource::Ind { width, offset }
        }
        _ => BSource::Imm(parse_num(s)?),
    })
}

fn parse_store(s: &str) -> Result<Store, String> {
    Ok(match s {
        _ if is_reg(s) => Store::Reg(parse_reg(s)?),
        _ if s.starts_with('[') => Store::Mem(parse_mem(s)?),
        _ if is_ind(s) => {
            let (width, offset) = parse_ind(s)?;
            Store::Ind { width, offset }
        }
        _ => return Err(format!("invalid storage `{s}`")),
    })
}

/// Parses a number operand: a literal (decimal / `0x` hex) or a symbol (a label
/// or data name), resolved to its address by the assembler.
fn parse_num(s: &str) -> Result<Num, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty operand".into());
    }
    if is_identifier(s) {
        Ok(Num::Sym(s.to_string()))
    } else {
        Ok(Num::Lit(parse_u64(s)?))
    }
}

fn parse_target(s: &str) -> Result<Target, String> {
    // A leading digit or sign means a numeric offset; otherwise it is a label.
    let first = s.chars().next().ok_or("empty jump target")?;
    if first.is_ascii_digit() || first == '-' || first == '+' {
        Ok(Target::Offset(parse_i64(s)?))
    } else if is_identifier(s) {
        Ok(Target::Label(s.to_string()))
    } else {
        Err(format!("invalid jump target `{s}`"))
    }
}

fn is_reg(s: &str) -> bool {
    s.len() >= 2 && s.starts_with('r') && s[1..].chars().all(|c| c.is_ascii_digit())
}

fn parse_reg(s: &str) -> Result<u64, String> {
    s[1..].parse::<u64>().map_err(|_| format!("invalid register `{s}`"))
}

/// Parses a `[N]` memory operand (absolute address: a literal or a symbol).
fn parse_mem(s: &str) -> Result<Num, String> {
    let inner = s
        .strip_prefix('[')
        .and_then(|x| x.strip_suffix(']'))
        .ok_or_else(|| format!("malformed memory operand `{s}`"))?;
    parse_num(inner.trim())
}

/// `W[a ± N]` — a width digit immediately followed by `[a ...]`.
fn is_ind(s: &str) -> bool {
    let s = s.trim();
    let bracket = match s.find('[') {
        Some(b) => b,
        None => return false,
    };
    bracket > 0 && s[..bracket].chars().all(|c| c.is_ascii_digit()) && s.ends_with(']')
}

/// Parses `W[a + N]` / `W[a - N]` into (width, signed offset).
fn parse_ind(s: &str) -> Result<(u64, i64), String> {
    let s = s.trim();
    let bracket = s.find('[').ok_or_else(|| format!("malformed indirect operand `{s}`"))?;
    let width = s[..bracket].parse::<u64>().map_err(|_| format!("invalid width in `{s}`"))?;
    if !matches!(width, 1 | 2 | 4 | 8) {
        return Err(format!("indirect width must be 1, 2, 4 or 8, got {width}"));
    }
    let inner = s[bracket + 1..s.len() - 1].trim(); // e.g. "a + 8" or "a - 4" or "a"
    let inner = inner
        .strip_prefix('a')
        .ok_or_else(|| format!("indirect operand must be based on `a`: `{s}`"))?
        .trim();
    let offset = if inner.is_empty() {
        0
    } else if let Some(n) = inner.strip_prefix('+') {
        parse_i64(n.trim())?
    } else if let Some(n) = inner.strip_prefix('-') {
        -parse_i64(n.trim())?
    } else {
        return Err(format!("indirect offset must be `+ N` or `- N`: `{s}`"));
    };
    Ok((width, offset))
}

/// Parses an unsigned integer, decimal or `0x` hexadecimal.
fn parse_u64(s: &str) -> Result<u64, String> {
    let s = s.trim();
    let r = if let Some(h) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u64::from_str_radix(h, 16)
    } else {
        s.parse::<u64>()
    };
    r.map_err(|_| format!("invalid number `{s}`"))
}

/// Parses a signed integer, decimal or `0x` hexadecimal, with optional sign.
fn parse_i64(s: &str) -> Result<i64, String> {
    let s = s.trim();
    let (neg, body) = match s.strip_prefix('-') {
        Some(b) => (true, b.trim()),
        None => (false, s.strip_prefix('+').map(str::trim).unwrap_or(s)),
    };
    let mag = if let Some(h) = body.strip_prefix("0x").or_else(|| body.strip_prefix("0X")) {
        i64::from_str_radix(h, 16)
    } else {
        body.parse::<i64>()
    }
    .map_err(|_| format!("invalid number `{s}`"))?;
    Ok(if neg { -mag } else { mag })
}

/// Splits `s` at occurrences of `delim` that are not nested inside `()` or `[]`.
fn split_top_level(s: &str, delim: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    for c in s.chars() {
        match c {
            '(' | '[' => {
                depth += 1;
                cur.push(c);
            }
            ')' | ']' => {
                depth -= 1;
                cur.push(c);
            }
            _ if c == delim && depth == 0 => {
                parts.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    let last = cur.trim();
    if !last.is_empty() || !parts.is_empty() {
        parts.push(last.to_string());
    }
    parts
}
