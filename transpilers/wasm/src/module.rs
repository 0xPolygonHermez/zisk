//! Structural scan of a wasm module with `wasmparser`.
//!
//! The module is first run through `wasmparser::Validator`, so everything downstream (the scan
//! here and the lowering) can rely on the spec's static guarantees: in-range indices, well-typed
//! function bodies with known operand-stack depths, no writes to immutable globals, and so on.
//! The scan then collects everything the lowering needs (signatures, imports, function bodies,
//! globals, active data/element segments, memory size, exports) and rejects the valid-but-
//! unsupported constructs (SIMD, reference types, threads, multiple memories, passive segments,
//! and floating point without the `float` feature) with a clear error, so the supported subset
//! fails loudly rather than miscompiling.

use std::error::Error;

use super::layout::WASM_MAX_PAGES;
use wasmparser::{
    DataKind, ElementItems, ElementKind, ExternalKind, Operator, Parser, Payload, TypeRef, ValType,
    Validator, WasmFeatures,
};

const SUPPORTED_FEATURES: WasmFeatures = WasmFeatures::WASM1
    .union(WasmFeatures::BULK_MEMORY)
    .union(WasmFeatures::REFERENCE_TYPES)
    .union(WasmFeatures::SIGN_EXTENSION)
    .union(WasmFeatures::SATURATING_FLOAT_TO_INT)
    .union(WasmFeatures::MULTI_VALUE);

/// The value kinds the lowering supports.  Every kind lives in a 64-bit Zisk slot: i32 values are
/// kept sign-extended, f32 values occupy the low 32 bits (the high half is don't-care).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValKind {
    I32,
    I64,
    F32,
    F64,
}

impl ValKind {
    pub fn from_valtype(t: ValType) -> Result<ValKind, Box<dyn Error>> {
        match t {
            ValType::I32 => Ok(ValKind::I32),
            ValType::I64 => Ok(ValKind::I64),
            ValType::F32 | ValType::F64 => {
                if !cfg!(feature = "float") {
                    return Err("wasm: floating-point types require the `float` feature (the \
                                soft-float library is not linked into this build)"
                        .into());
                }
                Ok(if t == ValType::F32 { ValKind::F32 } else { ValKind::F64 })
            }
            ValType::V128 => Err("wasm: SIMD (v128) is not supported".into()),
            ValType::Ref(_) => Err("wasm: reference types are not supported".into()),
        }
    }
}

/// A function signature from the type section.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FuncSig {
    pub params: Vec<ValKind>,
    pub results: Vec<ValKind>,
}

/// A declared global with its (constant) initial value.
#[derive(Clone, Debug)]
pub struct GlobalDecl {
    pub kind: ValKind,
    pub mutable: bool,
    pub init: i64,
}

/// An active data segment, with its resolved linear-memory offset.
#[derive(Clone, Debug)]
pub struct DataSeg {
    pub offset: u64,
    pub bytes: Vec<u8>,
}

/// An active element segment: a run of function indices placed in the table at `table_offset`.
#[derive(Clone, Debug)]
pub struct ElemSeg {
    pub table_offset: u32,
    pub func_indices: Vec<u32>,
}

/// A defined (non-imported) function: its type index and the raw body bytes for later lowering.
pub struct DefinedFunc<'a> {
    pub type_index: u32,
    pub body: wasmparser::FunctionBody<'a>,
    pub num_locals: u32,
}

/// The collected, validated module.
pub struct WasmModule<'a> {
    pub sigs: Vec<FuncSig>,
    /// Number of imported functions; these occupy function indices `0..func_import_count`.
    pub func_import_count: u32,
    /// `(module, name, type_index)` for each imported function, in index order.
    pub imports: Vec<(String, String, u32)>,
    pub defined: Vec<DefinedFunc<'a>>,
    pub globals: Vec<GlobalDecl>,
    pub data: Vec<DataSeg>,
    pub elems: Vec<ElemSeg>,
    pub mem_initial_pages: u64,
    pub has_memory: bool,
    pub table_initial: u64,
    max_num_locals: u32,
    pub start_func: Option<u32>,
    pub exports: Vec<(String, ExternalKind, u32)>,
}

impl<'a> WasmModule<'a> {
    /// Total number of functions (imported + defined).
    pub fn func_count(&self) -> u32 {
        self.func_import_count + self.defined.len() as u32
    }

    pub fn num_locals(&self, func_index: u32) -> Result<u32, Box<dyn Error>> {
        if func_index < self.func_import_count {
            Ok(self.func_sig(func_index)?.params.len() as u32)
        } else {
            Ok(self.defined[(func_index - self.func_import_count) as usize].num_locals)
        }
    }

    pub fn max_num_locals(&self) -> u32 {
        self.max_num_locals
    }

