// AggregatePublics of the `chain_simple` example aggregation (recurser e2e
// test) — the minimal sibling of aggregate_publics.circom. Same stitch and
// endpoint merge, but no normalization and no free inputs: leaf publics enter
// raw, so slots [2, ZISK_PUBLICS()) are zero on leaves and are forced back to
// zero on every fold.
//
// `ZISK_PUBLICS()` (= 64) is the fixed user-publics width, defined by the
// recurser scaffolding ahead of this body; it is NOT a template parameter, and
// AggregatePublics takes no parameters when the recurser declares no free
// inputs (see recurser/docs/aggregator-flow.md §6).
//
// Existing alongside `chain`, this definition exercises two recursers over the
// same leaf program registered in one prover at once, and (via its TOML's
// `programs` list) the optional leaf allow-list.
template AggregatePublics() {
    signal output aggregated_publics[ZISK_PUBLICS()];
    signal input a_publics[ZISK_PUBLICS()];
    signal input b_publics[ZISK_PUBLICS()];

    // The stitch: A.new == B.old.
    a_publics[1] === b_publics[0];

    aggregated_publics[0] <== a_publics[0]; // older endpoint comes from A
    aggregated_publics[1] <== b_publics[1]; // newer endpoint comes from B
    for (var i = 2; i < ZISK_PUBLICS(); i++) {
        aggregated_publics[i] <== 0;
    }

    // Drain every input so Circom doesn't complain about unused signals.
    // (`_ <== x` binds an anonymous signal; harmless for signals also used above.)
    for (var i = 0; i < ZISK_PUBLICS(); i++) {
        _ <== a_publics[i];
        _ <== b_publics[i];
    }
}
