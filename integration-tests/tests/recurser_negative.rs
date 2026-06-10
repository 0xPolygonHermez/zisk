//! Recurser-aggregator end-to-end: the broken-chain rejection case.
//!
//! Folding two non-contiguous segments (`a.new != b.old`) must fail — the
//! `CheckPublics` stitch constraint rejects it. In its own test file so it gets
//! a fresh process/`ProverClient` (the SDK enforces a per-process singleton;
//! see `recurser/common.rs`), and so the abort-prone prover failure here can't
//! take down the happy-path tree test in `recurser.rs`.
//!
//! Heavy and ignored by default. Run explicitly:
//!
//!     cargo test -p integration-tests --test recurser_negative -- --ignored --nocapture

#[path = "recurser/common.rs"]
mod common;

use common::{build_client, prove_segment, setup_aggregator};

#[tokio::test]
#[ignore = "requires a generated proving key; run with --ignored"]
async fn test_recurser_aggregator_rejects_broken_chain() {
    let client = build_client();
    let agg = setup_aggregator(&client).await;

    // Non-contiguous segments: a.new = 20 != b.old = 30. CheckPublics must reject.
    let pa = prove_segment(&client, 10, 20).await.expect("leaf [10,20]");
    let pc = prove_segment(&client, 30, 40).await.expect("leaf [30,40]");

    let outcome = client.aggregate_proof(&agg, &pa, &pc).run().expect("submit aggregate").await;
    assert!(
        outcome.is_err(),
        "folding a broken chain (a.new=20 != b.old=30) must fail CheckPublics, got Ok"
    );
}
