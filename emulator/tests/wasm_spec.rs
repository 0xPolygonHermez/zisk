//! The official WebAssembly test suite (<https://github.com/WebAssembly/testsuite>) run through
//! `wasm2rom` and the emulator.
//!
//! The suite is not vendored. Fetch it at the pinned commit and point the test at it:
//!
//! ```bash
//! git clone https://github.com/WebAssembly/testsuite.git /path/to/testsuite
//! git -C /path/to/testsuite checkout 63908677d00585fdab4c4eccbf01f781d3a454d5
//! WASM_TESTSUITE=/path/to/testsuite cargo test -p ziskemu --release --features float \
//!     --test wasm_spec -- --ignored --nocapture
//! ```
//!
//! CI (`.github/workflows/pr.yml`) fetches the same commit; change both together.
//!
//! `WASM_SPEC_FILTER=i32,f64` restricts the run to the files whose name contains one of the
//! substrings. Without the `float` feature, every module that uses floats counts as unsupported.
//!
//! A ROM starts only at `_start`, so the harness adds one to each module: it calls the exports
//! the script invokes, in order, with the script's constant arguments, and stores each result
//! (as its bits, widened to `i64`) into a new global, plus a progress counter after each call.
//! After the run the harness reads those globals from the emulator's memory. A run that halts
//! with the error flag is a trap, and the progress counter says which call trapped. The calls
//! that should return run in one batch, rerun without a call that trapped unexpectedly; each
//! `assert_trap` runs alone, after the calls that precede it. A trap's partial effects (stores
//! before the trap) are therefore not seen by the calls after it.
//!
//! Each assertion ends up as one of:
//! - pass;
//! - fail: a wrong result, a missing or unexpected trap, an invalid module accepted, a panic;
//! - unsupported: `wasm2rom` rejects the module (a feature outside the supported subset), or the
//!   call uses values the harness cannot pass (`v128`, references);
//! - skipped: what this harness does not model: text-format errors (`module quote`), linking
//!   (`register`, imports other than WASI, `assert_unlinkable`), exceptions and threads.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;

use wasmparser::{CompositeInnerType, ExternalKind, Payload, TypeRef, ValType};
use wast::core::{NanPattern, WastArgCore, WastRetCore};
use wast::lexer::Lexer;
use wast::parser::{self, ParseBuffer};
use wast::{QuoteWat, Wast, WastArg, WastDirective, WastExecute, WastInvoke, WastRet, Wat};
use zisk_common::EmuTrace;
use zisk_transpiler_wasm::layout::WASM_GLOBALS_ADDR;
use zisk_transpiler_wasm::module::parse_module;
use zisk_transpiler_wasm::wasm2rom;
use ziskemu::{Emu, EmuOptions};

/// Steps allowed per run: far above what any batch needs, low enough to stop a runaway loop.
const MAX_STEPS: u64 = 500_000_000;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Ty {
    I32,
    I64,
    F32,
    F64,
}

#[derive(Clone, Copy, Debug)]
struct Val {
    ty: Ty,
    bits: u64,
}

#[derive(Clone, Debug)]
enum Expect {
    Bits(Val),
    CanonicalNan(Ty),
    ArithmeticNan(Ty),
}

#[derive(Debug)]
enum Call {
    Invoke { name: String, args: Vec<Val> },
    Get { name: String },
}

#[derive(Debug)]
enum Outcome {
    Return(Vec<Expect>),
    Trap,
    Nothing,
}

#[derive(Debug)]
struct Action {
    line: usize,
    call: Call,
    outcome: Outcome,
}

struct Instance {
    line: usize,
    bytes: Vec<u8>,
    actions: Vec<Action>,
    /// Set when the module itself is expected to trap while it is instantiated.
    expect_trap: bool,
}

#[derive(Default)]
struct Tally {
    pass: usize,
    fail: usize,
    unsupported: usize,
    skipped: usize,
    failures: Vec<String>,
    reasons: BTreeMap<String, usize>,
}

impl Tally {
    fn fail(&mut self, file: &str, line: usize, msg: impl Into<String>) {
        self.fail += 1;
        self.failures.push(format!("{file}:{line}: {}", msg.into()));
    }

    fn unsupported(&mut self, n: usize, reason: &str) {
        self.unsupported += n;
        *self.reasons.entry(reason.to_string()).or_default() += n;
    }

