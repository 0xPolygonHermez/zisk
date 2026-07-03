use recurser::{gen_recurser, templates::StarkInputBlocks, CircomTemplates, NormalizeCircuit};

// Minimal AggregatePublics bodies for use in tests. AggregatePublics sizes its
// arrays via ZISK_PUBLICS() (no nPublics param): 0 params when n_free=0, 1
// param (nFreeInputs) when n_free>0 — matching the tera's `AggregatePublics()`
// / `AggregatePublics(n)` instantiation.

const AGGREGATE_0_FREE: &str = r#"template AggregatePublics() {
    signal input a_publics[ZISK_PUBLICS()];
    signal input b_publics[ZISK_PUBLICS()];
    signal output aggregated_publics[ZISK_PUBLICS()];
    for (var i = 0; i < ZISK_PUBLICS(); i++) {
        _ <== b_publics[i];
        aggregated_publics[i] <== a_publics[i];
    }
}"#;

// AggregatePublics with free-value width (1 param: nFreeInputs).
const AGGREGATE_1_FREE: &str = r#"template AggregatePublics(nFreeInputs) {
    signal input a_publics[ZISK_PUBLICS()];
    signal input b_publics[ZISK_PUBLICS()];
    signal input free_inputs_a[nFreeInputs];
    signal input free_inputs_b[nFreeInputs];
    signal output aggregated_publics[ZISK_PUBLICS()];
    for (var i = 0; i < ZISK_PUBLICS(); i++) {
        _ <== b_publics[i];
        aggregated_publics[i] <== a_publics[i];
    }
    for (var i = 0; i < nFreeInputs; i++) {
        _ <== free_inputs_a[i];
        _ <== free_inputs_b[i];
    }
}"#;

// Minimal NormalizePublics bodies.
// n_free=0 -> 1 param (nPublics), no free_outputs required.
const NORMALIZE_0_FREE: &str = r#"template NormalizePublics(nPublics) {
    signal input publics[nPublics];
    signal output recurser_publics[nPublics];
    for (var i = 0; i < nPublics; i++) {
        recurser_publics[i] <== publics[i];
    }
}"#;

// n_free>0 -> 2 params (nPublics, nFreeInputs) AND must emit `free_outputs`
// (new contract: NormalizePublics produces the free values that feed AggregatePublics).
const NORMALIZE_1_FREE: &str = r#"template NormalizePublics(nPublics, nFreeInputs) {
    signal input publics[nPublics];
    signal input free_inputs[nFreeInputs];
    signal output recurser_publics[nPublics];
    signal output free_outputs[nFreeInputs];
    for (var i = 0; i < nFreeInputs; i++) {
        free_outputs[i] <== free_inputs[i];
    }
    for (var i = 0; i < nPublics; i++) {
        recurser_publics[i] <== publics[i];
    }
}"#;

// n_free>0 body that is MISSING `free_outputs` — used to exercise the
// declares_free_outputs rejection. Correct arity (2 params), wrong contract.
const NORMALIZE_1_FREE_NO_OUTPUTS: &str = r#"template NormalizePublics(nPublics, nFreeInputs) {
    signal input publics[nPublics];
    signal input free_inputs[nFreeInputs];
    signal output recurser_publics[nPublics];
    for (var i = 0; i < nFreeInputs; i++) {
        _ <== free_inputs[i];
    }
    for (var i = 0; i < nPublics; i++) {
        recurser_publics[i] <== publics[i];
    }
}"#;

fn templates() -> CircomTemplates {
    CircomTemplates {
        normalize: None,
        aggregate_publics: AGGREGATE_0_FREE.to_string(),
        n_free: 0,
        program_vks: vec![],
    }
}

/// Two distinct 4-limb program VKs for allow-list tests.
fn program_vks() -> Vec<[String; 4]> {
    vec![
        ["10".into(), "11".into(), "12".into(), "13".into()],
        ["20".into(), "21".into(), "22".into(), "23".into()],
    ]
}

