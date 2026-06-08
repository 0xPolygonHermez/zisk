//! End-to-end smoke tests that run the built `cargo-zisk` / `cargo-zisk-dev`
//! binaries. These exercise the binary entry points, clap dispatch, and the
//! generated help/version/error output — paths that in-crate unit tests can't
//! reach. They intentionally avoid commands that spawn provers/toolchains or
//! touch the network, so they stay fast and hermetic.

use assert_cmd::Command;
use predicates::prelude::*;

// Use the compile-time `CARGO_BIN_EXE_<name>` paths rather than
// `assert_cmd::cargo_bin`: the latter can't locate the binary when the test is
// run under `cargo llvm-cov` (different target dir), whereas the env vars are
// set correctly by cargo in both cases.
fn cargo_zisk() -> Command {
    Command::new(env!("CARGO_BIN_EXE_cargo-zisk"))
}

fn cargo_zisk_dev() -> Command {
    Command::new(env!("CARGO_BIN_EXE_cargo-zisk-dev"))
}

#[test]
fn user_version_succeeds() {
    cargo_zisk().arg("--version").assert().success();
}

#[test]
fn user_help_lists_command_groups() {
    cargo_zisk()
        .arg("--help")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("remote")
                .and(predicate::str::contains("setup"))
                .and(predicate::str::contains("prove"))
                .and(predicate::str::contains("execute"))
                .and(predicate::str::contains("wrap")),
        );
}

#[test]
fn dev_help_succeeds() {
    cargo_zisk_dev().arg("--help").assert().success();
}

#[test]
fn dev_version_succeeds() {
    cargo_zisk_dev().arg("--version").assert().success();
}

#[test]
fn unknown_command_fails() {
    cargo_zisk().arg("definitely-not-a-command").assert().failure();
}

#[test]
fn conflicting_proof_flags_fail_with_message() {
    cargo_zisk()
        .args(["prove", "--minimal", "--plonk"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn embedded_prove_help_succeeds() {
    cargo_zisk().args(["prove", "--help"]).assert().success();
}

#[test]
fn dev_prove_help_succeeds() {
    // Regression guard: `prove`'s clap model previously had a duplicate `-a`
    // short that panicked at runtime; rendering its help would have aborted.
    cargo_zisk_dev().args(["prove", "--help"]).assert().success();
}

#[test]
fn dev_stats_help_succeeds() {
    // Same regression guard for the `stats` command's duplicate `-a` short.
    cargo_zisk_dev().args(["stats", "--help"]).assert().success();
}

#[test]
fn utils_convert_input_end_to_end() {
    // Drives a real, hermetic command all the way through the dispatch chain
    // (ZiskCliCmd → SharedCmd → UtilsCmd → ConvertInput), no prover/network.
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.bin");
    let output = dir.path().join("out.bin");
    std::fs::write(&input, b"hello world").unwrap();

    cargo_zisk()
        .args(["utils", "convert-input", "-i"])
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .assert()
        .success();

    assert!(output.exists(), "converted output should be written");
}

#[test]
fn verify_missing_proof_fails_cleanly() {
    // Dispatches through SharedCmd → VerifyCmd::run, which fails to load the
    // (nonexistent) proof. Hermetic: just a missing file.
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("nope.proof");
    cargo_zisk().args(["verify", "-p"]).arg(&missing).assert().failure();
}
