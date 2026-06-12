use recurser::{gen_recurser, templates::StarkInputBlocks, CircomTemplates, NormalizeGroup};

const NORMALIZE: &str = include_str!("fixtures/normalize.circom");
// A second, distinct body for the multi-group tests — inline because only one
// example fixture ships; this one just proves per-group rename/injection.
const NORMALIZE_ALT: &str = "template NormalizePublics(nPublics, nFreeInputs) {
    signal input publics[nPublics];
    signal input free_inputs[nFreeInputs];
    signal output recurser_publics[nPublics];

    for (var i = 1; i < nFreeInputs; i++) { _ <== free_inputs[i]; }
    for (var i = 0; i < nPublics; i++) {
        recurser_publics[i] <== publics[i] + free_inputs[0];
    }
}
";
const AGGREGATE: &str = include_str!("fixtures/aggregate_publics.circom");

fn group(members: &[usize], body: &str, n: usize) -> NormalizeGroup {
    NormalizeGroup { member_indices: members.to_vec(), body: body.to_string(), n_free_inputs: n }
}

fn templates_one_group() -> CircomTemplates {
    CircomTemplates {
        normalize_groups: vec![group(&[0], NORMALIZE, 1)],
        aggregate_publics: AGGREGATE.to_string(),
    }
}

fn templates_no_groups() -> CircomTemplates {
    CircomTemplates { normalize_groups: vec![], aggregate_publics: AGGREGATE.to_string() }
}

fn vk_row(prefix: &str) -> [String; 4] {
    [format!("{prefix}1"), format!("{prefix}2"), format!("{prefix}3"), format!("{prefix}4")]
}

fn empty_stark() -> StarkInputBlocks<'static> {
    StarkInputBlocks { define_a: "", define_b: "", assign_a: "", assign_b: "" }
}

#[test]
fn recurser_renders_required_layout() {
    let stark = StarkInputBlocks {
        define_a: "    // <define a placeholder>",
        define_b: "    // <define b placeholder>",
        assign_a: "    // <assign a placeholder>",
        assign_b: "    // <assign b placeholder>",
    };
    let zisk_vk = ["1".to_string(), "2".to_string(), "3".to_string(), "4".to_string()];
    let program_vks = [vk_row("p")];
    let templates = templates_one_group();

    let out =
        gen_recurser("zisk_final.verifier.circom", &zisk_vk, &program_vks, &stark, &templates)
            .unwrap();

    // Includes and pragmas.
    assert!(out.contains("pragma circom 2.1.0;"));
    assert!(out.contains("include \"zisk_final.verifier.circom\";"));
    assert!(out.contains("include \"mux1.circom\";"));
    assert!(out.contains("include \"iszero.circom\";"));
    assert!(out.contains("include \"publics_helpers.circom\";"));

    // User-supplied sub-templates: aggregate verbatim, normalize renamed per group.
    assert!(out.contains("template AggregatePublics(nPublics)"));
    assert!(out.contains("template NormalizePublics_0(nPublics, nFreeInputs)"));
    assert!(!out.contains("template NormalizePublics(nPublics, nFreeInputs)"));
    // IsEqualVK helper is emitted exactly once and used twice in the membership check.
    assert_eq!(out.matches("template IsEqualVK()").count(), 1);
    assert!(out.contains("eqA[k] <== IsEqualVK()(programVK_A, programVKs[k])"));
    assert!(out.contains("eqB[k] <== IsEqualVK()(programVK_B, programVKs[k])"));

    // Hardcoded VK data.
    assert!(out.contains("var rootCVadcopFinalZisk[4] = [1,2,3,4];"));
    assert!(out.contains("[[p1,p2,p3,p4]]"));

    // Verifier mux.
    assert!(out.contains(
        "vA.rootC <== MultiMux1(4)([programVK_A, rootCVadcopFinalZisk], isRegisteredProgramA);"
    ));
    assert!(out.contains(
        "vB.rootC <== MultiMux1(4)([programVK_B, rootCVadcopFinalZisk], isRegisteredProgramB);"
    ));

    // Stark input blocks injected verbatim.
    assert!(out.contains("// <define a placeholder>"));
    assert!(out.contains("// <assign b placeholder>"));

    // component main: 64 user publics hardcoded, maxN=1 free inputs, 1 program VK.
    assert!(out.contains("component main = Main(64, 1, 1);"));
}

