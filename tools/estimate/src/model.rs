//! The proof-time model of one prover and the pieces the cluster model builds on: the coefficient
//! file, the per-AIR lines of a block and proofman's placement of the instances on the workers.
//!
//! On the local prover (`coeffs.json`):
//!
//!   phase = max(exec_factor * exec + exec_ramp, commit_ms_per_work * W + phase_const),
//!           W = sum_air n_air * commit_work_air
//!   proofs = proofs_factor * sum_air n_air * (basic + recursion) + proofs_const
//!   T = phase + proofs + final + overhead
//!
//! `phase` is the prover's execution + contributions: one GPU-bound stretch whose work is the
//! slot commits (unpack, LDE, hash) of every committed trace, linear in the stage-1 columns times
//! rows of the instance vector (`commit_work`, in columns x 2^20 rows, from the key's starkinfo);
//! the emulators, fills, collectors and secondary witness generation overlap it. The guard covers
//! a guest or machine whose emulation outlasts the commits (`exec` is measured by the tool).
//! `proofs` is additive because the GPU stays saturated through the basic and recursive proofs.
//! `final` is the vadcop final proof and `overhead` the client/coordinator time outside the worker
//! timers. `fixed_instances` are the std table instances proofman adds and the executor plan does
//! not list.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

use crate::cluster::ClusterModel;

const DEFAULT_COEFFS: &str = include_str!("coeffs.json");
const DEFAULT_CLUSTER_COEFFS: &str = include_str!("coeffs_cluster.json");

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AirCost {
    /// Proofs price of one instance, t2 of the cluster model, = `basic_ms + recursion_ms`.
    pub basic_ms: f64,
    pub recursion_ms: f64,
    /// Stage-1 committed columns x rows of one instance, in units of (column x 2^20 rows).
    #[serde(default)]
    pub commit_work: f64,
    /// proofman's placement weight of one instance (setup weight + recursion + compressor), the
    /// key of the least-loaded partition on a cluster.
    #[serde(default)]
    pub weight: u64,
    /// Phase-1 price of one instance on the worker that owns it, ms: t1 of the cluster model
    /// (the local model uses `commit_work`).
    #[serde(default)]
    pub phase_ms: f64,
}

// A key this struct does not know is an error: a default would silently drop the term.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Model {
    /// GPU commit cost per (column x 2^20 rows) of committed trace.
    pub commit_ms_per_work: f64,
    /// Ramp before the first commit plus the partially idle tail of the commit stream.
    pub phase_const_s: f64,
    /// Guard: the phase cannot be shorter than the emulation (slowed by the concurrent commits)
    /// plus the ramp. Never binds on the reference corpus; matters for emulation-bound guests.
    #[serde(default)]
    pub exec_factor: f64,
    #[serde(default)]
    pub exec_ramp_s: f64,
    /// Measured inner-proofs time over the coefficient sum (1.0 on the RTX 5090 reference).
    pub proofs_factor: f64,
    /// Constant of the inner-proofs fit (the serial recursion tail after the last basic).
    #[serde(default)]
    pub proofs_const_s: f64,
    /// Vadcop final proof.
    pub final_s: f64,
    /// Client + coordinator time outside the worker timers.
    #[serde(default)]
    pub overhead_s: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Coeffs {
    pub setup: String,
    pub gpu: String,
    pub model: Model,
    /// Used for an AIR with neither a row nor an alias.
    pub default: AirCost,
    pub airs: BTreeMap<String, AirCost>,
    #[serde(default)]
    pub aliases: BTreeMap<String, String>,
    /// Instances proofman adds beyond the executor plan (std virtual tables).
    #[serde(default)]
    pub fixed_instances: BTreeMap<String, usize>,
    /// The cluster model (`--workers`); absent = local estimates only.
    #[serde(default)]
    pub cluster: Option<ClusterModel>,
    /// Compute units per worker in the cluster's jobs: proofman balances the instances over
    /// `units_per_worker x workers` partitions and worker k owns the units u with u mod W = k.
    /// The sheets' cluster runs one unit per worker (whole-worker balancing); the worker's
    /// default capacity is 10, which splits the same instances unit by unit.
    #[serde(default = "default_units_per_worker")]
    pub units_per_worker: usize,
    /// Table AIRs every worker proves its own instance of (the rest ride on worker 0).
    #[serde(default = "default_per_worker_airs")]
    pub per_worker_airs: Vec<String>,
}

