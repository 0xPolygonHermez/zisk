//! Explicit custom imports lowered into the same ROM as the Wasm guest.

use std::{collections::BTreeSet, error::Error};

use super::{
    emit::Code,
    layout::*,
    module::{ValKind, WasmModule},
    wasi,
};

/// An explicitly registered import implemented by generated ZisK instructions.
///
/// `emit` runs during translation, not execution, and must be deterministic and independent of
/// private inputs. Its body reads parameters from `local_offset(i)` and leaves its result in
/// `REG_RET`. The transpiler supplies the function prologue/epilogue and normalizes i32 results.
/// Bodies must preserve `REG_FP`, `REG_MEM_END` and the frame header, and validate guest pointers
/// before memory access. They are trusted compiler code; emitted instructions are not sandboxed.
///
/// The resulting ROM/program verification key must bind these implementations. The Wasm hash
/// alone does not identify a program using custom imports; setup and execution must use the same
/// registrations. An emitter must never substitute host-computed private witness results.
pub struct Import<'a> {
    pub module: &'a str,
    pub name: &'a str,
    pub params: &'a [ValKind],
    pub results: &'a [ValKind],
    pub emit: fn(&mut Code),
}

pub(super) fn validate(imports: &[Import<'_>]) -> Result<(), Box<dyn Error>> {
    let mut names = BTreeSet::new();
    for import in imports {
        if import.module == "wasi_snapshot_preview1" {
            return Err("wasm: wasi_snapshot_preview1 is reserved for built-in imports".into());
        }
        if !names.insert((import.module, import.name)) {
            return Err(format!(
                "wasm: duplicate import registration {}::{}",
                import.module, import.name
            )
            .into());
        }
    }
    Ok(())
}

pub(super) fn build(
    module: &WasmModule,
    index: usize,
    imports: &[Import<'_>],
    nitro: bool,
) -> Result<Code, Box<dyn Error>> {
    let (namespace, name, _) = &module.imports[index];
    if namespace == "wasi_snapshot_preview1" || imports.is_empty() {
        return wasi::build_wasi_stub_with_finalize(
            module,
            index,
            nitro.then_some(super::nitro::runtime::emit_finalize),
        );
    }
    let import = imports
        .iter()
        .find(|i| i.module == namespace && i.name == name)
        .ok_or_else(|| format!("wasm: unregistered import {namespace}::{name}"))?;
    let signature = module.func_sig(index as u32)?;
    if signature.params != import.params || signature.results != import.results {
        return Err(format!(
            "wasm: import {namespace}::{name} signature {:?} -> {:?} does not match registered {:?} -> {:?}",
            signature.params, signature.results, import.params, import.results,
        ).into());
    }

    let mut code = Code::new();
    code.store_reg_to_slot(FRAME_RET_PC_OFF, REG_RA);
    (import.emit)(&mut code);
    if import.results == [ValKind::I32] {
        code.signextend_w_reg(REG_RET);
    }
    code.load_slot_to_reg(REG_T0, FRAME_RET_PC_OFF);
    code.load_slot_to_reg(REG_T2, FRAME_CALLER_FP_OFF);
    code.mov_reg(REG_FP, REG_T2);
    code.ret_reg(REG_T0);
    Ok(code)
}