    fn add(&mut self, other: Tally) {
        self.pass += other.pass;
        self.fail += other.fail;
        self.unsupported += other.unsupported;
        self.skipped += other.skipped;
        self.failures.extend(other.failures);
        for (reason, n) in other.reasons {
            *self.reasons.entry(reason).or_default() += n;
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Script directives → instances and actions
// ---------------------------------------------------------------------------------------------

fn arg(a: &WastArg<'_>) -> Option<Val> {
    match a {
        WastArg::Core(WastArgCore::I32(v)) => Some(Val { ty: Ty::I32, bits: *v as u32 as u64 }),
        WastArg::Core(WastArgCore::I64(v)) => Some(Val { ty: Ty::I64, bits: *v as u64 }),
        WastArg::Core(WastArgCore::F32(v)) => Some(Val { ty: Ty::F32, bits: v.bits as u64 }),
        WastArg::Core(WastArgCore::F64(v)) => Some(Val { ty: Ty::F64, bits: v.bits }),
        _ => None,
    }
}

fn expect(r: &WastRet<'_>) -> Option<Expect> {
    let nan = |p: &NanPattern<u64>, ty| match p {
        NanPattern::CanonicalNan => Expect::CanonicalNan(ty),
        NanPattern::ArithmeticNan => Expect::ArithmeticNan(ty),
        NanPattern::Value(bits) => Expect::Bits(Val { ty, bits: *bits }),
    };
    match r {
        WastRet::Core(WastRetCore::I32(v)) => {
            Some(Expect::Bits(Val { ty: Ty::I32, bits: *v as u32 as u64 }))
        }
        WastRet::Core(WastRetCore::I64(v)) => {
            Some(Expect::Bits(Val { ty: Ty::I64, bits: *v as u64 }))
        }
        WastRet::Core(WastRetCore::F32(p)) => Some(nan(
            &match p {
                NanPattern::CanonicalNan => NanPattern::CanonicalNan,
                NanPattern::ArithmeticNan => NanPattern::ArithmeticNan,
                NanPattern::Value(v) => NanPattern::Value(v.bits as u64),
            },
            Ty::F32,
        )),
        WastRet::Core(WastRetCore::F64(p)) => Some(nan(
            &match p {
                NanPattern::CanonicalNan => NanPattern::CanonicalNan,
                NanPattern::ArithmeticNan => NanPattern::ArithmeticNan,
                NanPattern::Value(v) => NanPattern::Value(v.bits),
            },
            Ty::F64,
        )),
        _ => None,
    }
}

/// Why a call's values cannot be expressed: SIMD or references.
fn value_reason(v128: bool) -> &'static str {
    if v128 {
        "harness: v128 value"
    } else {
        "harness: reference value"
    }
}

fn arg_is_v128(a: &WastArg<'_>) -> bool {
    matches!(a, WastArg::Core(WastArgCore::V128(_)))
}

fn ret_is_v128(r: &WastRet<'_>) -> bool {
    matches!(r, WastRet::Core(WastRetCore::V128(_)))
}

fn invoke_call(inv: &WastInvoke<'_>) -> Option<Call> {
    let args = inv.args.iter().map(arg).collect::<Option<Vec<_>>>()?;
    Some(Call::Invoke { name: inv.name.to_string(), args })
}

/// Parses one script and runs it.
fn run_file(path: &Path) -> Tally {
    let file = path.file_name().unwrap().to_string_lossy().to_string();
    let mut tally = Tally::default();
    let text = std::fs::read_to_string(path).expect("read .wast");
    // `names.wast` exports names with bidirectional-text characters, which the lexer refuses
    // by default.
    let mut lexer = Lexer::new(&text);
    lexer.allow_confusing_unicode(true);
    let buf = match ParseBuffer::new_with_lexer(lexer) {
        Ok(b) => b,
        Err(e) => {
            tally.fail(&file, 0, format!("lexer: {e}"));
            return tally;
        }
    };
    let wast = match parser::parse::<Wast<'_>>(&buf) {
        Ok(w) => w,
        Err(e) => {
            tally.fail(&file, 0, format!("parser: {e}"));
            return tally;
        }
    };
    let line_of = |span: wast::token::Span| span.linecol_in(&text).0 + 1;

    let mut instances: Vec<Instance> = Vec::new();
    let mut by_id: HashMap<String, usize> = HashMap::new();
    let mut definitions: HashMap<String, Vec<u8>> = HashMap::new();
    let mut current: Option<usize> = None;
    // An instance whose module failed to encode: its actions are skipped.
    const BROKEN: usize = usize::MAX;
    // Instances registered under a name, and the ones a module this harness does not run (it
    // imports from them) may have changed through a shared memory, table or global: the later
    // actions on those are skipped too.
    let mut registered: HashMap<String, usize> = HashMap::new();
    let mut tainted: HashSet<usize> = HashSet::new();
    let taint =
        |bytes: &[u8], registered: &HashMap<String, usize>, tainted: &mut HashSet<usize>| {
            if let Ok(info) = analyze(bytes) {
                for import in &info.imports {
                    let module = import.split("::").next().unwrap();
                    if let Some(&i) = registered.get(module) {
                        tainted.insert(i);
                    }
                }
            }
        };

    let target = |id: Option<wast::token::Id<'_>>,
                  current: Option<usize>,
                  by_id: &HashMap<String, usize>,
                  tainted: &HashSet<usize>|
     -> Option<usize> {
        let i = match id {
            Some(id) => by_id.get(id.name()).copied(),
            None => current,
        };
        i.map(|i| if tainted.contains(&i) { BROKEN } else { i })
    };

    for directive in wast.directives {
        match directive {
            WastDirective::Module(mut qw) => {
                let line = line_of(qw.span());
                let name = qw.name().map(|id| id.name().to_string());
                let idx = match qw.encode() {
                    Ok(bytes) => {
                        taint(&bytes, &registered, &mut tainted);
                        instances.push(Instance {
                            line,
                            bytes,
                            actions: Vec::new(),
                            expect_trap: false,
                        });
                        instances.len() - 1
                    }
                    Err(_) => BROKEN,
                };
                if let Some(name) = name {
                    by_id.insert(name, idx);
                }
                current = Some(idx);
            }
            WastDirective::ModuleDefinition(mut qw) => {
                if let (Some(id), Ok(bytes)) = (qw.name(), qw.encode()) {
                    definitions.insert(id.name().to_string(), bytes);
                }
            }
            WastDirective::ModuleInstance { span, instance, module } => {
                let idx = match module.and_then(|m| definitions.get(m.name())) {
                    Some(bytes) => {
                        taint(bytes, &registered, &mut tainted);
                        instances.push(Instance {
                            line: line_of(span),
                            bytes: bytes.clone(),
                            actions: Vec::new(),
                            expect_trap: false,
                        });
                        instances.len() - 1
                    }
                    None => BROKEN,
                };
                if let Some(id) = instance {
                    by_id.insert(id.name().to_string(), idx);
                }
                current = Some(idx);
            }
            WastDirective::AssertMalformed { span, module, .. }
            | WastDirective::AssertInvalid { span, module, .. } => {
                let line = line_of(span);
                let mut module = module;
                match &module {
                    // Malformed text is the text parser's business, not the transpiler's.
                    QuoteWat::QuoteModule(..) | QuoteWat::QuoteComponent(..) => tally.skipped += 1,
                    QuoteWat::Wat(Wat::Component(_)) => tally.skipped += 1,
                    QuoteWat::Wat(Wat::Module(_)) => match module.encode() {
                        // The text format already refuses it (unresolved names, bad types).
                        Err(_) => tally.skipped += 1,
                        Ok(bytes) => match catch_unwind(|| parse_module(&bytes).is_ok()) {
                            Ok(false) => tally.pass += 1,
                            Ok(true) => tally.fail(&file, line, "invalid module was accepted"),
                            Err(_) => tally.fail(&file, line, "parse_module panicked"),
                        },
                    },
                }
            }
            WastDirective::Invoke(inv) => {
                let line = line_of(inv.span);
                match (target(inv.module, current, &by_id, &tainted), invoke_call(&inv)) {
                    (Some(BROKEN) | None, _) => tally.skipped += 1,
                    (Some(_), None) => {
                        tally.unsupported(1, value_reason(inv.args.iter().any(arg_is_v128)))
                    }
                    (Some(i), Some(call)) => {
                        instances[i].actions.push(Action { line, call, outcome: Outcome::Nothing })
                    }
                }
            }
            WastDirective::AssertReturn { span, exec, results } => {
                let line = line_of(span);
                let (module, call) = match &exec {
                    WastExecute::Invoke(inv) => (inv.module, invoke_call(inv)),
                    WastExecute::Get { module, global, .. } => {
                        (*module, Some(Call::Get { name: global.to_string() }))
                    }
                    WastExecute::Wat(_) => {
                        tally.skipped += 1;
                        continue;
                    }
                };
                let expected = results.iter().map(expect).collect::<Option<Vec<_>>>();
                match (target(module, current, &by_id, &tainted), call, expected) {
                    (Some(BROKEN) | None, ..) => tally.skipped += 1,
                    (Some(_), None, _) | (Some(_), _, None) => {
                        let v128 = results.iter().any(ret_is_v128)
                            || matches!(&exec, WastExecute::Invoke(inv) if inv.args.iter().any(arg_is_v128));
                        tally.unsupported(1, value_reason(v128))
                    }
                    (Some(i), Some(call), Some(expected)) => instances[i].actions.push(Action {
                        line,
                        call,
                        outcome: Outcome::Return(expected),
                    }),
                }
            }
            WastDirective::AssertTrap { span, exec, .. } => {
                let line = line_of(span);
                match exec {
                    WastExecute::Invoke(inv) => {
                        match (target(inv.module, current, &by_id, &tainted), invoke_call(&inv)) {
                            (Some(BROKEN) | None, _) => tally.skipped += 1,
                            (Some(_), None) => {
                                tally.unsupported(1, value_reason(inv.args.iter().any(arg_is_v128)))
                            }
                            (Some(i), Some(call)) => instances[i].actions.push(Action {
                                line,
                                call,
                                outcome: Outcome::Trap,
                            }),
                        }
                    }
                    WastExecute::Wat(mut wat) => match wat.encode() {
                        Ok(bytes) => {
                            taint(&bytes, &registered, &mut tainted);
                            instances.push(Instance {
                                line,
                                bytes,
                                actions: Vec::new(),
                                expect_trap: true,
                            })
                        }
                        Err(_) => tally.skipped += 1,
                    },
                    WastExecute::Get { .. } => tally.skipped += 1,
                }
            }
            WastDirective::AssertExhaustion { span, call, .. } => {
                let line = line_of(span);
                match (target(call.module, current, &by_id, &tainted), invoke_call(&call)) {
                    (Some(BROKEN) | None, _) => tally.skipped += 1,
                    (Some(_), None) => {
                        tally.unsupported(1, value_reason(call.args.iter().any(arg_is_v128)))
                    }
                    (Some(i), Some(c)) => {
                        instances[i].actions.push(Action { line, call: c, outcome: Outcome::Trap })
                    }
                }
            }
            WastDirective::Register { name, module, .. } => {
                if let Some(i) = target(module, current, &by_id, &tainted) {
                    registered.insert(name.to_string(), i);
                }
            }
            WastDirective::AssertUnlinkable { mut module, .. } => {
                if let Ok(bytes) = module.encode() {
                    taint(&bytes, &registered, &mut tainted);
                }
                tally.skipped += 1;
            }
            WastDirective::AssertInvalidCustom { .. }
            | WastDirective::AssertMalformedCustom { .. }
            | WastDirective::AssertException { .. }
            | WastDirective::AssertSuspension { .. }
            | WastDirective::Thread(_)
            | WastDirective::Wait { .. } => tally.skipped += 1,
        }
    }

    for instance in &instances {
        run_instance(&file, instance, &mut tally);
    }
    tally
}

// ---------------------------------------------------------------------------------------------
// Module analysis and instrumentation
// ---------------------------------------------------------------------------------------------

struct ModuleInfo {
    /// Every type, `None` for the ones that are not function types.
    types: Vec<Option<(Vec<ValType>, Vec<ValType>)>>,
    /// The type of every function, imported ones first.
    funcs: Vec<u32>,
    /// The type of every global, imported ones first.
    globals: Vec<ValType>,
    exports: HashMap<String, (ExternalKind, u32)>,
    imports: Vec<String>,
}

fn analyze(bytes: &[u8]) -> Result<ModuleInfo, String> {
    let mut info = ModuleInfo {
        types: Vec::new(),
        funcs: Vec::new(),
        globals: Vec::new(),
        exports: HashMap::new(),
        imports: Vec::new(),
    };
    for payload in wasmparser::Parser::new(0).parse_all(bytes) {
        match payload.map_err(|e| e.to_string())? {
            Payload::TypeSection(r) => {
                for group in r {
                    for sub in group.map_err(|e| e.to_string())?.into_types() {
                        info.types.push(match &sub.composite_type.inner {
                            CompositeInnerType::Func(f) => {
                                Some((f.params().to_vec(), f.results().to_vec()))
                            }
                            _ => None,
                        });
                    }
                }
            }
            Payload::ImportSection(r) => {
                for import in r.into_imports() {
                    let import = import.map_err(|e| e.to_string())?;
                    info.imports.push(format!("{}::{}", import.module, import.name));
                    match import.ty {
                        TypeRef::Func(t) => info.funcs.push(t),
                        TypeRef::Global(g) => info.globals.push(g.content_type),
                        _ => {}
                    }
                }
            }
            Payload::FunctionSection(r) => {
                for t in r {
                    info.funcs.push(t.map_err(|e| e.to_string())?);
                }
            }
            Payload::GlobalSection(r) => {
                for g in r {
                    info.globals.push(g.map_err(|e| e.to_string())?.ty.content_type);
                }
            }
            Payload::ExportSection(r) => {
                for e in r {
                    let e = e.map_err(|e| e.to_string())?;
                    info.exports.insert(e.name.to_string(), (e.kind, e.index));
                }
            }
            _ => {}
        }
    }
    Ok(info)
}

fn ty_of(v: ValType) -> Option<Ty> {
    match v {
        ValType::I32 => Some(Ty::I32),
        ValType::I64 => Some(Ty::I64),
        ValType::F32 => Some(Ty::F32),
        ValType::F64 => Some(Ty::F64),
        _ => None,
    }
}

/// A call resolved against the module: what to push, what to call, what comes back.
enum Target {
    Func(u32),
    Global(u32),
}

struct Resolved {
    args: Vec<Val>,
    target: Target,
    results: Vec<Ty>,
}

fn resolve(info: &ModuleInfo, call: &Call) -> Result<Resolved, (bool, String)> {
    // `Err((true, _))` is unsupported, `Err((false, _))` a failure.
    match call {
        Call::Invoke { name, args } => {
            let Some(&(ExternalKind::Func, index)) = info.exports.get(name) else {
                return Err((false, format!("no exported function {name:?}")));
            };
            let Some(Some((params, results))) = info.types.get(info.funcs[index as usize] as usize)
            else {
                return Err((false, format!("{name:?} has no function type")));
            };
            let v128 = params.iter().chain(results).any(|&t| t == ValType::V128);
            let params: Option<Vec<Ty>> = params.iter().map(|&p| ty_of(p)).collect();
            let results: Option<Vec<Ty>> = results.iter().map(|&r| ty_of(r)).collect();
            let (Some(params), Some(results)) = (params, results) else {
                return Err((true, value_reason(v128).into()));
            };
            if params.len() != args.len() || params.iter().zip(args).any(|(p, a)| *p != a.ty) {
                return Err((false, format!("arguments do not match the type of {name:?}")));
            }
            Ok(Resolved { args: args.clone(), target: Target::Func(index), results })
        }
        Call::Get { name } => {
            let Some(&(ExternalKind::Global, index)) = info.exports.get(name) else {
                return Err((false, format!("no exported global {name:?}")));
            };
            match ty_of(info.globals[index as usize]) {
                Some(ty) => Ok(Resolved {
                    args: Vec::new(),
                    target: Target::Global(index),
                    results: vec![ty],
                }),
                None => {
                    Err((true, value_reason(info.globals[index as usize] == ValType::V128).into()))
                }
            }
        }
    }
}

fn uleb(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let byte = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

fn sleb(out: &mut Vec<u8>, mut v: i64) {
    loop {
        let byte = (v & 0x7f) as u8;
        v >>= 7;
        let done = (v == 0 && byte & 0x40 == 0) || (v == -1 && byte & 0x40 != 0);
        out.push(if done { byte } else { byte | 0x80 });
        if done {
            return;
        }
    }
}

fn read_uleb(bytes: &[u8], pos: &mut usize) -> u64 {
    let (mut v, mut shift) = (0u64, 0);
    loop {
        let b = bytes[*pos];
        *pos += 1;
        v |= ((b & 0x7f) as u64) << shift;
        if b & 0x80 == 0 {
            return v;
        }
        shift += 7;
    }
}

/// The module with a `_start` that runs `calls` in order. Global `base` is the progress counter
/// (the number of calls completed), and the results of the calls marked `true` go to the
/// globals after it, in order.
fn instrument(bytes: &[u8], info: &ModuleInfo, calls: &[(&Resolved, bool)]) -> (Vec<u8>, u32) {
    let base = info.globals.len() as u32;
    let recorded: usize = calls.iter().filter(|c| c.1).map(|c| c.0.results.len()).sum();

    let mut body = vec![0u8]; // no locals
    let mut slot = base + 1;
    for (i, (call, record)) in calls.iter().enumerate() {
        for a in &call.args {
            match a.ty {
                Ty::I32 => {
                    body.push(0x41);
                    sleb(&mut body, a.bits as u32 as i32 as i64);
                }
                Ty::I64 => {
                    body.push(0x42);
                    sleb(&mut body, a.bits as i64);
                }
                Ty::F32 => {
                    body.push(0x43);
                    body.extend_from_slice(&(a.bits as u32).to_le_bytes());
                }
                Ty::F64 => {
                    body.push(0x44);
                    body.extend_from_slice(&a.bits.to_le_bytes());
                }
            }
        }
        match call.target {
            Target::Func(f) => {
                body.push(0x10);
                uleb(&mut body, f as u64);
            }
            Target::Global(g) => {
                body.push(0x23);
                uleb(&mut body, g as u64);
            }
        }
        // The results are on the stack, the last one on top.
        let first = slot;
        for (j, ty) in call.results.iter().enumerate().rev() {
            if *record {
                match ty {
                    Ty::I32 => body.push(0xad), // i64.extend_i32_u
                    Ty::I64 => {}
                    Ty::F32 => body.extend_from_slice(&[0xbc, 0xad]), // i32.reinterpret_f32
                    Ty::F64 => body.push(0xbd),                       // i64.reinterpret_f64
                }
                body.push(0x24);
                uleb(&mut body, (first + j as u32) as u64);
            } else {
                body.push(0x1a); // drop
            }
        }
        if *record {
            slot += call.results.len() as u32;
        }
        body.push(0x42);
        sleb(&mut body, i as i64 + 1);
        body.push(0x24);
        uleb(&mut body, base as u64);
    }
    body.push(0x0b);

    let start_func = info.funcs.len() as u64;
    let start_type = info.types.len() as u64;
    let mut additions: HashMap<u8, (u64, Vec<u8>)> = HashMap::new();
    additions.insert(1, (1, vec![0x60, 0, 0]));
    let mut f = Vec::new();
    uleb(&mut f, start_type);
    additions.insert(3, (1, f));
    let mut g = Vec::new();
    for _ in 0..=recorded {
        g.extend_from_slice(&[0x7e, 0x01, 0x42, 0x00, 0x0b]); // (mut i64) (i64.const 0)
    }
    additions.insert(6, (recorded as u64 + 1, g));
    let mut e = Vec::new();
    uleb(&mut e, 6);
    e.extend_from_slice(b"_start");
    e.push(0x00);
    uleb(&mut e, start_func);
    additions.insert(7, (1, e));
    let mut c = Vec::new();
    uleb(&mut c, body.len() as u64);
    c.extend_from_slice(&body);
    additions.insert(10, (1, c));

    // Section order of the binary format (custom sections may appear anywhere).
    let rank = |id: u8| [1u8, 2, 3, 4, 5, 13, 6, 7, 8, 9, 12, 10, 11].iter().position(|&s| s == id);
    let mut out = bytes[..8].to_vec();
    let emit = |out: &mut Vec<u8>, id: u8, payload: &[u8]| {
        out.push(id);
        uleb(out, payload.len() as u64);
        out.extend_from_slice(payload);
    };
    let fresh = |(count, items): &(u64, Vec<u8>)| {
        let mut p = Vec::new();
        uleb(&mut p, *count);
        p.extend_from_slice(items);
        p
    };
    let mut pos = 8;
    while pos < bytes.len() {
        let id = bytes[pos];
        pos += 1;
        let size = read_uleb(bytes, &mut pos) as usize;
        let payload = &bytes[pos..pos + size];
        pos += size;
        if let Some(r) = rank(id) {
            // Missing sections that go before this one.
            let mut missing: Vec<u8> =
                additions.keys().copied().filter(|&a| rank(a).unwrap() < r).collect();
            missing.sort_by_key(|&a| rank(a));
            for a in missing {
                let add = additions.remove(&a).unwrap();
                emit(&mut out, a, &fresh(&add));
            }
        }
        match additions.remove(&id) {
            Some((count, items)) => {
                let mut p = 0;
                let old = read_uleb(payload, &mut p);
                let mut merged = Vec::new();
                uleb(&mut merged, old + count);
                merged.extend_from_slice(&payload[p..]);
                merged.extend_from_slice(&items);
                emit(&mut out, id, &merged);
            }
            None => emit(&mut out, id, payload),
        }
    }
    let mut rest: Vec<u8> = additions.keys().copied().collect();
    rest.sort_by_key(|&a| rank(a));
    for a in rest {
        let add = additions.remove(&a).unwrap();
        emit(&mut out, a, &fresh(&add));
    }
    (out, base)
}

// ---------------------------------------------------------------------------------------------
// Running
// ---------------------------------------------------------------------------------------------

enum Run {
    /// `wasm2rom` refused the module.
    Rejected(String),
    Panicked(String),
    Done {
        /// Calls completed before the run ended.
        completed: usize,
        trapped: bool,
        terminated: bool,
        values: Vec<u64>,
    },
}

fn run(bytes: &[u8], info: &ModuleInfo, calls: &[(&Resolved, bool)]) -> Run {
    let (wasm, base) = instrument(bytes, info, calls);
    let recorded: usize = calls.iter().filter(|c| c.1).map(|c| c.0.results.len()).sum();
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let rom = match wasm2rom(&wasm) {
            Ok(rom) => rom,
            Err(e) => return Run::Rejected(e.to_string()),
        };
        let mut emu = Emu::new(&rom);
        let options = EmuOptions { max_steps: MAX_STEPS, ..Default::default() };
        emu.run(Vec::new(), &options, None::<fn(EmuTrace)>);
        let read = |g: u32| emu.ctx.inst_ctx.mem.read(WASM_GLOBALS_ADDR + 8 * g as u64, 8);
        Run::Done {
            completed: read(base) as usize,
            trapped: emu.ctx.inst_ctx.error,
            terminated: emu.terminated(),
            values: (0..recorded as u32).map(|j| read(base + 1 + j)).collect(),
        }
    }));
    outcome.unwrap_or_else(|p| {
        let msg = p
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default();
        Run::Panicked(msg)
    })
}

