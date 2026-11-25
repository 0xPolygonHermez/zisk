//! `zisk-estimate-measure` (prototype): time the two proving phases of one worker of a job of W
//! workers, without a coordinator.
//!
//! One prover is set up once and plays worker k of W for every (input, W, k): it takes its
//! partition of the instances (`set_partition`) and runs the contributions phase and the proofs
//! phase, the calls a distributed worker makes (`distributed/crates/worker/src/worker.rs`:
//! `prepare_for_new_job`, `execute_contribution_task`, `execute_prove_task`). As in the
//! coordinator's simulation mode, the worker's own contribution stands in for everybody's and
//! there is no aggregation.

use anyhow::{Context, Result};
use clap::Parser;
use proofman::{ProvePhase, ProvePhaseInputs, ProvePhaseResult};
use proofman_common::ProofOptions;
use std::path::PathBuf;
use std::time::Instant;
use zisk_common::io::ZiskStdin;
use zisk_common::RandomSeed;
use zisk_estimate::files::{input_files, write_json};
use zisk_estimate::fit::Record;
use zisk_prover_backend::{AsmOptions, BackendProverOpts, GuestProgram, ProverClientBuilder};

#[derive(Parser)]
#[command(
    name = "zisk-estimate-measure",
    version,
    about = "Time the proving phases of one worker of a job of W workers"
)]
struct Args {
    /// Guest program (RISC-V ELF)
    #[arg(short = 'e', long)]
    elf: PathBuf,

    /// Directory of inputs; every file is measured
    #[arg(short = 'i', long)]
    inputs: PathBuf,

    /// Proving key
    #[arg(short = 'k', long)]
    proving_key: PathBuf,

    /// Cluster sizes to measure: every worker of each
    #[arg(long, value_delimiter = ',', default_value = "1,2,4")]
    workers: Vec<usize>,

    /// Measure only the first N inputs (sorted by name)
    #[arg(long)]
    max_blocks: Option<usize>,

    /// Unrecorded jobs on the first input before measuring
    #[arg(long, default_value_t = 1)]
    warmup: usize,

    /// Prove on the GPU
    #[arg(short = 'g', long)]
    gpu: bool,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Where the records go (JSON array, rewritten after every input)
    #[arg(long)]
    out: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut files = input_files(&args.inputs)?;
    files.truncate(args.max_blocks.unwrap_or(usize::MAX));

    // The worker's prover (Worker::new_asm).
    let mut opts = BackendProverOpts::default()
        .proving_key(args.proving_key.clone())
        .verbose(args.verbose)
        .aggregation(true);
    if args.gpu {
        opts = opts.gpu();
    }
    let opts = opts.with_asm_options(AsmOptions::default().is_distributed());
    let prover = ProverClientBuilder::new().asm().prove().with_prover_options(opts).build()?;
    let program = GuestProgram::from_uri(args.elf.to_str().context("the ELF path is not UTF-8")?)?;
    prover.setup(&program).run()?;

    // The worker's proof options (Worker::get_prove_options).
    let options = || ProofOptions {
        verify_constraints: false,
        aggregation: true,
        verify_proofs: false,
        rma: true,
        minimal_memory: false,
        compressed: false,
    };

    let run = |file: &PathBuf, workers: usize, node: usize| -> Result<Record> {
        let stdin = ZiskStdin::from_file(file)?;
        let seed = RandomSeed::random()?;
        prover.wait_until_proofman_ready();
        let job = prover.begin_job_for(&program.program_id, false)?;
        prover.set_active_services(true)?;

        let t1 = Instant::now();
        prover.set_stdin_and_random_seed(stdin, seed)?;
        // one compute unit per worker: worker k owns unit k
        prover.set_partition(workers, vec![node as u32], 0)?;
        let challenges = match prover.prove_phase(
            ProvePhaseInputs::Contributions(),
            options(),
            ProvePhase::Contributions,
        )? {
            ProvePhaseResult::Contributions(c) => c,
            _ => anyhow::bail!("the contributions phase returned something else"),
        };
        let phase1_s = t1.elapsed().as_secs_f64();
        let execute_s = prover.get_executor_time()?.total_duration as f64 / 1000.0;

        let t2 = Instant::now();
        match prover.prove_phase(
            ProvePhaseInputs::Internal(challenges),
            options(),
            ProvePhase::Internal,
        )? {
            ProvePhaseResult::Internal(_) => {}
            _ => anyhow::bail!("the proofs phase returned something else"),
        }
        let proofs_s = t2.elapsed().as_secs_f64();
        drop(job);

        let input = file.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        Ok(Record { input, workers, node, phase1_s, proofs_s, execute_s, ..Default::default() })
    };

    for _ in 0..args.warmup {
        run(&files[0], 1, 0)?;
    }
    let mut records = Vec::new();
    for file in &files {
        for &workers in &args.workers {
            for node in 0..workers {
                let r = run(file, workers, node)?;
                println!(
                    "{:<44} W={} k={}  phase 1 {:.3}s  proofs {:.3}s  exec {:.3}s",
                    r.input, r.workers, r.node, r.phase1_s, r.proofs_s, r.execute_s
                );
                records.push(r);
            }
        }
        write_json(&args.out, &records)?;
    }
    Ok(())
}
