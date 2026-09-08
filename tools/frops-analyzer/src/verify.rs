//! Cross-check of the FROPS multiplicity column: the table the ROM-histogram assembly builds against
//! the same column computed in Rust from an operation trace.
//!
//! The two are produced by completely independent code — the assembly by `zisk_core::frops_asm`, the
//! reference by `zisk_core::frops::FropsMultiplicity` — over the same execution, so they must agree
//! row by row. They are the only two producers of that column, and the proof's lookup argument only
//! balances if whichever one is used matches what the state machines claim.
//!
//! Inputs:
//! * the assembly dump, written by `ziskemuasm -s -f --gen=2` to `/tmp/<shm_prefix>_RH_output.bin`,
//! * the operation trace of the same ELF and input, from `ziskemu --store-op-output`.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use zisk_core::frops::{FropsMultiplicity, FROPS_TABLE_ROWS};

use crate::ingest::RECORD_SIZE;

/// The ROM-histogram output layout (see `get_rom_histogram_trace_address` in
/// `core/src/zisk_rom_2_asm.rs`): four control words, then each table preceded by its length.
struct AsmDump {
    steps: u64,
    exit_code: u64,
    inst_count: Vec<u64>,
    frops_count: Vec<u64>,
}

fn read_asm_dump(path: &Path) -> std::io::Result<AsmDump> {
    let bytes = std::fs::read(path)?;
    let words: Vec<u64> =
        bytes.chunks_exact(8).map(|c| u64::from_le_bytes(c.try_into().unwrap())).collect();
    let bad = |m: String| std::io::Error::new(std::io::ErrorKind::InvalidData, m);
    if words.len() < 6 {
        return Err(bad(format!("{} is too short to be a ROM histogram dump", path.display())));
    }
    let (exit_code, steps, rom_len) = (words[1], words[3], words[4] as usize);
    let frops_at = 5 + rom_len;
    if words.len() <= frops_at {
        return Err(bad(format!(
            "dump holds {} words, too few for {rom_len} instruction counters",
            words.len()
        )));
    }
    let frops_len = words[frops_at] as usize;
    if words.len() < frops_at + 1 + frops_len {
        return Err(bad(format!(
            "dump holds {} words, too few for {rom_len} instruction and {frops_len} FROPS counters",
            words.len()
        )));
    }
    Ok(AsmDump {
        steps,
        exit_code,
        inst_count: words[5..5 + rom_len].to_vec(),
        frops_count: words[frops_at + 1..frops_at + 1 + frops_len].to_vec(),
    })
}

/// Accumulates the reference column over every 17-byte record of the traces.
fn reference_column(traces: &[PathBuf]) -> std::io::Result<(FropsMultiplicity, u64)> {
    let mut mult = FropsMultiplicity::new();
    let mut ops = 0u64;
    for path in traces {
        let mut reader = BufReader::with_capacity(1 << 20, File::open(path)?);
        let mut buf = vec![0u8; RECORD_SIZE * 4096];
        loop {
            let mut filled = 0;
            while filled < buf.len() {
                match reader.read(&mut buf[filled..])? {
                    0 => break,
                    n => filled += n,
                }
            }
            if filled == 0 {
                break;
            }
            if filled % RECORD_SIZE != 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "{} is not a whole number of {RECORD_SIZE}-byte records",
                        path.display()
                    ),
                ));
            }
            for rec in buf[..filled].chunks_exact(RECORD_SIZE) {
                let a = u64::from_le_bytes(rec[1..9].try_into().unwrap());
                let b = u64::from_le_bytes(rec[9..17].try_into().unwrap());
                mult.count(rec[0], a, b);
                ops += 1;
            }
            if filled < buf.len() {
                break;
            }
        }
    }
    Ok((mult, ops))
}

/// Compares the two columns and reports. Returns false when they disagree.
pub fn verify(asm_dump: &Path, traces: &[PathBuf]) -> std::io::Result<bool> {
    let dump = read_asm_dump(asm_dump)?;
    println!("assembly dump {}", asm_dump.display());
    println!("  exit code        {}", dump.exit_code);
    println!("  steps            {}", dump.steps);
    println!(
        "  ROM counters     {} (sum {})",
        dump.inst_count.len(),
        dump.inst_count.iter().sum::<u64>()
    );
    println!("  FROPS counters   {}", dump.frops_count.len());

    let (mult, ops) = reference_column(traces)?;
    println!("reference column from {} trace file(s)", traces.len());
    println!("  operations read  {ops}");
    println!("  frequent ops     {}", mult.counted());

    let mut ok = true;
    if dump.frops_count.len() as u64 != FROPS_TABLE_ROWS {
        println!(
            "MISMATCH: the assembly counted {} rows, the in-tree table has {FROPS_TABLE_ROWS}; \
             the assembly was generated from different FROPS data",
            dump.frops_count.len()
        );
        return Ok(false);
    }
    let asm_total: u64 = dump.frops_count.iter().sum();
    if asm_total != mult.counted() {
        println!(
            "MISMATCH: assembly counted {asm_total} frequent operations, reference {}",
            mult.counted()
        );
        ok = false;
    }

    let mut diffs = 0u64;
    let mut shown = 0;
    for (row, (&asm, &reference)) in dump.frops_count.iter().zip(mult.rows()).enumerate() {
        if asm != reference {
            diffs += 1;
            if shown < 20 {
                println!("  row {row}: assembly {asm}, reference {reference}");
                shown += 1;
            }
        }
    }
    if diffs > 0 {
        println!("MISMATCH: {diffs} of {FROPS_TABLE_ROWS} rows differ");
        ok = false;
    }
    if ok {
        println!(
            "OK: both columns agree on all {FROPS_TABLE_ROWS} rows ({} counted operations)",
            mult.counted()
        );
    }
    Ok(ok)
}
