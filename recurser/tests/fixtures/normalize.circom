// Example NormalizePublics — identity passthrough.
template NormalizePublics(nFreeInputs) {
    signal input publics[ZISK_PUBLICS()];
    signal input free_inputs[nFreeInputs];
    signal output recurser_publics[ZISK_PUBLICS()];
    signal output free_outputs[nFreeInputs];

    // Identity: free inputs pass straight through as free outputs.
    for (var i = 0; i < nFreeInputs; i++) {
        free_outputs[i] <== free_inputs[i];
    }

    for (var i = 0; i < ZISK_PUBLICS(); i++) {
        recurser_publics[i] <== publics[i];
    }
}