fn empty_stark() -> StarkInputBlocks<'static> {
    StarkInputBlocks { define_a: "", define_b: "", assign_a: "", assign_b: "" }
}

fn zisk_vk() -> [String; 4] {
    ["1".to_string(), "2".to_string(), "3".to_string(), "4".to_string()]
}

// --- Test 1: no-normalize, n_free=0 (raw passthrough) ---

/// No normalize, no free inputs: raw passthrough, flag/boolean constraint, slot-0
/// forced to 0, no free-value machinery, no old allowlist/membership machinery.
#[test]
fn no_normalize_raw_passthrough() {
    let out = gen_recurser("zisk_final.verifier.circom", &zisk_vk(), &empty_stark(), &templates())
        .unwrap();

    // Basic pragmas and includes.
    assert!(out.contains("pragma circom 2.1.0;"));
    assert!(out.contains("include \"zisk_final.verifier.circom\";"));
    assert!(out.contains("include \"mux1.circom\";"));

    // Flag is read from slot 0 (IS_VADCOP_FINAL_SLOT = 0).
    assert!(out.contains("isFinalA <== a_sv_publics[0]"), "must read is_vadcop_final from slot 0");
    assert!(out.contains("isFinalB <== b_sv_publics[0]"), "must read is_vadcop_final from slot 0");

    // Boolean self-defence constraint.
    assert!(out.contains("isFinalA * (1 - isFinalA) === 0"), "must boolean-constrain isFinalA");
    assert!(out.contains("isFinalB * (1 - isFinalB) === 0"), "must boolean-constrain isFinalB");

    // VK and publics offsets from the template variables.
    assert!(out.contains("var VK_BASE = 1;"), "VK_BASE must be 1");
    assert!(out.contains("var PUBLICS_BASE = VK_BASE + PROGRAM_VK_LEN;"), "PUBLICS_BASE defined");

    // Slot 0 of output forced to 0.
    assert!(out.contains("aggregatedPublics[0] === 0"), "slot 0 must be forced to 0");

    // Raw passthrough: ziskPublicsA/B assigned directly from aPublics/bPublics.
    assert!(out.contains("ziskPublicsA[i] <== aPublics[i];"), "no-normalize must pass A raw");
    assert!(out.contains("ziskPublicsB[i] <== bPublics[i];"), "no-normalize must pass B raw");

    // No normalize machinery.
    assert!(!out.contains("NormalizePublics("), "no-normalize must not emit NormalizePublics");
    assert!(!out.contains("selNormA"), "no-normalize must not emit selNormA mux signal");
    assert!(!out.contains("selRawA"), "no-normalize must not emit selRawA mux signal");

    // n_free=0: NO free-input signals and NO free-value machinery.
    assert!(
        !out.contains("signal input freeInputsA"),
        "n_free=0 must not declare freeInputsA signal"
    );
    assert!(
        !out.contains("signal input freeInputsB"),
        "n_free=0 must not declare freeInputsB signal"
    );
    assert!(!out.contains("aggFreeA"), "n_free=0 must not emit aggFreeA");
    assert!(!out.contains("aggFreeB"), "n_free=0 must not emit aggFreeB");

    // With an empty allow-list (the VK-agnostic default) none of the optional
    // membership machinery is emitted. See `programs_allowlist_*` for the
    // non-empty case where IsEqualVK / programVKs / isRegisteredProgram appear.
    assert!(!out.contains("programVKs["), "empty allow-list must not bake programVKs");
    assert!(!out.contains("IsEqualVK"), "empty allow-list must not emit IsEqualVK");
    assert!(
        !out.contains("isRegisteredProgram"),
        "empty allow-list must not emit isRegisteredProgram"
    );
    assert!(!out.contains("normalize_groups"), "must not contain normalize_groups");

    // AggregatePublics call: 0-free form uses no param in instantiation.
    assert!(
        out.contains("AggregatePublics()(ziskPublicsA, ziskPublicsB)"),
        "0-free AggregatePublics must use no-arg instantiation"
    );

    // VK match when both aggregated.
    assert!(out.contains("bothAggregated"), "must emit bothAggregated for VK match");
}

