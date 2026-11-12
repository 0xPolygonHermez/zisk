include "hash/blake3/linearhash.circom";

// AggregatePublics: sum the two 12-element secret vectors (from free inputs) and
// output BLAKE3(sum) (n-publics-agg = 4). Each free vector is bound to its
// proof's digest via BLAKE3(free) === digest, so it can't be forged. The
// digest is what NormalizePublics wrote to x_publics[0..4).
template AggregatePublics(nFreeInputs, nPublicsAgg) {
    signal output aggregated_publics[nPublicsAgg];
    signal input a_publics[ZISK_PUBLICS()];
    signal input b_publics[ZISK_PUBLICS()];
    signal input free_inputs_a[nFreeInputs];
    signal input free_inputs_b[nFreeInputs];

    // 12 elements in, a 4-element digest out (the BLAKE3 leaf hash of the STARK verifier).
    assert(nFreeInputs == 12);
    assert(nPublicsAgg == 4);

    // Bind each free vector to its proof's committed digest.
    component ha = LinearHash(nFreeInputs, 2, 1);
    for (var i = 0; i < nFreeInputs; i++) { ha.in[i][0] <== free_inputs_a[i]; }
    for (var i = 0; i < 4; i++) { ha.out[i] === a_publics[i]; }

    component hb = LinearHash(nFreeInputs, 2, 1);
    for (var i = 0; i < nFreeInputs; i++) { hb.in[i][0] <== free_inputs_b[i]; }
    for (var i = 0; i < 4; i++) { hb.out[i] === b_publics[i]; }

    // Sum the two verified vectors, then output BLAKE3(sum).
    signal sum[nFreeInputs];
    for (var i = 0; i < nFreeInputs; i++) {
        sum[i] <== free_inputs_a[i] + free_inputs_b[i];
    }

    component h = LinearHash(nFreeInputs, 2, 1);
    for (var i = 0; i < nFreeInputs; i++) { h.in[i][0] <== sum[i]; }
    aggregated_publics <== h.out;

    // Drain the input publics slots not consumed above.
    for (var i = 0; i < ZISK_PUBLICS(); i++) {
        _ <== a_publics[i];
        _ <== b_publics[i];
    }
}
