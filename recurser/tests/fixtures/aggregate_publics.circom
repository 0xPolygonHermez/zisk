// Example AggregatePublics — inherits every slot from A (the "prev" side).
// The publics width is ZisK's fixed 64 (the slots after the 4-limb programVK),
// exposed by the `ZISK_PUBLICS()` function the recurser scaffolding defines
// ahead of this body, so it is not a template parameter. Inputs bind
// positionally to the aggregator's instantiation:
//   AggregatePublics(nFreeInputs)(ziskPublicsA, ziskPublicsB,
//                                 freeInputsA, freeInputsB)
// so declare them in that exact order. freeInputs are available here so a
// hash-style public can be checked against its preimage and re-hashed; this
// example just drains them.
template AggregatePublics(nFreeInputs) {
    signal output aggregated_publics[ZISK_PUBLICS()];
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

    for (var i = 0; i < ZISK_PUBLICS(); i++) {
        aggregated_publics[i] <== a_publics[i];
    }
}