    /// Returns the signature of function `func_index` (across the whole index space).
    pub fn func_sig(&self, func_index: u32) -> Result<&FuncSig, Box<dyn Error>> {
        let type_index = if func_index < self.func_import_count {
            self.imports[func_index as usize].2
        } else {
            self.defined[(func_index - self.func_import_count) as usize].type_index
        };
        self.sigs
            .get(type_index as usize)
            .ok_or_else(|| format!("wasm: function {func_index} has invalid type index").into())
    }

    /// Returns a canonical id for a type index: the index of the first structurally-equal type.
    /// `call_indirect` type checks must compare these, because the type section may contain
    /// duplicate structurally-equal entries with different indices.
    pub fn canonical_type(&self, type_index: u32) -> u32 {
        let sig = &self.sigs[type_index as usize];
        self.sigs.iter().position(|s| s == sig).unwrap_or(type_index as usize) as u32
    }

    /// Resolves the function index of an exported function by name.
    pub fn exported_func(&self, name: &str) -> Option<u32> {
        self.exports.iter().find_map(|(n, kind, idx)| {
            if n == name && matches!(kind, ExternalKind::Func | ExternalKind::FuncExact) {
                Some(*idx)
            } else {
                None
            }
        })
    }
}

/// Evaluates a constant initializer expression to an `i64`.  Supports `i32.const`/`i64.const`
/// and `global.get` of an earlier (already-resolved) global; anything else is rejected.
fn eval_const_expr(
    expr: &wasmparser::ConstExpr,
    globals: &[GlobalDecl],
) -> Result<i64, Box<dyn Error>> {
    let mut reader = expr.get_operators_reader();
    let op = reader.read()?;
    let value = match op {
        Operator::I32Const { value } => value as i64,
        Operator::I64Const { value } => value,
        Operator::F32Const { value } => value.bits() as i64,
        Operator::F64Const { value } => value.bits() as i64,
        Operator::GlobalGet { global_index } => {
            globals
                .get(global_index as usize)
                .ok_or("wasm: const expr references undefined global")?
                .init
        }
        other => {
            return Err(format!("wasm: unsupported constant initializer: {other:?}").into());
        }
    };
    Ok(value)
}

