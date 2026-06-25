// AggregatePublics of the `chain` example aggregation (recurser e2e test).
// Stitch + endpoint merge with no free inputs: leaf publics enter raw,
// so slots [2, ZISK_PUBLICS()) are zero on leaves and forced back to zero on
// every fold. `ZISK_PUBLICS()` (= 64) is the fixed user-publics width, defined
// by the recurser scaffolding ahead of this body; the width is not a template
// parameter. `nFreeInputs` is 0 here, so the free-input arrays are empty.
template AggregatePublics(nFreeInputs) {
    signal output aggregated_publics[ZISK_PUBLICS()];
    signal input a_publics[ZISK_PUBLICS()];
    signal input b_publics[ZISK_PUBLICS()];
    signal input free_inputs_a[nFreeInputs];
    signal input free_inputs_b[nFreeInputs];

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
    for (var i = 0; i < nFreeInputs; i++) {
        _ <== free_inputs_a[i];
        _ <== free_inputs_b[i];
    }
}