fn default_units_per_worker() -> usize {
    1
}

fn default_per_worker_airs() -> Vec<String> {
    vec!["VirtualTableZisk1".to_string()]
}

impl Coeffs {
    /// The embedded set of the local prover.
    pub fn embedded() -> Result<Self> {
        serde_json::from_str(DEFAULT_COEFFS).context("embedded estimate coefficients")
    }

    /// The embedded set of coordinator jobs (`--workers`): its own per-AIR table.
    pub fn embedded_cluster() -> Result<Self> {
        serde_json::from_str(DEFAULT_CLUSTER_COEFFS).context("embedded cluster coefficients")
    }

    pub fn from_file(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading coefficients {}", path.display()))?;
        serde_json::from_str(&text)
            .with_context(|| format!("parsing coefficients {}", path.display()))
    }

    pub fn cluster_model(&self) -> Result<&ClusterModel> {
        self.cluster.as_ref().context("the coefficient file has no `cluster` model (--workers)")
    }

    /// The placement of `lines` on `workers` workers with this set's cluster parameters.
    pub fn partition(&self, lines: &[AirLine], workers: usize) -> Vec<WorkerLoad> {
        let rom_extra = self.cluster.as_ref().map(|c| c.rom_weight_extra).unwrap_or(0);
        partition(lines, workers, self.units_per_worker, &self.per_worker_airs, rom_extra)
    }

    /// The row for `air`, through an alias when needed. `None` when the default applies.
    pub fn lookup(&self, air: &str) -> Option<&AirCost> {
        self.airs.get(air).or_else(|| self.aliases.get(air).and_then(|a| self.airs.get(a)))
    }
}

/// One planned AIR costed. An estimate lists them in the executor's registration order: AIRs of
/// equal placement weight go to the workers in that order.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AirLine {
    pub air: String,
    pub instances: usize,
    pub basic_s: f64,
    pub recursion_s: f64,
    /// Commit work of the instances, in (column x 2^20 rows).
    pub commit_work: f64,
    /// Phase-1 price of the instances on their worker (cluster model).
    #[serde(default)]
    pub phase_s: f64,
    /// Placement weight of one instance.
    #[serde(default)]
    pub weight: u64,
    /// Row used: the AIR itself, an alias target, or "default".
    pub source: String,
}

/// What one worker of a cluster owns.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WorkerLoad {
    pub instances: usize,
    /// Proofs prices of the owned instances, summed.
    pub proofs_s: f64,
    pub commit_work: f64,
    /// Phase-1 prices of the owned instances, summed.
    pub phase_s: f64,
    weight: u64,
    /// Owned instances per AIR (the std tables counted on worker 0).
    #[serde(default)]
    pub airs: BTreeMap<String, usize>,
    /// When the worker ends phase 1, delivery of the input included (cluster model).
    #[serde(default)]
    pub phase1_s: f64,
    /// How long the worker's proofs take (cluster model).
    #[serde(default)]
    pub ready_s: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Estimate {
    /// Workers the estimate is for (1 = the single-GPU model).
    #[serde(default)]
    pub workers: usize,
    /// The replicated partition (`--workers` only).
    #[serde(default)]
    pub workers_load: Vec<WorkerLoad>,
    pub instances: usize,
    pub steps: u64,
    /// Measured here: emulation + count-and-plan. The x of the cluster model.
    pub exec_s: f64,
    /// Size of the input, MB: a coordinator sends it to every worker.
    #[serde(default)]
    pub input_mb: f64,
    /// Sum of the commit work of the instance vector, in (column x 2^20 rows).
    pub commit_work: f64,
    /// Execution + contributions of the prover (commit term, or the emulation guard).
    pub phase_s: f64,
    /// True when the emulation guard set `phase_s`.
    pub guard_bound: bool,
    pub proofs_s: f64,
    /// The final proof; on a cluster of several workers, the whole aggregation (phase 3).
    pub final_s: f64,
    pub overhead_s: f64,
    pub total_s: f64,
    pub sum_proofs_raw_s: f64,
    pub lines: Vec<AirLine>,
    pub unknown_airs: Vec<String>,
}

