//! The proof-time model (v3.2): a block proved as a coordinator job on W workers.
//!
//! The instances are placed on the W workers as proofman does (`partition`); every phase of the
//! job then lasts as long as its slowest worker. Each worker's time carries noise, so a phase is
//! the expected maximum over the workers, not the maximum of the expected times.
//!
//!   phase 1 (execution + contributions)
//!     S_k     = rho x + c1 + tau N + sum_a n(k,a) t1(a) - kappa [W >= 2, k owns a Keccakf]
//!     p1      = E[max_k (D + S_k + N(0, sigma1))],  D = W MB / R [W >= 2]
//!   phase 2 (proofs)
//!     ready_k = c2 + f sum_a n(k,a) t2(a) + pi [k lacks a Keccakf or a Main] + hop [W >= 2]
//!     p2      = E[max_k (ready_k + N(0, sigma2))]
//!   phase 3 (aggregation)
//!     W = 1: F1.  W >= 2: time past the slowest worker of a replay of the coordinator's fold
//!     scheduler on ready_k; a fold costs rec2 + 2 hop + upload, the last one F1 + rec2 + hop.
//!     W >= 3: mean over `fold_replays` replays with N(0, sigma2) on each ready_k, because
//!     which workers fold first decides the shape of the tree.
//!
//! The symbols are the model report's. x is the tool's execution of the block (`exec_s`), MB the
//! size of its input, N its instances (std virtual tables aside: every worker plans all of
//! them) and n(k,a) the instances of AIR a that worker k owns. The prices come from the per-AIR
//! table: t1 = `phase_ms`, t2 = `basic_ms + recursion_ms`. `hop` is one recursive2 proof over the
//! link; every other symbol is a field of `ClusterModel`. D is the delivery of the input, with
//! every worker remote from the coordinator.
//! `tests/parity.rs` holds this file to the reference implementation of the model.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use zisk_cluster_common::{AggDispatch, AggScheduler, AggSet, WorkerId};

use crate::model::{cost_plan, AirLine, Coeffs, Estimate, WorkerLoad};

// A key this struct does not know is an error: a default would silently drop the term.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClusterModel {
    /// rho: weight of the tool's execution time x (every worker executes the whole block).
    pub exec_ratio: f64,
    /// c1: constant of phase 1.
    pub phase_const_s: f64,
    /// tau: planning cost per instance of the block, owned or not.
    #[serde(default)]
    pub planning_s: f64,
    /// kappa: phase-1 time a worker saves when it owns a Keccakf (W >= 2).
    #[serde(default)]
    pub keccakf_saving_s: f64,
    /// sigma1: per-worker noise of phase 1 (standard deviation).
    #[serde(default)]
    pub phase_sigma_s: f64,
    /// f: speed of the cluster in phase 2, a scale on the owned proofs prices.
    pub proofs_factor: f64,
    /// c2: constant of phase 2.
    pub proofs_const_s: f64,
    /// pi: phase-2 wait exposed on a worker that lacks a Keccakf or a Main.
    #[serde(default)]
    pub nopair_s: f64,
    /// sigma2: per-worker noise of phase 2 (standard deviation), also on each worker's ready
    /// time in the fold replays.
    #[serde(default)]
    pub proofs_sigma_s: f64,
    /// F1: final proof at one worker, compressed.
    pub final_s: f64,
    /// F1 when the job asks for the plain (not compressed) proof.
    #[serde(default)]
    pub final_plain_s: Option<f64>,
    /// R: coordinator link, MB/s; 0 = no delivery and no hop.
    #[serde(default)]
    pub link_mb_per_s: f64,
    /// Size of one recursive2 proof, MB.
    #[serde(default)]
    pub rec2_mb: f64,
    /// One recursive2 proof of two inputs.
    #[serde(default)]
    pub rec2_s: f64,
    /// Upload of a folded proof to the coordinator.
    #[serde(default)]
    pub upload_s: f64,
    #[serde(default = "default_fold_replays")]
    pub fold_replays: usize,
    /// Aggregation arity of the key.
    #[serde(default = "default_arity")]
    pub arity: usize,
    /// Placement weight proofman adds to the Rom instance (its custom commit).
    #[serde(default)]
    pub rom_weight_extra: u64,
}