// --- Test 2: no-normalize, n_free>0 (raw free-value passthrough, no mux) ---

/// No normalize but with a free-value width: freeInputsX declared once per side,
/// fed to AggregatePublics DIRECTLY as aggFreeX (no norm path, no free mux).
#[test]
fn no_normalize_with_nfree_gt0() {
    let templates = CircomTemplates {
        normalize: None,
        aggregate_publics: AGGREGATE_1_FREE.to_string(),
        n_free: 1,
        program_vks: vec![],
    };

    let out = gen_recurser("v.circom", &zisk_vk(), &empty_stark(), &templates).unwrap();

    // ONE free-input array per side, declared with the n_free width.
    assert!(out.contains("signal input freeInputsA[1]"), "must declare freeInputsA[1]");
    assert!(out.contains("signal input freeInputsB[1]"), "must declare freeInputsB[1]");

    // Direct passthrough into aggFreeX (no mux, no normalize component).
    assert!(
        out.contains("signal aggFreeA[1] <== freeInputsA;"),
        "no-normalize must wire freeInputsA straight into aggFreeA"
    );
    assert!(
        out.contains("signal aggFreeB[1] <== freeInputsB;"),
        "no-normalize must wire freeInputsB straight into aggFreeB"
    );

    // No normalize / no free-value mux.
    assert!(!out.contains("NormalizePublics("), "no-normalize must not emit NormalizePublics");
    assert!(!out.contains("selFreeNormA"), "no-normalize must not emit the free-value mux");
    assert!(!out.contains("normFreeOutA"), "no-normalize must not emit normFreeOutA");

    // Publics still pass raw.
    assert!(out.contains("ziskPublicsA[i] <== aPublics[i];"), "publics must pass A raw");

    // 1-free AggregatePublics instantiation with both free arrays.
    assert!(
        out.contains("AggregatePublics(1)("),
        "n_free=1 AggregatePublics must use 1-arg instantiation"
    );
    assert!(
        out.contains("ziskPublicsA, ziskPublicsB, aggFreeA, aggFreeB"),
        "AggregatePublics must receive both free-value arrays"
    );
}

// --- Test 3: with-normalize, n_free>0 ---