/// Costs the plan with `coeffs`; `exec_s` is the measured execution (+ planning) time.
pub fn estimate(plan: &[(String, usize)], steps: u64, exec_s: f64, coeffs: &Coeffs) -> Estimate {
    let mut plan: Vec<(String, usize)> = plan.to_vec();
    for (air, n) in &coeffs.fixed_instances {
        if !plan.iter().any(|(a, _)| a == air) {
            plan.push((air.clone(), *n));
        }
    }
    let mut lines = Vec::with_capacity(plan.len());
    let mut unknown = Vec::new();
    for (air, n) in &plan {
        let (row, source) = match coeffs.lookup(air) {
            Some(r) => (
                r,
                if coeffs.airs.contains_key(air) {
                    air.clone()
                } else {
                    coeffs.aliases[air].clone()
                },
            ),
            None => {
                unknown.push(air.clone());
                (&coeffs.default, "default".to_string())
            }
        };
        let k = *n as f64 / 1000.0;
        lines.push(AirLine {
            air: air.clone(),
            instances: *n,
            basic_s: k * row.basic_ms,
            recursion_s: k * row.recursion_ms,
            commit_work: *n as f64 * row.commit_work,
            phase_s: k * row.phase_ms,
            weight: row.weight,
            source,
        });
    }
    let m = &coeffs.model;
    let sum_proofs_raw_s: f64 = lines.iter().map(|l| l.basic_s + l.recursion_s).sum();
    let commit_work: f64 = lines.iter().map(|l| l.commit_work).sum();
    let commit_s = m.commit_ms_per_work * commit_work / 1000.0 + m.phase_const_s;
    let guard_s = m.exec_factor * exec_s + m.exec_ramp_s;
    let guard_bound = guard_s > commit_s;
    let phase_s = commit_s.max(guard_s);
    let proofs_s = m.proofs_factor * sum_proofs_raw_s + m.proofs_const_s;
    let total_s = phase_s + proofs_s + m.final_s + m.overhead_s;
    Estimate {
        workers: 1,
        workers_load: Vec::new(),
        instances: plan.iter().map(|(_, n)| n).sum(),
        steps,
        exec_s,
        input_mb: 0.0,
        commit_work,
        phase_s,
        guard_bound,
        proofs_s,
        final_s: m.final_s,
        overhead_s: m.overhead_s,
        total_s,
        sum_proofs_raw_s,
        lines,
        unknown_airs: unknown,
    }
}

/// Re-costs an estimate's plan with other coefficients (same execution).
pub fn estimate_plan(e: &Estimate, coeffs: &Coeffs) -> Estimate {
    let plan: Vec<(String, usize)> = e.lines.iter().map(|l| (l.air.clone(), l.instances)).collect();
    let mut est = estimate(&plan, e.steps, e.exec_s, coeffs);
    est.input_mb = e.input_mb;
    est
}

