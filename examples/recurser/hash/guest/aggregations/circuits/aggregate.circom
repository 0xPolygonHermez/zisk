// AggregatePublics: sum the two 12-element secret vectors (from free inputs) and
// output Poseidon1(sum) (n-publics-agg = 4). Each free vector is bound to its
// proof's digest via Poseidon1(free) === digest, so it can't be forged. The
// digest is what NormalizePublics wrote to x_publics[0..4).
template AggregatePublics(nFreeInputs, nPublicsAgg) {
    signal output aggregated_publics[nPublicsAgg];
    signal input a_publics[ZISK_PUBLICS()];
    signal input b_publics[ZISK_PUBLICS()];
    signal input free_inputs_a[nFreeInputs];
    signal input free_inputs_b[nFreeInputs];

    // Fixed sponge shape: 12 rate + 4 digest.
    assert(nFreeInputs == 12);
    assert(nPublicsAgg == 4);

    // Bind each free vector to its proof's committed digest.
    component ha = Poseidon(nPublicsAgg);
    ha.in <== free_inputs_a;
    for (var i = 0; i < 4; i++) { ha.capacity[i] <== 0; }
    for (var i = 0; i < 4; i++) { ha.out[i] === a_publics[i]; }

    component hb = Poseidon(nPublicsAgg);
    hb.in <== free_inputs_b;
    for (var i = 0; i < 4; i++) { hb.capacity[i] <== 0; }
    for (var i = 0; i < 4; i++) { hb.out[i] === b_publics[i]; }

    // Sum the two verified vectors, then output Poseidon1(sum).
    signal sum[nFreeInputs];
    for (var i = 0; i < nFreeInputs; i++) {
        sum[i] <== free_inputs_a[i] + free_inputs_b[i];
    }

    component h = Poseidon(nPublicsAgg);
    h.in <== sum;
    for (var i = 0; i < 4; i++) { h.capacity[i] <== 0; }
    aggregated_publics <== h.out;

    // Drain the input publics slots not consumed above.
    for (var i = 0; i < ZISK_PUBLICS(); i++) {
        _ <== a_publics[i];
        _ <== b_publics[i];
    }
}
