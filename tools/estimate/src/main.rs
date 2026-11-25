//! `zisk-estimate`: predict the proof time of a block from one execution of it.
//!
//! Runs the executor standalone (execution + count-and-plan: no witness, no proving key) and costs
//! the planned instances with the model of this crate, for the local prover or, with
//! `--workers W`, for a coordinator job on W workers.
//!
//! `-i <dir>` estimates every file of the directory on one client: the ASM services and the
//! planner arena are set up once, so each input costs its execution, not the 5 s startup.

use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tracing::{info, warn};
use zisk_common::io::ZiskStdin;
use zisk_estimate::files::{input_files, write_json};
use zisk_estimate::model::{estimate, sort_by_planning_order, AirLine, Coeffs, Estimate};
use zisk_prover_backend::{
    AsmOptions, BackendProverOpts, ExecuteClient, ExecuteOutput, GuestProgram, ProverClientBuilder,
};

/// One input of a directory run.
#[derive(Clone, Debug, Serialize)]
struct BatchEntry {
    pub input: String,
    /// Wall time of this input in the tool (execution + estimate), ms.
    pub cost_ms: u64,
    #[serde(flatten)]
    pub est: Estimate,
}

/// How an input is costed: on the local prover, or as a coordinator job on `workers` workers.
#[derive(Clone, Copy, Debug)]
struct Target {
    pub workers: Option<usize>,
    /// The job's final proof is not compressed.
    pub plain_final: bool,
    /// Executions per input; the fastest one is the model's `exec`.
    pub exec_runs: usize,
}

/// Executes `input` on a set-up client and costs its plan. The model takes the execution time as
/// the block's emulation, so with several runs the fastest counts: a slow outlier would go
/// straight into the estimate.
fn execute_and_estimate(
    client: &Client,
    input: Option<&String>,
    coeffs: &Coeffs,
    target: Target,
) -> Result<(Estimate, ExecuteOutput)> {
    let exec_s = |r: &ExecuteOutput| r.get_executor_time().total_duration as f64 / 1000.0;
    let mut best: Option<ExecuteOutput> = None;
    for _ in 0..target.exec_runs.max(1) {
        let result = client.execute(input)?;
        let faster = match &best {
            Some(b) => exec_s(&result) < exec_s(b),
            None => true,
        };
        if faster {
            best = Some(result);
        }
    }
    let result = best.expect("at least one execution");
    let plan = plan_of(&result)?;
    let mut est = estimate(&plan, result.get_execution_steps(), exec_s(&result), coeffs);
    est.input_mb =
        input.and_then(|i| std::fs::metadata(i).ok()).map(|m| m.len() as f64 / 1e6).unwrap_or(0.0);
    if let Some(w) = target.workers {
        est = est.on_cluster(w, coeffs, target.plain_final);
    }
    Ok((est, result))
}

/// Executes and estimates `files` on a set-up client, one log line each; returns the estimates
/// and the names of the inputs that failed.
fn estimate_inputs(
    client: &Client,
    files: &[PathBuf],
    coeffs: &Coeffs,
    target: Target,
) -> (Vec<BatchEntry>, Vec<String>) {
    let mut entries: Vec<BatchEntry> = Vec::with_capacity(files.len());
    let mut failed: Vec<String> = Vec::new();
    for f in files {
        let name = f.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let t0 = std::time::Instant::now();
        let uri = f.display().to_string();
        let est = match execute_and_estimate(client, Some(&uri), coeffs, target) {
            Ok((e, _)) => e,
            Err(e) => {
                warn!("{name}: {e:#}");
                failed.push(name);
                continue;
            }
        };
        let cost_ms = t0.elapsed().as_millis() as u64;
        info!(
            "{:<44} {:>5} {:>7.2}s {:>8.2}s {:>5}ms",
            name, est.instances, est.exec_s, est.total_s, cost_ms
        );
        entries.push(BatchEntry { input: name, cost_ms, est });
    }
    (entries, failed)
}