fn default_fold_replays() -> usize {
    100
}

fn default_arity() -> usize {
    2
}

impl ClusterModel {
    /// One recursive2 proof over the link.
    pub fn hop_s(&self) -> f64 {
        if self.link_mb_per_s > 0.0 {
            self.rec2_mb / self.link_mb_per_s
        } else {
            0.0
        }
    }

    /// The final proof of a job of the given kind.
    pub fn final_of(&self, plain: bool) -> f64 {
        if plain {
            self.final_plain_s.unwrap_or(self.final_s)
        } else {
            self.final_s
        }
    }
}

/// The three phases of a job and what each worker contributes to them.
pub struct ClusterPhases {
    pub load: Vec<WorkerLoad>,
    /// S_k: phase 1 of worker k, without the delivery of the input.
    pub s: Vec<f64>,
    /// D + S_k: when worker k ends phase 1.
    pub phase1: Vec<f64>,
    /// ready_k: how long the proofs of worker k take.
    pub ready: Vec<f64>,
    /// p1, p2, p3.
    pub phase: [f64; 3],
}

fn owns(load: &WorkerLoad, air: &str) -> bool {
    load.airs.get(air).copied().unwrap_or(0) >= 1
}

/// The model on `workers` workers for a block of these `lines`, execution time and input size.
/// `rng_seed` seeds the fold replays (W >= 3).
pub fn cluster_phases(
    lines: &[AirLine],
    exec_s: f64,
    input_mb: f64,
    workers: usize,
    coeffs: &Coeffs,
    final_s: f64,
    rng_seed: u64,
) -> ClusterPhases {
    let cm = &coeffs.cluster;
    let load = coeffs.partition(lines, workers);
    let multi = workers > 1;
    let block_instances: usize =
        lines.iter().filter(|l| !l.air.starts_with("VirtualTable")).map(|l| l.instances).sum();
    let common = cm.exec_ratio * exec_s + cm.planning_s * block_instances as f64 + cm.phase_const_s;
    let delivery = if multi && cm.link_mb_per_s > 0.0 {
        workers as f64 * input_mb / cm.link_mb_per_s
    } else {
        0.0
    };
    let hop = if multi { cm.hop_s() } else { 0.0 };
    let s: Vec<f64> = load
        .iter()
        .map(|l| {
            common + l.phase_s - if multi && owns(l, "Keccakf") { cm.keccakf_saving_s } else { 0.0 }
        })
        .collect();
    let phase1: Vec<f64> = s.iter().map(|s| delivery + s).collect();
    let ready: Vec<f64> = load
        .iter()
        .map(|l| {
            let pair = owns(l, "Keccakf") && owns(l, "Main");
            cm.proofs_const_s
                + cm.proofs_factor * l.proofs_s
                + if pair { 0.0 } else { cm.nopair_s }
                + hop
        })
        .collect();
    let p1 = expected_max(&phase1, cm.phase_sigma_s);
    let p2 = expected_max(&ready, cm.proofs_sigma_s);
    let p3 = if multi { aggregation_s(&ready, cm, final_s, rng_seed) } else { final_s };
    ClusterPhases { load, s, phase1, ready, phase: [p1, p2, p3] }
}

