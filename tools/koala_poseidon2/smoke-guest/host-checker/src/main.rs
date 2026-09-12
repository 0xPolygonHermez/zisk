//! Runs a guest through the ASM executor and checks its committed publics byte for byte.

use anyhow::{ensure, Context, Result};
use proofman_common::VerboseMode;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{env, fs, path::PathBuf, time::Instant};
use zisk_common::io::{StreamSource, ZiskStdin};
use zisk_prover_backend::{AsmExecClient, GuestProgram};

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn check_hint_input(hints: &[u8], expected: &[u8], calls: usize) -> Result<()> {
    let mut offset = 0;
    let mut input = Vec::new();
    let mut koala_calls = 0;
    while offset < hints.len() {
        ensure!(hints.len() - offset >= 8, "truncated hint header");
        let length = u32::from_le_bytes(hints[offset..offset + 4].try_into()?) as usize;
        let code = u32::from_le_bytes(hints[offset + 4..offset + 8].try_into()?);
        offset += 8;
        let padded = length.div_ceil(8) * 8;
        ensure!(padded <= hints.len() - offset, "truncated hint payload");
        if code == zisk_definitions::HINT_INPUT {
            input.extend_from_slice(&hints[offset..offset + length]);
        }
        if code == zisk_definitions::HINT_KOALA_POSEIDON2 {
            ensure!(length == 64, "wrong Koala hint length");
            koala_calls += 1;
        }
        offset += padded;
    }
    ensure!(input == expected, "hint stream input differs from benchmark input");
    let expected_hints = if zisk_definitions::KOALA_POSEIDON2_RESULTS { calls } else { 0 };
    ensure!(koala_calls == expected_hints, "hint call count differs from independent profile");
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<_> = env::args().collect();
    ensure!(
        args.len() == 6 || args.len() == 7,
        "usage: koala-asm-output-checker ELF INPUT EXPECTED OUTPUT_DIR KOALA_CALLS [HINTS]"
    );
    let program = GuestProgram::from_uri(&args[1])?;
    let input = fs::read(&args[2])?;
    let expected = fs::read(&args[3])?;
    ensure!(expected.len() == 196, "expected exactly196 pessimistic public bytes");
    let output_dir = PathBuf::from(&args[4]);
    fs::create_dir_all(&output_dir)?;
    let calls: usize = args[5].parse()?;
    let capacity =
        zisk_pil::KoalaPoseidon2Trace::<()>::NUM_ROWS / zisk_precomp_koala_poseidon2::CLOCKS;
    let with_hints = args.len() == 7;
    if with_hints {
        check_hint_input(&fs::read(&args[6])?, &input, calls)?;
    }
    let client = AsmExecClient::new(VerboseMode::Info, Some(output_dir.join("cache")), false)?
        .with_unlock_mapped_memory(true);
    let start = Instant::now();
    client.setup(&program, with_hints)?;
    let setup_ms = start.elapsed().as_secs_f64() * 1_000.0;
    let mut runs = Vec::new();
    for run in 0..3 {
        let hints = if with_hints { Some(StreamSource::from_uri(&args[6])?) } else { None };
        let stdin = if with_hints { ZiskStdin::new() } else { ZiskStdin::from_vec(input.clone()) };
        let started = Instant::now();
        let result = client.execute(stdin, hints).with_context(|| format!("ASM run {run}"))?;
        let wall_ms = started.elapsed().as_secs_f64() * 1_000.0;
        let mut public_bytes = [0; 256];
        result.get_public_values_slice(&mut public_bytes);
        ensure!(public_bytes[..196] == expected, "public-value mismatch on run {run}");
        ensure!(public_bytes[196..].iter().all(|value| *value == 0), "nonzero public padding");
        let plan = result.get_plan().context("standalone plan missing")?;
        let koala_instances: usize = plan
            .iter()
            .filter(|entry| entry.name == "KoalaPoseidon2")
            .map(|entry| entry.count)
            .sum();
        ensure!(koala_instances == calls.div_ceil(capacity), "Koala AIR plan mismatch");
        fs::write(output_dir.join(format!("publics-{run}.bin")), public_bytes)?;
        runs.push(json!({
            "run": run,
            "wall_ms": wall_ms,
            "guest_steps": result.get_execution_steps(),
            "executor_ms": result.get_execution_time(),
            "timings": result.get_executor_time(),
            "publics_match": true,
            "plan": plan.iter().map(|entry| json!({
                "airgroup_id": entry.airgroup_id,
                "air_id": entry.air_id,
                "name": entry.name,
                "count": entry.count,
            })).collect::<Vec<_>>(),
        }));
    }
    let report = json!({
        "mode": if with_hints { "asm-with-hints" } else { "asm" },
        "proof_generated": false,
        "gpu": false,
        "host_checker_sha256": digest(&fs::read(env::current_exe()?)?),
        "elf_sha256": digest(program.elf()),
        "input_sha256": digest(&input),
        "expected_publics_sha256": digest(&expected),
        "hints_sha256": if with_hints { Some(digest(&fs::read(&args[6])?)) } else { None },
        "koala_calls_from_independent_profile": calls,
        "calls_per_air": capacity,
        "setup_ms": setup_ms,
        "runs": runs,
    });
    let bytes = serde_json::to_vec_pretty(&report)?;
    fs::write(output_dir.join("report.json"), &bytes)?;
    println!("{}", String::from_utf8(bytes)?);
    Ok(())
}
