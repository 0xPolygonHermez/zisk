// Example AggregatePublics — inherits the used slots from A. Inputs are ZisK's
// fixed 64-slot publics; the output is only `nPublicsAgg` wide (scaffolding
// zero-fills the tail). Inputs bind positionally in declaration order:
//   AggregatePublics(nFreeInputs, nPublicsAgg)(ziskPublicsA, ziskPublicsB,
//                                              freeInputsA, freeInputsB)
// freeInputs are available for e.g. preimage checks; this example drains them.
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