fn matches(e: &Expect, got: u64) -> bool {
    match e {
        Expect::Bits(v) => v.bits == got,
        Expect::CanonicalNan(Ty::F32) => got & 0x7fff_ffff == 0x7fc0_0000,
        Expect::ArithmeticNan(Ty::F32) => got & 0x7fc0_0000 == 0x7fc0_0000,
        Expect::CanonicalNan(_) => got & 0x7fff_ffff_ffff_ffff == 0x7ff8_0000_0000_0000,
        Expect::ArithmeticNan(_) => got & 0x7ff8_0000_0000_0000 == 0x7ff8_0000_0000_0000,
    }
}

/// The first line of a rejection, with numbers removed so that equal causes group together.
fn reason(msg: &str) -> String {
    let first = msg.lines().next().unwrap_or("");
    let first = first.split(" (at offset").next().unwrap();
    let mut out = String::new();
    let (mut in_number, mut in_word) = (false, false);
    for ch in first.chars() {
        if ch.is_ascii_digit() && !in_word {
            if !in_number {
                out.push('N');
            }
            in_number = true;
        } else {
            in_word = ch.is_alphanumeric() && !in_number || (in_word && ch.is_ascii_digit());
            in_number = false;
            out.push(ch);
        }
    }
    out.chars().take(100).collect()
}

