//! Recurser example — an L2 folding three contiguous block-range proofs into
//! one. Each leaf attests a `BlocksInfoStruct`; the recurser checks contiguity
//! and merges the ranges. See the crate README for the full walkthrough.
//!
//! Run: `cargo run --release -p recurser-l2-host`
//!
//! Set `ZISK_L2_PLONK=1` to also wrap the final folded proof into a PLONK/SNARK
//! and verify it (heavy — needs the `provingKeySnark` artifacts).

use std::error::Error;

use alloy_sol_types::SolValue;
use proofman_util::{timer_start_info, timer_stop_and_log_info};
use recurser_l2_common::{segment, BlocksInfoStruct};
use zisk_sdk::{
    load_aggregation_program, load_program, AggregationProgram, GuestProgram, ProofKind,
    ProverClient, ZiskStdin,
};

static LEAF: GuestProgram = load_program!("recurser_l2_guest");
/// A different guest (different programVK) — NOT on the aggregation's allow-list.
static FOREIGN: GuestProgram = load_program!("recurser_l2_foreign");
static AGG_L2: AggregationProgram = load_aggregation_program!("l2");

async fn prove_segment(
    client: &zisk_sdk::EmbeddedClient,
    seg: &BlocksInfoStruct,
) -> Result<zisk_sdk::Proof, Box<dyn Error>> {
    let stdin = ZiskStdin::new();
    stdin.write_slice(&seg.abi_encode());
    let proof = client.prove(&LEAF, stdin).run()?.await?.get_proof().clone();
    Ok(proof)
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    // PLONK must be enabled at client-build time, so opt in up front.
    let plonk = std::env::var_os("ZISK_L2_PLONK").is_some();
    let mut builder = ProverClient::embedded();
    if plonk {
        builder = builder.plonk();
    }
    let client = builder.build()?;

    client.setup(&LEAF).run()?.await?;
    client.setup(&FOREIGN).run()?.await?;
    client.setup(&*AGG_L2).run()?.await?;

    // Three contiguous ranges: [100,200) -> [200,300) -> [300,400).
    let seg_a = segment(100, 200);
    let seg_b = segment(200, 300);
    let seg_c = segment(300, 400);
    let pa = prove_segment(&client, &seg_a).await?;
    let pb = prove_segment(&client, &seg_b).await?;
    let pc = prove_segment(&client, &seg_c).await?;

    // Fold (A + B) then (AB + C). n_free = 0, so a plain &Proof suffices.
    timer_start_info!(AGG_L2_ABC);
    let ab = client.aggregate_proofs(&AGG_L2, &pa, &pb).run()?.await?.get_proof().clone();
    let abc = client.aggregate_proofs(&AGG_L2, &ab, &pc).run()?.await?.get_proof().clone();

    // The collapsed publics decode back into one BlocksInfoStruct.
    let folded: BlocksInfoStruct = abc.get_publics().read_abi()?;
    assert_eq!(folded.startBlock, seg_a.startBlock, "start must come from the oldest segment");
    assert_eq!(folded.endBlock, seg_c.endBlock, "end must come from the newest segment");
    assert_eq!(folded.globalExitRoot, seg_c.globalExitRoot, "post-state root from newest segment");
    assert_eq!(folded.oldGlobalExitRoot, seg_a.oldGlobalExitRoot, "pre-state root from oldest");
    println!("Folded three ranges into [{}, {}).", folded.startBlock, folded.endBlock);
    timer_stop_and_log_info!(AGG_L2_ABC);

    println!("Verifying the final folded proof...");
    abc.verify()?;
    println!("Final folded proof verified successfully.");
    println!("Testing invalid folds...");
    // Non-contiguous fold (A then C, skipping B) must fail the stitch.
    let broken = client.aggregate_proofs(&AGG_L2, &pa, &pc).run()?.await;
    assert!(broken.is_err(), "folding [100,200) + [300,400) must fail the contiguity stitch");
    println!("Non-contiguous fold correctly rejected.");

    // Allow-list: a proof from a non-listed guest (different programVK) makes the
    // circuit unsatisfiable, so the fold is rejected.
    let foreign_stdin = ZiskStdin::new();
    let foreign = client.prove(&FOREIGN, foreign_stdin).run()?.await?.get_proof().clone();
    let rejected = client.aggregate_proofs(&AGG_L2, &foreign, &pb).run()?.await;
    assert!(rejected.is_err(), "folding a non-allow-listed leaf must be rejected");
    println!("Foreign programVK correctly rejected by the allow-list.");

    // Optional (ZISK_L2_PLONK=1): wrap the folded proof into a PLONK/SNARK.
    if plonk {
        timer_start_info!(WRAP_PLONK);
        let plonk_proof =
            client.wrap_proof(&abc, ProofKind::Plonk).run()?.await?.get_proof().clone();
        timer_stop_and_log_info!(WRAP_PLONK);
        plonk_proof.verify()?;
        println!("PLONK proof verified.");
    }

    Ok(())
}