/// Normalize with 1 free input: 2-arg NormalizePublics instantiation that returns
/// free_outputs, publics mux, AND the free-value mux; freeInputsX declared once
/// per side; AggregatePublics receives the muxed free values.
#[test]
fn with_normalize_and_nfree_gt0() {
    let templates = CircomTemplates {
        normalize: Some(NormalizeCircuit { body: NORMALIZE_1_FREE.to_string() }),
        aggregate_publics: AGGREGATE_1_FREE.to_string(),
        n_free: 1,
        program_vks: vec![],
    };

    let out = gen_recurser("v.circom", &zisk_vk(), &empty_stark(), &templates).unwrap();

    // 2-arg NormalizePublics instantiation (n_free=1).
    assert!(
        out.contains("NormalizePublics(nPublics, 1)"),
        "n_free=1 normalize must use 2-arg NormalizePublics instantiation"
    );

    // NormalizePublics returns free_outputs, captured into normFreeOutX.
    assert!(
        out.contains("normACmp.free_outputs"),
        "must read free_outputs from the normalize component"
    );
    assert!(
        out.contains("signal normFreeOutA[1] <== normACmp.free_outputs;"),
        "must capture normFreeOutA from normACmp.free_outputs"
    );
    assert!(
        out.contains("signal normFreeOutB[1] <== normBCmp.free_outputs;"),
        "must capture normFreeOutB from normBCmp.free_outputs"
    );

    // ONE free-input array per side, declared with the n_free width.
    assert!(
        out.contains("signal input freeInputsA[1]"),
        "must declare freeInputsA[1] once for side A"
    );
    assert!(
        out.contains("signal input freeInputsB[1]"),
        "must declare freeInputsB[1] once for side B"
    );
    // Declared exactly once each (no per-stage duplication).
    assert_eq!(
        out.matches("signal input freeInputsA[1]").count(),
        1,
        "freeInputsA must be declared exactly once"
    );
    assert_eq!(
        out.matches("signal input freeInputsB[1]").count(),
        1,
        "freeInputsB must be declared exactly once"
    );

    // Publics mux signals.
    assert!(out.contains("selNormA["), "must emit selNormA mux signal");
    assert!(out.contains("selNormB["), "must emit selNormB mux signal");
    assert!(out.contains("selRawA["), "must emit selRawA mux signal");
    assert!(out.contains("selRawB["), "must emit selRawB mux signal");
    assert!(
        out.contains("selNormA[i] <== isFinalA * normA[i]"),
        "publics mux must multiply isFinalA by normA"
    );
    assert!(
        out.contains("ziskPublicsA[i] <== selNormA[i] + selRawA[i]"),
        "ziskPublicsA must be mux of norm and raw"
    );

    // Free-value mux, parallel to the publics mux:
    //   leaf (isFinalX=1) -> normalize's free_outputs; aggregated -> freeInputsX.
    assert!(
        out.contains("selFreeNormA[i] <== isFinalA * normFreeOutA[i]"),
        "free mux: leaf side takes isFinalA * normFreeOutA"
    );
    assert!(
        out.contains("selFreeRawA[i] <== (1 - isFinalA) * freeInputsA[i]"),
        "free mux: aggregated side takes (1 - isFinalA) * freeInputsA"
    );
    assert!(
        out.contains("aggFreeA[i] <== selFreeNormA[i] + selFreeRawA[i]"),
        "aggFreeA must be the free-value mux sum"
    );
    assert!(
        out.contains("aggFreeB[i] <== selFreeNormB[i] + selFreeRawB[i]"),
        "aggFreeB must be the free-value mux sum"
    );

    // Injected normalize body is present verbatim.
    assert!(
        out.contains("template NormalizePublics(nPublics, nFreeInputs)"),
        "injected normalize body must appear in output"
    );

    // 1-free AggregatePublics instantiation receiving the muxed free values.
    assert!(
        out.contains("AggregatePublics(1)("),
        "n_free=1 AggregatePublics must use 1-arg instantiation"
    );
    assert!(
        out.contains("ziskPublicsA, ziskPublicsB, aggFreeA, aggFreeB"),
        "AggregatePublics must receive both muxed free-value arrays"
    );
}

// --- Test 4: with-normalize, n_free=0 ---

/// Normalize with 0 free inputs: 1-arg NormalizePublics instantiation,
/// no free-input arrays, no free-value mux, no-arg AggregatePublics.
#[test]
fn with_normalize_and_nfree_zero() {
    let templates = CircomTemplates {
        normalize: Some(NormalizeCircuit { body: NORMALIZE_0_FREE.to_string() }),
        aggregate_publics: AGGREGATE_0_FREE.to_string(),
        n_free: 0,
        program_vks: vec![],
    };

    let out = gen_recurser("v.circom", &zisk_vk(), &empty_stark(), &templates).unwrap();

    // 1-arg NormalizePublics instantiation (n_free=0).
    assert!(
        out.contains("NormalizePublics(nPublics)("),
        "n_free=0 normalize must use 1-arg NormalizePublics instantiation"
    );

    // No free-input signal declarations and no free-value machinery.
    assert!(
        !out.contains("signal input freeInputsA"),
        "n_free=0 must not declare freeInputsA signal"
    );
    assert!(
        !out.contains("signal input freeInputsB"),
        "n_free=0 must not declare freeInputsB signal"
    );
    assert!(!out.contains("aggFreeA"), "n_free=0 must not emit aggFreeA");
    assert!(!out.contains("normFreeOutA"), "n_free=0 must not emit normFreeOutA");

    // Publics mux signals still present (normalize path).
    assert!(out.contains("selNormA["), "must still emit selNormA mux signal");
    assert!(out.contains("selRawA["), "must still emit selRawA mux signal");

    // Injected normalize body present.
    assert!(
        out.contains("template NormalizePublics(nPublics)"),
        "injected normalize body must appear in output"
    );

    // No-arg AggregatePublics instantiation.
    assert!(
        out.contains("AggregatePublics()(ziskPublicsA, ziskPublicsB)"),
        "0-free AggregatePublics must use no-arg instantiation"
    );
}