fn run_instance(file: &str, instance: &Instance, tally: &mut Tally) {
    let assertions =
        instance.actions.iter().filter(|a| !matches!(a.outcome, Outcome::Nothing)).count()
            + instance.expect_trap as usize;
    let info = match analyze(&instance.bytes) {
        Ok(info) => info,
        Err(e) => {
            tally.unsupported(assertions, &format!("wasmparser: {}", reason(&e)));
            return;
        }
    };
    if info.exports.contains_key("_start") {
        tally.unsupported(assertions, "harness: the module exports _start");
        return;
    }
    // A module that imports anything but WASI needs linking, which this harness does not model.
    if let Some(import) = info.imports.iter().find(|i| !i.starts_with("wasi_snapshot_preview1::")) {
        let module = import.split("::").next().unwrap();
        tally.skipped += assertions;
        *tally.reasons.entry(format!("skipped: imports from {module}")).or_default() += assertions;
        return;
    }

    // Instantiation alone.
    match run(&instance.bytes, &info, &[]) {
        Run::Rejected(e) => {
            tally.unsupported(assertions, &reason(&e));
            return;
        }
        Run::Panicked(e) => {
            tally.fail(file, instance.line, format!("transpiler or emulator panicked: {e}"));
            return;
        }
        Run::Done { trapped, terminated, .. } => {
            if instance.expect_trap {
                if trapped {
                    tally.pass += 1;
                } else {
                    tally.fail(file, instance.line, "module instantiated, expected a trap");
                }
                return;
            }
            if !terminated || trapped {
                tally.fail(file, instance.line, "instantiation trapped or did not end");
                return;
            }
        }
    }

    // Resolve every action; the ones that cannot run are accounted for here.
    let mut resolved: Vec<Option<Resolved>> = Vec::new();
    for action in &instance.actions {
        match resolve(&info, &action.call) {
            Ok(r) => resolved.push(Some(r)),
            Err((unsupported, msg)) => {
                if !matches!(action.outcome, Outcome::Nothing) {
                    if unsupported {
                        tally.unsupported(1, &msg);
                    } else {
                        tally.fail(file, action.line, msg);
                    }
                }
                resolved.push(None);
            }
        }
    }

    // The calls that should return, in one batch, dropping each one that traps unexpectedly.
    let mut batch: Vec<usize> = (0..instance.actions.len())
        .filter(|&i| resolved[i].is_some() && !matches!(instance.actions[i].outcome, Outcome::Trap))
        .collect();
    loop {
        let calls: Vec<(&Resolved, bool)> = batch
            .iter()
            .map(|&i| {
                let record = matches!(instance.actions[i].outcome, Outcome::Return(_));
                (resolved[i].as_ref().unwrap(), record)
            })
            .collect();
        match run(&instance.bytes, &info, &calls) {
            Run::Rejected(e) => {
                let n = batch
                    .iter()
                    .filter(|&&i| matches!(instance.actions[i].outcome, Outcome::Return(_)))
                    .count();
                tally.unsupported(n, &reason(&e));
                batch.clear();
                break;
            }
            Run::Panicked(e) => {
                tally.fail(file, instance.line, format!("transpiler or emulator panicked: {e}"));
                batch.clear();
                break;
            }
            Run::Done { completed, trapped, terminated, values } => {
                if completed < batch.len() {
                    let i = batch.remove(completed);
                    let action = &instance.actions[i];
                    let what = if !terminated {
                        "did not end within the step limit"
                    } else if trapped {
                        "trapped unexpectedly"
                    } else {
                        "the run ended early"
                    };
                    if matches!(action.outcome, Outcome::Return(_)) {
                        tally.fail(file, action.line, what);
                    }
                    continue;
                }
                let mut values = values.into_iter();
                for &i in &batch {
                    let action = &instance.actions[i];
                    let Outcome::Return(expected) = &action.outcome else { continue };
                    let got: Vec<u64> =
                        values.by_ref().take(resolved[i].as_ref().unwrap().results.len()).collect();
                    if got.len() != expected.len() {
                        tally.fail(
                            file,
                            action.line,
                            format!("{} results, expected {}", got.len(), expected.len()),
                        );
                    } else if expected.iter().zip(&got).all(|(e, &g)| matches(e, g)) {
                        tally.pass += 1;
                    } else {
                        tally.fail(
                            file,
                            action.line,
                            format!("{:?}: got {:x?}, expected {:x?}", action.call, got, expected),
                        );
                    }
                }
                break;
            }
        }
    }

    // Each expected trap, after the calls that precede it.
    for (t, action) in instance.actions.iter().enumerate() {
        if !matches!(action.outcome, Outcome::Trap) {
            continue;
        }
        let Some(trapping) = resolved[t].as_ref() else { continue };
        let mut calls: Vec<(&Resolved, bool)> = batch
            .iter()
            .take_while(|&&i| i < t)
            .map(|&i| (resolved[i].as_ref().unwrap(), false))
            .collect();
        let at = calls.len();
        calls.push((trapping, false));
        match run(&instance.bytes, &info, &calls) {
            Run::Rejected(e) => tally.unsupported(1, &reason(&e)),
            Run::Panicked(e) => {
                tally.fail(file, action.line, format!("transpiler or emulator panicked: {e}"))
            }
            Run::Done { completed, trapped, terminated, .. } => {
                if trapped && terminated && completed == at {
                    tally.pass += 1;
                } else if !terminated {
                    tally.fail(file, action.line, "did not end within the step limit");
                } else if completed > at {
                    tally.fail(
                        file,
                        action.line,
                        format!("{:?} returned, expected a trap", action.call),
                    );
                } else {
                    tally.fail(file, action.line, "an earlier call trapped");
                }
            }
        }
    }
}

