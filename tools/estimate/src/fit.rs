//! The fit: from measurements of single workers (`zisk-estimate-measure`) to the coefficients of
//! the model.
//!
//! A record is one worker of a job of W workers: how long its two phases took, for a block whose
//! plan and execution time the tool knows (`zisk-estimate -i <dir> --json-out`). What the worker
//! owned comes from the record when it says so, else from the placement of `model::partition`.
//!
//!   phase 1   y = c + rho x + tau N - kappa [owns a Keccakf] + sum_a n(a) t1(a)
//!   proofs    y = c2 + pi [lacks a Keccakf or a Main] + sum_a n(a) t2(a)       (pi is not fitted)
//!
//! Both are least squares with a ridge that pulls each per-AIR price towards the prior table (an
//! AIR that few blocks exercise keeps its prior) and prices >= 0. Every measurement file is a
//! corpus and the corpora weigh the same, whatever their sizes. The noise of a phase is the spread
//! of the per-worker error between the workers of one job.
//!
//! The constants that belong to a cluster and not to a GPU (c1, f, F1) come from 1-worker jobs
//! of that cluster (`calibrate_cluster`).
//!
//! The phase-1 fit is the frozen recipe of model v3.2 (tools/data/estimate-review-2026-11-20/
//! p1table.py); the test at the end holds it to that table.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::model::{cost_plan, Coeffs};

/// One worker of one job, as `zisk-estimate-measure` writes it.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Record {
    pub input: String,
    pub workers: usize,
    /// Which worker of the job.
    #[serde(default)]
    pub node: usize,
    /// Contributions phase, seconds.
    pub phase1_s: f64,
    /// Proofs phase, seconds.
    pub proofs_s: f64,
    /// Aggregation or final proof, seconds: only for whole jobs of a cluster.
    #[serde(default)]
    pub final_s: f64,
    /// The executor's own time inside phase 1, seconds.
    #[serde(default)]
    pub execute_s: f64,
    /// Instances the worker proved, per AIR, when known; else the placement is computed.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub airs: BTreeMap<String, usize>,
}

