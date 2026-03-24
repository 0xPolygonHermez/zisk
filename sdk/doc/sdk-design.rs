use anyhow::Result;
use std::path::{Path, PathBuf};
use zisk_sdk::VerboseMode::Trace;
use zisk_sdk::{load_program, ElfBinary, ProofOpts, ProverClient, ZiskStdin};

enum Tracing {
    Input,
    Hints,
    Summary,
}

enum Executor {
    Emulator,
    Assembly,
}

struct ProgramId {
    project_name: String,
    program_name: String,
}

pub struct Elf {
    data: &'static [u8],
    hash_id: &'static str,
}

struct GuestProgram {
    program_id: ProgramId,
    elf: Elf,
}

pub static PROGRAM: GuestProgram = load_program!("guest");

fn main() -> Result<()> {
    // Alternative to load the program from a URI (file or http(s)://...)
    let program = GuestProgram::from_uri("http://www.eample.com/guest_program")?;
    let program = GuestProgram::from_bytes("program_name", &program_bytes)?;

    let stdin = ZiskStdin::new();
    let stdin = ZiskStdin::stream("unix://tmp/stdin.sock")?;
    let hints = ZiskHints::stream("unix://tmp/hints.sock")?;
    let hints = ZiskHints::stream("/home/user/hints.bin")?;

    #[derive(Default)]
    struct EmbeddedOptions {
        proving_key: Option<PathBuf>,
    }

    let embedded_client = ProverClient::builder().gpu().build()?;
    // .embedded(EmbeddedOptions::default())

    struct RemoteOptions {
        url: std::net::SocketAddr,
        // API key sent as gRPC metadata ("x-api-key").
        api_key: Option<String>,
        // TLS config for the gRPC connection.
        tls: TlsConfig,
    }

    let remote_options = RemoteOptions::builder().url("localhost:3000").build()?;
    let remote_client = ProverClient::builder()
        .gpu()
        .executor(Executor::Assembly)
        .remote(remote_options)
        .build()?;

    // Client Default
    let client = ProverClient::default(); // defaults to embedded + cpu client

    // Setup
    client
        .upload(&PROGRAM) // Embedded -> it does nothing, as the program is already available at compile time.
        .run()?; // Remote   -> uploads the elf data and the remote registers the program.

    client
        .setup(&PROGRAM) // Embedded -> Executes the ROM setup if it has not been done yet. Enables the program for proving (starts the asm infra).
        .run()?; // Remote   -> Enable the program for proving in the coordinator. If it is already enabled, it does nothing.
                 //             Otherwise, it executes the ROM setup in the workers and enables the program for proving (starts the asm infra).
                 // Setup options
                 // .with_hints()       // Enables the ROM setup with hints

    // --- Basic API (sync) -------------------------------------------------------
    // run() blocks the calling thread until the proof is ready.
    // Internally wraps submit() + proof().await inside a Tokio runtime.
    // No async required from the caller — suitable for most use cases.
    let proof = client.prove(&guest_program, stdin).stark().run()?; // sync: blocks until proof is ready, returns Proof directly

    assert!(proof.verify().is_ok(), "Proof verification failed");
    assert!(
        proof.publics(&pv).verification_key(&vk).verify().is_ok(),
        "Public values and verification key verification failed"
    );

    // --- Advanced API (async) ---------------------------------------------------
    // submit() returns a ProofHandle immediately without blocking.
    // Useful when the caller needs to watch events, prove multiple programs
    // concurrently, or integrate into an existing async runtime.
    // Requires an async context (e.g. #[tokio::main]).

    let on_event = |event: WatchEvent| {
        println!("{:?}", event);
    };

    let handle = client
        .prove(&guest_program, stdin)
        .executor(Executor::Assembly)
        .minimal_memory()
        .stark()
        .hints(hints)
        .timeout(std::time::Duration::from_secs(60))
        .subscribe(WatchEvent::All, on_event) // optional: register event callbacks before submission
        .submit()?; // sync: submits the job, returns a ProofHandle immediately

    let proof = handle.proof().await?; // async: awaits proof completion

    assert!(proof.verify().is_ok(), "Proof verification failed");
    assert!(
        proof.publics(&pv).verification_key(&vk).verify().is_ok(),
        "Public values and verification key verification failed"
    );

    // Execute the program using the `ProverClient.execute` method, without generating a proof.
    let result = client
        .execute(&guest_program, stdin)
        .timeout(std::time::Duration::from_secs(5))
        .trace(Tracing::Input)
        .trace(Tracing::Hints)
        .run()?;

    Ok(())
}