#[test]
#[ignore = "needs the WebAssembly test suite: set WASM_TESTSUITE (see the module docs)"]
fn webassembly_spec_testsuite() {
    let dir = std::env::var("WASM_TESTSUITE")
        .expect("set WASM_TESTSUITE to a checkout of github.com/WebAssembly/testsuite");
    let filters: Vec<String> = std::env::var("WASM_SPEC_FILTER")
        .map(|f| f.split(',').map(str::to_string).collect())
        .unwrap_or_default();
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("read WASM_TESTSUITE")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "wast"))
        .filter(|p| {
            let name = p.file_name().unwrap().to_string_lossy();
            filters.is_empty() || filters.iter().any(|f| name.contains(f.as_str()))
        })
        .collect();
    files.sort();

    // Panics are caught and counted; keep their messages out of the report.
    std::panic::set_hook(Box::new(|_| {}));
    let mut total = Tally::default();
    println!("{:<36} {:>6} {:>6} {:>6} {:>6}", "file", "pass", "fail", "unsup", "skip");
    for path in &files {
        let tally = run_file(path);
        println!(
            "{:<36} {:>6} {:>6} {:>6} {:>6}",
            path.file_name().unwrap().to_string_lossy(),
            tally.pass,
            tally.fail,
            tally.unsupported,
            tally.skipped
        );
        total.add(tally);
    }
    let _ = std::panic::take_hook();
    println!(
        "{:<36} {:>6} {:>6} {:>6} {:>6}",
        "TOTAL", total.pass, total.fail, total.unsupported, total.skipped
    );
    let mut reasons: Vec<_> = total.reasons.iter().filter(|r| *r.1 > 0).collect();
    reasons.sort_by(|a, b| b.1.cmp(a.1));
    println!("\nunsupported and skipped, by reason:");
    for (r, n) in reasons {
        println!("{n:>7}  {r}");
    }
    println!("\nfailures:");
    for f in &total.failures {
        println!("  {f}");
    }
    assert_eq!(total.fail, 0, "{} spec assertions failed (listed above)", total.fail);
}