/// What the tool knows of a block: the fields of `zisk-estimate --json-out` the fit reads.
#[derive(Clone, Debug, Deserialize)]
pub struct Block {
    pub input: String,
    pub exec_s: f64,
    pub lines: Vec<BlockLine>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct BlockLine {
    pub air: String,
    pub instances: usize,
}

/// A record joined with its block: one row of both regressions.
#[derive(Clone, Debug)]
struct Row {
    block: String,
    workers: usize,
    x: f64,
    /// Instances of the block, std virtual tables aside.
    block_instances: f64,
    airs: BTreeMap<String, usize>,
    phase1_s: f64,
    proofs_s: f64,
}

impl Row {
    fn owns(&self, air: &str) -> bool {
        self.airs.get(air).copied().unwrap_or(0) >= 1
    }
}

fn rows(records: &[Record], blocks: &BTreeMap<String, Block>, prior: &Coeffs) -> Vec<Row> {
    let mut out = Vec::new();
    for r in records {
        let Some(b) = blocks.get(&r.input) else { continue };
        let airs = if r.airs.is_empty() {
            let plan: Vec<(String, usize)> =
                b.lines.iter().map(|l| (l.air.clone(), l.instances)).collect();
            let mut load = prior.partition(&cost_plan(&plan, prior), r.workers);
            if r.node >= load.len() {
                continue;
            }
            load.swap_remove(r.node).airs
        } else {
            r.airs.clone()
        };
        let block_instances: usize = b
            .lines
            .iter()
            .filter(|l| !l.air.starts_with("VirtualTable"))
            .map(|l| l.instances)
            .sum();
        out.push(Row {
            block: r.input.clone(),
            workers: r.workers,
            x: b.exec_s,
            block_instances: block_instances as f64,
            airs,
            phase1_s: r.phase1_s,
            proofs_s: r.proofs_s,
        });
    }
    out
}

/// Weighted least squares where the columns from `free` on are ridge-pulled towards `prior`
/// (penalty `lam n / (1000 scale^2) (t - prior)^2`) and must come out >= 0: a column that comes
/// out negative is dropped and the fit repeated.
fn ridge_nonneg(
    x: &[Vec<f64>],
    y: &[f64],
    w: &[f64],
    free: usize,
    prior: &[f64],
    scale: &[f64],
    lam: f64,
) -> Vec<f64> {
    let n = x.first().map_or(0, Vec::len);
    let rows = x.len() as f64;
    let mut active: Vec<usize> = (0..n).collect();
    loop {
        let m = active.len();
        let mut a = vec![vec![0.0; m]; m];
        let mut b = vec![0.0; m];
        for ((row, t), wi) in x.iter().zip(y).zip(w) {
            let v: Vec<f64> = active.iter().map(|&i| row[i] * wi).collect();
            let t = t * wi;
            for i in 0..m {
                if v[i] == 0.0 {
                    continue;
                }
                for j in 0..m {
                    a[i][j] += v[i] * v[j];
                }
                b[i] += v[i] * t;
            }
        }
        for (q, &i) in active.iter().enumerate() {
            if i >= free {
                let pen = lam * rows / (scale[i - free] * scale[i - free]) / 1e3;
                a[q][q] += pen;
                b[q] += pen * prior[i - free];
            } else {
                a[q][q] += 1e-9;
            }
        }
        for i in 0..m {
            let pivot = a[i].clone();
            for j in i + 1..m {
                let f = a[j][i] / pivot[i];
                if f == 0.0 {
                    continue;
                }
                for (t, p) in a[j].iter_mut().zip(&pivot) {
                    *t -= f * p;
                }
                b[j] -= f * b[i];
            }
        }
        let mut c = vec![0.0; m];
        for i in (0..m).rev() {
            let s: f64 = (i + 1..m).map(|k| a[i][k] * c[k]).sum();
            c[i] = (b[i] - s) / a[i][i];
        }
        let negative: Vec<usize> =
            (0..m).filter(|&q| active[q] >= free && c[q] < 0.0).map(|q| active[q]).collect();
        if negative.is_empty() {
            let mut full = vec![0.0; n];
            for (q, &i) in active.iter().enumerate() {
                full[i] = c[q];
            }
            return full;
        }
        active.retain(|i| !negative.contains(i));
    }
}

/// Row weights that make every corpus weigh as much as the first.
fn corpus_weights(corpora: &[Vec<Row>]) -> Vec<f64> {
    let first = corpora.first().map_or(0, Vec::len) as f64;
    corpora
        .iter()
        .flat_map(|c| {
            let w = if c.is_empty() { 1.0 } else { (first / c.len() as f64).sqrt() };
            std::iter::repeat(w).take(c.len())
        })
        .collect()
}

/// The fitted numbers, in seconds.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Fit {
    /// Phase 1: c, rho, tau, kappa and the per-AIR t1.
    pub phase_const_s: f64,
    pub exec_ratio: f64,
    pub planning_s: f64,
    pub keccakf_saving_s: f64,
    pub phase_s: BTreeMap<String, f64>,
    /// Proofs: c2 and the per-AIR t2.
    pub proofs_const_s: f64,
    pub proofs_s: BTreeMap<String, f64>,
    /// Per-worker noise of each phase.
    pub phase_sigma_s: f64,
    pub proofs_sigma_s: f64,
    /// Per corpus and W: records, then median error and median |error| of phase 1 and of the
    /// proofs phase, ms (model - measured).
    pub errors: Vec<(usize, usize, usize, [f64; 4])>,
}

fn median(v: &mut [f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        0.5 * (v[n / 2 - 1] + v[n / 2])
    }
}