/// The planned (AIR, instances) of an execution, in the executor's planning order.
fn plan_of(result: &ExecuteOutput) -> Result<Vec<(String, usize)>> {
    let plan = result.get_plan().context("the executor returned no plan")?;
    let mut plan: Vec<(String, usize)> =
        plan.iter().map(|e| (e.name.to_string(), e.count)).collect();
    sort_by_planning_order(&mut plan);
    Ok(plan)
}

#[derive(Parser)]
#[command(
    name = "zisk-estimate",
    version,
    about = "Predict the proof time of a block from one execution of it"
)]
struct Args {
    /// Guest program (RISC-V ELF)
    #[arg(short = 'e', long)]
    elf: PathBuf,

    /// Input of the guest: a file, or a directory to estimate every file of
    #[arg(short = 'i', long)]
    inputs: Option<String>,

    /// Use the ASM emulator instead of the Rust emulator
    #[arg(short = 'a', long)]
    asm: bool,

    /// Run the memory-ops count-and-plan on the GPU
    #[arg(short = 'g', long)]
    gpu: bool,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Coefficient file (JSON, see src/coeffs.json). Defaults to the embedded table.
    #[arg(long)]
    coeffs: Option<PathBuf>,

    /// Write the estimate as JSON to this file (the log keeps the human table); an array of
    /// per-input estimates when `-i` is a directory.
    #[arg(long)]
    json_out: Option<PathBuf>,

    /// Estimate the proof as a coordinator job on this many workers (the `cluster` model)
    /// instead of the local single-GPU prover.
    #[arg(long)]
    workers: Option<usize>,

    /// The job's final proof is the plain one, not the compressed one (`--workers`).
    #[arg(long, requires = "workers")]
    plain_final: bool,

    /// Execute each input this many times and take the fastest as its execution time.
    #[arg(long, default_value_t = 1)]
    exec_runs: usize,
}

/// A set-up standalone executor: execution + count-and-plan, no witness and no proving key.
struct Client {
    prover: Box<dyn ExecuteClient>,
    guest: GuestProgram,
}

impl Client {
    /// Sets the program up once; the ASM services and the planner arena then stay alive across
    /// executions.
    fn new(args: &Args) -> Result<Self> {
        let mut opts = BackendProverOpts::default().verbose(args.verbose);
        if args.gpu {
            opts = opts.gpu();
        }
        let guest =
            GuestProgram::from_uri(args.elf.to_str().context("the ELF path is not UTF-8")?)?;
        let prover: Box<dyn ExecuteClient> = if args.asm && !cfg!(target_os = "macos") {
            Box::new(
                ProverClientBuilder::new()
                    .asm()
                    .with_prover_options(opts.with_asm_options(AsmOptions::default()))
                    .execute_only()
                    .build()?,
            )
        } else {
            Box::new(
                ProverClientBuilder::new()
                    .emu()
                    .with_prover_options(opts)
                    .execute_only()
                    .build()?,
            )
        };
        prover.setup(&guest, false)?;
        Ok(Client { prover, guest })
    }

    fn execute(&self, input: Option<&String>) -> Result<ExecuteOutput> {
        self.prover.execute(&self.guest, ZiskStdin::from_uri(input)?, None, None)
    }
}

fn banner_field(label: &str, value: impl std::fmt::Display) {
    println!("{} {}", format!("{label: >12}").bright_green().bold(), value);
}

/// What the last phase is: several workers fold their proofs before the final one.
fn last_phase(est: &Estimate) -> &'static str {
    if est.workers > 1 {
        "aggregation"
    } else {
        "final"
    }
}