/// The publics blob is VK-FIRST: `[programVK(4)][userPublics(64)]`, per ZisK's
/// `state-machines/publics.json` (rom_root initialPos 0, inputs initialPos 4) and
/// `common/src/proof.rs`. The circuit must read/emit the VK from the LEADING slots,
/// not the trailing ones. This pins that so the layout can't silently flip back —
/// a flip would make the recurser verify the wrong 4 limbs as the VK (soundness bug).
#[test]
fn recurser_uses_vk_first_publics_layout() {
    let zisk_vk: [String; 4] = std::array::from_fn(|_| "0".to_string());
    let program_vks = [vk_row("p")];
    let templates = templates_one_group();

    let out = gen_recurser("v.circom", &zisk_vk, &program_vks, &empty_stark(), &templates).unwrap();

    // VK extracted from the leading slots a_sv_publics[i], i in [0,4).
    assert!(out.contains("programVK_A[i] <== a_sv_publics[i];"), "VK_A must read leading slots");
    assert!(out.contains("programVK_B[i] <== b_sv_publics[i];"), "VK_B must read leading slots");
    // User publics live AFTER the VK slot.
    assert!(
        out.contains("aPublics[i] <== a_sv_publics[PROGRAM_VK_LEN + i];"),
        "user publics A must be offset past the VK"
    );
    assert!(
        out.contains("bPublics[i] <== b_sv_publics[PROGRAM_VK_LEN + i];"),
        "user publics B must be offset past the VK"
    );
    // Output re-emits the same layout: VK in the leading slots, user publics after.
    assert!(
        out.contains("aggregatedPublics[PROGRAM_VK_LEN + i] <== aggPublics[i];"),
        "output user publics must be offset past the VK"
    );
    assert!(
        out.contains("aggregatedPublics[i] <== aggTerm[i] + aTerm[i] + bTerm[i];"),
        "output VK must land in the leading slots"
    );
    // The old VK-last bug used `a_sv_publics[nPublics + i]` / `aggregatedPublics[nPublics + i]`.
    assert!(!out.contains("a_sv_publics[nPublics + i]"), "must not read VK from trailing slots");
    assert!(!out.contains("b_sv_publics[nPublics + i]"), "must not read VK from trailing slots");
    assert!(
        !out.contains("aggregatedPublics[nPublics + i]"),
        "must not write VK to trailing slots"
    );
}

/// The flat witness buffer (proofman's zkin) maps onto Main's inputs in
/// declaration order — pin the order so the positional contract with
/// `generate_recurser_aggregator_proof` can't silently break.
#[test]
fn recurser_declares_free_inputs_in_zkin_order() {
    let zisk_vk: [String; 4] = std::array::from_fn(|_| "0".to_string());
    let program_vks = [vk_row("p")];
    let templates = templates_one_group();

    let out = gen_recurser("v.circom", &zisk_vk, &program_vks, &empty_stark(), &templates).unwrap();

    let pos_a = out.find("signal input freeInputsA[nFreeInputs];").expect("freeInputsA");
    let pos_b = out.find("signal input freeInputsB[nFreeInputs];").expect("freeInputsB");
    let pos_root = out.find("signal input rootCRecurserAgg[4];").expect("rootCRecurserAgg");
    assert!(pos_a < pos_b, "freeInputsA must precede freeInputsB");
    assert!(pos_b < pos_root, "freeInputsB must precede rootCRecurserAgg");
}

/// Two groups over a 3-program allowlist: each group's template is renamed,
/// membership flags are product-complements over the right eq indices, and
/// the output mux is a sum of masks with the identity complement weight.
#[test]
fn recurser_muxes_multiple_normalize_groups() {
    let zisk_vk: [String; 4] = std::array::from_fn(|_| "0".to_string());
    let program_vks = [vk_row("x"), vk_row("y"), vk_row("z")];
    let templates = CircomTemplates {
        normalize_groups: vec![group(&[0, 2], NORMALIZE, 3), group(&[1], NORMALIZE_ALT, 1)],
        aggregate_publics: AGGREGATE.to_string(),
    };

    let out = gen_recurser("v.circom", &zisk_vk, &program_vks, &empty_stark(), &templates).unwrap();

    // Both bodies injected under distinct names.
    assert!(out.contains("template NormalizePublics_0(nPublics, nFreeInputs)"));
    assert!(out.contains("template NormalizePublics_1(nPublics, nFreeInputs)"));

    // Group 0 = programs {0, 2}: two-step product-complement chain.
    assert!(out.contains("noGrpA_0[0] <== 1 - eqA[0];"));
    assert!(out.contains("noGrpA_0[1] <== noGrpA_0[0] * (1 - eqA[2]);"));
    assert!(out.contains("signal {binary} inGroupA_0 <== 1 - noGrpA_0[1];"));
    // Group 1 = program {1}: single-step chain.
    assert!(out.contains("noGrpB_1[0] <== 1 - eqB[1];"));
    assert!(out.contains("signal {binary} inGroupB_1 <== 1 - noGrpB_1[0];"));

    // Identity weight is the complement of all group flags.
    assert!(out.contains("signal wIdA <== 1 - inGroupA_0 - inGroupA_1;"));
    assert!(out.contains("signal wIdB <== 1 - inGroupB_0 - inGroupB_1;"));

    // Each group instantiated with its own n on both sides, fed its slice.
    assert!(out.contains(
        "signal normA_0[nPublics] <== NormalizePublics_0(nPublics, 3)(aPublics, fInA_0);"
    ));
    assert!(out.contains(
        "signal normB_1[nPublics] <== NormalizePublics_1(nPublics, 1)(bPublics, fInB_1);"
    ));
    assert!(out.contains("fInA_0[i] <== freeInputsA[i];"));
    assert!(out.contains("fInB_1[i] <== freeInputsB[i];"));

    // Sum-of-masks mux.
    assert!(out.contains("selA_0[i] <== inGroupA_0 * normA_0[i];"));
    assert!(out.contains("ziskPublicsA[i] <== idA[i] + selA_0[i] + selA_1[i];"));
    assert!(out.contains("ziskPublicsB[i] <== idB[i] + selB_0[i] + selB_1[i];"));

    // maxN = max(3, 1) = 3.
    assert!(out.contains("component main = Main(64, 3, 3);"));
}

