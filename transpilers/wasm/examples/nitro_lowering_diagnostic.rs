//! Diagnostic only: register trapping imports to inspect real Nitro lowering.
//! This does not execute Nitro, implement an adapter, or establish a proof/semantic claim.
//!
//! Run with `cargo run --release -p zisk-transpiler-wasm --features float
//! --example nitro_lowering_diagnostic -- <replay.wasm>` under external time/RSS limits.

use std::{error::Error, fs, time::Instant};
use zisk_transpiler_wasm::{
    emit::{Code, PendingInst},
    lowering::lower_function,
    module::parse_module,
    wasm2rom_with_imports, Import,
};

fn main() -> Result<(), Box<dyn Error>> {
    if !cfg!(feature = "float") {
        return Err("diagnostic requires --features float".into());
    }
    let path = std::env::args().nth(1).ok_or("expected path to Nitro replay Wasm")?;
    if fs::metadata(&path)?.len() > 32 * 1024 * 1024 {
        return Err("diagnostic input exceeds 32 MiB".into());
    }
    let bytes = fs::read(path)?;
    let module = parse_module(&bytes)?;
    let imports = module
        .imports
        .iter()
        .filter(|(namespace, _, _)| namespace != "wasi_snapshot_preview1")
        .map(|(namespace, name, type_index)| {
            assert!(matches!(
                namespace.as_str(),
                "arbcrypto" | "wavmio" | "arbcompress" | "hooks" | "programs"
            ));
            let sig = &module.sigs[*type_index as usize];
            eprintln!("trap import {namespace}::{name}: {:?} -> {:?}", sig.params, sig.results);
            Import {
                module: namespace,
                name,
                params: &sig.params,
                results: &sig.results,
                emit: Code::trap,
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(imports.len(), 25, "expected exactly 25 Nitro custom imports");
    eprintln!(
        "DIAGNOSTIC ONLY: {} bytes, {} functions, {} defined; no execution or proof",
        bytes.len(),
        module.func_count(),
        module.defined.len()
    );
    let start = Instant::now();
    // Drop each body after inspecting it, so a large earlier body cannot hide later compiler errors.
    let mut instructions = 0usize;
    for index in module.func_import_count..module.func_count() {
        let code = lower_function(&module, index).map_err(|error| {
            format!("defined-body preflight failed at function {index}: {error}")
        })?;
        instructions += code.len();
        if index % 100 == 0 {
            eprintln!(
                "preflight function {index}: {} instructions; cumulative {instructions}",
                code.len()
            );
        }
    }
    eprintln!("all defined bodies lowered individually: {instructions} instructions in {:?}; {} bytes of PendingInst payload alone, before Vec capacity/labels/ROM copies", start.elapsed(), instructions * std::mem::size_of::<PendingInst>());
    eprintln!("defined-body ROM end lower bound: {:#x}; float-enabled program ROM end: {:#x}; full ROM construction next", zisk_core::ROM_ADDR + instructions as u64 * 4, zisk_core::FLOAT_LIB_ROM_ADDR);
    match wasm2rom_with_imports(&bytes, &imports) {
        Ok(_) => {
            eprintln!("full lowering succeeded in {:?}; all custom imports trap", start.elapsed())
        }
        Err(error) => {
            eprintln!("full lowering failed after {:?}: {error}", start.elapsed());
            return Err(error);
        }
    }
    Ok(())
}
