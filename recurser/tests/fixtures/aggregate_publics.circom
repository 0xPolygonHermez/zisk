// Example AggregatePublics — inherits the used slots from A (the "prev" side).
// Inputs are ZisK's fixed 64-slot publics (via ZISK_PUBLICS()); the output is
// only `nPublicsAgg` wide (the trailing template param, from the
// recurser's `n-publics-agg` config). The scaffolding zero-fills the tail
// outside this template, so a prover cannot inject values into unused slots.
//
// Inputs bind positionally, in declaration order:
//   AggregatePublics(nFreeInputs, nPublicsAgg)(ziskPublicsA, ziskPublicsB,
//                                                     freeInputsA, freeInputsB)
// freeInputs are available for e.g. preimage checks; this example just drains them.
// See docs/aggregator-flow.md §2, §6.
template AggregatePublics(nFreeInputs, nPublicsAgg) {
    signal output aggregated_publics[nPublicsAgg];
    signal input a_publics[ZISK_PUBLICS()];
    signal input b_publics[ZISK_PUBLICS()];
    signal input free_inputs_a[nFreeInputs];
    signal input free_inputs_b[nFreeInputs];

    // Drain unused B-side publics and both free-input arrays.
    for (var i = 0; i < ZISK_PUBLICS(); i++) {
        _ <== b_publics[i];
    }
    for (var i = 0; i < nFreeInputs; i++) {
        _ <== free_inputs_a[i];
        _ <== free_inputs_b[i];
    }

    for (var i = 0; i < nPublicsAgg; i++) {
        aggregated_publics[i] <== a_publics[i];
    }
}
