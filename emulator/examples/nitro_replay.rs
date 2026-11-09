//! Run an explicitly compiled Nitro adapter against a separately checked expected claim.
use std::{error::Error, fs, time::Instant};
use zisk_common::EmuTrace;
use ziskemu::{Emu, EmuOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !(5..=6).contains(&args.len()) {
        return Err("usage: nitro_replay replay.wasm runtime.elf witness.stdin expected.bin actual.bin [program.ziskbin.elf]".into());
    }
    let wasm = fs::read(&args[0])?;
    let runtime = fs::read(&args[1])?;
    let stdin = fs::read(&args[2])?;
    let expected = fs::read(&args[3])?;
    if expected.len() != 256 {
        return Err("expected claim must be exactly 256 bytes".into());
    }
    let start = Instant::now();
    eprintln!("Compiling Nitro and native runtime ({} Wasm bytes)", wasm.len());
    let rom = zisk_transpiler_wasm::nitro::wasm2rom(&wasm, &runtime)?;
    eprintln!("Compiled {} instructions in {:?}; executing", rom.insts.len(), start.elapsed());
    let options = EmuOptions { max_steps: 2_000_000_000, log_metrics: true, ..Default::default() };
    let mut emu = Emu::new(&rom);
    emu.run(stdin, &options, None::<fn(EmuTrace)>);
    if let Some((reason, step, pc)) = emu.failure() {
        return Err(format!("execution failed: {reason:?}, step={step}, pc={pc:#x}, native return={:#x}, SP={:#x}, args={:x?}", emu.get_reg(1), emu.get_reg(2), (0..8).map(|i| emu.ctx.inst_ctx.mem.read(0xa0540200 + i * 8, 8)).collect::<Vec<_>>()).into());
    }
    if !emu.terminated() {
        return Err("execution exceeded the step limit".into());
    }
    let output = emu.get_output_8();
    eprintln!("Execution steps: {}", emu.number_of_steps());
    fs::write(&args[4], &output)?;
    if output != expected {
        let first = output.iter().zip(&expected).position(|(a, b)| a != b);
        return Err(format!(
            "Nitro public claim mismatch at byte {first:?}; actual saved to {}",
            args[4]
        )
        .into());
    }
    eprintln!("PASS: all 256 public bytes match; total {:?}", start.elapsed());
    if let Some(path) = args.get(5) {
        fs::write(path, zisk_core::ziskbin::rom_to_elf(&rom))?;
        eprintln!("Saved exact linked program to {path}");
    }
    Ok(())
}