/// The executor's planning order of the state machines (`executor/src/sm`: the builtin positions,
/// then the precompiles as registered), with the memory side last as its own round. proofman
/// places instances heaviest first with a stable sort, so AIRs of equal weight keep this order:
/// it decides which worker gets which of them.
const PLANNING_ORDER: &[&[&str]] = &[
    &["Rom"],
    &["VirtualTableZisk0", "VirtualTableZisk1"],
    &["Main"],
    &[
        "Binary",
        "BinaryLarge",
        "BinaryHuge",
        "BinaryAdd",
        "BinaryAddLarge",
        "BinaryAddHuge",
        "BinaryAddHi",
        "BinaryAddHiLarge",
        "BinaryAddHiHuge",
        "BinaryExtension",
        "BinaryExtensionLarge",
    ],
    &["Arith"],
    &[
        "Dma",
        "Dma64Aligned",
        "Dma64AlignedLarge",
        "Dma64AlignedMemSet",
        "Dma64AlignedMem",
        "Dma64AlignedMemLarge",
        "Dma64AlignedMemCpy",
        "DmaUnaligned",
        "DmaPrePost",
    ],
    &["JumpDest"],
    &["Keccakf"],
    &["Sha256f"],
    &["Poseidon"],
    &["Blake2br"],
    &["Blake3f"],
    &["Blake2s"],
    &["Poseidon2KoalaBear"],
    &[
        "ArithEq",
        "ArithEqLarge",
        "Arith256X",
        "Arith256XLarge",
        "ArithSecp256K1",
        "ArithSecp256K1Large",
        "ArithBn254",
        "ArithBn254Large",
    ],
    &["ArithEq384", "ArithEq384Large"],
    &["Add256"],
    &["TwistedEdwards"],
    &[
        "MemAlign",
        "MemAlignLarge",
        "MemAlignByte",
        "MemAlignReadByte",
        "MemAlignWriteByte",
        "MemAlignByteLarge",
        "MemAlignReadByteLarge",
        "RomData",
        "InputData",
        "Mem",
    ],
];

/// Where `air`'s state machine sits in the executor's planning order; `None` for an AIR this
/// version of the tool does not know.
pub fn planning_slot(air: &str) -> Option<usize> {
    PLANNING_ORDER.iter().position(|group| group.contains(&air))
}

/// Orders the lines of a plan as the executor registers their AIRs. The order inside one state
/// machine depends on the block and is not reproduced: it would only matter for two AIRs of one
/// state machine with the same weight, whose prices are close.
pub fn sort_by_planning_order(plan: &mut [(String, usize)]) {
    let rank = |air: &str| {
        let slot = planning_slot(air)?;
        Some((slot, PLANNING_ORDER[slot].iter().position(|a| *a == air)?))
    };
    plan.sort_by_key(|(air, _)| rank(air).unwrap_or((usize::MAX, 0)));
}

fn is_memory_air(air: &str) -> bool {
    air.starts_with("Mem") || air == "RomData" || air == "InputData"
}

fn is_table_air(air: &str) -> bool {
    air.starts_with("VirtualTable") || air.ends_with("Table")
}

