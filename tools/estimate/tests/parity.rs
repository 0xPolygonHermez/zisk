//! Holds the model to its reference implementation (Python, model v3.2): for every case, the
//! end of phase 1 and the ready time of each worker and the three phases of the job, within
//! 1e-9 s. `data/parity_v32.json` is a sample of the reference's cases.

use serde::Deserialize;
use std::collections::BTreeMap;
use zisk_estimate::cluster::cluster_phases;
use zisk_estimate::model::{cost_plan, Coeffs};

#[derive(Deserialize)]
struct Block {
    plan: Vec<(String, usize)>,
    exec_s: f64,
    mb: f64,
}

#[derive(Deserialize)]
struct Case {
    b: String,
    #[serde(rename = "W")]
    w: usize,
    seed: u64,
    final_s: f64,
    p: [f64; 3],
    // `base` in the reference
    #[serde(default, rename = "base")]
    phase1: Vec<f64>,
    #[serde(default)]
    ready: Vec<f64>,
}

#[derive(Deserialize)]
struct Parity {
    blocks: BTreeMap<String, Block>,
    cases: Vec<Case>,
}

/// Checks every case against the reference; returns how many there are.
fn parity(text: &str) -> usize {
    let parity: Parity = serde_json::from_str(text).unwrap();
    let coeffs = Coeffs::embedded().unwrap();
    for c in &parity.cases {
        let b = &parity.blocks[&c.b];
        // the reference plan already lists the fixed instances
        let lines = cost_plan(&b.plan, &coeffs);
        let got = cluster_phases(&lines, b.exec_s, b.mb, c.w, &coeffs, c.final_s, c.seed);
        let diff = |a: f64, r: f64, what: &str| {
            assert!((a - r).abs() < 1e-9, "{} W={} {what}: {a} vs reference {r}", c.b, c.w);
        };
        for (k, r) in c.phase1.iter().enumerate() {
            diff(got.phase1[k], *r, "phase 1 of a worker");
        }
        for (k, r) in c.ready.iter().enumerate() {
            diff(got.ready[k], *r, "ready time of a worker");
        }
        for (i, what) in ["phase 1", "phase 2", "phase 3"].iter().enumerate() {
            diff(got.phase[i], c.p[i], what);
        }
    }
    parity.cases.len()
}

#[test]
fn matches_the_reference_model_on_the_fixture() {
    assert!(parity(include_str!("data/parity_v32.json")) > 50);
}

/// Every block of both test corpora: `ESTIMATE_PARITY_FILE=<parity_full.json> cargo test
/// -p zisk-estimate --test parity -- --ignored --nocapture`.
#[test]
#[ignore = "needs ESTIMATE_PARITY_FILE, every case of the reference"]
fn matches_the_reference_model_on_every_block() {
    let path = std::env::var("ESTIMATE_PARITY_FILE").expect("ESTIMATE_PARITY_FILE");
    let n = parity(&std::fs::read_to_string(path).unwrap());
    println!("{n} cases within 1e-9 s of the reference");
}
