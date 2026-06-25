use recurser::{gen_recurser, templates::StarkInputBlocks, CircomTemplates};

const AGGREGATE: &str = include_str!("fixtures/aggregate_publics.circom");

fn templates() -> CircomTemplates {
    CircomTemplates { aggregate_publics: AGGREGATE.to_string(), aggregate_n_free_inputs: 0 }
}

fn templates_with_free_inputs(n: usize) -> CircomTemplates {
    CircomTemplates { aggregate_publics: AGGREGATE.to_string(), aggregate_n_free_inputs: n }
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
    let templates = templates();

    let out =
        gen_recurser("zisk_final.verifier.circom", &zisk_vk, &program_vks, &stark, &templates)
            .unwrap();

    // Includes and pragmas.
    assert!(out.contains("pragma circom 2.1.0;"));
    assert!(out.contains("include \"zisk_final.verifier.circom\";"));
    assert!(out.contains("include \"mux1.circom\";"));
    assert!(out.contains("include \"iszero.circom\";"));
    assert!(out.contains("include \"publics_helpers.circom\";"));

    // User-supplied AggregatePublics is injected verbatim and wired both sides'
    // publics AND both sides' free inputs. It takes only nFreeInputs — the
    // publics width is ZisK's fixed 64, hardcoded in the template.
    assert!(out.contains("template AggregatePublics(nFreeInputs)"));
    assert!(out.contains(
        "signal aggPublics[nPublics] <== AggregatePublics(nFreeInputs)(\n        ziskPublicsA, ziskPublicsB, freeInputsA, freeInputsB);"
    ));
    // The scaffolding defines the shared publics-width constant the body uses.
    assert!(out.contains("function ZISK_PUBLICS()"));
    assert!(out.contains("return 64;"));
    // No normalize machinery exists anymore.
    assert!(!out.contains("template NormalizePublics"));
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

    // component main: 64 user publics, 0 free inputs, 1 program VK.
    assert!(out.contains("component main = Main(64, 0, 1);"));
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
    let templates = templates();

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
    let templates = templates();

    let out = gen_recurser("v.circom", &zisk_vk, &program_vks, &empty_stark(), &templates).unwrap();

    let pos_a = out.find("signal input freeInputsA[nFreeInputs];").expect("freeInputsA");
    let pos_b = out.find("signal input freeInputsB[nFreeInputs];").expect("freeInputsB");
    let pos_root = out.find("signal input rootCRecurserAgg[4];").expect("rootCRecurserAgg");
    assert!(pos_a < pos_b, "freeInputsA must precede freeInputsB");
    assert!(pos_b < pos_root, "freeInputsB must precede rootCRecurserAgg");
}

/// Publics always pass through raw to AggregatePublics — there is no normalize
/// stage. Pins that no normalize machinery leaks back into the output.
#[test]
fn recurser_passes_publics_through_raw() {
    let zisk_vk: [String; 4] = std::array::from_fn(|_| "0".to_string());
    let program_vks = [vk_row("p")];
    let templates = templates();

    let out = gen_recurser("v.circom", &zisk_vk, &program_vks, &empty_stark(), &templates).unwrap();

    assert!(!out.contains("template NormalizePublics"));
    assert!(!out.contains("inGroupA"));
    assert!(!out.contains("wIdA"));
    assert!(out.contains("ziskPublicsA[i] <== aPublics[i];"));
    assert!(out.contains("ziskPublicsB[i] <== bPublics[i];"));
    // Positional contract holds even with no side inputs (zero-sized arrays).
    assert!(out.contains("signal input freeInputsA[nFreeInputs];"));
    assert!(out.contains("component main = Main(64, 0, 1);"));
}

/// The aggregate stage's free-input count sizes the per-side freeInputs arrays.
#[test]
fn recurser_sizes_free_inputs_from_aggregate_stage() {
    let zisk_vk: [String; 4] = std::array::from_fn(|_| "0".to_string());
    let program_vks = [vk_row("p")];
    let templates = templates_with_free_inputs(5);

    let out = gen_recurser("v.circom", &zisk_vk, &program_vks, &empty_stark(), &templates).unwrap();
    assert!(out.contains("component main = Main(64, 5, 1);"));
}

#[test]
fn recurser_injects_all_program_vks() {
    let zisk_vk: [String; 4] = std::array::from_fn(|_| "0".to_string());
    let program_vks = [vk_row("x"), vk_row("y"), vk_row("z")];
    let templates = templates();

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
