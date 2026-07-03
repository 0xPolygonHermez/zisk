// AggregatePublics of the `chain` example aggregation (recurser e2e test) — the
// rich sibling of `chain_simple`'s minimal aggregate. It folds two segments
// AND checks the normalized digest the NormalizePublics hook derived on each
// leaf (see circuits/normalize.circom).
//
// Publics layout used by this example (width ZISK_PUBLICS() = 64, the fixed
// user-publics count exposed by the recurser scaffolding — NOT a template
// parameter):
//   [0]      chain endpoint `old`
//   [1]      chain endpoint `new`
//   [2..6)   Poseidon digest of [1, 2, 3, free_inputs[0]] (from NormalizePublics)
//   [6..64)  zero
//
// `nFreeInputs` is 1 here (chain.toml sets `free-inputs = 1`). The free array is
// the free_out NormalizePublics emitted on a leaf, or the propagated free_out on
// an aggregated proof; this example passes it through the digest and drains it.
template AggregatePublics(nFreeInputs) {
    signal output aggregated_publics[ZISK_PUBLICS()];
    signal input a_publics[ZISK_PUBLICS()];
    signal input b_publics[ZISK_PUBLICS()];
    signal input free_inputs_a[nFreeInputs];
    signal input free_inputs_b[nFreeInputs];

    // The stitch: A.new == B.old (contiguous segments).
    a_publics[1] === b_publics[0];

    // Both sides must agree on the normalized digest slots [2..6). On leaves the
    // digest is derived from identical tuples, so they match; on aggregated
    // proofs the digest was propagated unchanged, so it still matches.
    for (var i = 2; i < 6; i++) {
        a_publics[i] === b_publics[i];
    }

    aggregated_publics[0] <== a_publics[0]; // older endpoint comes from A
    aggregated_publics[1] <== b_publics[1]; // newer endpoint comes from B
    for (var i = 2; i < 6; i++) {
        aggregated_publics[i] <== a_publics[i]; // propagate the shared digest
    }
    for (var i = 6; i < ZISK_PUBLICS(); i++) {
        aggregated_publics[i] <== 0;
    }

    // Drain every input so Circom doesn't complain about unused signals.
    // (`_ <== x` binds an anonymous signal; harmless for signals also used above.)
    for (var i = 0; i < ZISK_PUBLICS(); i++) {
        _ <== a_publics[i];
        _ <== b_publics[i];
    }
    for (var i = 0; i < nFreeInputs; i++) {
        _ <== free_inputs_a[i];
        _ <== free_inputs_b[i];
    }
}
