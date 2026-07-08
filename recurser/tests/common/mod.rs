//! Shared circom template fixtures for the recurser aggregator tests
//! (`recurser_aggregator_circom_snapshot.rs` and `..._compile.rs`). Each test
//! file uses a subset, so `dead_code` is expected per-file.
#![allow(dead_code)]

// Minimal AggregatePublics bodies. Inputs stay full-width (ZISK_PUBLICS()); the
// output is only `nPublicsAgg` wide (the generator zero-fills the tail).
// AggregatePublics carries a leading `nPublicsAgg` param, plus `nFreeInputs` when
// n_free>0 — matching the tera's `AggregatePublics(nPublicsAgg)` /
// `AggregatePublics(n, nPublicsAgg)` instantiation.

pub const AGGREGATE_0_FREE: &str = r#"template AggregatePublics(nPublicsAgg) {
    signal input a_publics[ZISK_PUBLICS()];
    signal input b_publics[ZISK_PUBLICS()];
    signal output aggregated_publics[nPublicsAgg];
    for (var i = 0; i < ZISK_PUBLICS(); i++) {
        _ <== b_publics[i];
    }
    for (var i = 0; i < nPublicsAgg; i++) {
        aggregated_publics[i] <== a_publics[i];
    }
}"#;

// AggregatePublics with free-value width (2 params: nFreeInputs, nPublicsAgg).
pub const AGGREGATE_1_FREE: &str = r#"template AggregatePublics(nFreeInputs, nPublicsAgg) {
    signal input a_publics[ZISK_PUBLICS()];
    signal input b_publics[ZISK_PUBLICS()];
    signal input free_inputs_a[nFreeInputs];
    signal input free_inputs_b[nFreeInputs];
    signal output aggregated_publics[nPublicsAgg];
    for (var i = 0; i < ZISK_PUBLICS(); i++) {
        _ <== b_publics[i];
    }
    for (var i = 0; i < nPublicsAgg; i++) {
        aggregated_publics[i] <== a_publics[i];
    }
    for (var i = 0; i < nFreeInputs; i++) {
        _ <== free_inputs_a[i];
        _ <== free_inputs_b[i];
    }
}"#;

// Minimal NormalizePublics bodies. Publics arrays are sized via ZISK_PUBLICS()
// (the file-scope function the generator emits), so there is no publics-width param.
// n_free=0 -> 0 params, no free_outputs required.
pub const NORMALIZE_0_FREE: &str = r#"template NormalizePublics() {
    signal input publics[ZISK_PUBLICS()];
    signal output recurser_publics[ZISK_PUBLICS()];
    for (var i = 0; i < ZISK_PUBLICS(); i++) {
        recurser_publics[i] <== publics[i];
    }
}"#;

// n_free>0 -> 1 param (nFreeInputs) AND must emit `free_outputs`
// (new contract: NormalizePublics produces the free values that feed AggregatePublics).
pub const NORMALIZE_1_FREE: &str = r#"template NormalizePublics(nFreeInputs) {
    signal input publics[ZISK_PUBLICS()];
    signal input free_inputs[nFreeInputs];
    signal output recurser_publics[ZISK_PUBLICS()];
    signal output free_outputs[nFreeInputs];
    for (var i = 0; i < nFreeInputs; i++) {
        free_outputs[i] <== free_inputs[i];
    }
    for (var i = 0; i < ZISK_PUBLICS(); i++) {
        recurser_publics[i] <== publics[i];
    }
}"#;

// n_free>0 body that is MISSING `free_outputs` — used to exercise the
// declares_free_outputs rejection. Correct arity (1 param), wrong contract.
pub const NORMALIZE_1_FREE_NO_OUTPUTS: &str = r#"template NormalizePublics(nFreeInputs) {
    signal input publics[ZISK_PUBLICS()];
    signal input free_inputs[nFreeInputs];
    signal output recurser_publics[ZISK_PUBLICS()];
    for (var i = 0; i < nFreeInputs; i++) {
        _ <== free_inputs[i];
    }
    for (var i = 0; i < ZISK_PUBLICS(); i++) {
        recurser_publics[i] <== publics[i];
    }
}"#;
