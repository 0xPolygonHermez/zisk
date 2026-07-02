use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use test_artifacts::{AGG_CHAIN, ELF_CHAIN_SEGMENT};
use zisk_sdk::{
    AggregationInput, AggregationProgramBuilder, CircomCircuit, EmbeddedClient,
    EmbeddedClientBuilder, GuestProgram, JobHandle, ProveResult, ProverClient, Recurser,
    RemoteClient, SetupTarget, UploadTarget, ZiskStdin,
};

/// Embedded and remote clients expose the same inherent surface but share no
/// public trait, so the test dispatches through this enum. Set
/// `ZISK_TEST_REMOTE_URL` (e.g. `http://127.0.0.1:7000`) to prove against a
/// live coordinator; otherwise the test proves in-process. The embedded-only
/// knobs (`ZISK_TEST_GPU`, `ZISK_TEST_PROVING_KEY`) are worker-side concerns
/// on remote and are ignored there.
enum TestClient {
    Embedded(EmbeddedClient),
    Remote(RemoteClient),
}

macro_rules! with_client {
    ($self:expr, $c:ident => $e:expr) => {
        match $self {
            TestClient::Embedded($c) => $e,
            TestClient::Remote($c) => $e,
        }
    };
}

impl TestClient {
    fn from_env() -> Result<Self> {
        if let Ok(url) = std::env::var("ZISK_TEST_REMOTE_URL") {
            eprintln!("[recurser] ZISK_TEST_REMOTE_URL={url} — proving via remote coordinator");
            return Ok(Self::Remote(ProverClient::remote(url).build()?));
        }
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
        Ok(Self::Embedded(builder.build().context("failed to build EmbeddedClient")?))
    }

    /// Upload (registration the coordinator requires; a no-op embedded), then
    /// run setup to completion.
    async fn setup<'a, T>(&'a self, target: T) -> Result<()>
    where
        T: Into<SetupTarget<'a>> + Into<UploadTarget<'a>> + Copy,
    {
        with_client!(self, c => {
            c.upload(target).run().context("upload")?;
            c.setup(target).run().context("submit setup")?.await?;
        });
        Ok(())
    }

    fn prove(&self, program: &GuestProgram, stdin: ZiskStdin) -> Result<JobHandle<ProveResult>> {
        with_client!(self, c => Ok(c.prove(program, stdin).run()?))
    }

    fn aggregate_proofs<'a>(
        &'a self,
        agg: &'a Recurser,
        input_a: impl Into<AggregationInput<'a>>,
        input_b: impl Into<AggregationInput<'a>>,
    ) -> Result<JobHandle<ProveResult>> {
        with_client!(self, c => Ok(c.aggregate_proofs(agg, input_a, input_b).run()?))
    }
}

