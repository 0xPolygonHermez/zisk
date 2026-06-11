use recurser::{gen_recurser, templates::StarkInputBlocks, CircomTemplates};

const PREPARE: &str = include_str!("fixtures/prepare_publics.circom");
const CHECK: &str = include_str!("fixtures/check_publics.circom");
const AGGREGATE: &str = include_str!("fixtures/aggregate_publics.circom");

fn fixture_templates() -> CircomTemplates {
    CircomTemplates {
        prepare_publics: Some(PREPARE.to_string()),
        check_publics: Some(CHECK.to_string()),
        aggregate_publics: AGGREGATE.to_string(),
    }
}

fn fixture_templates_default_prepare() -> CircomTemplates {
    CircomTemplates {
        prepare_publics: None,
        check_publics: Some(CHECK.to_string()),
        aggregate_publics: AGGREGATE.to_string(),
    }
}

fn fixture_templates_default_check() -> CircomTemplates {
    CircomTemplates {
        prepare_publics: Some(PREPARE.to_string()),
        check_publics: None,
        aggregate_publics: AGGREGATE.to_string(),
    }
}

fn vk_row(prefix: &str) -> [String; 4] {
    [format!("{prefix}1"), format!("{prefix}2"), format!("{prefix}3"), format!("{prefix}4")]
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
    let templates = fixture_templates();

    let out =
        gen_recurser(0, "zisk_final.verifier.circom", &zisk_vk, &program_vks, &stark, &templates)
            .unwrap();

    // Includes and pragmas.
    assert!(out.contains("pragma circom 2.1.0;"));
    assert!(out.contains("include \"zisk_final.verifier.circom\";"));
    assert!(out.contains("include \"mux1.circom\";"));
    assert!(out.contains("include \"iszero.circom\";"));
    assert!(out.contains("include \"publics_helpers.circom\";"));

    // User-supplied sub-templates injected verbatim.
    assert!(out.contains("template PreparePublics(nPublics, nPrivateInputs)"));
    assert!(out.contains("template CheckPublics(nPublics, nPrivateInputs)"));
    assert!(out.contains("template AggregatePublics(nPublics, nPrivateInputs)"));
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

    // component main: 64 user publics hardcoded, 0 private inputs, 1 program VK.
    assert!(out.contains("component main = Main(64, 0, 1);"));
}

/// The publics blob is VK-FIRST: `[programVK(4)][userPublics(64)]`, per ZisK's
/// `state-machines/publics.json` (rom_root initialPos 0, inputs initialPos 4) and
/// `common/src/proof.rs`. The circuit must read/emit the VK from the LEADING slots,
/// not the trailing ones. This pins that so the layout can't silently flip back —
/// a flip would make the recurser verify the wrong 4 limbs as the VK (soundness bug).
#[test]
fn recurser_uses_vk_first_publics_layout() {
    let stark = StarkInputBlocks { define_a: "", define_b: "", assign_a: "", assign_b: "" };
    let zisk_vk: [String; 4] = std::array::from_fn(|_| "0".to_string());
    let program_vks = [vk_row("p")];
    let templates = fixture_templates();

    let out = gen_recurser(0, "v.circom", &zisk_vk, &program_vks, &stark, &templates).unwrap();

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

#[test]
fn recurser_threads_private_inputs_count() {
    let stark = StarkInputBlocks { define_a: "", define_b: "", assign_a: "", assign_b: "" };
    let zisk_vk: [String; 4] = std::array::from_fn(|_| "0".to_string());
    let program_vks = [vk_row("p")];
    let templates = fixture_templates();

    let out = gen_recurser(7, "v.circom", &zisk_vk, &program_vks, &stark, &templates).unwrap();
    assert!(out.contains("component main = Main(64, 7, 1);"));
}

#[test]
fn recurser_uses_default_prepare_publics_when_omitted() {
    let stark = StarkInputBlocks { define_a: "", define_b: "", assign_a: "", assign_b: "" };
    let zisk_vk: [String; 4] = std::array::from_fn(|_| "0".to_string());
    let program_vks = [vk_row("p")];
    let templates = fixture_templates_default_prepare();

    let out = gen_recurser(0, "v.circom", &zisk_vk, &program_vks, &stark, &templates).unwrap();
    // Default identity body shows up — recurser_publics[i] <== publics[i].
    assert!(out.contains("template PreparePublics(nPublics, nPrivateInputs)"));
    assert!(out.contains("recurser_publics[i] <== publics[i]"));
}

#[test]
fn recurser_uses_default_check_publics_when_omitted() {
    let stark = StarkInputBlocks { define_a: "", define_b: "", assign_a: "", assign_b: "" };
    let zisk_vk: [String; 4] = std::array::from_fn(|_| "0".to_string());
    let program_vks = [vk_row("p")];
    let templates = fixture_templates_default_check();

    let out = gen_recurser(0, "v.circom", &zisk_vk, &program_vks, &stark, &templates).unwrap();
    // Default no-op body shows up — emits no `===` constraints in CheckPublics.
    assert!(out.contains("template CheckPublics(nPublics, nPrivateInputs)"));
    assert!(out.contains("// Default CheckPublics — no-op"));
}

#[test]
fn recurser_injects_all_program_vks() {
    let stark = StarkInputBlocks { define_a: "", define_b: "", assign_a: "", assign_b: "" };
    let zisk_vk: [String; 4] = std::array::from_fn(|_| "0".to_string());
    let program_vks = [vk_row("x"), vk_row("y"), vk_row("z")];
    let templates = fixture_templates();

    let out = gen_recurser(0, "v.circom", &zisk_vk, &program_vks, &stark, &templates).unwrap();
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