/// Parses and validates a wasm module into a [`WasmModule`].
pub fn parse_module(bytes: &[u8]) -> Result<WasmModule<'_>, Box<dyn Error>> {
    Validator::new_with_features(SUPPORTED_FEATURES)
        .validate_all(bytes)
        .map_err(|e| format!("wasm: invalid module: {e}"))?;

    let mut sigs: Vec<FuncSig> = Vec::new();
    let mut func_import_count: u32 = 0;
    let mut imports: Vec<(String, String, u32)> = Vec::new();
    let mut defined_types: Vec<u32> = Vec::new();
    let mut bodies: Vec<wasmparser::FunctionBody> = Vec::new();
    let mut globals: Vec<GlobalDecl> = Vec::new();
    let mut data: Vec<DataSeg> = Vec::new();
    let mut elems: Vec<ElemSeg> = Vec::new();
    let mut mem_initial_pages: u64 = 0;
    let mut mem_max_pages: u64 = WASM_MAX_PAGES;
    let mut has_memory = false;
    let mut table_initial: u64 = 0;
    let mut has_table = false;
    let mut start_func: Option<u32> = None;
    let mut exports: Vec<(String, ExternalKind, u32)> = Vec::new();

    for payload in Parser::new(0).parse_all(bytes) {
        match payload? {
            Payload::TypeSection(reader) => {
                for func_ty in reader.into_iter_err_on_gc_types() {
                    let func_ty = func_ty.map_err(|_| {
                        "wasm: GC / non-function types are not supported".to_string()
                    })?;
                    let params = func_ty
                        .params()
                        .iter()
                        .map(|t| ValKind::from_valtype(*t))
                        .collect::<Result<Vec<_>, _>>()?;
                    let results = func_ty
                        .results()
                        .iter()
                        .map(|t| ValKind::from_valtype(*t))
                        .collect::<Result<Vec<_>, _>>()?;
                    sigs.push(FuncSig { params, results });
                }
            }
            Payload::ImportSection(reader) => {
                for group in reader {
                    for item in group? {
                        let (_pos, import) = item?;
                        match import.ty {
                            TypeRef::Func(type_index) | TypeRef::FuncExact(type_index) => {
                                imports.push((
                                    import.module.to_string(),
                                    import.name.to_string(),
                                    type_index,
                                ));
                                func_import_count += 1;
                            }
                            TypeRef::Memory(_)
                            | TypeRef::Global(_)
                            | TypeRef::Table(_)
                            | TypeRef::Tag(_) => {
                                return Err(format!(
                                    "wasm: unsupported import {}::{} (only function imports are \
                                 supported)",
                                    import.module, import.name
                                )
                                .into());
                            }
                        }
                    }
                }
            }
            Payload::FunctionSection(reader) => {
                for type_index in reader {
                    defined_types.push(type_index?);
                }
            }
            Payload::MemorySection(reader) => {
                for mem in reader {
                    let mem = mem?;
                    if has_memory {
                        return Err("wasm: multiple memories are not supported".into());
                    }
                    if mem.memory64 {
                        return Err("wasm: 64-bit memory is not supported".into());
                    }
                    if mem.shared {
                        return Err("wasm: shared memory (threads) is not supported".into());
                    }
                    has_memory = true;
                    mem_initial_pages = mem.initial;
                    if let Some(max) = mem.maximum {
                        mem_max_pages = mem_max_pages.min(max);
                    }
                }
            }
            Payload::TableSection(reader) => {
                for table in reader {
                    let table = table?;
                    if has_table {
                        return Err("wasm: only table 0 is supported".into());
                    }
                    if table.ty.table64 {
                        return Err("wasm: 64-bit tables are not supported".into());
                    }
                    has_table = true;
                    table_initial = table.ty.initial;
                }
            }
            Payload::GlobalSection(reader) => {
                for global in reader {
                    let global = global?;
                    let kind = ValKind::from_valtype(global.ty.content_type)?;
                    let init = eval_const_expr(&global.init_expr, &globals)?;
                    globals.push(GlobalDecl { kind, mutable: global.ty.mutable, init });
                }
            }
            Payload::ExportSection(reader) => {
                for export in reader {
                    let export = export?;
                    exports.push((export.name.to_string(), export.kind, export.index));
                }
            }
            Payload::StartSection { func, .. } => {
                start_func = Some(func);
            }
            Payload::ElementSection(reader) => {
                for element in reader {
                    let element = element?;
                    let ElementKind::Active { table_index, offset_expr } = element.kind else {
                        return Err("wasm: only active element segments are supported".into());
                    };
                    if table_index.unwrap_or(0) != 0 {
                        return Err("wasm: only table 0 is supported".into());
                    }
                    let table_offset = eval_const_expr(&offset_expr, &globals)? as u32;
                    let ElementItems::Functions(funcs) = element.items else {
                        return Err("wasm: expression element segments are not supported".into());
                    };
                    let func_indices = funcs.into_iter().collect::<Result<Vec<u32>, _>>()?;
                    elems.push(ElemSeg { table_offset, func_indices });
                }
            }
            Payload::DataSection(reader) => {
                for segment in reader {
                    let segment = segment?;
                    match segment.kind {
                        DataKind::Active { memory_index, offset_expr } => {
                            if memory_index != 0 {
                                return Err("wasm: only memory 0 is supported".into());
                            }
                            let offset = eval_const_expr(&offset_expr, &globals)? as u32 as u64;
                            data.push(DataSeg { offset, bytes: segment.data.to_vec() });
                        }
                        DataKind::Passive => {
                            return Err("wasm: passive data segments are not supported".into());
                        }
                    }
                }
            }
            Payload::CodeSectionEntry(body) => {
                bodies.push(body);
            }
            Payload::TagSection(_) => {
                return Err("wasm: exception handling (tags) is not supported".into());
            }
            _ => {}
        }
    }

    if defined_types.len() != bodies.len() {
        return Err(format!(
            "wasm: function/code section mismatch ({} declared, {} bodies)",
            defined_types.len(),
            bodies.len()
        )
        .into());
    }

    let mut defined = Vec::with_capacity(bodies.len());
    for (type_index, body) in defined_types.into_iter().zip(bodies) {
        let sig = sigs
            .get(type_index as usize)
            .ok_or_else(|| format!("wasm: function has invalid type index {type_index}"))?;
        let mut num_locals = sig.params.len() as u32;
        for local in body.get_locals_reader()? {
            let (count, ty) = local?;
            ValKind::from_valtype(ty)?; // reject unsupported local types
            num_locals =
                num_locals.checked_add(count).ok_or("wasm: function declares too many locals")?;
        }
        defined.push(DefinedFunc { type_index, body, num_locals });
    }
    let import_params = imports.iter().filter_map(|(_, _, ty)| sigs.get(*ty as usize));
    let max_num_locals = import_params
        .map(|sig| sig.params.len() as u32)
        .chain(defined.iter().map(|f| f.num_locals))
        .max()
        .unwrap_or(0);

    Ok(WasmModule {
        sigs,
        func_import_count,
        imports,
        defined,
        globals,
        data,
        elems,
        mem_initial_pages,
        mem_max_pages,
        has_memory,
        table_initial,
        max_num_locals,
        start_func,
        exports,
    })
}
