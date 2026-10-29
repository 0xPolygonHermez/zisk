//! End-to-end proof of a WebAssembly guest on the Emulator executor: proves `wasm-fibonacci`
//! (`examples/wasm-fibonacci`) for n = 90, then checks that
//!
//! - the proof verifies under the program verification key of the wasm module itself, so it is
//!   bound to that program;
//! - it does not verify under any other program key;
//! - the public output is the guest's stdout.
//!
//! Lives in its own file because the SDK allows one `ProverClient` per process (see
//! `diagnostic_embedded.rs`). Ignored by default because proving requires a generated proving key
//! on disk and takes a few minutes. Run with:
//!
//!     cargo test --release -p integration-tests --test wasm_prove -- --ignored --nocapture

use std::path::PathBuf;

use zisk_sdk::{EmbeddedClientBuilder, ExecutorKind, GuestProgram, ZiskStdin};

const WASM_FIBONACCI: &[u8] = include_bytes!("../../emulator/tests/data/wasm-fibonacci.wasm");

#[tokio::test]
#[ignore = "requires a generated proving key; run with --ignored"]
async fn wasm_fibonacci_proves_and_verifies() {
    let mut builder = EmbeddedClientBuilder::default().emulator();
    if let Some(pk) = std::env::var_os("ZISK_TEST_PROVING_KEY").map(PathBuf::from) {
        eprintln!("[wasm_prove] using ZISK_TEST_PROVING_KEY={}", pk.display());
        builder = builder.proving_key(pk);
    }
    let client = builder.build().expect("failed to build EmbeddedClient");

    let program = GuestProgram::from_bytes("wasm-fibonacci", WASM_FIBONACCI.to_vec());
    client.setup(&program).run().expect("failed to submit setup").await.expect("ROM setup failed");

    let result = client
        .prove(&program, ZiskStdin::from_bytes(90u64.to_le_bytes().to_vec()))
        .executor(ExecutorKind::Emulator)
        .run()
        .expect("failed to submit prove")
        .await
        .expect("prove failed");

    let mut publics = [0u8; 256];
    result.get_public_values_slice(&mut publics);
    let expected = b"fib(90) = 2880067194370816120\n";
    assert_eq!(&publics[..expected.len()], expected);
    assert!(publics[expected.len()..].iter().all(|&b| b == 0), "unexpected trailing output");

    let proof = result.get_proof();
    let program_vk = program.vk().expect("program verification key");
    proof.with_program_vk(&program_vk).verify().expect("proof must verify under the program's key");

    let mut other_vk = program_vk.clone();
    other_vk.vk[0] ^= 1;
    assert!(
        proof.with_program_vk(&other_vk).verify().is_err(),
        "proof must not verify under another program's key"
    );
}
