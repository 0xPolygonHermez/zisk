//! Recurser-aggregator end-to-end: the full fold tree plus the broken-chain
//! rejection, in one process.
//!
//! Four leaf segments `10→20→30→40→50` collapse to a single `[10, 50]` proof
//! via three folds (two leaf+leaf, one agg+agg). This is the empirical
//! acceptance test for the VK-first publics layout: the agg+agg fold at the
//! root only succeeds if VK extraction, VK emission, and the §9 immutability
//! check all agree the program VK lives in the leading slots.
//!
//! The negative case (folding non-contiguous segments) shares this process and
//! the warm setup: a rejected fold surfaces as a clean `Err` from witness
//! generation (the `CheckPublics` stitch constraint), so it no longer needs
//! the separate per-process test file it had back when a bad fold aborted.
//!
//! Heavy and ignored by default. Run explicitly:
//!
//!     cargo test -p integration-tests --test recurser -- --ignored --nocapture
//!
//! Overrides: `ZISK_TEST_PROVING_KEY=<dir>`, `ZISK_TEST_GPU=1`.

use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use test_artifacts::ELF_RECURSER_CHAIN;
use zisk_sdk::{EmbeddedClientBuilder, ZiskStdin};

#[tokio::test]
#[ignore = "requires a generated proving key; run with --ignored"]
async fn test_recurser_aggregator_chain_full_tree() -> Result<()> {
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
    let client = builder.build().expect("failed to build EmbeddedClient");

    // ROM setup for the leaf program.
    client
        .setup(&ELF_RECURSER_CHAIN)
        .run()
        .context("submit leaf setup")?
        .await
        .context("leaf ROM setup failed")?;

    // Register + set up the chain-fold aggregator: `CheckPublics` enforces
    // `a.new == b.old`, `AggregatePublics` emits `[a.old, b.new]`, and
    // `PreparePublics` defaults to identity.
    let agg = client
        .register_setup_recurser(&[&ELF_RECURSER_CHAIN])
        .aggregate_template(include_str!("recurser/fixtures/aggregate_publics.circom"))
        .check_template(include_str!("recurser/fixtures/check_publics.circom"))
        .run()
        .context("register aggregator")?;
    eprintln!("[recurser] recurser_id = {}", agg.recurser_id());

    client
        .setup(&agg)
        .run()
        .context("submit aggregator setup")?
        .await
        .context("aggregator setup failed (circom/plonk2pil/verkey)")?;

    let agg_vk = agg.vk().context("aggregator VK available after setup")?.vk;

    // Four contiguous leaf segments: 10→20→30→40→50.
    let stdin_a = ZiskStdin::new();
    stdin_a.write(&10u32);
    stdin_a.write(&20u32);
    let pa = client
        .prove(&ELF_RECURSER_CHAIN, stdin_a)
        .run()
        .map_err(|e| anyhow!("submit prove [10,20] failed: {e}"))?
        .await
        .map_err(|e| anyhow!("prove [10,20] failed: {e}"))?
        .get_proof()
        .clone();

    let stdin_b = ZiskStdin::new();
    stdin_b.write(&20u32);
    stdin_b.write(&30u32);
    let pb = client
        .prove(&ELF_RECURSER_CHAIN, stdin_b)
        .run()
        .map_err(|e| anyhow!("submit prove [20,30] failed: {e}"))?
        .await
        .map_err(|e| anyhow!("prove [20,30] failed: {e}"))?
        .get_proof()
        .clone();

    let stdin_c = ZiskStdin::new();
    stdin_c.write(&30u32);
    stdin_c.write(&40u32);
    let pc = client
        .prove(&ELF_RECURSER_CHAIN, stdin_c)
        .run()
        .map_err(|e| anyhow!("submit prove [30,40] failed: {e}"))?
        .await
        .map_err(|e| anyhow!("prove [30,40] failed: {e}"))?
        .get_proof()
        .clone();

    let stdin_d = ZiskStdin::new();
    stdin_d.write(&40u32);
    stdin_d.write(&50u32);
    let pd = client
        .prove(&ELF_RECURSER_CHAIN, stdin_d)
        .run()
        .map_err(|e| anyhow!("submit prove [40,50] failed: {e}"))?
        .await
        .map_err(|e| anyhow!("prove [40,50] failed: {e}"))?
        .get_proof()
        .clone();

    let pubs_a = pa.get_publics().public_u64();
    let pubs_d = pd.get_publics().public_u64();

    assert_eq!((pubs_a[0], pubs_a[1]), (10, 20), "leaf publics must round-trip");
    assert_eq!((pubs_d[0], pubs_d[1]), (40, 50), "leaf publics must round-trip");

    // Level 1: two leaf+leaf folds.
    let ab = client
        .recurser_prove(&agg, &pa, &pb)
        .run()
        .map_err(|e| anyhow!("submit recurser prove ab failed: {e}"))?
        .await
        .map_err(|e| anyhow!("recurser prove ab [10,20]+[20,30] failed: {e}"))?
        .get_proof()
        .clone();

    let cd = client
        .recurser_prove(&agg, &pc, &pd)
        .run()
        .map_err(|e| anyhow!("submit recurser prove cd failed: {e}"))?
        .await
        .map_err(|e| anyhow!("recurser prove cd [30,40]+[40,50] failed: {e}"))?
        .get_proof()
        .clone();

    let pubs_ab = ab.get_publics().public_u64();
    let pubs_cd = cd.get_publics().public_u64();

    assert_eq!((pubs_ab[0], pubs_ab[1]), (10, 30), "leaf+leaf must merge to [a.old, b.new]");
    assert_eq!((pubs_cd[0], pubs_cd[1]), (30, 50), "leaf+leaf must merge to [a.old, b.new]");

    // A leaf+leaf fold stamps the aggregator's own VK as the chain identity (§8 row 1).
    assert_eq!(ab.get_program_vk().vk, agg_vk, "ab chain identity must be rootCRecurserAgg");
    assert_eq!(cd.get_program_vk().vk, agg_vk, "cd chain identity must be rootCRecurserAgg");

    // Level 2: agg+agg fold. CheckPublics sees 30==30; §9 forces ab.VK==cd.VK
    // (both rootCRecurserAgg, so it passes). This is the strongest single
    // assertion that the VK-first layout is correct end-to-end.
    let root = client
        .recurser_prove(&agg, &ab, &cd)
        .run()
        .map_err(|e| anyhow!("submit recurser prove root failed: {e}"))?
        .await
        .map_err(|e| anyhow!("recurser prove root ab+cd failed: {e}"))?
        .get_proof()
        .clone();

    let pubs_root = root.get_publics().public_u64();
    assert_eq!((pubs_root[0], pubs_root[1]), (10, 50), "full tree must collapse to [10, 50]");
    assert_eq!(
        root.get_program_vk().vk,
        agg_vk,
        "chain identity must propagate unchanged up the tree"
    );

    // Negative: non-contiguous segments (pa.new=20 != pc.old=30) must be
    // rejected by the CheckPublics stitch constraint — a clean `Err` out of
    // witness generation, not a process abort, which is why this no longer
    // needs its own per-process test file.
    let broken = client
        .recurser_prove(&agg, &pa, &pc)
        .run()
        .context("submit broken-chain recurser prove")?
        .await;
    assert!(broken.is_err(), "folding [10,20]+[30,40] must fail CheckPublics, got Ok");

    Ok(())
}
