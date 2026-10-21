//! Executes the `input_2gb` guest on an input that fills the whole 2GB input region and checks
//! that it read the u64 it should.
//!
//! The first u64 of the input is the read stride `s` (0 is taken as 1): the guest reads every
//! `s`-th u64 plus the last one, and commits 4 u64: how many it read, their wrapping sum, and the
//! first and last u64 of the input. This host builds the input, computes the same 4 values and compares them with
//! the public output, on the backend given as first argument:
//!
//! ```bash
//! cargo run --release -p input-2gb-host -- emu            # Rust emulator, read every u64
//! cargo run --release -p input-2gb-host -- asm 2048 16    # assembly emulator, 2GB, stride 16
//! cargo run --release -p input-2gb-host -- asm 256        # assembly emulator, 256MB
//! ```
//!
//! Arguments: backend (`emu` | `asm`), input data size in MB (default: as large as the input
//! region allows, `MAX_DATA_BYTES`), and read stride (default 1). With the maximum size the input
//! data starts at INPUT_ADDR + 16 and ends at the last word of the region, so it exercises the
//! region boundaries and every 128MB step the BIOS reads.
//!
//! Reading every u64 of a 2GB input touches 2^28 distinct addresses, more than the CPU memory
//! planner of the assembly backend has slots for (`MAX_SLOT_GB` in
//! state-machines/mem-cpp/cpp/mem_config.hpp), and a stride that is a multiple of 4 words lands
//! every access on one of its counter threads and exhausts that thread's slots first; use a
//! stride of 17 or more there.

use std::error::Error;
use std::time::Instant;
use zisk_sdk::{ProverClient, ZiskStdin};
use zisk_test_artifacts::ELF_INPUT_2GB;

/// Size of the input region (`MAX_INPUT_SIZE` in core/src/mem.rs).
const MAX_INPUT_SIZE: usize = 2 << 30;
/// Input data the guest can read: the region minus the free-input word at INPUT_ADDR and the
/// length prefix of the input data.
const MAX_DATA_BYTES: usize = MAX_INPUT_SIZE - 16;

/// Value of the u64 at index `i` (but the first one, the stride): both 32-bit halves are used, so
/// the sum wraps around 2^64.
fn value(i: u64) -> u64 {
    (i << 32) | (i ^ 0x5A5A_5A5A)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let backend = args.get(1).map(String::as_str).unwrap_or("emu");
    let data_bytes = match args.get(2) {
        Some(mb) => (mb.parse::<usize>()? << 20).min(MAX_DATA_BYTES),
        None => MAX_DATA_BYTES,
    };
    // Written to the input as given; the guest takes 0 as 1, and so does the read model below
    let stride_arg: u64 = match args.get(3) {
        Some(s) => s.parse()?,
        None => 1,
    };
    let stride = stride_arg.max(1);
    let count = data_bytes / 8;
    if count < 2 {
        return Err("the input needs at least 2 u64".into());
    }

    println!("Building input: {count} u64 ({data_bytes} bytes), read stride {stride_arg}");
    let start = Instant::now();
    let mut data = Vec::with_capacity(count * 8);
    data.extend_from_slice(&stride_arg.to_le_bytes());
    for i in 1..count as u64 {
        data.extend_from_slice(&value(i).to_le_bytes());
    }
    let at = |i: u64| if i == 0 { stride_arg } else { value(i) };
    let last = count as u64 - 1;
    let (mut read, mut sum) = (0u64, 0u64);
    for i in (0..count as u64).step_by(stride as usize) {
        sum = sum.wrapping_add(at(i));
        read += 1;
    }
    if last % stride != 0 {
        sum = sum.wrapping_add(at(last));
        read += 1;
    }
    let expected = [read, sum, at(0), at(last)];
    println!("Input built in {:.1}s, expected {:x?}", start.elapsed().as_secs_f64(), expected);

    // write_slice prefixes the data with its length, as the guest's `read_slice` expects.
    let stdin = ZiskStdin::new();
    stdin.write_slice(&data);
    drop(data);

    let builder = ProverClient::embedded().execute_only();
    let builder = match backend {
        "emu" => builder.emulator(),
        "asm" => builder.assembly(),
        other => return Err(format!("unknown backend '{other}', expected emu or asm").into()),
    };
    let client = builder.build()?;
    client.setup(&ELF_INPUT_2GB, false)?;

    let start = Instant::now();
    let result = client.execute(&ELF_INPUT_2GB, stdin, None, None)?;
    println!(
        "Executed on {backend}: {} steps in {:.1}s",
        result.get_execution_steps(),
        start.elapsed().as_secs_f64()
    );

    let mut out = [0u8; 32];
    result.get_public_values_slice(&mut out);
    let got: Vec<u64> =
        out.chunks_exact(8).map(|c| u64::from_le_bytes(c.try_into().unwrap())).collect();
    println!("Public output {got:x?}");

    if got != expected {
        return Err(format!("mismatch: expected {expected:x?}, got {got:x?}").into());
    }
    println!("\u{2713} input_2gb OK on {backend}");
    Ok(())
}
