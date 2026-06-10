//! Shared helpers + fixtures for the recurser-aggregator end-to-end tests.
//!
//! Each leaf proof attests a chain segment `[old, new]` (see the
//! `recurser_chain` guest). Folding two contiguous segments stitches them:
//! `CheckPublics` enforces `a.new == b.old` and `AggregatePublics` emits
//! `[a.old, b.new]`.
//!
//! Pulled into each test binary via `#[path = "recurser/common.rs"] mod common;`.
//! The positive and negative cases live in SEPARATE test files because the SDK
//! enforces a per-process `ProverClient` singleton (`sdk/src/client.rs`) and
//! `cargo test` runs a file's tests as threads in one process — two clients in
//! one process panics. Splitting the files gives each its own process/client.
//!
//! Heavy and ignored by default (real proving key + proves + folds + a
//! circom/STARK setup). Overrides: `ZISK_TEST_PROVING_KEY=<dir>`, `ZISK_TEST_GPU=1`.

// Each test binary only uses a subset of these; silence the per-binary dead-code warns.
#![allow(dead_code)]

use std::path::PathBuf;

use anyhow::{anyhow, Result};
use test_artifacts::ELF_RECURSER_CHAIN;
use zisk_sdk::{EmbeddedClient, EmbeddedClientBuilder, Proof, RecurserAggregator, ZiskStdin};

pub const AGGREGATE_TEMPLATE: &str = include_str!("fixtures/aggregate_publics.circom");
pub const CHECK_TEMPLATE: &str = include_str!("fixtures/check_publics.circom");

pub fn build_client() -> EmbeddedClient {
    let mut builder = EmbeddedClientBuilder::default();
    #[cfg(target_os = "linux")]
    {
        builder = builder.assembly();
    }
    if std::env::var_os("ZISK_TEST_GPU").is_some() {
        eprintln!("[recurser] ZISK_TEST_GPU set — enabling GPU");
        builder = builder.gpu();
    }
    if let Some(pk) = std::env::var_os("ZISK_TEST_PROVING_KEY").map(PathBuf::from) {
        eprintln!("[recurser] using ZISK_TEST_PROVING_KEY={}", pk.display());
        builder = builder.proving_key(pk);
    }
    builder.build().expect("failed to build EmbeddedClient")
}

/// Set up the leaf program and a chain-fold aggregator, returning the handle.
pub async fn setup_aggregator(client: &EmbeddedClient) -> RecurserAggregator {
    // ROM setup for the leaf program.
    client
        .setup(&ELF_RECURSER_CHAIN)
        .run()
        .expect("submit leaf setup")
        .await
        .expect("leaf ROM setup failed");

    // Register + build the aggregator over the single leaf program with the
    // chain-stitch templates. PreparePublics defaults to identity.
    let agg = client
        .register_setup_aggregation(&[&ELF_RECURSER_CHAIN])
        .aggregate_template(AGGREGATE_TEMPLATE)
        .check_template(CHECK_TEMPLATE)
        .run()
        .expect("register aggregator");
    eprintln!("[recurser] recurser_id = {}", agg.recurser_id());

    client
        .setup(&agg)
        .run()
        .expect("submit aggregator setup")
        .await
        .expect("aggregator setup failed (circom/plonk2pil/verkey)");

    agg
}

/// Two endpoints (`old`, `new`) read back from a proof's first two user publics.
pub fn endpoints(proof: &Proof) -> (u64, u64) {
    let pubs = proof.get_publics().public_u64();
    (pubs[0], pubs[1])
}

/// 4-limb program VK of a proof.
pub fn program_vk(proof: &Proof) -> Vec<u64> {
    proof.get_program_vk().vk.clone()
}

/// Prove one chain segment `[old, new]` and return the owned leaf proof.
pub async fn prove_segment(client: &EmbeddedClient, old: u32, new: u32) -> Result<Proof> {
    let stdin = ZiskStdin::new();
    stdin.write(&old);
    stdin.write(&new);
    let result = client
        .prove(&ELF_RECURSER_CHAIN, stdin)
        .run()
        .map_err(|e| anyhow!("submit prove [{old},{new}] failed: {e}"))?
        .await
        .map_err(|e| anyhow!("prove [{old},{new}] failed: {e}"))?;
    Ok(result.get_proof().clone())
}

/// Fold two proofs through the aggregator and return the owned merged proof.
pub async fn fold(
    client: &EmbeddedClient,
    agg: &RecurserAggregator,
    a: &Proof,
    b: &Proof,
) -> Result<Proof> {
    let result = client
        .aggregate_proof(agg, a, b)
        .run()
        .map_err(|e| anyhow!("submit aggregate failed: {e}"))?
        .await
        .map_err(|e| anyhow!("aggregate failed: {e}"))?;
    Ok(result.get_proof().clone())
}
