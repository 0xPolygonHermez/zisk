//! Recurser-aggregator end-to-end: the happy-path full fold tree.
//!
//! Four leaf segments `10→20→30→40→50` collapse to a single `[10, 50]` proof
//! via three folds (two leaf+leaf, one agg+agg). This is the empirical
//! acceptance test for the VK-first publics layout: the agg+agg fold at the
//! root only succeeds if VK extraction, VK emission, and the §9 immutability
//! check all agree the program VK lives in the leading slots.
//!
//! The negative (broken-chain) case is in `recurser_negative.rs` — a separate
//! file so it gets its own process/`ProverClient` (see `recurser/common.rs`).
//!
//! Heavy and ignored by default. Run explicitly:
//!
//!     cargo test -p integration-tests --test recurser -- --ignored --nocapture

#[path = "recurser/common.rs"]
mod common;

use common::{build_client, endpoints, fold, program_vk, prove_segment, setup_aggregator};

#[tokio::test]
#[ignore = "requires a generated proving key; run with --ignored"]
async fn test_recurser_aggregator_chain_full_tree() {
    let client = build_client();
    let agg = setup_aggregator(&client).await;
    let agg_vk = agg.vk().expect("aggregator VK available after setup").vk;

    // Four contiguous leaf segments: 10→20→30→40→50.
    let pa = prove_segment(&client, 10, 20).await.expect("leaf [10,20]");
    let pb = prove_segment(&client, 20, 30).await.expect("leaf [20,30]");
    let pc = prove_segment(&client, 30, 40).await.expect("leaf [30,40]");
    let pd = prove_segment(&client, 40, 50).await.expect("leaf [40,50]");

    assert_eq!(endpoints(&pa), (10, 20), "leaf publics must round-trip");
    assert_eq!(endpoints(&pd), (40, 50), "leaf publics must round-trip");

    // Level 1: two leaf+leaf folds.
    let ab = fold(&client, &agg, &pa, &pb).await.expect("fold [10,20]+[20,30]");
    let cd = fold(&client, &agg, &pc, &pd).await.expect("fold [30,40]+[40,50]");

    assert_eq!(endpoints(&ab), (10, 30), "leaf+leaf must merge to [a.old, b.new]");
    assert_eq!(endpoints(&cd), (30, 50), "leaf+leaf must merge to [a.old, b.new]");

    // A leaf+leaf fold stamps the aggregator's own VK as the chain identity (§8 row 1).
    assert_eq!(program_vk(&ab), agg_vk, "ab chain identity must be rootCRecurserAgg");
    assert_eq!(program_vk(&cd), agg_vk, "cd chain identity must be rootCRecurserAgg");

    // Level 2: agg+agg fold. CheckPublics sees 30==30; §9 forces ab.VK==cd.VK
    // (both rootCRecurserAgg, so it passes). This is the strongest single
    // assertion that the VK-first layout is correct end-to-end.
    let root = fold(&client, &agg, &ab, &cd).await.expect("fold [10,30]+[30,50] (agg+agg)");

    assert_eq!(endpoints(&root), (10, 50), "full tree must collapse to [10, 50]");
    assert_eq!(program_vk(&root), agg_vk, "chain identity must propagate unchanged up the tree");
}