// --- Test 5: arity-mismatch and contract errors ---

/// A NormalizePublics body with 1 param but n_free=1 (requires 2 params) must error.
#[test]
fn normalize_arity_mismatch_1param_but_nfree1() {
    let templates = CircomTemplates {
        normalize: Some(NormalizeCircuit {
            body: NORMALIZE_0_FREE.to_string(), // 1-param body; n_free=1 needs NormalizePublics(nPublics, nFree)
        }),
        aggregate_publics: AGGREGATE_1_FREE.to_string(), // valid: 1-param AggregatePublics(nFreeInputs)
        n_free: 1,
        program_vks: vec![],
    };
    let result = gen_recurser("v.circom", &zisk_vk(), &empty_stark(), &templates);
    assert!(result.is_err(), "arity mismatch (1-param body, n_free=1) must return Err");
    assert!(
        matches!(result.unwrap_err(), recurser::RecurserError::InvalidTemplates(_)),
        "error must be InvalidTemplates variant"
    );
}

/// A NormalizePublics body with 2 params but n_free=0 (requires 1 param) must error.
#[test]
fn normalize_arity_mismatch_2param_but_nfree0() {
    let templates = CircomTemplates {
        normalize: Some(NormalizeCircuit {
            body: NORMALIZE_1_FREE.to_string(), // 2-param body; n_free=0 needs NormalizePublics(nPublics)
        }),
        aggregate_publics: AGGREGATE_0_FREE.to_string(), // valid: 0-param AggregatePublics()
        n_free: 0,
        program_vks: vec![],
    };
    let result = gen_recurser("v.circom", &zisk_vk(), &empty_stark(), &templates);
    assert!(result.is_err(), "arity mismatch (2-param body, n_free=0) must return Err");
    assert!(
        matches!(result.unwrap_err(), recurser::RecurserError::InvalidTemplates(_)),
        "error must be InvalidTemplates variant"
    );
}

/// A NormalizePublics body with correct 2-param arity but MISSING `free_outputs`
/// when n_free>0 must error (the `expect_template_arity` free_outputs contract check).
#[test]
fn normalize_missing_free_outputs_but_nfree_gt0() {
    let templates = CircomTemplates {
        normalize: Some(NormalizeCircuit {
            body: NORMALIZE_1_FREE_NO_OUTPUTS.to_string(), // 2 params, no free_outputs
        }),
        aggregate_publics: AGGREGATE_1_FREE.to_string(),
        n_free: 1,
        program_vks: vec![],
    };
    let result = gen_recurser("v.circom", &zisk_vk(), &empty_stark(), &templates);
    assert!(result.is_err(), "NormalizePublics missing free_outputs with n_free>0 must return Err");
    assert!(
        matches!(result.unwrap_err(), recurser::RecurserError::InvalidTemplates(_)),
        "error must be InvalidTemplates variant"
    );
}

