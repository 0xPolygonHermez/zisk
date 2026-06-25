// Example AggregatePublics — inherits every slot from A (the "prev" side).
// Inputs bind positionally to the aggregator's instantiation:
//   AggregatePublics(nPublics, nFreeInputs)(ziskPublicsA, ziskPublicsB,
//                                           freeInputsA, freeInputsB)
// so declare them in that exact order. freeInputs are available here so a
// hash-style public can be checked against its preimage and re-hashed without
// a NormalizePublics stage; this example just drains them.
template AggregatePublics(nPublics, nFreeInputs) {
    signal output aggregated_publics[nPublics];
    signal input a_publics[nPublics];
    signal input b_publics[nPublics];
    signal input free_inputs_a[nFreeInputs];
    signal input free_inputs_b[nFreeInputs];

    // Drain unused B-side publics and both free-input arrays.
    for (var i = 0; i < nPublics; i++) {
        _ <== b_publics[i];
    }
    for (var i = 0; i < nFreeInputs; i++) {
        _ <== free_inputs_a[i];
        _ <== free_inputs_b[i];
    }

    for (var i = 0; i < nPublics; i++) {
        aggregated_publics[i] <== a_publics[i];
    }
}
