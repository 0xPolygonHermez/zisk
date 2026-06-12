// Example AggregatePublics — inherits every slot from A (the "prev" side).
template AggregatePublics(nPublics) {
    signal output aggregated_publics[nPublics];
    signal input a_publics[nPublics];
    signal input b_publics[nPublics];

    // Drain unused B-side inputs.
    for (var i = 0; i < nPublics; i++) {
        _ <== b_publics[i];
    }

    for (var i = 0; i < nPublics; i++) {
        aggregated_publics[i] <== a_publics[i];
    }
}
