// Example NormalizePublics — identity passthrough.
template NormalizePublics(nPublics, nFreeInputs) {
    signal input publics[nPublics];
    signal input free_inputs[nFreeInputs];
    signal output recurser_publics[nPublics];
    signal output free_outputs[nFreeInputs];

    // Identity: free inputs pass straight through as free outputs.
    for (var i = 0; i < nFreeInputs; i++) {
        free_outputs[i] <== free_inputs[i];
    }

    for (var i = 0; i < nPublics; i++) {
        recurser_publics[i] <== publics[i];
    }
}