/// Spread of the error between the workers of one job, over the jobs of `workers` workers: the
/// mean of the standard deviation and of 1.4826 x the median absolute deviation. `error[i]` is
/// the error of `rows[i]`.
fn within_job_sigma(rows: &[Row], error: &[f64], workers: usize) -> f64 {
    let mut jobs: BTreeMap<&str, Vec<f64>> = BTreeMap::new();
    for (r, e) in rows.iter().zip(error).filter(|(r, _)| r.workers == workers) {
        jobs.entry(&r.block).or_default().push(*e);
    }
    let (mut ss, mut dof, mut dev) = (0.0, 0usize, Vec::new());
    for v in jobs.values().filter(|v| v.len() >= 2) {
        let n = v.len() as f64;
        let mean = v.iter().sum::<f64>() / n;
        for t in v {
            let d = t - mean;
            ss += d * d;
            dev.push((d * (n / (n - 1.0)).sqrt()).abs());
        }
        dof += v.len() - 1;
    }
    if dof == 0 {
        return 0.0;
    }
    0.5 * ((ss / dof as f64).sqrt() + 1.4826 * median(&mut dev))
}

/// How to fit.
#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// Strength of the pull towards the prior prices.
    pub ridge: f64,
    /// Job size whose jobs give the noise of each phase (first corpus only).
    pub sigma_workers: usize,
    /// Hold c2 at this value, measured elsewhere, instead of fitting it.
    pub proofs_const: Option<f64>,
    /// Leave the proofs phase as the prior has it: fit phase 1 only.
    pub keep_proofs: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options { ridge: 1.0, sigma_workers: 4, proofs_const: None, keep_proofs: false }
    }
}