// Multi-thread flavor: the remote client's sync entry points use
// `tokio::task::block_in_place`, which panics on a current-thread runtime.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires a generated proving key; run with --ignored"]
async fn test_recurser_aggregator_chain_full_tree() -> Result<()> {
    let client = TestClient::from_env()?;

    // ROM setup for the leaf program.
    client.setup(&ELF_CHAIN_SEGMENT).await.context("leaf ROM setup failed")?;

    let agg: &Recurser = &AGG_CHAIN;
    eprintln!("[recurser] recurser_id = {}", agg.recurser_id());

    client.setup(agg).await.context("aggregator setup failed (circom/plonk2pil/verkey)")?;

    let agg_vk = agg.vk().context("aggregator VK available after setup")?.vk;

    // The programmatic builder must derive the same id as the declarative TOML.
    let circuits_dir =
        concat!(env!("CARGO_MANIFEST_DIR"), "/../test-artifacts/programs/aggregations/circuits");
    let programmatic = AggregationProgramBuilder::new(CircomCircuit::from_path(format!(
        "{circuits_dir}/aggregate_publics.circom"
    ))?)
    .build()
    .context("programmatic AggregationProgramBuilder build failed")?;
    assert_eq!(
        programmatic.recurser_id(),
        agg.recurser_id(),
        "programmatic and declarative definitions must derive the same recurser_id"
    );

    // Four contiguous leaf segments: 10→20→30→40→50.
    let stdin_a = ZiskStdin::new();
    stdin_a.write(&10u32);
    stdin_a.write(&20u32);
    let pa = client
        .prove(&ELF_CHAIN_SEGMENT, stdin_a)
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
        .map_err(|e| anyhow!("submit prove [40,50] failed: {e}"))?
        .await
        .map_err(|e| anyhow!("prove [40,50] failed: {e}"))?
        .get_proof()
        .clone();

    let pubs_a = pa.get_publics().public_u64();
    let pubs_d = pd.get_publics().public_u64();

    assert_eq!((pubs_a[0], pubs_a[1]), (10, 20), "leaf publics must round-trip");
    assert_eq!((pubs_d[0], pubs_d[1]), (40, 50), "leaf publics must round-trip");

    // Level 1: two leaf+leaf folds. The recurser takes no free inputs, so the
    // proofs are passed plain.
    let ab = client
        .aggregate_proofs(agg, &pa, &pb)
        .map_err(|e| anyhow!("submit recurser prove ab failed: {e}"))?
        .await
        .map_err(|e| anyhow!("recurser prove ab [10,20]+[20,30] failed: {e}"))?
        .get_proof()
        .clone();

    let cd = client
        .aggregate_proofs(agg, &pc, &pd)
        .map_err(|e| anyhow!("submit recurser prove cd failed: {e}"))?
        .await
        .map_err(|e| anyhow!("recurser prove cd [30,40]+[40,50] failed: {e}"))?
        .get_proof()
        .clone();

    let pubs_ab = ab.get_publics().public_u64();
    let pubs_cd = cd.get_publics().public_u64();

    assert_eq!((pubs_ab[0], pubs_ab[1]), (10, 30), "leaf+leaf must merge to [a.old, b.new]");
    assert_eq!((pubs_cd[0], pubs_cd[1]), (30, 50), "leaf+leaf must merge to [a.old, b.new]");

    // The aggregate circuit zero-fills slots [2, 64), so the merged publics keep
    // the well-formed all-zero tail their inputs had (essential: an aggregated
    // proof is fed straight back into the next fold level).
    assert_eq!(&pubs_ab[2..6], &[0u64; 4], "merged tail must stay zero");
    assert_eq!(&pubs_cd[2..6], &[0u64; 4], "merged tail must stay zero");

    // A leaf+leaf fold stamps the aggregator's own VK as the chain identity (§6 row 1).
    assert_eq!(ab.get_program_vk().vk, agg_vk, "ab chain identity must be rootCRecurserAgg");
    assert_eq!(cd.get_program_vk().vk, agg_vk, "cd chain identity must be rootCRecurserAgg");

    // Level 2: agg+agg fold. AggregatePublics sees 30==30; §7 forces ab.VK==cd.VK
    // (both rootCRecurserAgg, so it passes). This is the strongest single
    // assertion that the VK-first layout is correct end-to-end.
    let root = client
        .aggregate_proofs(agg, &ab, &cd)
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
    assert_eq!(&pubs_root[2..6], &[0u64; 4], "tail must stay zero to the root");

    // Opt-in: persist the root VadcopFinal proof so it can be fed into a
    // separate PLONK wrap. Set ZISK_TEST_SAVE_ROOT_PROOF=/path/to/root.bin.
    if let Some(path) = std::env::var_os("ZISK_TEST_SAVE_ROOT_PROOF") {
        let path = PathBuf::from(path);
        root.save(&path)
            .with_context(|| format!("failed to save root proof to {}", path.display()))?;
        eprintln!("[recurser] saved root proof to {}", path.display());
    }

    // Negative: non-contiguous segments (pa.new=20 != pc.old=30) must be
    // rejected by the AggregatePublics stitch constraint — a clean `Err` out of
    // witness generation, not a process abort. On remote the same failure comes
    // back as a failed job through the JobHandle.
    let broken =
        client.aggregate_proofs(agg, &pa, &pc).context("submit broken-chain recurser prove")?.await;
    assert!(
        broken.is_err(),
        "folding [10,20]+[30,40] must fail the AggregatePublics stitch, got Ok"
    );

    Ok(())
}