/// An AggregatePublics body with 0 params but n_free=1 (requires 1 param) must error.
/// AggregatePublics has no leading nPublics param, so n_free=1 needs exactly one
/// param (nFreeInputs); the 0-param body is a mismatch.
#[test]
fn aggregate_arity_mismatch_0param_but_nfree1() {
    let templates = CircomTemplates {
        normalize: None,
        aggregate_publics: AGGREGATE_0_FREE.to_string(), // 0-param body
        n_free: 1,                                       // requires 1 param
        program_vks: vec![],
    };
    let result = gen_recurser("v.circom", &zisk_vk(), &empty_stark(), &templates);
    assert!(result.is_err(), "arity mismatch (aggregate 0-param body, n_free=1) must return Err");
    assert!(
        matches!(result.unwrap_err(), recurser::RecurserError::InvalidTemplates(_)),
        "error must be InvalidTemplates variant"
    );
}

// --- Test 6: optional `programs` allow-list (in-circuit access control) ---

/// With a non-empty allow-list, the circuit emits the membership machinery:
/// the `iszero.circom` include, the `IsEqualVK` helper, a `programVKs[]` array
/// carrying the baked limbs, per-side `isRegisteredProgram` membership, and the
/// hard-reject constraint tying it to `isFinal`.
#[test]
fn programs_allowlist_emits_membership_and_hard_reject() {
    let templates = CircomTemplates {
        normalize: None,
        aggregate_publics: AGGREGATE_0_FREE.to_string(),
        n_free: 0,
        program_vks: program_vks(),
    };

    let out = gen_recurser("v.circom", &zisk_vk(), &empty_stark(), &templates).unwrap();

    // Guarded include and helper.
    assert!(out.contains("include \"iszero.circom\";"), "allow-list must include iszero.circom");
    assert!(out.contains("template IsEqualVK()"), "allow-list must emit the IsEqualVK helper");

    // Baked programVKs array with both members' limbs, in order.
    assert!(out.contains("var programVKs[2][4] ="), "must size programVKs[n_programs][4]");
    assert!(out.contains("[10,11,12,13]"), "must bake first program's VK limbs");
    assert!(out.contains("[20,21,22,23]"), "must bake second program's VK limbs");

    // Membership signals per side.
    assert!(
        out.contains("isRegisteredProgramA <== 1 - noMatchA["),
        "must derive isRegisteredProgramA"
    );
    assert!(
        out.contains("isRegisteredProgramB <== 1 - noMatchB["),
        "must derive isRegisteredProgramB"
    );
    assert!(
        out.contains("eqA[k] <== IsEqualVK()(programVK_A, programVKs[k])"),
        "A membership loop"
    );

    // Hard reject: a claimed leaf must be a registered program.
    assert!(
        out.contains("isFinalA * (1 - isRegisteredProgramA) === 0"),
        "must hard-reject unregistered A leaves"
    );
    assert!(
        out.contains("isFinalB * (1 - isRegisteredProgramB) === 0"),
        "must hard-reject unregistered B leaves"
    );

    // The is_vadcop_final flag still drives leaf selection (feature is additive).
    assert!(out.contains("isFinalA <== a_sv_publics[0]"), "flag still read from slot 0");
    assert!(
        out.contains("MultiMux1(4)([programVK_A, rootCVadcopFinalZisk], isFinalA)"),
        "rootC selection still driven by isFinal, not membership"
    );
}

/// The empty allow-list (default) must emit NONE of the membership machinery —
/// the guard `{% if n_programs > 0 %}` keeps the VK-agnostic circuit clean.
#[test]
fn empty_allowlist_omits_membership() {
    // `templates()` uses an empty program_vks.
    let out = gen_recurser("v.circom", &zisk_vk(), &empty_stark(), &templates()).unwrap();
    assert!(!out.contains("include \"iszero.circom\";"), "no allow-list must not include iszero");
    assert!(!out.contains("IsEqualVK"), "no allow-list must not emit IsEqualVK");
    assert!(!out.contains("programVKs["), "no allow-list must not bake programVKs");
    assert!(!out.contains("isRegisteredProgram"), "no allow-list must not emit membership");
}
