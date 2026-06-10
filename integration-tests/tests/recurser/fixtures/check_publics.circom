// Chain-stitch CheckPublics for the recurser e2e test.
//
// Each proof attests a transition [old, new] = [publics[0], publics[1]].
// To fold A then B into one segment, A's `new` endpoint must equal B's `old`
// endpoint — i.e. the two segments are contiguous. A failure aborts the fold.
template CheckPublics(nPublics, nPrivateInputs) {
    signal input a_publics[nPublics];
    signal input b_publics[nPublics];
    signal input private_inputs[nPrivateInputs];

    // The stitch: A.new == B.old.
    a_publics[1] === b_publics[0];

    // Drain every input so Circom doesn't complain about unused signals.
    // (`_ <== x` binds an anonymous signal; harmless for signals also used above.)
    for (var i = 0; i < nPublics; i++) {
        _ <== a_publics[i];
        _ <== b_publics[i];
    }
    for (var i = 0; i < nPrivateInputs; i++) {
        _ <== private_inputs[i];
    }
}
