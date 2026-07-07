//! Circom **compile** smoke test for the generated aggregator.
//!
//! The snapshot tests only string-match the rendered output; they cannot catch a
//! declared-but-unassigned (T3001) or double-assigned (T2001) signal. This test
//! invokes `circom` on the aggregator's own body, substituting stub includes and
//! a stub verifier for the pil2 STARK tree (unavailable in a unit test) so the
//! flag/allow-list/mux/output-slot constraints compile for real.
//!
//! Skipped (not failed) when the `circom` binary is absent.

use std::fs;
use std::process::Command;

use recurser::{gen_recurser, templates::StarkInputBlocks, CircomTemplates, NormalizeCircuit};

mod common;
use common::{AGGREGATE_0_FREE, AGGREGATE_1_FREE, NORMALIZE_1_FREE};

/// The include filename the aggregator emits (`include "<verifier_filename>"`).
const STUB_VERIFIER_FILENAME: &str = "stub_verifier.circom";

/// Stub STARK verifier: exposes only what the aggregator touches — `publics[69]`
/// and a `rootC[4]` input driven by its rootC mux.
const STUB_VERIFIER: &str = r#"pragma circom 2.1.0;
template StubVerifier() {
    signal input publics[69];
    signal input rootC[4];
    // Consume inputs so circom does not warn/error on unused signals.
    signal sink;
    var acc = 0;
    for (var i = 0; i < 69; i++) { acc += publics[i]; }
    for (var i = 0; i < 4; i++) { acc += rootC[i]; }
    sink <== acc;
}"#;

/// Stub `mux1.circom`: a real, sound linear MultiMux1 (sel in {0,1}).
const STUB_MUX1: &str = r#"pragma circom 2.1.0;
// Matches the aggregator's call `MultiMux1(n)([choice0, choice1], s)`:
// two n-wide choices, selector s in {0,1}. Linear and genuinely sound.
template MultiMux1(n) {
    signal input c[2][n];
    signal input s;
    signal output out[n];
    for (var i = 0; i < n; i++) {
        out[i] <== (c[1][i] - c[0][i]) * s + c[0][i];
    }
}"#;

/// Stub `iszero.circom` (only pulled in when an allow-list is configured).
const STUB_ISZERO: &str = r#"pragma circom 2.1.0;
template IsZero() {
    signal input in;
    signal output out;
    signal inv;
    inv <-- in != 0 ? 1 / in : 0;
    out <== -in * inv + 1;
    in * out === 0;
}"#;

/// Empty stub: the aggregator body calls none of its templates.
const STUB_PUBLICS_HELPERS: &str = r#"pragma circom 2.1.0;"#;

/// Declares what the aggregator reads (`a_sv_publics`/`b_sv_publics`) and
/// instantiates the stub verifier so its `vA.rootC`/`vB.rootC` writes resolve.
fn stub_stark() -> StarkInputBlocks<'static> {
    StarkInputBlocks {
        define_a: "    signal input a_sv_publics[69];\n    component vA = StubVerifier();",
        define_b: "    signal input b_sv_publics[69];\n    component vB = StubVerifier();",
        assign_a: "    for (var i = 0; i < 69; i++) { vA.publics[i] <== a_sv_publics[i]; }",
        assign_b: "    for (var i = 0; i < 69; i++) { vB.publics[i] <== b_sv_publics[i]; }",
    }
}

fn zisk_vk() -> [String; 4] {
    ["1".into(), "2".into(), "3".into(), "4".into()]
}

fn program_vks() -> Vec<[String; 4]> {
    vec![
        ["10".into(), "11".into(), "12".into(), "13".into()],
        ["20".into(), "21".into(), "22".into(), "23".into()],
    ]
}

/// True if a `circom` binary is on PATH.
fn circom_available() -> bool {
    Command::new("circom").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

/// Render the aggregator, drop it plus stub includes into a fresh temp dir, and
/// run `circom --r1cs`. Panics with circom's stderr on a compile error.
fn assert_compiles(label: &str, templates: &CircomTemplates) {
    let out = gen_recurser(STUB_VERIFIER_FILENAME, &zisk_vk(), &stub_stark(), templates)
        .unwrap_or_else(|e| panic!("[{label}] gen_recurser failed: {e}"));

    let dir = std::env::temp_dir().join(format!("recurser_circom_compile_{label}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(dir.join("recurser_aggregator.circom"), &out).unwrap();
    fs::write(dir.join(STUB_VERIFIER_FILENAME), STUB_VERIFIER).unwrap();
    fs::write(dir.join("mux1.circom"), STUB_MUX1).unwrap();
    fs::write(dir.join("iszero.circom"), STUB_ISZERO).unwrap();
    fs::write(dir.join("publics_helpers.circom"), STUB_PUBLICS_HELPERS).unwrap();

    let output = Command::new("circom")
        .args(["--r1cs", "--prime", "goldilocks", "-l"])
        .arg(&dir)
        .arg(dir.join("recurser_aggregator.circom"))
        .arg("-o")
        .arg(&dir)
        .output()
        .expect("failed to spawn circom");

    let _ = fs::remove_dir_all(&dir);

    assert!(
        output.status.success(),
        "[{label}] circom compilation failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

/// Compile every structural branch combination of the template, guarding each
/// against unassigned / double-assigned signals.
#[test]
fn aggregator_circom_compiles_all_branches() {
    if !circom_available() {
        eprintln!("skipping: circom binary not found on PATH");
        return;
    }

    // (label, normalize, aggregate body, n_free, allow-list)
    let cases: Vec<(&str, Option<NormalizeCircuit>, &str, usize, Vec<[String; 4]>)> = vec![
        ("plain", None, AGGREGATE_0_FREE, 0, vec![]),
        ("nfree1", None, AGGREGATE_1_FREE, 1, vec![]),
        (
            "normalize_nfree1",
            Some(NormalizeCircuit { body: NORMALIZE_1_FREE.to_string() }),
            AGGREGATE_1_FREE,
            1,
            vec![],
        ),
        ("allowlist", None, AGGREGATE_0_FREE, 0, program_vks()),
        (
            "allowlist_normalize_nfree1",
            Some(NormalizeCircuit { body: NORMALIZE_1_FREE.to_string() }),
            AGGREGATE_1_FREE,
            1,
            program_vks(),
        ),
    ];

    for (label, normalize, aggregate, n_free, vks) in cases {
        let templates = CircomTemplates {
            normalize,
            aggregate_publics: aggregate.to_string(),
            n_free,
            n_publics_agg: recurser::templates::ZISK_PUBLICS,
            program_vks: vks,
        };
        assert_compiles(label, &templates);
    }
}
