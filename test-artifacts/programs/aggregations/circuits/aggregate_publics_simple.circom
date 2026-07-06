// AggregatePublics of the `chain_simple` example aggregation (recurser e2e
// test) — the minimal sibling of aggregate_publics.circom. Same stitch and
// endpoint merge, but no normalization and no free inputs: leaf publics enter
// raw, so slots [2, ZISK_PUBLICS()) are zero on leaves and are forced back to
// zero on every fold.
//
// Inputs are full-width (ZISK_PUBLICS() = 64); the output is only
// `nPublicsAgg` (= 2, from chain_simple.toml) wide — just the two
// endpoints — and the scaffolding zero-fills the rest (§6). With no free inputs,
// AggregatePublics takes a single `nPublicsAgg` param.
//
// Alongside `chain`, this exercises two recursers over the same leaf program in
// one prover, and (via its TOML's `programs` list) the optional leaf allow-list.
template AggregatePublics(nPublicsAgg) {
    signal output aggregated_publics[nPublicsAgg];
    signal input a_publics[ZISK_PUBLICS()];
    signal input b_publics[ZISK_PUBLICS()];

    // The stitch: A.new == B.old.
    a_publics[1] === b_publics[0];

    aggregated_publics[0] <== a_publics[0]; // older endpoint comes from A
    aggregated_publics[1] <== b_publics[1]; // newer endpoint comes from B

    // Drain every input so Circom doesn't complain about unused signals.
    // (`_ <== x` binds an anonymous signal; harmless for signals also used above.)
    for (var i = 0; i < ZISK_PUBLICS(); i++) {
        _ <== a_publics[i];
        _ <== b_publics[i];
    }
}
