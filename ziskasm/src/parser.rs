//! Parser for the ZisK assembly (`.zisk`) source format.
//!
//! See `ziskasm/ziskasm.md` for the full syntax. This is a line-oriented parser:
//! each definition, label and instruction lives on its own line. It produces a
//! flat list of [`Instruction`]s (labels attached to the instruction they
//! precede); [`crate::assembler`] turns that into a `ZiskRom`.

use std::collections::HashMap;

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

#[derive(Debug, Clone)]
pub enum Kind {
    /// A regular `operation(a, b) -> c, ...` instruction.
    Op(Op),
    /// `call LABEL` pseudo-instruction.
    Call(Target),
    /// `ret` pseudo-instruction.
    Ret,
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
    Mem(u64),
    Imm(u64),
    Step,
}

/// `b_source` (adds `ind`).
#[derive(Debug, Clone)]
pub enum BSource {
    C,
    Reg(u64),
    Mem(u64),
    Imm(u64),
    Ind { width: u64, offset: i64 },
}

#[derive(Debug, Clone)]
pub enum Store {
    Reg(u64),
    Mem(u64),
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

/// Parses one `.zisk` source file into a list of instructions.
pub fn parse_program(src: &str, file: &str) -> Result<Vec<Instruction>, String> {
    let mut defs: HashMap<String, String> = HashMap::new();
    let mut pending_label: Option<String> = None;
    let mut out = Vec::new();

    for (idx, raw) in src.lines().enumerate() {
        let line = idx + 1;
        // Strip the `;` comment; anything after it is ignored by the parser.
        let code = raw.split(';').next().unwrap_or("").trim();
        if code.is_empty() {
            continue;
        }

        // Definition: `define NAME VALUE`.
        if let Some(rest) = code.strip_prefix("define ") {
            let mut it = rest.split_whitespace();
            let name = it.next().ok_or_else(|| err(file, line, "define without identifier"))?;
            let value = it.next().ok_or_else(|| err(file, line, "define without value"))?;
            defs.insert(name.to_string(), value.to_string());
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
        out.push(Instruction { label: pending_label.take(), kind, verbose, file: file.to_string(), line });
    }

    if let Some(l) = pending_label {
        return Err(format!("{file}: label `{l}:` at end of file has no instruction"));
    }
    Ok(out)
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
    if let Some(rest) = s.strip_prefix("call ") {
        return Ok(Kind::Call(parse_target(rest.trim())?));
    }
    parse_op(s).map(Kind::Op)
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
        _ if is_ind(s) => return Err("`a` source cannot be an indirect (`W[a + N]`) operand".into()),
        _ => ASource::Imm(parse_u64(s)?),
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
        _ => BSource::Imm(parse_u64(s)?),
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

/// Parses a `[N]` memory operand (absolute address).
fn parse_mem(s: &str) -> Result<u64, String> {
    let inner = s
        .strip_prefix('[')
        .and_then(|x| x.strip_suffix(']'))
        .ok_or_else(|| format!("malformed memory operand `{s}`"))?;
    parse_u64(inner.trim())
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
