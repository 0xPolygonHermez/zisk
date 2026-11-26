//! What the model is built on: the coefficient file, the per-AIR lines of a block and proofman's
//! placement of the instances on the workers. The model itself is in `cluster`.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

use crate::cluster::ClusterModel;

const DEFAULT_COEFFS: &str = include_str!("coeffs.json");

/// The std table every worker proves its own instance of (the other tables ride on worker 0).
const PER_WORKER_AIR: &str = "VirtualTableZisk1";

/// The prices of one instance of an AIR.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AirCost {
    /// Proofs price, t2 of the model, = `basic_ms + recursion_ms`.
    pub basic_ms: f64,
    pub recursion_ms: f64,
    /// Phase-1 price on the worker that owns the instance, t1 of the model.
    pub phase_ms: f64,
    /// proofman's placement weight (setup weight + recursion + compressor), the key of the
    /// least-loaded partition.
    pub weight: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Coeffs {
    pub setup: String,
    pub gpu: String,
    /// The constants of the model.
    pub cluster: ClusterModel,
    /// Instances proofman adds beyond the executor plan (std virtual tables).
    #[serde(default)]
    pub fixed_instances: BTreeMap<String, usize>,
    /// Compute units per worker in the cluster's jobs: proofman balances the instances over
    /// `units_per_worker x workers` partitions and worker k owns the units u with u mod W = k.
    /// The sheets' cluster runs one unit per worker (whole-worker balancing); the worker's
    /// default capacity is 10, which splits the same instances unit by unit.
    #[serde(default = "default_units_per_worker")]
    pub units_per_worker: usize,
    /// Used for an AIR without a row.
    pub default: AirCost,
    pub airs: BTreeMap<String, AirCost>,
}

fn default_units_per_worker() -> usize {
    1
}

impl Coeffs {
    pub fn embedded() -> Result<Self> {
        serde_json::from_str(DEFAULT_COEFFS).context("embedded estimate coefficients")
    }

    pub fn from_file(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading coefficients {}", path.display()))?;
        serde_json::from_str(&text)
            .with_context(|| format!("parsing coefficients {}", path.display()))
    }

    /// The row of `air`: its own, or the default one.
    pub fn row(&self, air: &str) -> &AirCost {
        self.airs.get(air).unwrap_or(&self.default)
    }

    /// The placement of `lines` on `workers` workers.
    pub fn partition(&self, lines: &[AirLine], workers: usize) -> Vec<WorkerLoad> {
        partition(lines, workers, self.units_per_worker, self.cluster.rom_weight_extra)
    }
}

/// One planned AIR costed. The lines of a block are in the executor's planning order: AIRs of
/// equal placement weight go to the workers in that order.
#[derive(Clone, Debug, Serialize)]
pub struct AirLine {
    pub air: String,
    pub instances: usize,
    pub basic_s: f64,
    pub recursion_s: f64,
    /// Phase-1 price of the instances on their worker.
    pub phase_s: f64,
    /// Placement weight of one instance.
    pub weight: u64,
}

/// What one worker owns.
#[derive(Clone, Debug, Default, Serialize)]
pub struct WorkerLoad {
    pub instances: usize,
    /// Proofs prices of the owned instances, summed.
    pub proofs_s: f64,
    /// Phase-1 prices of the owned instances, summed.
    pub phase_s: f64,
    weight: u64,
    /// Owned instances per AIR (the std tables counted on worker 0).
    pub airs: BTreeMap<String, usize>,
    /// When the worker ends phase 1, delivery of the input included.
    pub phase1_s: f64,
    /// How long the worker's proofs take.
    pub ready_s: f64,
}

/// The estimate of one block as a coordinator job.
#[derive(Clone, Debug, Serialize)]
pub struct Estimate {
    pub workers: usize,
    pub workers_load: Vec<WorkerLoad>,
    pub instances: usize,
    pub steps: u64,
    /// Measured by the tool: emulation + count-and-plan. The x of the model.
    pub exec_s: f64,
    /// Size of the input, MB: the coordinator sends it to every worker.
    pub input_mb: f64,
    /// Phase 1: execution + contributions.
    pub phase_s: f64,
    /// Phase 2: the basic and recursive proofs.
    pub proofs_s: f64,
    /// Phase 3: the final proof, after the folds when there are several workers.
    pub final_s: f64,
    pub total_s: f64,
    pub lines: Vec<AirLine>,
    /// AIRs costed with the default row.
    pub unknown_airs: Vec<String>,
}

