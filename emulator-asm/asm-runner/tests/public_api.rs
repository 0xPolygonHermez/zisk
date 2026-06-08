//! Integration tests for `asm-runner`'s public API.
//!
//! These exercise the crate as an external consumer would, without the
//! `ziskemuasm` binaries or live shared memory — i.e. the deterministic,
//! binary-free surface: the options/CLI-flag contract, service identity, the
//! ROM-histogram payload type, and `AsmServices::new`'s input validation.

use std::path::Path;
use std::process::Command;

use asm_runner::{AsmRHData, AsmRunnerOptions, AsmRunnerTraceLevel, AsmService, AsmServices};

/// Collect the args `apply_to_command` adds, as owned strings.
fn applied_args(opts: &AsmRunnerOptions, service: AsmService) -> Vec<String> {
    let mut cmd = Command::new("ziskemuasm");
    opts.apply_to_command(&mut cmd, &service, "ZISK_1_0", "ZISK_1_h_0");
    cmd.get_args().map(|a| a.to_string_lossy().into_owned()).collect()
}

#[test]
fn options_builder_and_command_flags_are_consistent() {
    let opts = AsmRunnerOptions::new()
        .with_verbose(true)
        .with_metrics(true)
        .with_trace_level(AsmRunnerTraceLevel::ExtendedTrace);

    let args = applied_args(&opts, AsmService::MO);

    // Mandatory flags + per-service gen index.
    for expected in ["-s", "--gen=7", "--stdio", "--open_all_shm", "--share_input_shm", "-v", "-m", "-tt"] {
        assert!(args.iter().any(|a| a == expected), "missing {expected} in {args:?}");
    }
    // Prefixes are passed as flag/value pairs.
    let i = args.iter().position(|a| a == "--shm_prefix").unwrap();
    assert_eq!(args[i + 1], "ZISK_1_0");
}

#[test]
fn service_identity_is_the_documented_wire_contract() {
    assert_eq!(AsmServices::SERVICES, [AsmService::MO, AsmService::MT, AsmService::RH]);
    assert_eq!(AsmService::MO.gen_index(), 7);
    assert_eq!(AsmService::MT.gen_index(), 1);
    assert_eq!(AsmService::RH.gen_index(), 2);
    assert_eq!(AsmService::RH.as_str(), "RH");
    assert_eq!(AsmService::RH.command_path_for("/opt/zisk/ziskemuasm"), "/opt/zisk/ziskemuasm-rh.bin");
}

#[test]
fn rom_histogram_payload_round_trips_through_public_api() {
    let data = AsmRHData::new(123, vec![1, 2, 3, 4]);
    assert_eq!(data.steps, 123);
    assert_eq!(data.inst_count, vec![1, 2, 3, 4]);
}

#[test]
fn asm_services_new_rejects_paths_without_the_bin_suffix() {
    let opts = AsmRunnerOptions::new();
    // Too short to carry a "-??.bin" suffix → rejected before any process spawn.
    assert!(AsmServices::new(0, 0, "deadbeef".into(), Path::new("foo"), false, opts.clone()).is_err());
    // Long enough and ends in ".bin", but missing the "-" before the service id.
    assert!(
        AsmServices::new(0, 0, "deadbeef".into(), Path::new("abcdefg.bin"), false, opts.clone())
            .is_err()
    );
    // Right length but wrong extension.
    assert!(
        AsmServices::new(0, 0, "deadbeef".into(), Path::new("/x/binary.txt"), false, opts).is_err()
    );
}