/// proofman's placement of the instances on `workers` workers of `units` compute units each:
/// every instance goes to the least loaded of the `units x workers` unit partitions (weight,
/// then count), in the order the cluster registers them: the ROM, the Main segments as the
/// emulation creates them, the minimal-trace secondaries heaviest first, then the memory
/// instances heaviest first; the coordinator hands unit u to worker u mod W. The std tables
/// are not balanced: those in `per_worker` are proved by every worker, the rest ride on worker 0.
/// `rom_extra` is added to the Rom's weight (its custom commit).
pub fn partition(
    lines: &[AirLine],
    workers: usize,
    units: usize,
    per_worker: &[String],
    rom_extra: u64,
) -> Vec<WorkerLoad> {
    let workers = workers.max(1);
    let mut unit = vec![WorkerLoad::default(); workers * units.max(1)];
    let per_instance = |l: &AirLine| {
        let n = l.instances as f64;
        ((l.basic_s + l.recursion_s) / n, l.commit_work / n, l.phase_s / n)
    };
    let mut order: Vec<&AirLine> = Vec::new();
    for name in ["Rom", "Main"] {
        order.extend(lines.iter().filter(|l| l.air == name));
    }
    for memory in [false, true] {
        let mut round: Vec<&AirLine> = lines
            .iter()
            .filter(|l| l.air != "Rom" && l.air != "Main" && !is_table_air(&l.air))
            .filter(|l| is_memory_air(&l.air) == memory)
            .collect();
        round.sort_by_key(|l| std::cmp::Reverse(l.weight));
        order.extend(round);
    }
    for l in order {
        let (proofs_s, commit_work, phase_s) = per_instance(l);
        for _ in 0..l.instances {
            let k = (0..unit.len()).min_by_key(|&k| (unit[k].weight, unit[k].instances)).unwrap();
            unit[k].instances += 1;
            unit[k].weight += l.weight + if l.air == "Rom" { rom_extra } else { 0 };
            unit[k].proofs_s += proofs_s;
            unit[k].commit_work += commit_work;
            unit[k].phase_s += phase_s;
            *unit[k].airs.entry(l.air.clone()).or_default() += 1;
        }
    }
    let mut load = vec![WorkerLoad::default(); workers];
    for (u, w) in unit.into_iter().enumerate() {
        let k = &mut load[u % workers];
        k.instances += w.instances;
        k.weight += w.weight;
        k.proofs_s += w.proofs_s;
        k.commit_work += w.commit_work;
        k.phase_s += w.phase_s;
        for (a, n) in w.airs {
            *k.airs.entry(a).or_default() += n;
        }
    }
    for l in lines.iter().filter(|l| is_table_air(&l.air)) {
        let on: Vec<usize> =
            if per_worker.contains(&l.air) { (0..workers).collect() } else { vec![0] };
        for k in on {
            load[k].proofs_s += l.basic_s + l.recursion_s;
            load[k].commit_work += l.commit_work;
            load[k].phase_s += l.phase_s;
            *load[k].airs.entry(l.air.clone()).or_default() += l.instances;
        }
    }
    load
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_coeffs_parse_and_alias() {
        let c = Coeffs::embedded().unwrap();
        assert!(c.lookup("Main").is_some());
        assert!(c.lookup("MemAlignByteLarge").is_some());
        assert!(c.airs["Main"].commit_work > 0.0);
        assert!(c.lookup("NoSuchAir").is_none());
    }

    #[test]
    fn estimate_composes_terms() {
        let c = Coeffs::embedded().unwrap();
        let plan =
            vec![("Main".to_string(), 2), ("Mystery".to_string(), 1), ("Mem".to_string(), 1)];
        let e = estimate(&plan, 10, 1.0, &c);
        let (main, mem, dflt) = (&c.airs["Main"], &c.airs["Mem"], &c.default);
        // + the fixed VirtualTableZisk1 instance proofman adds
        let vt = &c.airs["VirtualTableZisk1"];
        let raw = (2.0 * (main.basic_ms + main.recursion_ms)
            + (dflt.basic_ms + dflt.recursion_ms)
            + (mem.basic_ms + mem.recursion_ms)
            + (vt.basic_ms + vt.recursion_ms))
            / 1000.0;
        assert!((e.sum_proofs_raw_s - raw).abs() < 1e-9);
        let work = 2.0 * main.commit_work + dflt.commit_work + mem.commit_work + vt.commit_work;
        assert!((e.commit_work - work).abs() < 1e-9);
        let commit = c.model.commit_ms_per_work * work / 1000.0 + c.model.phase_const_s;
        let guard = c.model.exec_factor * 1.0 + c.model.exec_ramp_s;
        assert!((e.phase_s - commit.max(guard)).abs() < 1e-9);
        assert_eq!(e.guard_bound, guard > commit);
        let total = e.phase_s
            + c.model.proofs_factor * raw
            + c.model.proofs_const_s
            + c.model.final_s
            + c.model.overhead_s;
        assert!((e.total_s - total).abs() < 1e-9);
        assert_eq!(e.unknown_airs, vec!["Mystery".to_string()]);
        assert_eq!(e.instances, 5);
    }

    #[test]
    fn partition_follows_registration_order() {
        let c = Coeffs::embedded().unwrap();
        // ROM, 3 Mains, Keccakf (heaviest secondary), Arith, Mem, then the fixed table
        let plan = vec![
            ("Main".to_string(), 3),
            ("Arith".to_string(), 1),
            ("Keccakf".to_string(), 1),
            ("Mem".to_string(), 1),
            ("Rom".to_string(), 1),
        ];
        let e = estimate(&plan, 10, 0.5, &c);
        let load = partition(&e.lines, 2, 1, &c.per_worker_airs, 0);
        let w = |air: &str| c.airs[air].weight;
        // least (weight, count) at each step: Rom -> 0, Main -> 1, Main -> 0 (Rom < Main),
        // Main -> 1, Keccakf -> 0, Arith -> 1, Mem -> 1 (Keccakf outweighs Arith + Mem)
        assert_eq!(load[0].weight, w("Rom") + w("Main") + w("Keccakf"));
        assert_eq!(load[1].weight, 2 * w("Main") + w("Arith") + w("Mem"));
        assert_eq!((load[0].instances, load[1].instances), (3, 4));
        let vt = &c.airs["VirtualTableZisk1"];
        let cost = |air: &str| (c.airs[air].basic_ms + c.airs[air].recursion_ms) / 1000.0;
        let cost0 =
            cost("Rom") + cost("Main") + cost("Keccakf") + (vt.basic_ms + vt.recursion_ms) / 1000.0;
        assert!((load[0].proofs_s - cost0).abs() < 1e-9);
        // worker 1 also proves its own VirtualTableZisk1
        let work1 = 2.0 * c.airs["Main"].commit_work
            + c.airs["Arith"].commit_work
            + c.airs["Mem"].commit_work
            + vt.commit_work;
        assert!((load[1].commit_work - work1).abs() < 1e-9);
        let phase1 = (2.0 * c.airs["Main"].phase_ms
            + c.airs["Arith"].phase_ms
            + c.airs["Mem"].phase_ms
            + vt.phase_ms)
            / 1000.0;
        assert!((load[1].phase_s - phase1).abs() < 1e-9);
        assert_eq!(load[1].airs["Main"], 2);
        assert_eq!(load[0].airs["VirtualTableZisk1"], 1);
        // every worker proves VirtualTableZisk1: the loads sum to the plan plus one extra copy
        let total: f64 = load.iter().map(|l| l.proofs_s).sum();
        let extra = (vt.basic_ms + vt.recursion_ms) / 1000.0;
        assert!((total - extra - e.sum_proofs_raw_s).abs() < 1e-9);
    }

    #[test]
    fn estimate_on_cluster_composes_the_phases() {
        let c = Coeffs::embedded_cluster().unwrap();
        let cm = c.cluster_model().unwrap();
        let plan = vec![
            ("Rom".to_string(), 1),
            ("Main".to_string(), 3),
            ("Keccakf".to_string(), 1),
            ("Mem".to_string(), 1),
        ];
        let mut e1 = estimate(&plan, 10, 0.5, &c);
        e1.input_mb = 2.0;
        // one worker: no noise to take a maximum over, no delivery, no fold
        let w1 = e1.clone().on_cluster(1, &c, false);
        let load = &c.partition(&e1.lines, 1)[0];
        let planning = cm.planning_s * 6.0;
        let phase = cm.exec_ratio * 0.5 + planning + cm.phase_const_s + load.phase_s;
        assert!((w1.phase_s - phase).abs() < 1e-12);
        let proofs = cm.proofs_const_s + cm.proofs_factor * load.proofs_s;
        assert!((w1.proofs_s - proofs).abs() < 1e-12);
        assert_eq!(w1.final_s, cm.final_s);
        assert_eq!(e1.clone().on_cluster(1, &c, true).final_s, cm.final_plain_s.unwrap());
        // two workers: the Rom and a Main leave worker 0 the lighter one when the Keccakf comes
        let w2 = e1.clone().on_cluster(2, &c, false);
        let load = c.partition(&e1.lines, 2);
        assert_eq!((load[0].airs["Rom"], load[0].airs["Main"], load[0].airs["Keccakf"]), (1, 1, 1));
        assert_eq!((load[1].airs["Main"], load[1].airs["Mem"]), (2, 1));
        let delivery = 2.0 * 2.0 / cm.link_mb_per_s;
        let end = |k: usize, saving: f64| {
            delivery + cm.exec_ratio * 0.5 + planning + cm.phase_const_s + load[k].phase_s - saving
        };
        assert!((w2.workers_load[0].phase1_s - end(0, cm.keccakf_saving_s)).abs() < 1e-12);
        assert!((w2.workers_load[1].phase1_s - end(1, 0.0)).abs() < 1e-12);
        // worker 1 has no Keccakf: it lacks the pair
        let ready = |k: usize, nopair: f64| {
            cm.proofs_const_s + cm.proofs_factor * load[k].proofs_s + nopair + cm.hop_s()
        };
        assert!((w2.workers_load[0].ready_s - ready(0, 0.0)).abs() < 1e-12);
        assert!((w2.workers_load[1].ready_s - ready(1, cm.nopair_s)).abs() < 1e-12);
        // each phase is at least its slowest worker, and the fold comes after the final proof
        assert!(w2.phase_s >= end(0, cm.keccakf_saving_s).max(end(1, 0.0)));
        assert!(w2.proofs_s >= ready(0, 0.0).max(ready(1, cm.nopair_s)));
        assert!(w2.final_s > cm.final_s + cm.rec2_s);
        assert!((w2.total_s - (w2.phase_s + w2.proofs_s + w2.final_s)).abs() < 1e-12);
        assert!(w2.total_s < w1.total_s);
    }

    #[test]
    fn unit_partitions_spread_the_instances_round_robin() {
        let c = Coeffs::embedded().unwrap();
        // 8 equal instances over 2 workers of 4 units: units fill in order, worker k gets units
        // k and k + 2, so both own 4 instances
        let plan = vec![("Main".to_string(), 8)];
        let e = estimate(&plan, 10, 0.5, &c);
        let load = partition(&e.lines, 2, 4, &c.per_worker_airs, 0);
        assert_eq!((load[0].airs["Main"], load[1].airs["Main"]), (4, 4));
        // one unit each: the W-bin placement
        let load = partition(&e.lines, 2, 1, &c.per_worker_airs, 0);
        assert_eq!((load[0].airs["Main"], load[1].airs["Main"]), (4, 4));
        let plan = vec![("Main".to_string(), 3)];
        let e = estimate(&plan, 10, 0.5, &c);
        let load = partition(&e.lines, 2, 10, &c.per_worker_airs, 0);
        assert_eq!((load[0].airs["Main"], load[1].airs["Main"]), (2, 1));
        // the per-worker table is on both workers
        assert_eq!((load[0].airs["VirtualTableZisk1"], load[1].airs["VirtualTableZisk1"]), (1, 1));
    }
    #[test]
    fn every_embedded_air_has_a_planning_slot() {
        for c in [Coeffs::embedded().unwrap(), Coeffs::embedded_cluster().unwrap()] {
            for air in c.airs.keys().chain(c.fixed_instances.keys()) {
                assert!(planning_slot(air).is_some(), "{air} is not in PLANNING_ORDER");
            }
        }
    }

    #[test]
    fn plans_sort_into_the_planning_order() {
        let mut plan: Vec<(String, usize)> =
            ["Mem", "JumpDest", "Arith256X", "Dma", "ArithBn254", "BinaryAddHi", "Main", "Rom"]
                .iter()
                .map(|a| (a.to_string(), 1))
                .collect();
        sort_by_planning_order(&mut plan);
        let airs: Vec<&str> = plan.iter().map(|(a, _)| a.as_str()).collect();
        assert_eq!(
            airs,
            ["Rom", "Main", "BinaryAddHi", "Dma", "JumpDest", "Arith256X", "ArithBn254", "Mem"]
        );
    }
}
