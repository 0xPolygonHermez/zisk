// Default PreparePublics — identity passthrough.
template PreparePublics(nPublics, nPrivateInputs) {
    signal input publics[nPublics];
    signal input private_inputs[nPrivateInputs];
    signal output recurser_publics[nPublics];

    // Drain unused private inputs so Circom doesn't complain.
    for (var i = 0; i < nPrivateInputs; i++) {
        _ <== private_inputs[i];
    }

    for (var i = 0; i < nPublics; i++) {
        recurser_publics[i] <== publics[i];
    }
}
