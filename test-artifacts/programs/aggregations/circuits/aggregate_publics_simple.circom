// AggregatePublics of the `chain_simple` example aggregation (recurser e2e
// test) — the minimal sibling of aggregate_publics.circom. Same stitch and
// endpoint merge, but no normalization hash and no free inputs: leaf publics
// enter through the identity path, so slots [2, nPublics) are zero on leaves
// and are forced back to zero on every fold.
//
// Existing alongside `chain`, this definition exercises two recursers over
// the same leaf program registered in one prover at once.
template AggregatePublics(nPublics) {
    signal output aggregated_publics[nPublics];
    signal input a_publics[nPublics];
    signal input b_publics[nPublics];

    // The stitch: A.new == B.old.
    a_publics[1] === b_publics[0];

    aggregated_publics[0] <== a_publics[0]; // older endpoint comes from A
    aggregated_publics[1] <== b_publics[1]; // newer endpoint comes from B
    for (var i = 2; i < nPublics; i++) {
        aggregated_publics[i] <== 0;
    }

    // Drain every input so Circom doesn't complain about unused signals.
    // (`_ <== x` binds an anonymous signal; harmless for signals also used above.)
    for (var i = 0; i < nPublics; i++) {
        _ <== a_publics[i];
        _ <== b_publics[i];
    }
}