/// The default report: the plan size and the three phases. `-v` prints the per-AIR table and
/// the terms of each phase instead.
fn print_summary(est: &Estimate, workers: Option<usize>) {
    info!("Instances: {}  steps: {}", est.instances, est.steps);
    info!("Execution {:.2}s", est.exec_s);
    if workers.is_some() {
        info!(
            "Workers: {} instances",
            est.workers_load
                .iter()
                .map(|l| format!("[{}]", l.instances))
                .collect::<Vec<_>>()
                .join(" ")
        );
    }
    info!("{:<26} {:>6.2}s", "execution + contributions", est.phase_s);
    info!("{:<26} {:>6.2}s", "proofs", est.proofs_s);
    info!("{:<26} {:>6.2}s", last_phase(est), est.final_s);
    if est.overhead_s != 0.0 {
        info!("{:<26} {:>6.2}s", "overhead", est.overhead_s);
    }
    info!(
        "{} {}{}",
        "Estimated proof time:".bright_white().bold(),
        format!("{:.2}s", est.total_s).bright_green().bold(),
        workers.map(|w| format!("  ({w} workers)")).unwrap_or_default()
    );
}

/// The verbose report: the per-AIR table and how the estimate is composed.
fn print_details(
    est: &Estimate,
    result: &ExecuteOutput,
    coeffs: &Coeffs,
    workers: Option<usize>,
) -> Result<()> {
    let t = result.get_executor_time();
    info!(
        "{:<24} {:>5} {:>8} {:>8} {:>8}  {}",
        "air".bold(),
        "inst".bold(),
        "basic".bold(),
        "recurs".bold(),
        "total".bold(),
        "coeff row".dimmed()
    );
    // costliest first; `lines` itself stays in registration order, which the placement follows
    let mut table: Vec<&AirLine> = est.lines.iter().collect();
    table.sort_by(|a, b| {
        (b.basic_s + b.recursion_s).partial_cmp(&(a.basic_s + a.recursion_s)).unwrap()
    });
    for l in table {
        info!(
            "{:<24} {:>5} {:>7.2}s {:>7.2}s {:>7.2}s  {}",
            l.air,
            l.instances,
            l.basic_s,
            l.recursion_s,
            l.basic_s + l.recursion_s,
            if l.source == l.air { String::new() } else { l.source.dimmed().to_string() }
        );
    }
    info!(
        "Instances: {}  steps: {}  sum basic+recursion {:.2}s",
        est.instances, est.steps, est.sum_proofs_raw_s
    );
    if !est.unknown_airs.is_empty() {
        info!("{} {}", "AIRs costed with the default row:".yellow(), est.unknown_airs.join(", "));
    }
    info!(
        "Execution {:.2}s (emulation {}ms, count-and-plan {}ms, mops {}ms)",
        est.exec_s, t.execution_duration, t.count_and_plan_duration, t.count_and_plan_mo_duration
    );
    if let Some(w) = workers {
        let cm = coeffs.cluster_model()?;
        info!(
            "Workers (instances, end of phase 1, proofs): {}",
            est.workers_load
                .iter()
                .map(|l| format!("[{} {:.2}s {:.2}s]", l.instances, l.phase1_s, l.ready_s))
                .collect::<Vec<_>>()
                .join(" ")
        );
        info!(
            "{} {} workers: execution+contributions {:.2} + proofs {:.2} + {} {:.2} = {}  (each phase: expected slowest worker, noise {:.0} / {:.0} ms; input {:.2} MB)",
            "Estimated proof time:".bright_white().bold(),
            w,
            est.phase_s,
            est.proofs_s,
            last_phase(est),
            est.final_s,
            format!("{:.2}s", est.total_s).bright_green().bold(),
            1e3 * cm.phase_sigma_s,
            1e3 * cm.proofs_sigma_s,
            est.input_mb
        );
    } else {
        info!(
            "{} execution+contributions {:.2} ({}: {:.4} ms x {:.0} commit work + {:.2}) + proofs {:.2} (= {:.2} x {:.2} + {:.2}) + final {:.2} + overhead {:.2} = {}",
            "Estimated proof time:".bright_white().bold(),
            est.phase_s,
            if est.guard_bound { "emulation guard" } else { "commit term" },
            coeffs.model.commit_ms_per_work, est.commit_work, coeffs.model.phase_const_s,
            est.proofs_s, coeffs.model.proofs_factor, est.sum_proofs_raw_s, coeffs.model.proofs_const_s,
            est.final_s, est.overhead_s,
            format!("{:.2}s", est.total_s).bright_green().bold()
        );
    }
    Ok(())
}