/// No groups: publics pass through raw, no normalize machinery is emitted,
/// and the per-side free-input arrays are zero-sized.
#[test]
fn recurser_without_groups_passes_publics_through() {
    let zisk_vk: [String; 4] = std::array::from_fn(|_| "0".to_string());
    let program_vks = [vk_row("p")];
    let templates = templates_no_groups();

    let out = gen_recurser("v.circom", &zisk_vk, &program_vks, &empty_stark(), &templates).unwrap();

    assert!(!out.contains("NormalizePublics"));
    assert!(!out.contains("inGroupA"));
    assert!(!out.contains("wIdA"));
    assert!(out.contains("ziskPublicsA[i] <== aPublics[i];"));
    assert!(out.contains("ziskPublicsB[i] <== bPublics[i];"));
    // Positional contract holds even with no side inputs (zero-sized arrays).
    assert!(out.contains("signal input freeInputsA[nFreeInputs];"));
    assert!(out.contains("component main = Main(64, 0, 1);"));
}

#[test]
fn recurser_rejects_invalid_groups() {
    let zisk_vk: [String; 4] = std::array::from_fn(|_| "0".to_string());
    let program_vks = [vk_row("p"), vk_row("q")];

    // Out-of-range member index.
    let templates = CircomTemplates {
        normalize_groups: vec![group(&[2], NORMALIZE, 0)],
        aggregate_publics: AGGREGATE.to_string(),
    };
    let err =
        gen_recurser("v.circom", &zisk_vk, &program_vks, &empty_stark(), &templates).unwrap_err();
    assert!(err.to_string().contains("references program index 2"));

    // Program in two groups.
    let templates = CircomTemplates {
        normalize_groups: vec![group(&[0], NORMALIZE, 0), group(&[0], NORMALIZE_ALT, 0)],
        aggregate_publics: AGGREGATE.to_string(),
    };
    let err =
        gen_recurser("v.circom", &zisk_vk, &program_vks, &empty_stark(), &templates).unwrap_err();
    assert!(err.to_string().contains("more than once across normalize groups"));

    // Body without the required template name.
    let templates = CircomTemplates {
        normalize_groups: vec![group(&[0], "// not a template", 0)],
        aggregate_publics: AGGREGATE.to_string(),
    };
    let err =
        gen_recurser("v.circom", &zisk_vk, &program_vks, &empty_stark(), &templates).unwrap_err();
    assert!(err.to_string().contains("exactly once"));
}

#[test]
fn recurser_injects_all_program_vks() {
    let zisk_vk: [String; 4] = std::array::from_fn(|_| "0".to_string());
    let program_vks = [vk_row("x"), vk_row("y"), vk_row("z")];
    let templates = templates_no_groups();

    let out = gen_recurser("v.circom", &zisk_vk, &program_vks, &empty_stark(), &templates).unwrap();
    assert!(out.contains("[[x1,x2,x3,x4],[y1,y2,y3,y4],[z1,z2,z3,z4]]"));
    assert!(out.contains("component main = Main(64, 0, 3);"));
}

#[test]
fn publics_helpers_exposes_get_public_le_and_be() {
    let body = recurser::templates::PUBLICS_HELPERS_CIRCOM;
    assert!(body.contains("pragma circom 2.1.0;"));
    assert!(body.contains("include \"bitify.circom\";"));
    assert!(body.contains("template GetPublicLE(numBytes, initialByte)"));
    assert!(body.contains("template GetPublicBE(numBytes, initialByte)"));
    assert!(body.contains("signal input publics[64];"));
    assert!(body.contains("assert(numBytes >= 1);"));
    assert!(body.contains("assert(numBytes <= 4);"));
    assert!(body.contains("assert(initialByte + numBytes <= 256);"));
}