/// Fits both phases on the corpora (one list of records each).
pub fn fit(
    corpora: &[Vec<Record>],
    blocks: &BTreeMap<String, Block>,
    prior: &Coeffs,
    options: Options,
) -> Result<Fit> {
    let Options { ridge: lam, sigma_workers, proofs_const, keep_proofs } = options;
    let proofs_const = if keep_proofs { Some(prior.cluster.proofs_const_s) } else { proofs_const };
    let corpora: Vec<Vec<Row>> = corpora.iter().map(|c| rows(c, blocks, prior)).collect();
    let all: Vec<&Row> = corpora.iter().flatten().collect();
    anyhow::ensure!(!all.is_empty(), "no measurement matches a block of the estimates");
    let w = corpus_weights(&corpora);
    let mut airs: Vec<&str> = all.iter().flat_map(|r| r.airs.keys().map(String::as_str)).collect();
    airs.sort_unstable();
    airs.dedup();
    let counts = |r: &Row| -> Vec<f64> {
        airs.iter().map(|a| r.airs.get(*a).copied().unwrap_or(0) as f64).collect()
    };
    let pi = prior.cluster.nopair_s;
    let pair = |r: &Row| r.owns("Keccakf") && r.owns("Main");

    // phase 1: c, rho, tau, -kappa, then the prices
    let p1: Vec<f64> = airs.iter().map(|a| prior.row(a).phase_ms / 1e3).collect();
    let s1: Vec<f64> = p1.iter().map(|p| p.max(0.020)).collect();
    let x1: Vec<Vec<f64>> = all
        .iter()
        .map(|r| {
            let k = if r.owns("Keccakf") { -1.0 } else { 0.0 };
            [1.0, r.x, r.block_instances, k].into_iter().chain(counts(r)).collect()
        })
        .collect();
    let y1: Vec<f64> = all.iter().map(|r| r.phase1_s).collect();
    let c1 = ridge_nonneg(&x1, &y1, &w, 4, &p1, &s1, lam);

    // proofs: c2 (its column is 0 when c2 is given), then the prices; pi is the prior's
    let p2: Vec<f64> =
        airs.iter().map(|a| (prior.row(a).basic_ms + prior.row(a).recursion_ms) / 1e3).collect();
    let s2: Vec<f64> = p2.iter().map(|p| p.max(1e-3)).collect();
    let intercept = if proofs_const.is_some() { 0.0 } else { 1.0 };
    let x2: Vec<Vec<f64>> =
        all.iter().map(|r| std::iter::once(intercept).chain(counts(r)).collect()).collect();
    let y2: Vec<f64> = all
        .iter()
        .map(|r| r.proofs_s - if pair(r) { 0.0 } else { pi } - proofs_const.unwrap_or(0.0))
        .collect();
    let mut c2 = if keep_proofs {
        std::iter::once(0.0).chain(p2.iter().copied()).collect()
    } else {
        ridge_nonneg(&x2, &y2, &w, 1, &p2, &s2, lam)
    };

    // model - measured, per row, in the order of `all`
    let dot = |c: &[f64], x: &[f64]| c.iter().zip(x).map(|(a, b)| a * b).sum::<f64>();
    let e1: Vec<f64> = x1.iter().zip(&y1).map(|(x, y)| dot(&c1, x) - y).collect();
    let e2: Vec<f64> = x2.iter().zip(&y2).map(|(x, y)| dot(&c2, x) - y).collect();
    if let Some(given) = proofs_const {
        c2[0] = given;
    }

    let mut errors = Vec::new();
    let mut start = 0;
    for (g, corpus) in corpora.iter().enumerate() {
        let mut sizes: Vec<usize> = corpus.iter().map(|r| r.workers).collect();
        sizes.sort_unstable();
        sizes.dedup();
        for workers in sizes {
            let of = |e: &[f64]| -> Vec<f64> {
                corpus
                    .iter()
                    .zip(&e[start..])
                    .filter(|(r, _)| r.workers == workers)
                    .map(|(_, t)| 1e3 * t)
                    .collect()
            };
            let (mut a, mut b) = (of(&e1), of(&e2));
            let n = a.len();
            let (m1, m2) = (median(&mut a), median(&mut b));
            let mut abs1: Vec<f64> = a.iter().map(|t| t.abs()).collect();
            let mut abs2: Vec<f64> = b.iter().map(|t| t.abs()).collect();
            errors.push((g, workers, n, [m1, median(&mut abs1), m2, median(&mut abs2)]));
        }
        start += corpus.len();
    }

    let first = corpora.first().map(Vec::as_slice).unwrap_or(&[]);
    Ok(Fit {
        phase_const_s: c1[0],
        exec_ratio: c1[1],
        planning_s: c1[2],
        keccakf_saving_s: c1[3],
        phase_s: airs.iter().map(|a| a.to_string()).zip(c1[4..].iter().copied()).collect(),
        proofs_const_s: c2[0],
        proofs_s: if keep_proofs {
            BTreeMap::new()
        } else {
            airs.iter().map(|a| a.to_string()).zip(c2[1..].iter().copied()).collect()
        },
        phase_sigma_s: within_job_sigma(first, &e1, sigma_workers),
        proofs_sigma_s: within_job_sigma(first, &e2, sigma_workers),
        errors,
    })
}

impl Fit {
    /// The prior with the fitted numbers written in. An AIR the prior has no row for keeps the
    /// default row; basic and recursion keep their prior proportion.
    pub fn apply(&self, prior: &Coeffs) -> Coeffs {
        let mut out = prior.clone();
        for (air, row) in out.airs.iter_mut() {
            if let Some(t1) = self.phase_s.get(air) {
                row.phase_ms = 1e3 * t1;
            }
            let old = row.basic_ms + row.recursion_ms;
            if let (Some(t2), true) = (self.proofs_s.get(air), old > 0.0) {
                let k = 1e3 * t2 / old;
                row.basic_ms *= k;
                row.recursion_ms *= k;
            }
        }
        let cm = &mut out.cluster;
        cm.exec_ratio = self.exec_ratio;
        cm.phase_const_s = self.phase_const_s;
        cm.planning_s = self.planning_s;
        cm.keccakf_saving_s = self.keccakf_saving_s;
        if !self.proofs_s.is_empty() {
            cm.proofs_const_s = self.proofs_const_s;
            cm.proofs_factor = 1.0;
        }
        if self.phase_sigma_s > 0.0 {
            cm.phase_sigma_s = self.phase_sigma_s;
        }
        if self.proofs_sigma_s > 0.0 {
            cm.proofs_sigma_s = self.proofs_sigma_s;
        }
        out
    }
}

