// Example AggregatePublics — inherits every slot from A (the "prev" side).
template AggregatePublics(nPublics, nPrivateInputs) {
    signal output aggregated_publics[nPublics];
    signal input a_publics[nPublics];
    signal input b_publics[nPublics];
    signal input private_inputs[nPrivateInputs];

    // Drain unused B-side inputs + private inputs.
    for (var i = 0; i < nPublics; i++) {
        _ <== b_publics[i];
    }
    for (var i = 0; i < nPrivateInputs; i++) {
        _ <== private_inputs[i];
    }

    for (var i = 0; i < nPublics; i++) {
        aggregated_publics[i] <== a_publics[i];
    }
}
