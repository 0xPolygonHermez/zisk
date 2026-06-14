//! `wasmemu` — transpile a `wasm32-wasip1` guest to a Zisk ROM and emulate it.
//!
//! This is the wasm counterpart of `ziskemu`.  It reads a `.wasm` file, optionally an input file
//! (mapped to the guest's stdin via the WASI `fd_read` shim), runs the program on the Zisk
//! emulator, and prints the program's public output.  Guest stdout/stderr (`fd_write`) is streamed
//! live to the console via the UART during emulation.

use std::{fs, process};

use clap::Parser;
use zisk_common::EmuTrace;
use zisk_core::wasm::wasm2rom;
use ziskemu::{EmuOptions, ZiskEmulator};

#[derive(Parser)]
#[command(
    name = "wasmemu",
    about = "Emulate a wasm32-wasip1 guest on the Zisk zkVM (transpile + run)"
)]
struct Args {
    /// Path to the `.wasm` guest module.
    wasm: String,

    /// Optional input file, fed to the guest's stdin (`fd_read`).
    #[arg(short = 'i', long = "input")]
    input: Option<String>,

    /// Maximum number of emulation steps.
    #[arg(short = 'n', long = "max-steps")]
    max_steps: Option<u64>,

    /// Print the public output region as hex.
    #[arg(short = 'x', long = "hex")]
    hex: bool,

    #[arg(short = 'v', long = "verbose")]
    verbose: bool,
}

fn main() {
    let args = Args::parse();

    let wasm = match fs::read(&args.wasm) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("wasmemu: cannot read '{}': {e}", args.wasm);
            process::exit(1);
        }
    };

    if !zisk_core::is_wasm_file(&wasm) {
        eprintln!("wasmemu: '{}' is not a WebAssembly binary (missing \\0asm magic)", args.wasm);
        process::exit(1);
    }

    // Build the stdin blob in the ziskos input format: an 8-byte little-endian length prefix
    // followed by the raw bytes, padded to an 8-byte boundary.  Always emit at least the length
    // prefix (length 0) so the guest's `fd_read` can read the length without hitting unmapped
    // memory when no input is provided.
    let data = match &args.input {
        Some(input_path) => match fs::read(input_path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("wasmemu: cannot read input '{input_path}': {e}");
                process::exit(1);
            }
        },
        None => Vec::new(),
    };
    let mut inputs = Vec::with_capacity(8 + data.len());
    inputs.extend_from_slice(&(data.len() as u64).to_le_bytes());
    inputs.extend_from_slice(&data);
    let pad = (8 - (inputs.len() % 8)) % 8;
    inputs.resize(inputs.len() + pad, 0);

    let rom = match wasm2rom(&wasm) {
        Ok(rom) => rom,
        Err(e) => {
            eprintln!("wasmemu: transpilation failed: {e}");
            process::exit(1);
        }
    };

    let mut options = EmuOptions { verbose: args.verbose, ..Default::default() };
    if let Some(n) = args.max_steps {
        options.max_steps = n;
    }

    match ZiskEmulator::process_rom(&rom, &inputs, &options, None::<fn(EmuTrace)>) {
        Ok(output) => {
            if args.hex {
                let hex: String = output.iter().map(|b| format!("{b:02x}")).collect();
                println!("{hex}");
            }
        }
        Err(e) => {
            eprintln!("wasmemu: emulation failed: {e:?}");
            process::exit(1);
        }
    }
}