/// The estimate of a block as a job on `workers` workers, from one execution of it: its plan,
/// steps, execution time and input size. `plain` = the final proof is not compressed.
pub fn estimate(
    plan: &[(String, usize)],
    steps: u64,
    exec_s: f64,
    input_mb: f64,
    workers: usize,
    plain: bool,
    coeffs: &Coeffs,
) -> Estimate {
    let lines = cost_plan(plan, coeffs);
    let final_s = coeffs.cluster.final_of(plain);
    // the replays are seeded by the worker count: one input, one estimate
    let c = cluster_phases(&lines, exec_s, input_mb, workers, coeffs, final_s, workers as u64);
    let mut workers_load = c.load;
    for (k, l) in workers_load.iter_mut().enumerate() {
        l.phase1_s = c.phase1[k];
        l.ready_s = c.ready[k];
    }
    let [phase_s, proofs_s, final_s] = c.phase;
    Estimate {
        workers,
        workers_load,
        instances: lines.iter().map(|l| l.instances).sum(),
        steps,
        exec_s,
        input_mb,
        phase_s,
        proofs_s,
        final_s,
        total_s: phase_s + proofs_s + final_s,
        unknown_airs: lines
            .iter()
            .filter(|l| !coeffs.airs.contains_key(&l.air))
            .map(|l| l.air.clone())
            .collect(),
        lines,
    }
}

/// E[max_k (mu_k + N(0, sigma))] for independent noise: with `lo` far below every mu,
/// lo + integral over t >= lo of 1 - prod_k Phi((t - mu_k) / sigma), by the midpoint rule.
pub fn expected_max(mu: &[f64], sigma: f64) -> f64 {
    let top = mu.iter().cloned().fold(f64::MIN, f64::max);
    if sigma == 0.0 || mu.len() < 2 {
        return top;
    }
    const STEPS: usize = 240;
    let lo = top - 6.0 * sigma;
    let h = 12.0 * sigma / STEPS as f64;
    let cdf = |z: f64| 0.5 * (1.0 + libm::erf(z / std::f64::consts::SQRT_2));
    let mut area = 0.0;
    for i in 0..STEPS {
        let t = lo + (i as f64 + 0.5) * h;
        let all_done = mu.iter().fold(1.0, |p, m| p * cdf((t - m) / sigma));
        area += (1.0 - all_done) * h;
    }
    lo + area
}

/// Aggregation time past the slowest worker.
fn aggregation_s(ready: &[f64], cm: &ClusterModel, final_s: f64, rng_seed: u64) -> f64 {
    let hop = cm.hop_s();
    let fold = cm.rec2_s + 2.0 * hop + cm.upload_s;
    let last_fold = final_s + cm.rec2_s + hop;
    let slowest = |r: &[f64]| r.iter().cloned().fold(f64::MIN, f64::max);
    if ready.len() < 3 || cm.proofs_sigma_s == 0.0 || cm.fold_replays == 0 {
        return fold_end(ready, cm.arity, fold, last_fold) - slowest(ready);
    }
    let mut rng = PyRandom::new(rng_seed);
    let mut sum = 0.0;
    for _ in 0..cm.fold_replays {
        let noisy: Vec<f64> = ready.iter().map(|r| r + rng.gauss() * cm.proofs_sigma_s).collect();
        sum += fold_end(&noisy, cm.arity, fold, last_fold) - slowest(&noisy);
    }
    sum / cm.fold_replays as f64
}