/// One line per AIR of the plan, in the plan's order, then the instances proofman adds.
pub fn cost_plan(plan: &[(String, usize)], coeffs: &Coeffs) -> Vec<AirLine> {
    let fixed = coeffs
        .fixed_instances
        .iter()
        .filter(|(air, _)| !plan.iter().any(|(a, _)| a == *air))
        .map(|(air, n)| (air.clone(), *n));
    plan.iter()
        .cloned()
        .chain(fixed)
        .map(|(air, instances)| {
            let row = coeffs.row(&air);
            let k = instances as f64 / 1000.0;
            AirLine {
                air,
                instances,
                basic_s: k * row.basic_ms,
                recursion_s: k * row.recursion_ms,
                phase_s: k * row.phase_ms,
                weight: row.weight,
            }
        })
        .collect()
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
/// are not balanced: every worker proves its own `PER_WORKER_AIR`, the rest ride on worker 0.
/// `rom_extra` is added to the Rom's weight (its custom commit).
pub fn partition(
    lines: &[AirLine],
    workers: usize,
    units: usize,
    rom_extra: u64,
) -> Vec<WorkerLoad> {
    let workers = workers.max(1);
    let mut unit = vec![WorkerLoad::default(); workers * units.max(1)];
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
        let n = l.instances as f64;
        for _ in 0..l.instances {
            let k = (0..unit.len()).min_by_key(|&k| (unit[k].weight, unit[k].instances)).unwrap();
            unit[k].instances += 1;
            unit[k].weight += l.weight + if l.air == "Rom" { rom_extra } else { 0 };
            unit[k].proofs_s += (l.basic_s + l.recursion_s) / n;
            unit[k].phase_s += l.phase_s / n;
            *unit[k].airs.entry(l.air.clone()).or_default() += 1;
        }
    }
    let mut load = vec![WorkerLoad::default(); workers];
    for (u, w) in unit.into_iter().enumerate() {
        let k = &mut load[u % workers];
        k.instances += w.instances;
        k.weight += w.weight;
        k.proofs_s += w.proofs_s;
        k.phase_s += w.phase_s;
        for (a, n) in w.airs {
            *k.airs.entry(a).or_default() += n;
        }
    }
    for l in lines.iter().filter(|l| is_table_air(&l.air)) {
        let on = if l.air == PER_WORKER_AIR { 0..workers } else { 0..1 };
        for k in on {
            load[k].proofs_s += l.basic_s + l.recursion_s;
            load[k].phase_s += l.phase_s;
            *load[k].airs.entry(l.air.clone()).or_default() += l.instances;
        }
    }
    load
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(airs: &[(&str, usize)]) -> Vec<(String, usize)> {
        airs.iter().map(|(a, n)| (a.to_string(), *n)).collect()
    }

    #[test]
    fn plans_are_costed_from_the_table() {
        let c = Coeffs::embedded().unwrap();
        let lines = cost_plan(&plan(&[("Main", 2), ("Mystery", 1)]), &c);
        // + the VirtualTableZisk1 instance proofman adds
        let airs: Vec<&str> = lines.iter().map(|l| l.air.as_str()).collect();
        assert_eq!(airs, ["Main", "Mystery", "VirtualTableZisk1"]);
        let main = &c.airs["Main"];
        assert!((lines[0].basic_s - 2.0 * main.basic_ms / 1000.0).abs() < 1e-12);
        assert!((lines[0].phase_s - 2.0 * main.phase_ms / 1000.0).abs() < 1e-12);
        // an AIR without a row takes the default one
        assert_eq!(lines[1].weight, c.default.weight);
        assert!((lines[1].recursion_s - c.default.recursion_ms / 1000.0).abs() < 1e-12);
    }

    #[test]
    fn partition_follows_registration_order() {
        let c = Coeffs::embedded().unwrap();
        // ROM, 3 Mains, Keccakf (heaviest secondary), Arith, Mem, then the fixed table
        let lines = cost_plan(
            &plan(&[("Main", 3), ("Arith", 1), ("Keccakf", 1), ("Mem", 1), ("Rom", 1)]),
            &c,
        );
        let load = partition(&lines, 2, 1, 0);
        let w = |air: &str| c.airs[air].weight;
        // least (weight, count) at each step: Rom -> 0, Main -> 1, Main -> 0 (Rom < Main),
        // Main -> 1, Keccakf -> 0, Arith -> 1, Mem -> 1 (Keccakf outweighs Arith + Mem)
        assert_eq!(load[0].weight, w("Rom") + w("Main") + w("Keccakf"));
        assert_eq!(load[1].weight, 2 * w("Main") + w("Arith") + w("Mem"));
        assert_eq!((load[0].instances, load[1].instances), (3, 4));
        let cost = |air: &str| (c.airs[air].basic_ms + c.airs[air].recursion_ms) / 1000.0;
        let cost0 = cost("Rom") + cost("Main") + cost("Keccakf") + cost("VirtualTableZisk1");
        assert!((load[0].proofs_s - cost0).abs() < 1e-9);
        // worker 1 also proves its own VirtualTableZisk1
        let phase = |air: &str| c.airs[air].phase_ms / 1000.0;
        let phase1 =
            2.0 * phase("Main") + phase("Arith") + phase("Mem") + phase("VirtualTableZisk1");
        assert!((load[1].phase_s - phase1).abs() < 1e-9);
        assert_eq!(load[1].airs["Main"], 2);
        assert_eq!(load[0].airs["VirtualTableZisk1"], 1);
        // the loads sum to the plan plus that extra copy
        let total: f64 = load.iter().map(|l| l.proofs_s).sum();
        let planned: f64 = lines.iter().map(|l| l.basic_s + l.recursion_s).sum();
        assert!((total - cost("VirtualTableZisk1") - planned).abs() < 1e-9);
    }

    #[test]
    fn unit_partitions_spread_the_instances_round_robin() {
        let c = Coeffs::embedded().unwrap();
        // 8 equal instances over 2 workers of 4 units: units fill in order, worker k gets units
        // k and k + 2, so both own 4 instances
        let lines = cost_plan(&plan(&[("Main", 8)]), &c);
        let load = partition(&lines, 2, 4, 0);
        assert_eq!((load[0].airs["Main"], load[1].airs["Main"]), (4, 4));
        // one unit each: the W-bin placement
        let load = partition(&lines, 2, 1, 0);
        assert_eq!((load[0].airs["Main"], load[1].airs["Main"]), (4, 4));
        let lines = cost_plan(&plan(&[("Main", 3)]), &c);
        let load = partition(&lines, 2, 10, 0);
        assert_eq!((load[0].airs["Main"], load[1].airs["Main"]), (2, 1));
        // the per-worker table is on both workers
        assert_eq!((load[0].airs["VirtualTableZisk1"], load[1].airs["VirtualTableZisk1"]), (1, 1));
    }

    #[test]
    fn every_embedded_air_has_a_planning_slot() {
        let c = Coeffs::embedded().unwrap();
        for air in c.airs.keys().chain(c.fixed_instances.keys()) {
            assert!(planning_slot(air).is_some(), "{air} is not in PLANNING_ORDER");
        }
    }

    #[test]
    fn plans_sort_into_the_planning_order() {
        let mut plan = plan(&[
            ("Mem", 1),
            ("JumpDest", 1),
            ("Arith256X", 1),
            ("Dma", 1),
            ("ArithBn254", 1),
            ("BinaryAddHi", 1),
            ("Main", 1),
            ("Rom", 1),
        ]);
        sort_by_planning_order(&mut plan);
        let airs: Vec<&str> = plan.iter().map(|(a, _)| a.as_str()).collect();
        assert_eq!(
            airs,
            ["Rom", "Main", "BinaryAddHi", "Dma", "JumpDest", "Arith256X", "ArithBn254", "Mem"]
        );
    }
}