/// Sets the constants of one cluster from whole 1-worker jobs proved on it: c1 = median of what
/// phase 1 took beyond the model's terms, f = median speed of the proofs phase against the
/// table's prices, F1 = median final proof. Returns (c1, f, F1).
pub fn calibrate_cluster(
    coeffs: &mut Coeffs,
    jobs: &[Record],
    blocks: &BTreeMap<String, Block>,
) -> Result<[f64; 3]> {
    let cm = coeffs.cluster.clone();
    let (mut d1, mut ratio, mut d3) = (Vec::new(), Vec::new(), Vec::new());
    for r in jobs.iter().filter(|r| r.workers == 1) {
        let Some(b) = blocks.get(&r.input) else { continue };
        let plan: Vec<(String, usize)> =
            b.lines.iter().map(|l| (l.air.clone(), l.instances)).collect();
        let lines = cost_plan(&plan, coeffs);
        let n: usize =
            lines.iter().filter(|l| !l.air.starts_with("VirtualTable")).map(|l| l.instances).sum();
        let t1: f64 = lines.iter().map(|l| l.phase_s).sum();
        let t2: f64 = lines.iter().map(|l| l.basic_s + l.recursion_s).sum();
        d1.push(r.phase1_s - cm.exec_ratio * b.exec_s - cm.planning_s * n as f64 - t1);
        ratio.push((r.proofs_s - cm.proofs_const_s) / t2);
        d3.push(r.final_s);
    }
    anyhow::ensure!(d1.len() >= 3, "fewer than 3 one-worker jobs match a block of the estimates");
    let found = [median(&mut d1), median(&mut ratio), median(&mut d3)];
    coeffs.cluster.phase_const_s = found[0];
    coeffs.cluster.proofs_factor = found[1];
    coeffs.cluster.final_s = found[2];
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(input: &str, exec_s: f64, plan: &[(&str, usize)]) -> Block {
        Block {
            input: input.to_string(),
            exec_s,
            lines: plan
                .iter()
                .map(|(a, n)| BlockLine { air: a.to_string(), instances: *n })
                .collect(),
        }
    }

    #[test]
    fn recovers_the_numbers_that_made_the_records() {
        let prior = Coeffs::embedded().unwrap();
        let (c, rho, tau, kappa, c2) = (0.070, 1.03, 0.0013, 0.024, -0.050);
        let pi = prior.cluster.nopair_s;
        let truth1 = |a: &str| 1.1 * prior.airs[a].phase_ms / 1e3;
        let truth2 = |a: &str| 0.9 * (prior.airs[a].basic_ms + prior.airs[a].recursion_ms) / 1e3;
        let mut blocks = BTreeMap::new();
        let mut records = Vec::new();
        for i in 0..40usize {
            let plan = [
                ("Rom", 1),
                ("Main", 2 + i % 7),
                ("Keccakf", 1 + i % 4),
                ("BinaryAdd", 1 + i % 3),
                ("Arith", i % 2),
                ("Mem", 1 + i % 5),
            ];
            let b = block(&format!("b{i}"), 0.2 + 0.01 * i as f64, &plan);
            let lines: Vec<(String, usize)> =
                b.lines.iter().map(|l| (l.air.clone(), l.instances)).collect();
            let lines = cost_plan(&lines, &prior);
            let n: usize = plan.iter().map(|(_, n)| n).sum();
            for workers in [1, 2, 4] {
                for (node, load) in prior.partition(&lines, workers).iter().enumerate() {
                    // without the std tables: one per worker, which no fit tells from a constant
                    let airs: BTreeMap<String, usize> = load
                        .airs
                        .iter()
                        .filter(|(a, _)| !a.starts_with("VirtualTable"))
                        .map(|(a, k)| (a.clone(), *k))
                        .collect();
                    let owns = |a: &str| airs.contains_key(a);
                    let sum = |t: &dyn Fn(&str) -> f64| -> f64 {
                        airs.iter().map(|(a, k)| *k as f64 * t(a)).sum()
                    };
                    let k = if owns("Keccakf") { kappa } else { 0.0 };
                    let nopair = if owns("Keccakf") && owns("Main") { 0.0 } else { pi };
                    records.push(Record {
                        input: b.input.clone(),
                        workers,
                        node,
                        phase1_s: c + rho * b.exec_s + tau * n as f64 - k + sum(&truth1),
                        proofs_s: c2 + nopair + sum(&truth2),
                        airs: airs.clone(),
                        ..Default::default()
                    });
                }
            }
            blocks.insert(b.input.clone(), b);
        }
        // no ridge: the records are exact
        let exact = Options { ridge: 0.0, ..Default::default() };
        let f = fit(&[records], &blocks, &prior, exact).unwrap();
        let near = |a: f64, b: f64, what: &str| assert!((a - b).abs() < 1e-6, "{what}: {a} vs {b}");
        near(f.phase_const_s, c, "c");
        near(f.exec_ratio, rho, "rho");
        near(f.planning_s, tau, "tau");
        near(f.keccakf_saving_s, kappa, "kappa");
        near(f.proofs_const_s, c2, "c2");
        near(f.phase_s["Main"], truth1("Main"), "t1 of Main");
        near(f.proofs_s["Keccakf"], truth2("Keccakf"), "t2 of Keccakf");
        near(f.phase_sigma_s, 0.0, "noise of exact records");
    }

    /// The frozen single-GPU records of model v3.2 must give its phase-1 table. The files come
    /// from tools/data/estimate-fit-rust-parity-2026-11-25/export.py.
    #[test]
    #[ignore = "needs ESTIMATE_FIT_PARITY_DIR, the exported records of model v3.2"]
    fn reproduces_the_phase_1_table_of_model_v3_2() {
        #[derive(Deserialize)]
        struct Expected {
            phase_ms: BTreeMap<String, f64>,
            exec_ratio: f64,
            phase_const_s: f64,
            planning_s: f64,
        }
        let dir = std::env::var("ESTIMATE_FIT_PARITY_DIR").expect("ESTIMATE_FIT_PARITY_DIR");
        let read = |name: &str| std::fs::read_to_string(format!("{dir}/{name}")).unwrap();
        let prior: Coeffs = serde_json::from_str(&read("prior_coeffs.json")).unwrap();
        let blocks: Vec<Block> = serde_json::from_str(&read("estimates.json")).unwrap();
        let blocks: BTreeMap<String, Block> =
            blocks.into_iter().map(|b| (b.input.clone(), b)).collect();
        let day: Vec<Record> = serde_json::from_str(&read("day_records.json")).unwrap();
        let killer: Vec<Record> = serde_json::from_str(&read("killer_records.json")).unwrap();
        let expected: Expected = serde_json::from_str(&read("expected.json")).unwrap();

        let f = fit(&[day, killer], &blocks, &prior, Options::default()).unwrap();
        let mut worst: f64 = 0.0;
        for (air, t1) in &f.phase_s {
            if let Some(e) = expected.phase_ms.get(air) {
                worst = worst.max((1e3 * t1 - e).abs());
            }
        }
        println!(
            "worst t1 difference {worst:.2e} ms; rho {} c {} tau {} kappa {}; sigma1 {} sigma2 {}; c2 {}",
            f.exec_ratio,
            f.phase_const_s,
            f.planning_s,
            f.keccakf_saving_s,
            f.phase_sigma_s,
            f.proofs_sigma_s,
            f.proofs_const_s
        );
        assert!(worst < 1e-6, "a phase-1 price is {worst} ms off the frozen table");
        assert!((f.exec_ratio - expected.exec_ratio).abs() < 1e-9);
        assert!((f.phase_const_s - expected.phase_const_s).abs() < 1e-9);
        assert!((f.planning_s - expected.planning_s).abs() < 1e-9);
    }
}