/// When the final proof completes: the coordinator's fold scheduler driven by the workers' ready
/// times. A fold costs `fold` and the last one `last_fold`; a dispatch that only hands a worker an
/// input costs nothing.
fn fold_end(ready: &[f64], arity: usize, fold: f64, last_fold: f64) -> f64 {
    enum Event {
        Leaf(usize),
        Done(AggDispatch),
    }
    let set =
        |covers: BTreeSet<u32>, location: WorkerId| AggSet { proofs: Vec::new(), covers, location };
    let mut scheduler = AggScheduler::new(arity, ready.len(), true);
    let mut leaves: Vec<usize> = (0..ready.len()).collect();
    leaves.sort_by(|&a, &b| ready[a].partial_cmp(&ready[b]).unwrap().then(a.cmp(&b)));
    // (time, sequence): ties go to the earlier event
    let mut events: Vec<(f64, usize, Event)> =
        leaves.into_iter().enumerate().map(|(seq, k)| (ready[k], seq, Event::Leaf(k))).collect();
    let mut seq = events.len();
    while !events.is_empty() {
        let first = (0..events.len())
            .min_by(|&a, &b| {
                (events[a].0, events[a].1).partial_cmp(&(events[b].0, events[b].1)).unwrap()
            })
            .unwrap();
        let (t, _, event) = events.swap_remove(first);
        let dispatches = match event {
            Event::Leaf(k) => {
                scheduler.on_set_ready(set([k as u32].into(), WorkerId::from(k.to_string())))
            }
            Event::Done(done) => {
                if done.final_proof {
                    return t;
                }
                let mut out: Vec<AggDispatch> =
                    scheduler.on_ack(&done.worker).into_iter().collect();
                if done.last_proof {
                    let node = scheduler.take_live(&done.worker).expect("live fold node");
                    out.extend(scheduler.on_set_ready(set(node.covers, done.worker)));
                }
                out
            }
        };
        for d in dispatches {
            let cost = match (d.last_proof, d.final_proof) {
                (_, true) => last_fold,
                (true, false) => fold,
                (false, false) => 0.0,
            };
            events.push((t + cost, seq, Event::Done(d)));
            seq += 1;
        }
    }
    unreachable!("the scheduler ends every job with a final proof")
}

/// CPython's `random.Random(seed).gauss`: MT19937 seeded by `init_by_array`, 53-bit uniforms.
/// The fold replays draw from it so an estimate is the reference model's number, bit for bit.
struct PyRandom {
    mt: [u32; 624],
    index: usize,
    spare: Option<f64>,
}

impl PyRandom {
    fn new(seed: u64) -> Self {
        let mut mt = [0u32; 624];
        mt[0] = 19650218;
        for i in 1..624 {
            mt[i] =
                1812433253u32.wrapping_mul(mt[i - 1] ^ (mt[i - 1] >> 30)).wrapping_add(i as u32);
        }
        let mut key = vec![seed as u32];
        if seed >> 32 != 0 {
            key.push((seed >> 32) as u32);
        }
        let (mut i, mut j) = (1, 0);
        for _ in 0..624.max(key.len()) {
            mt[i] = (mt[i] ^ (mt[i - 1] ^ (mt[i - 1] >> 30)).wrapping_mul(1664525))
                .wrapping_add(key[j])
                .wrapping_add(j as u32);
            i += 1;
            j += 1;
            if i >= 624 {
                mt[0] = mt[623];
                i = 1;
            }
            if j >= key.len() {
                j = 0;
            }
        }
        for _ in 0..623 {
            mt[i] = (mt[i] ^ (mt[i - 1] ^ (mt[i - 1] >> 30)).wrapping_mul(1566083941))
                .wrapping_sub(i as u32);
            i += 1;
            if i >= 624 {
                mt[0] = mt[623];
                i = 1;
            }
        }
        mt[0] = 0x8000_0000;
        PyRandom { mt, index: 624, spare: None }
    }

