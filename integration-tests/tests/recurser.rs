use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use test_artifacts::{AGG_CHAIN, ELF_CHAIN_SEGMENT};
use zisk_sdk::{EmbeddedClientBuilder, ProofExt, Recurser, ZiskStdin};

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
        .setup(&ELF_CHAIN_SEGMENT)
        .run()
        .context("submit leaf setup")?
        .await
        .context("leaf ROM setup failed")?;

    let agg: &Recurser = &AGG_CHAIN;
    eprintln!("[recurser] recurser_id = {}", agg.recurser_id());

    client
        .setup(agg)
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
        .prove(&ELF_CHAIN_SEGMENT, stdin_a)
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
        .prove(&ELF_CHAIN_SEGMENT, stdin_b)
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
        .prove(&ELF_CHAIN_SEGMENT, stdin_c)
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
        .prove(&ELF_CHAIN_SEGMENT, stdin_d)
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
        .aggregate_proofs(agg, pa.with_free_inputs(vec![4u64]), pb.with_free_inputs(vec![4u64]))
        .run()
        .map_err(|e| anyhow!("submit recurser prove ab failed: {e}"))?
        .await
        .map_err(|e| anyhow!("recurser prove ab [10,20]+[20,30] failed: {e}"))?
        .get_proof()
        .clone();

    let cd = client
        .aggregate_proofs(agg, pc.with_free_inputs(vec![4u64]), pd.with_free_inputs(vec![4u64]))
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

    // NormalizePublics hashed [1, 2, 3, 4] on both leaves, so slots [2..6) hold
    // the Poseidon1_8 digest. Both folds hashed the same tuple, so both carry
    // the same (non-zero) digest. AggregatePublics already enforced A==B in-circuit;
    // here we confirm it surfaced in the public output and is genuinely a hash
    // (not zeros, which untouched slots would have carried).
    let digest = &pubs_ab[2..6];
    assert_ne!(digest, &[0u64; 4], "NormalizePublics must write a non-zero hash digest to [2..6)");
    assert_eq!(&pubs_cd[2..6], digest, "both leaf+leaf folds must carry the same digest");

    // A leaf+leaf fold stamps the aggregator's own VK as the chain identity (§7 row 1).
    assert_eq!(ab.get_program_vk().vk, agg_vk, "ab chain identity must be rootCRecurserAgg");
    assert_eq!(cd.get_program_vk().vk, agg_vk, "cd chain identity must be rootCRecurserAgg");

    // Level 2: agg+agg fold. AggregatePublics sees 30==30; §8 forces ab.VK==cd.VK
    // (both rootCRecurserAgg, so it passes). This is the strongest single
    // assertion that the VK-first layout is correct end-to-end.
    let root = client
        .aggregate_proofs(agg, &ab, &cd)
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
    assert_eq!(&pubs_root[2..6], digest, "hash digest must propagate unchanged to the root");

    // Negative: non-contiguous segments (pa.new=20 != pc.old=30) must be
    // rejected by the AggregatePublics stitch constraint — a clean `Err` out of
    // witness generation, not a process abort, which is why this no longer
    // needs its own per-process test file.
    let broken = client
        .aggregate_proofs(agg, pa.with_free_inputs(vec![4u64]), pc.with_free_inputs(vec![4u64]))
        .run()
        .context("submit broken-chain recurser prove")?
        .await;
    assert!(
        broken.is_err(),
        "folding [10,20]+[30,40] must fail the AggregatePublics stitch, got Ok"
    );

    let broken2 = client
        .aggregate_proofs(agg, pa.with_free_inputs(vec![3u64]), pb.with_free_inputs(vec![4u64]))
        .run()
        .context("submit broken-chain recurser prove")?
        .await;
    assert!(
        broken2.is_err(),
        "mismatched free inputs (A=3 vs B=4) must fail the AggregatePublics stitch, got Ok"
    );

    Ok(())
}
