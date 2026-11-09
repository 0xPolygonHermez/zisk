//! Translate six scalar-field stages of one hash-pinned verifier, not general Yul.
use crate::{ensure, root, Result};
use num_bigint::BigUint;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const SOURCE_HASH: &str = "a531f5bed7c5e8c85f0ab96e54706d718c42c003cc3c1c900f0eb118d9a70155";
const MODULUS: &str =
    "21888242871839275222246405745257275088548364400416034343698204186575808495617";
const STAGES: [(&str, &str); 6] = [
    ("computeZh", "compute_zh"),
    ("computeZi", "compute_zi"),
    ("computeQ", "compute_q"),
    ("computeZ", "compute_z"),
    ("computeInversions", "compute_inversions"),
    ("computeR", "compute_r"),
];
type Constants = BTreeMap<String, BigUint>;
type Locals = BTreeSet<String>;

#[derive(Debug, PartialEq, Eq)]
enum Expr {
    Atom(String),
    Call(String, Vec<Expr>),
}

struct Parser {
    tokens: Vec<String>,
    at: usize,
}

impl Parser {
    fn new(text: &str) -> Result<Self> {
        let text = text
            .lines()
            .map(|line| line.split("//").next().unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\n");
        let mut input = text.as_str();
        let mut tokens = Vec::new();
        while !input.is_empty() {
            input = input.trim_start();
            if input.is_empty() {
                break;
            }
            let bytes = input.as_bytes();
            let length = if input.starts_with(":=") {
                2
            } else if b"(),{}".contains(&bytes[0]) {
                1
            } else if input.starts_with("0x") {
                let digits = bytes[2..].iter().take_while(|b| b.is_ascii_hexdigit()).count();
                ensure(digits > 0, "invalid hex literal")?;
                digits + 2
            } else if bytes[0].is_ascii_digit() {
                bytes.iter().take_while(|b| b.is_ascii_digit()).count()
            } else if bytes[0].is_ascii_alphabetic() || bytes[0] == b'_' {
                bytes.iter().take_while(|b| b.is_ascii_alphanumeric() || **b == b'_').count()
            } else {
                return Err("unrecognized syntax".into());
            };
            tokens.push(input[..length].to_owned());
            input = &input[length..];
        }
        Ok(Self { tokens, at: 0 })
    }

    fn peek(&self) -> Option<&str> {
        self.tokens.get(self.at).map(String::as_str)
    }

    fn pop(&mut self) -> Result<String> {
        let value = self.tokens.get(self.at).ok_or("unexpected end")?.clone();
        self.at += 1;
        Ok(value)
    }

    fn expect(&mut self, expected: &str) -> Result<()> {
        let actual = self.pop()?;
        ensure(actual == expected, &format!("expected {expected}, got {actual}"))
    }

    fn expr(&mut self) -> Result<Expr> {
        let name = self.pop()?;
        if self.peek() != Some("(") {
            return Ok(Expr::Atom(name));
        }
        self.expect("(")?;
        let mut args = Vec::new();
        while self.peek() != Some(")") {
            if !args.is_empty() {
                self.expect(",")?;
            }
            args.push(self.expr()?);
        }
        self.expect(")")?;
        Ok(Expr::Call(name, args))
    }
}

fn number(text: &str) -> Result<BigUint> {
    let (digits, radix) = text.strip_prefix("0x").map_or((text, 10), |s| (s, 16));
    BigUint::parse_bytes(digits.as_bytes(), radix)
        .ok_or_else(|| format!("invalid integer: {text}").into())
}

fn address(expr: &Expr, constants: &Constants) -> Result<BigUint> {
    match expr {
        Expr::Atom(name) => constants.get(name).cloned().map_or_else(|| number(name), Ok),
        Expr::Call(name, args) if name == "add" && args.len() == 2 => {
            Ok(address(&args[0], constants)? + address(&args[1], constants)?)
        }
        _ => Err(format!("not a constant address: {expr:?}").into()),
    }
}

fn offset(expr: &Expr, constants: &Constants) -> Result<u64> {
    Ok(u64::try_from(address(expr, constants)?)?)
}

fn literal(n: &BigUint) -> Result<String> {
    let modulus = number(MODULUS)?;
    ensure(n < &modulus, "noncanonical scalar constant")?;
    let montgomery = (n << 261usize) % modulus;
    let mask = BigUint::from((1u32 << 29) - 1);
    let limbs =
        (0..9usize).map(|i| ((&montgomery >> (29 * i)) & &mask).to_string()).collect::<Vec<_>>();
    Ok(format!("Fr([{}])", limbs.join(", ")))
}

fn is_modulus(expr: &Expr) -> bool {
    matches!(expr, Expr::Call(name, args) if name == "mload" && args == &[Expr::Atom("0x60".into())])
}

fn negated(expr: &Expr) -> Option<&Expr> {
    match expr {
        Expr::Call(name, args) if name == "sub" && args.len() == 2 && is_modulus(&args[0]) => {
            Some(&args[1])
        }
        _ => None,
    }
}

fn field(expr: &Expr, constants: &Constants, locals: &Locals) -> Result<String> {
    let Expr::Call(name, args) = expr else {
        if let Expr::Atom(name) = expr {
            if locals.contains(name) {
                return Ok(name.clone());
            }
        }
        return literal(&address(expr, constants)?);
    };
    let f = |e| field(e, constants, locals);
    match (name.as_str(), args.as_slice()) {
        ("addmod" | "mulmod", [a, b, modulus]) if is_modulus(modulus) => {
            if name == "mulmod" && a == b {
                return Ok(format!("square({})", f(a)?));
            }
            if name == "addmod" {
                for (left, right) in [(a, b), (b, a)] {
                    if let Some(value) = negated(right) {
                        return Ok(format!("sub({}, {})", f(left)?, f(value)?));
                    }
                }
            }
            Ok(format!("{}({}, {})", if name == "addmod" { "add" } else { "mul" }, f(a)?, f(b)?))
        }
        ("sub", [modulus, a]) if is_modulus(modulus) => Ok(format!("(-{})", f(a)?)),
        ("mload", [a]) => {
            let a = offset(a, constants)?;
            ensure(a % 32 == 0 && (128..3136).contains(&a), "invalid scalar memory address")?;
            Ok(format!("mem[{}]", a / 32))
        }
        ("calldataload", [a]) => {
            let a = offset(a, constants)?;
            if a == 6756 {
                return Ok("public".into());
            }
            ensure(
                (580..=6724).contains(&a) && (a - 4) % 32 == 0,
                "invalid scalar calldata address",
            )?;
            Ok(format!("evals[{}]", (a - 4) / 32 - 18))
        }
        ("powMod", [a, exponent]) => {
            Ok(format!("pow({}, {}u64)", f(a)?, offset(exponent, constants)?))
        }
        _ => Err(format!("unsupported field expression: {expr:?}").into()),
    }
}

fn condition(expr: &Expr, constants: &Constants, locals: &Locals) -> Result<String> {
    if let Expr::Call(name, args) = expr {
        if name == "iszero" && args.len() == 1 {
            if let Expr::Call(eq, values) = &args[0] {
                if eq == "eq" && values.len() == 2 {
                    return Ok(format!(
                        "{} != {}",
                        field(&values[0], constants, locals)?,
                        field(&values[1], constants, locals)?
                    ));
                }
            }
            return Ok(format!("{} == Fr::ZERO", field(&args[0], constants, locals)?));
        }
    }
    Err(format!("unsupported condition {expr:?}").into())
}

fn statements(
    parser: &mut Parser,
    constants: &Constants,
    locals: &mut Locals,
) -> Result<Vec<String>> {
    let mut lines = Vec::new();
    while let Some(name) = parser.peek().map(str::to_owned) {
        match name.as_str() {
            "}" => break,
            "{" => {
                parser.expect("{")?;
                let body = statements(parser, constants, &mut locals.clone())?;
                parser.expect("}")?;
                lines.push(format!("{{\n{}\n}}", body.join("\n")));
            }
            "let" => {
                parser.pop()?;
                let variable = parser.pop()?;
                ensure(!locals.contains(&variable), "shadowed variable")?;
                let value = if parser.peek() == Some(":=") {
                    parser.pop()?;
                    field(&parser.expr()?, constants, locals)?
                } else {
                    "Fr::ZERO".into()
                };
                locals.insert(variable.clone());
                lines.push(format!("let mut {variable} = {value};"));
            }
            "if" => {
                parser.pop()?;
                let test = condition(&parser.expr()?, constants, locals)?;
                parser.expect("{")?;
                let body = statements(parser, constants, &mut locals.clone())?;
                parser.expect("}")?;
                lines.push(format!("if {test} {{\n{}\n}}", body.join("\n")));
            }
            _ if locals.contains(&name) => {
                parser.pop()?;
                parser.expect(":=")?;
                lines.push(format!("{name} = {};", field(&parser.expr()?, constants, locals)?));
            }
            _ => {
                let expr = parser.expr()?;
                let Expr::Call(name, args) = &expr else {
                    return Err(format!("unknown statement {expr:?}").into());
                };
                let f = |e| field(e, constants, locals);
                match (name.as_str(), args.as_slice()) {
                    ("mstore", [a, value]) => {
                        let a = offset(a, constants)?;
                        ensure(a % 32 == 0 && (128..3136).contains(&a), "invalid store")?;
                        lines.push(format!("mem[{}] = {};", a / 32, f(value)?));
                    }
                    ("fail", []) => lines.push("return Err(VerifyError::InvalidProof);".into()),
                    ("checkInv", [a, b]) => lines.push(format!(
                        "if mul({}, {}) != Fr::ONE {{ return Err(VerifyError::InvalidProof); }}",
                        f(a)?,
                        f(b)?
                    )),
                    ("batchInverse", [start, length, value]) => {
                        ensure(
                            offset(start, constants)? == 1600 && offset(length, constants)? == 10,
                            "unexpected inverse batch",
                        )?;
                        lines.push(format!(
                            "let inverse = {};\nbatch_inverse(&mut mem[50..60], inverse)?;",
                            f(value)?
                        ));
                    }
                    _ => return Err(format!("unsupported statement {expr:?}").into()),
                }
            }
        }
    }
    Ok(lines)
}

fn packed(constants: &Constants, names: &[String]) -> Result<String> {
    let mut bytes = Vec::new();
    for name in names {
        let value =
            constants.get(name).ok_or_else(|| format!("missing constant {name}"))?.to_bytes_be();
        ensure(value.len() <= 32, "constant exceeds 256 bits")?;
        bytes.resize(bytes.len() + 32 - value.len(), 0);
        bytes.extend(value);
    }
    Ok(bytes.iter().map(u8::to_string).collect::<Vec<_>>().join(", "))
}

fn generate(source: &str) -> Result<String> {
    ensure(
        hex::encode(Sha256::digest(source.as_bytes())) == SOURCE_HASH,
        "reference source hash mismatch",
    )?;
    let mut constants = Constants::new();
    for line in source.lines() {
        let line = line.trim();
        if !line.starts_with("uint") || !line.contains(" constant ") {
            continue;
        }
        let declaration = line.split(';').next().ok_or("invalid constant declaration")?;
        let (left, right) = declaration.split_once('=').ok_or("invalid constant declaration")?;
        let name = left.split_whitespace().last().ok_or("missing constant name")?;
        ensure(
            constants.insert(name.into(), number(right.trim())?).is_none(),
            "duplicate constant",
        )?;
    }
    constants.insert("pPublic".into(), BigUint::from(6756u32));
    let mut out = vec![
        format!("// Generated by tools/src/generate.rs from SHA-256 {SOURCE_HASH}."),
        "// Do not edit; differential tests compare these stages with the release JS oracle.".into(),
        "#![allow(unused_assignments, unused_mut, unused_variables, non_snake_case, unused_parens)]".into(),
        "use crate::scalar::{Fr, pow};\nuse crate::{add, batch_inverse, mul, scalar::{sub, square}, VerifyError};".into(),
    ];
    for (original, name) in STAGES {
        let at = source
            .find(&format!("function {original}("))
            .ok_or_else(|| format!("missing stage {original}"))?;
        let at = at + source[at..].find('{').ok_or("missing stage body")? + 1;
        let mut depth = 1;
        let mut end = None;
        for (i, b) in source.as_bytes()[at..].iter().enumerate() {
            if *b == b'{' {
                depth += 1;
            }
            if *b == b'}' {
                depth -= 1;
            }
            if depth == 0 {
                end = Some(at + i);
                break;
            }
        }
        let mut parser = Parser::new(&source[at..end.ok_or("unterminated stage")?])?;
        let mut body = statements(&mut parser, &constants, &mut Locals::new())?;
        ensure(parser.peek().is_none(), "unconsumed stage")?;
        if name == "compute_z" {
            for (old, new) in [
                (
                    "mem[16] = pow(y, 2u64);",
                    "mem[16] = square(y);\nlet y4 = square(mem[16]);\nlet y8 = square(y4);",
                ),
                ("mem[17] = pow(y, 16u64);", "mem[17] = square(y8);"),
                ("mem[18] = pow(y, 18u64);", "mem[18] = mul(mem[17], mem[16]);"),
                ("mem[19] = pow(y, 24u64);", "mem[19] = mul(mem[17], y8);"),
                ("mem[20] = pow(y, 26u64);", "mem[20] = mul(mem[19], mem[16]);"),
                ("mem[21] = pow(y, 29u64);", "mem[21] = mul(mul(mem[19], y4), y);"),
            ] {
                ensure(
                    body.iter().filter(|line| *line == old).count() == 1,
                    "unexpected y-power schedule",
                )?;
                for line in &mut body {
                    if line == old {
                        *line = new.into();
                    }
                }
            }
        }
        if name == "compute_inversions" {
            let mut optimized = Vec::new();
            for line in body {
                if line.contains("pow(mem[10], 216u64)") {
                    optimized.push("let xi215 = pow(mem[10], 215u64);".into());
                }
                optimized.push(
                    line.replace("pow(mem[10], 216u64)", "mul(xi215, mem[10])")
                        .replace("pow(mem[10], 215u64)", "xi215"),
                );
            }
            body = optimized;
        }
        if name == "compute_q" {
            // Keep each generated block below SBF's 4 KiB stack-frame limit.
            ensure(
                body.iter().all(|line| line.starts_with("mem[") && !line.contains('\n')),
                "Q blocks must communicate only through scalar memory",
            )?;
            let mut calls = Vec::new();
            for (index, chunk) in body.chunks(8).enumerate() {
                let part = format!("{name}_{index}");
                out.extend([
                    "#[inline(never)]".into(),
                    format!("fn {part}(mem: &mut [Fr], evals: &[Fr], public: Fr) {{"),
                ]);
                out.extend(chunk.iter().cloned());
                out.push("}".into());
                calls.push(format!("{part}(mem, evals, public);"));
            }
            body = calls;
        }
        out.extend(["#[inline(never)]".into(), format!("pub(crate) fn {name}(mem: &mut [Fr], evals: &[Fr], public: Fr) -> Result<(), VerifyError> {{")]);
        out.extend(body);
        out.push("Ok(())\n}".into());
    }
    out.push("pub(crate) const FIXED: [[u8; 64]; 4] = [".into());
    for i in 0..4 {
        out.push(format!("[{}],", packed(&constants, &[format!("f{i}x"), format!("f{i}y")])?));
    }
    out.push("];".into());
    for (label, prefix) in [("G2", "G2"), ("TAU", "X2")] {
        let names = ["x2", "x1", "y2", "y1"].map(|suffix| format!("{prefix}{suffix}"));
        out.push(format!(
            "pub(crate) const {label}: [u8; 128] = [{}];",
            packed(&constants, &names)?
        ));
    }
    out.push(format!(
        "pub(crate) const DIGEST: Fr = {};",
        literal(constants.get("DIGEST").ok_or("missing digest")?)?
    ));
    Ok(out.join("\n") + "\n")
}

pub fn run(check: bool) -> Result<()> {
    let output = generate(&std::fs::read_to_string(
        root().join("../zisk-contracts/blake3/PilfflonkVerifier.sol"),
    )?)?;
    let target = root().join("verifier/src/profile.rs");
    if check {
        ensure(std::fs::read_to_string(target)? == output, "generated profile differs; regenerate")
    } else {
        Ok(std::fs::write(target, output)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expression(text: &str) -> Result<String> {
        let mut p = Parser::new(text)?;
        let output = field(&p.expr()?, &Constants::new(), &Locals::new())?;
        ensure(p.peek().is_none(), "trailing expression")?;
        Ok(output)
    }

    #[test]
    fn rejects_changed_source_and_unsupported_syntax() {
        assert!(generate("contract Changed {}").unwrap_err().to_string().contains("source hash"));
        for text in [
            "div(2, 3)",
            "mulmod(2, 3, 7)",
            "sub(4, 5)",
            "mload(123)",
            "calldataload(4)",
            "powMod(2, 18446744073709551616)",
            "addmod(1, 2)",
            "7 8",
            "-1",
            "addmod(1, 2,",
        ] {
            assert!(expression(text).is_err(), "{text}");
        }
    }

    #[test]
    fn scalar_bounds_and_lowering() {
        let r = number(MODULUS).unwrap();
        assert!(literal(&r).is_err());
        assert!(literal(&(&r - BigUint::from(1u32))).is_ok());
        assert_eq!(
            expression("sub(mload(0x60), 2)").unwrap(),
            format!("(-{})", literal(&2u32.into()).unwrap())
        );
        assert_eq!(
            expression("addmod(7, sub(mload(0x60), 4), mload(0x60))").unwrap(),
            format!("sub({}, {})", literal(&7u32.into()).unwrap(), literal(&4u32.into()).unwrap())
        );
        assert_eq!(
            expression("mulmod(7, 7, mload(0x60))").unwrap(),
            format!("square({})", literal(&7u32.into()).unwrap())
        );
    }

    #[test]
    fn rejects_unsupported_statements_and_conditions() {
        for text in [
            "mstore(129, 1)",
            "sstore(128, 1)",
            "batchInverse(1601, 10, 1)",
            "let a := 1 let a := 2",
            "if eq(1, 2) { fail() }",
            "unknown := 1",
            "{ fail()",
        ] {
            let mut parser = Parser::new(text).unwrap();
            assert!(
                statements(&mut parser, &Constants::new(), &mut Locals::new()).is_err(),
                "{text}"
            );
        }
    }

    #[test]
    fn reproduces_every_byte_of_the_pinned_arithmetic() {
        let source =
            std::fs::read_to_string(root().join("../zisk-contracts/blake3/PilfflonkVerifier.sol"))
                .unwrap();
        let output = generate(&source).unwrap();
        let checked_in = std::fs::read_to_string(root().join("verifier/src/profile.rs")).unwrap();
        // The generator's filename in the first comment changes during the port.
        assert_eq!(output.split_once('\n').unwrap().1, checked_in.split_once('\n').unwrap().1);
        for (_, name) in STAGES {
            assert!(output.contains(&format!("fn {name}(")));
        }
    }
}