impl Args {
    fn run(&self) -> Result<()> {
        let coeffs = match (&self.coeffs, self.workers) {
            (Some(p), _) => Coeffs::from_file(p)?,
            (None, Some(_)) => Coeffs::embedded_cluster()?,
            (None, None) => Coeffs::embedded()?,
        };
        if let Some(w) = self.workers {
            anyhow::ensure!(w >= 1, "--workers must be at least 1");
            let cm = coeffs.cluster_model()?;
            anyhow::ensure!(
                !self.plain_final || cm.final_plain_s.is_some(),
                "the coefficient file has no `final_plain_s` (--plain-final)"
            );
        }
        anyhow::ensure!(self.exec_runs >= 1, "--exec-runs must be at least 1");
        let target = Target {
            workers: self.workers,
            plain_final: self.plain_final,
            exec_runs: self.exec_runs,
        };

        banner_field("Program", self.elf.display());
        banner_field("Input", self.inputs.as_deref().unwrap_or("None"));
        banner_field(
            "Coeffs",
            self.coeffs
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "embedded".to_string()),
        );
        banner_field("Setup", &coeffs.setup);
        banner_field("GPU", &coeffs.gpu);
        banner_field(
            "Workers",
            self.workers.map(|w| format!("{w} (cluster)")).unwrap_or("local".into()),
        );

        if let Some(dir) = self.inputs.as_ref().map(PathBuf::from).filter(|p| p.is_dir()) {
            return self.run_batch(&dir, &coeffs, target);
        }

        let tool_start = std::time::Instant::now();
        let client = Client::new(self)?;
        let (est, result) = execute_and_estimate(&client, self.inputs.as_ref(), &coeffs, target)?;
        let tool_ms = tool_start.elapsed().as_millis() as u64;
        info!("{}", "--- ESTIMATE ------------------".bright_green().bold());
        if self.verbose == 0 {
            print_summary(&est, self.workers);
        } else {
            print_details(&est, &result, &coeffs, self.workers)?;
            info!("Estimator cost: {}ms (client setup + {} execution(s))", tool_ms, self.exec_runs);
        }
        if let Some(path) = &self.json_out {
            write_json(path, &est)?;
        }
        Ok(())
    }

    /// Every file of `dir`, sorted by name, on one set-up client; one line per input.
    fn run_batch(&self, dir: &Path, coeffs: &Coeffs, target: Target) -> Result<()> {
        let files = input_files(dir)?;

        let tool_start = std::time::Instant::now();
        let client = Client::new(self)?;
        let setup_ms = tool_start.elapsed().as_millis() as u64;

        info!("{}", "--- ESTIMATE (directory) ------".bright_green().bold());
        info!(
            "{:<44} {:>5} {:>8} {:>9} {:>7}",
            "input".bold(),
            "inst".bold(),
            "exec".bold(),
            "estimate".bold(),
            "cost".bold()
        );
        let (entries, failed) = estimate_inputs(&client, &files, coeffs, target);

        let mut costs: Vec<u64> = entries.iter().map(|e| e.cost_ms).collect();
        costs.sort_unstable();
        if !costs.is_empty() {
            info!(
                "{} {} inputs; client setup {}ms, per input median {}ms (min {}, max {}), total {}ms",
                "Estimator cost:".bright_white().bold(),
                costs.len(),
                setup_ms,
                costs[costs.len() / 2],
                costs[0],
                costs[costs.len() - 1],
                tool_start.elapsed().as_millis()
            );
        }
        if !failed.is_empty() {
            warn!("{} inputs failed: {}", failed.len(), failed.join(", "));
        }
        if let Some(path) = &self.json_out {
            write_json(path, &entries)?;
        }
        anyhow::ensure!(failed.is_empty(), "{} of {} inputs failed", failed.len(), files.len());
        Ok(())
    }
}

fn main() -> Result<()> {
    Args::parse().run()
}
