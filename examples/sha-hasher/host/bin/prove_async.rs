use anyhow::Result;
use sha2::{Digest, Sha256};
use sha_hasher_host::Output;
use zisk_sdk::{
    load_program, ExecutorKind, GuestProgram, ProverClient, ProverOpts, ZiskProofWithPublicValues,
    ZiskPublics, ZiskStdin,
};

static PROGRAM: GuestProgram = load_program!("sha-hasher-guest");

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting ZisK Prover Client (async)...");

    let n = 1000u32;
    let stdin = ZiskStdin::new();
    stdin.write(&n);
    println!("Input prepared: {} iterations", n);

    println!("Building prover client...");
    let proof_opts = ProverOpts::default().minimal_memory();
    let client =
        ProverClient::embedded().with_prover_options(proof_opts).gpu().assembly().build()?;

    println!("Setting up program...");
    client.setup(&PROGRAM).run()?;
    println!("Setup completed successfully");

    println!("Submitting proof (non-blocking)...");
    let handle = client.prove(&PROGRAM, stdin).executor(ExecutorKind::Assembly).run_async()?;
    println!("Proof submitted — handle returned immediately");

    println!("Awaiting proof...");
    let result = handle.proof().await?;
    println!("Proof generated successfully in {:?}", result.get_duration());
    println!("Execution steps: {}", result.get_execution_steps());

    println!("Verifying proof...");
    result.verify()?;
    println!("Proof verification successful!");

    println!("Saving proof to disk...");
    result.save_proof("tmp/sha_hasher_proof_async.bin")?;
    println!("Proofs saved to tmp/ directory");

    let mut hash = [0u8; 32];
    for _ in 0..n {
        let mut hasher = Sha256::new();
        hasher.update(hash);
        let digest = &hasher.finalize();
        hash = Into::<[u8; 32]>::into(*digest);
    }

    let output = Output { hash: hash.into(), iterations: n, magic_number: 0xDEADBEEF };
    println!("Expected output hash: {:02x?}", &hash[..8]);

    println!("Verifying saved proofs from disk...");
    let publics = ZiskPublics::write_abi(&output)?;
    let vk = client.vk(&PROGRAM)?;

    let proof_with_publics = ZiskProofWithPublicValues::load("tmp/sha_hasher_proof_async.bin")?;
    proof_with_publics.with_program_vk(&vk).with_publics(&publics).verify()?;
    println!("Proof verification successful!");

    println!("\u{2713} Successfully generated and verified all proofs!");

    Ok(())
}
