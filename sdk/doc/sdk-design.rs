use anyhow::Result;
use std::path::{Path, PathBuf};
use zisk_sdk::VerboseMode::Trace;
use zisk_sdk::{include_elf, ElfBinary, ProofOpts, ProverClient, ZiskStdin};

enum Tracing {
    Input,
    Hints,
    Summary,
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

    #[derive(Default)]
    struct RemoteOptions {
        url: std::net::SocketAddr,
        auth_token: Option<String>, // ?????
    }

    let remote_options = RemoteOptions::builder().url("localhost:3000").build()?;
    let remote_client = ProverClient::builder().gpu().executor(Executor::Assembly).remote(remote_options).build()?;

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

    let handle = client.prove(&guest_program, stdin).stark().submit()?;

    let proof = handle.proof().await?;

    assert!(proof.verify().is_ok(), "Proof verification failed");
    assert!(
        proof.publics(&pv).verification_key(&vk).verify().is_ok(),
        "Public values and verification key verification failed"
    );

    enum Executor {
        Emulator,
        Assembly,
    }

    let on_event = |event: WatchEvent| {
        println!("{:?}", event);
    };

    let handle = client
        .executor(Executor::Assembly)
        .minimal_memory()
        .prove(&guest_program, stdin)
        .stark()
        .hints(hints)
        .timeout(std::time::Duration::from_secs(60))
        .subscribe(WatchEvent::All, on_event)
        .submit()?; // sync: submits the job, returns a handle immediately

    let proof = handle.proof().await?;

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
