//! Two execute-only ASM clients in one process, each with its own program, taking turns.
//!
//! Every client in a process shares one set of shared-memory segments for the guest's RAM and
//! ROM, so a client must hand them back to its own program before each run: another client's
//! program may have run on them in between. This is the standalone path's half of what
//! `asm_two_programs` checks for the full client.
//!
//! It drives `zisk-prover-backend`'s clients directly: the SDK allows one client per process,
//! but the backend's standalone executors may share one (see `AsmResources::new_standalone`).
//!
//! Each run is checked against the Rust emulator, which shares nothing with the assembly
//! services.
//!
//! Linux-only, for the same reason as `asm_job_boundary`. Ignored by default: it starts the ASM
//! microservices and generates `big_rom`'s assembly, which takes about a minute. It needs no
//! proving key. Run it with:
//!
//!     cargo test -p integration-tests --test asm_two_execute_clients -- --ignored --nocapture

#![cfg(target_os = "linux")]

use zisk_common::io::ZiskStdin;
use zisk_prover_backend::{AsmExecClient, EmuExecClient, ExecuteClient, GuestProgram, VerboseMode};
use zisk_test_artifacts::{ELF_BIG_ROM, ELF_FIB_MOD};

const FIB_N: u32 = 1_000;
const FIB_MODULE: u32 = 233;
const BIG_ROM_SEED: u64 = 0x5eed;

fn fib_mod_input() -> ZiskStdin {
    let stdin = ZiskStdin::new();
    stdin.write(&FIB_N);
    stdin.write(&FIB_MODULE);
    stdin
}

fn big_rom_input() -> ZiskStdin {
    let stdin = ZiskStdin::new();
    stdin.write(&BIG_ROM_SEED);
    stdin
}

fn assembly_client_for(program: &GuestProgram) -> AsmExecClient {
    let client = AsmExecClient::new(VerboseMode::Info, None, false)
        .expect("failed to build the assembly client");
    client.setup(program, false).expect("setup failed");
    client
}

/// One execution of `program`, returning the public values it committed.
fn run_once(
    client: &dyn ExecuteClient,
    program: &GuestProgram,
    name: &str,
    stdin: ZiskStdin,
) -> Vec<u64> {
    client
        .execute(program, stdin, None)
        .unwrap_or_else(|e| panic!("execute of {name} failed: {e:#}"))
        .get_publics()
        .public_u64()
}

#[test]
#[ignore = "starts the ASM microservices; run with --ignored"]
fn two_execute_only_clients_in_one_process() {
    // The small program first, so its client's setup creates the shared segments.
    let fib_client = assembly_client_for(&ELF_FIB_MOD);
    let big_client = assembly_client_for(&ELF_BIG_ROM);

    // big_rom's client was set up last, so fib_mod's must take the segments back first.
    let fib = run_once(&fib_client, &ELF_FIB_MOD, "fib_mod", fib_mod_input());
    let big = run_once(&big_client, &ELF_BIG_ROM, "big_rom", big_rom_input());
    let fib_again = run_once(&fib_client, &ELF_FIB_MOD, "fib_mod", fib_mod_input());

    let emulator = EmuExecClient::new(VerboseMode::Info).expect("failed to build the emulator");
    ExecuteClient::setup(&emulator, &ELF_FIB_MOD, false).expect("emulator setup failed");
    let fib_emulated = run_once(&emulator, &ELF_FIB_MOD, "fib_mod", fib_mod_input());
    ExecuteClient::setup(&emulator, &ELF_BIG_ROM, false).expect("emulator setup failed");
    let big_emulated = run_once(&emulator, &ELF_BIG_ROM, "big_rom", big_rom_input());

    assert_eq!(
        fib, fib_emulated,
        "fib_mod committed other public values on assembly than on the emulator"
    );
    assert_eq!(
        big, big_emulated,
        "big_rom committed other public values on assembly than on the emulator"
    );
    assert_eq!(
        fib_again, fib,
        "fib_mod committed other public values after big_rom ran in between"
    );
}
