//! `zisk-estimate-fit`: the coefficients of the model from measurements.
//!
//!   zisk-estimate -e <elf> -i <dir> -a -g --json-out blocks.json            (what the tool sees)
//!   zisk-estimate-measure -e <elf> -i <dir> -k <key> -g --out workers.json   (what a worker takes)
//!   zisk-estimate-fit --estimates blocks.json --measurements workers.json --out coeffs.json
//!
//! Every `--measurements` file is a corpus and the corpora weigh the same. `--cluster-jobs` are
//! whole 1-worker jobs of a cluster (phase durations from its coordinator's journal): they set
//! the constants that belong to that cluster, c1, f and F1.

use anyhow::Result;
use clap::Parser;
use std::collections::BTreeMap;
use std::path::PathBuf;
use zisk_estimate::files::{read_json, write_json};
use zisk_estimate::fit::{calibrate_cluster, fit, Block, Options, Record};
use zisk_estimate::model::Coeffs;

#[derive(Parser)]
#[command(
    name = "zisk-estimate-fit",
    version,
    about = "Fit the coefficients of the proof-time model from measurements"
)]
struct Args {
    /// Blocks as the tool sees them: `zisk-estimate -i <dir> --json-out` files
    #[arg(long, required = true)]
    estimates: Vec<PathBuf>,

    /// Workers measured on one GPU: `zisk-estimate-measure --out` files, one corpus each
    #[arg(long, required = true)]
    measurements: Vec<PathBuf>,

    /// Whole 1-worker jobs of the cluster to predict (input, phase1_s, proofs_s, final_s)
    #[arg(long)]
    cluster_jobs: Option<PathBuf>,

    /// Prior coefficients: the prices the fit is pulled towards and everything it does not fit.
    /// Defaults to the embedded table.
    #[arg(long)]
    coeffs: Option<PathBuf>,

    /// Strength of the pull towards the prior prices
    #[arg(long, default_value_t = 1.0)]
    ridge: f64,

    /// Hold the constant of the proofs phase (c2) at this value, ms, instead of fitting it
    #[arg(long, allow_hyphen_values = true)]
    proofs_const_ms: Option<f64>,

    /// Fit phase 1 only: the proofs phase stays as the prior has it
    #[arg(long, conflicts_with = "proofs_const_ms")]
    keep_proofs: bool,

    /// Job size whose workers give the noise of each phase
    #[arg(long, default_value_t = 4)]
    sigma_workers: usize,

    /// Where the fitted coefficients go
    #[arg(long)]
    out: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let prior = match &args.coeffs {
        Some(p) => Coeffs::from_file(p)?,
        None => Coeffs::embedded()?,
    };
    let mut blocks: BTreeMap<String, Block> = BTreeMap::new();
    for p in &args.estimates {
        for b in read_json::<Vec<Block>>(p)? {
            blocks.insert(b.input.clone(), b);
        }
    }
    let corpora: Vec<Vec<Record>> =
        args.measurements.iter().map(|p| read_json(p)).collect::<Result<_>>()?;

    let options = Options {
        ridge: args.ridge,
        sigma_workers: args.sigma_workers,
        proofs_const: args.proofs_const_ms.map(|ms| ms / 1e3),
        keep_proofs: args.keep_proofs,
    };
    let f = fit(&corpora, &blocks, &prior, options)?;
    let mut coeffs = f.apply(&prior);

    println!(
        "phase 1   c {:.1} ms  rho {:.4}  tau {:.3} ms  kappa {:.1} ms  sigma1 {:.1} ms",
        1e3 * f.phase_const_s,
        f.exec_ratio,
        1e3 * f.planning_s,
        1e3 * f.keccakf_saving_s,
        1e3 * f.phase_sigma_s
    );
    println!(
        "proofs    c2 {:.1} ms  sigma2 {:.1} ms",
        1e3 * f.proofs_const_s,
        1e3 * f.proofs_sigma_s
    );
    println!("prices that moved 3 ms or more (prior -> fitted, ms):");
    for (air, new) in &coeffs.airs {
        let Some(old) = prior.airs.get(air) else { continue };
        let (o1, n1) = (old.phase_ms, new.phase_ms);
        let (o2, n2) = (old.basic_ms + old.recursion_ms, new.basic_ms + new.recursion_ms);
        if (n1 - o1).abs() >= 3.0 || (n2 - o2).abs() >= 3.0 {
            println!("  {air:<24} t1 {o1:7.1} -> {n1:7.1}   t2 {o2:7.1} -> {n2:7.1}");
        }
    }
    println!("error of the fit, model - measured: median and median |.|, ms");
    for (corpus, workers, n, e) in &f.errors {
        println!(
            "  {:<28} W={workers:<2} n={n:<5} phase 1 {:+6.1} {:5.1}   proofs {:+6.1} {:5.1}",
            args.measurements[*corpus].file_name().map(|s| s.to_string_lossy()).unwrap_or_default(),
            e[0],
            e[1],
            e[2],
            e[3]
        );
    }
    if let Some(p) = &args.cluster_jobs {
        let jobs: Vec<Record> = read_json(p)?;
        let [c1, speed, final_s] = calibrate_cluster(&mut coeffs, &jobs, &blocks)?;
        println!(
            "cluster   c1 {:.1} ms  f {:.4}  F1 {:.0} ms  ({} jobs)",
            1e3 * c1,
            speed,
            1e3 * final_s,
            jobs.len()
        );
    }
    write_json(&args.out, &coeffs)?;
    println!("coefficients written to {}", args.out.display());
    Ok(())
}
