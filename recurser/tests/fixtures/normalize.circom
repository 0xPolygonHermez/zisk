// Example NormalizePublics — identity passthrough.
template NormalizePublics(nPublics, nFreeInputs) {
    signal input publics[nPublics];
    signal input free_inputs[nFreeInputs];
    signal output recurser_publics[nPublics];

    // Drain unused free inputs.
    for (var i = 0; i < nFreeInputs; i++) {
        _ <== free_inputs[i];
    }

    for (var i = 0; i < nPublics; i++) {
        recurser_publics[i] <== publics[i];
    }
}