    fn next_u32(&mut self) -> u32 {
        if self.index >= 624 {
            for i in 0..624 {
                let y = (self.mt[i] & 0x8000_0000) | (self.mt[(i + 1) % 624] & 0x7fff_ffff);
                let mut next = self.mt[(i + 397) % 624] ^ (y >> 1);
                if y & 1 != 0 {
                    next ^= 0x9908_b0df;
                }
                self.mt[i] = next;
            }
            self.index = 0;
        }
        let mut y = self.mt[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^ (y >> 18)
    }

    fn random(&mut self) -> f64 {
        let a = (self.next_u32() >> 5) as f64;
        let b = (self.next_u32() >> 6) as f64;
        (a * 67108864.0 + b) * (1.0 / 9007199254740992.0)
    }

    /// A standard normal draw.
    fn gauss(&mut self) -> f64 {
        if let Some(z) = self.spare.take() {
            return z;
        }
        let angle = self.random() * (2.0 * std::f64::consts::PI);
        let radius = (-2.0 * (1.0 - self.random()).ln()).sqrt();
        self.spare = Some(angle.sin() * radius);
        angle.cos() * radius
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_estimate_composes_the_phases() {
        let c = Coeffs::embedded().unwrap();
        let cm = &c.cluster;
        let plan: Vec<(String, usize)> = [("Rom", 1), ("Main", 3), ("Keccakf", 1), ("Mem", 1)]
            .iter()
            .map(|(a, n)| (a.to_string(), *n))
            .collect();
        let lines = cost_plan(&plan, &c);
        // one worker: no noise to take a maximum over, no delivery, no fold
        let w1 = estimate(&plan, 10, 0.5, 2.0, 1, false, &c);
        let load = &c.partition(&lines, 1)[0];
        let planning = cm.planning_s * 6.0;
        let phase = cm.exec_ratio * 0.5 + planning + cm.phase_const_s + load.phase_s;
        assert!((w1.phase_s - phase).abs() < 1e-12);
        let proofs = cm.proofs_const_s + cm.proofs_factor * load.proofs_s;
        assert!((w1.proofs_s - proofs).abs() < 1e-12);
        assert_eq!(w1.final_s, cm.final_s);
        assert_eq!(estimate(&plan, 10, 0.5, 2.0, 1, true, &c).final_s, cm.final_plain_s.unwrap());
        assert_eq!(w1.instances, 7);
        assert!(w1.unknown_airs.is_empty());
        // two workers: the Rom and a Main leave worker 0 the lighter one when the Keccakf comes
        let w2 = estimate(&plan, 10, 0.5, 2.0, 2, false, &c);
        let load = c.partition(&lines, 2);
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
    fn python_random_stream() {
        // random.Random(7).random() twice
        let mut r = PyRandom::new(7);
        assert_eq!(r.random(), 0.32383276483316237);
        assert_eq!(r.random(), 0.15084917392450192);
        // random.Random(257453554).gauss(0, 1) twice; a seed over 32 bits takes two key words
        let mut r = PyRandom::new(257_453_554);
        assert!((r.gauss() - -0.06395262746349305).abs() < 1e-15);
        assert!((r.gauss() - 0.5307557759973821).abs() < 1e-15);
        assert!((PyRandom::new((1 << 40) + 5).gauss() - -0.7906503216763192).abs() < 1e-15);
    }

    #[test]
    fn expected_max_of_noisy_workers() {
        assert_eq!(expected_max(&[1.0, 3.0, 2.0], 0.0), 3.0);
        assert_eq!(expected_max(&[2.5], 0.1), 2.5);
        // two equal workers: E[max] = mu + sigma / sqrt(pi)
        let e = expected_max(&[1.0, 1.0], 0.033);
        assert!((e - (1.0 + 0.033 / std::f64::consts::PI.sqrt())).abs() < 1e-6, "{e}");
        // a worker far ahead of the rest is the maximum
        assert!((expected_max(&[5.0, 1.0, 1.0], 0.033) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn folds_pair_up_as_the_workers_finish() {
        // two workers: one fold, the final one
        assert_eq!(fold_end(&[1.0, 2.0], 2, 0.1, 0.3), 2.3);
        // four equal workers: two folds side by side, then the final one
        assert!((fold_end(&[1.0; 4], 2, 0.1, 0.3) - 1.4).abs() < 1e-12);
        // a straggler: the other three are folded and waiting when it arrives
        assert!((fold_end(&[1.0, 1.0, 1.0, 5.0], 2, 0.1, 0.3) - 5.3).abs() < 1e-12);
    }
}
