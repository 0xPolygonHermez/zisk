use anyhow::{ensure, Context, Result};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{env, fs, path::PathBuf, time::Instant};
use zisk_common::{program_publics, PlonkVkey, Proof, ProofBody, VadcopKind};
use zisk_sdk::{AsmOptions, GuestProgram, ProofKind, ProverClient, ZiskHints, ZiskStdin};

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn main() -> Result<()> {
    let args: Vec<_> = env::args().collect();
    ensure!(
        args.len() == 9 || args.len() == 10,
        "usage: gpu-proof ELF INPUT EXPECTED KEY SNARK_KEY OUTPUT_DIR KIND RUNS|verify [HINTS]"
    );
    let kind = match args[7].as_str() {
        "plonk" => ProofKind::Plonk,
        "vadcop" => ProofKind::VadcopFinal,
        _ => anyhow::bail!("KIND must be plonk or vadcop"),
    };
    let verify_only = args[8] == "verify";
    let count: usize = if verify_only { 1 } else { args[8].parse()? };
    ensure!(count > 0, "RUNS must be positive");
    let program = GuestProgram::from_uri(&args[1])?;
    let input = fs::read(&args[2])?;
    let expected = fs::read(&args[3])?;
    ensure!(!expected.is_empty() && expected.len() <= 256, "expected 1–256 public bytes");
    let directory = PathBuf::from(&args[6]);
    if !verify_only {
        fs::create_dir(&directory).context("OUTPUT_DIR must be new")?;
    }
    let key = PathBuf::from(&args[4]);
    let global: serde_json::Value =
        serde_json::from_slice(&fs::read(key.join("pilout.globalInfo.json"))?)?;
    let name = global["name"].as_str().context("missing circuit name")?;
    let root_bytes = fs::read(key.join(name).join("vadcop_final/vadcop_final.verkey.json"))?;
    let root: Vec<u64> = serde_json::from_slice(&root_bytes)?;
    ensure!(root.len() == 4, "expected four circuit-root words");
    let plonk_vk = if kind == ProofKind::Plonk {
        Some(PlonkVkey::load(PathBuf::from(&args[5]).join("final/final.verkey.json"))?)
    } else {
        None
    };
    let mut builder = ProverClient::embedded()
        .assembly()
        .gpu()
        .proving_key(&args[4])
        .asm_options(AsmOptions::default().unlock_mapped_memory());
    if kind == ProofKind::Plonk {
        builder = builder.plonk().proving_key_plonk(&args[5]);
    }
    let started = Instant::now();
    let client = builder.build()?;
    let initialization_s = started.elapsed().as_secs_f64();
    let started = Instant::now();
    let mut setup = client.setup(&program);
    if args.len() == 10 {
        setup = setup.with_hints();
    }
    setup.run_sync()?;
    let setup_s = started.elapsed().as_secs_f64();
    let program_vk = program.vk()?;
    let mut padded = [0_u8; 256];
    padded[..expected.len()].copy_from_slice(&expected);
    let mut expected_fields = program_vk.vk.clone();
    expected_fields.extend(
        padded
            .chunks_exact(4)
            .map(|bytes| u64::from(u32::from_le_bytes(bytes.try_into().unwrap()))),
    );
    let mut report = json!({
        "gpu_requested": true, "backend": "asm", "proof_kind": args[7],
        "elf_sha256": hash(program.elf()), "input_sha256": hash(&input),
        "expected_publics_sha256": hash(&expected),
        "expected_publics_len": expected.len(),
        "hints_sha256": if args.len() == 10 { Some(hash(&fs::read(&args[9])?)) } else { None },
        "binary_sha256": hash(&fs::read(env::current_exe()?)?),
        "circuit_root_sha256": hash(&root_bytes),
        "initialization_s": initialization_s, "setup_s": setup_s, "runs": [],
    });
    let report_path =
        directory.join(if verify_only { "verification-report.json" } else { "report.json" });
    fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;
    for run in 1..=count {
        let stdin = if args.len() == 10 {
            ZiskStdin::new()
        } else {
            let stdin = ZiskStdin::from_file(&args[2])?;
            ensure!(stdin.read_data() == input, "framed input changed before proving");
            stdin
        };
        let path = directory.join(format!("run-{run}.proof"));
        let (proof, proving_s, sdk_proving_ms, guest_steps) = if verify_only {
            (Proof::load(&path)?, None, None, None)
        } else {
            let mut request = client.prove(&program, stdin).wrap(kind);
            if args.len() == 10 {
                request = request.hints(ZiskHints::from_uri(&args[9])?);
            }
            let started = Instant::now();
            let result = request.run_sync()?;
            let elapsed = started.elapsed().as_secs_f64();
            result.save_proof(&path)?;
            let measurement = json!({
                "run": run, "proving_s": elapsed,
                "sdk_proving_ms": result.get_proving_time(),
                "guest_steps": result.get_execution_steps(), "verified": false,
            });
            fs::write(
                directory.join(format!("run-{run}.timing.json")),
                serde_json::to_vec_pretty(&measurement)?,
            )?;
            (
                result.get_proof().clone(),
                Some(elapsed),
                Some(result.get_proving_time()),
                Some(result.get_execution_steps()),
            )
        };
        ensure!(proof.kind() == kind, "wrong proof kind");
        let started = Instant::now();
        match &proof.body {
            ProofBody::Vadcop { zisk_vk, publics_full, kind, .. } => {
                ensure!(*kind == VadcopKind::Final, "wrong recursive layer");
                ensure!(*zisk_vk == root, "wrong circuit root");
                ensure!(*publics_full == expected_fields, "committed program/publics mismatch");
            }
            ProofBody::Plonk { plonk_vk: actual_vk, publics_full, rootc, .. } => {
                ensure!(*rootc == root && actual_vk.vadcop_vk == root, "wrong circuit root");
                ensure!(
                    publics_full.len() == expected_fields.len()
                        || (publics_full.len() == expected_fields.len() + 1
                            && publics_full[0] == 1),
                    "unexpected PLONK public layout"
                );
                ensure!(
                    program_publics(publics_full) == expected_fields,
                    "committed program/publics mismatch"
                );
                ensure!(
                    serde_json::to_value(&actual_vk.plonk_vkey)?
                        == serde_json::to_value(
                            plonk_vk.as_ref().context("missing trusted PLONK key")?
                        )?,
                    "wrong PLONK verification key"
                );
            }
        }
        proof.with_program_vk(&program_vk).verify()?;
        let verification_s = started.elapsed().as_secs_f64();
        let mut actual = [0; 256];
        proof.get_publics().read_slice(&mut actual);
        ensure!(actual[..expected.len()] == expected, "public output mismatch");
        ensure!(actual[expected.len()..].iter().all(|byte| *byte == 0), "nonzero public padding");
        let measurement = json!({
            "run": run, "proving_s": proving_s, "sdk_proving_ms": sdk_proving_ms,
            "verification_s": verification_s, "guest_steps": guest_steps,
            "proof_bytes": fs::metadata(&path)?.len(), "proof_path": path,
            "publics_match": true, "program_bound_verification": true,
        });
        println!("{measurement}");
        report["runs"].as_array_mut().unwrap().push(measurement);
        fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;
    }
    Ok(())
}
