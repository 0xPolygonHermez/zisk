// NormalizePublics: reassemble the leaf's 4-element digest from its 8 u32 limbs
// (element k = low slot[2k] + high slot[2k+1]) into 4 field elements, the width
// the fold works over (n-publics-agg = 4). Free inputs pass straight through.
template NormalizePublics(nFreeInputs) {
    signal input publics[ZISK_PUBLICS()];
    signal input free_inputs[nFreeInputs];
    signal output recurser_publics[ZISK_PUBLICS()];
    signal output free_outputs[nFreeInputs];

    for (var i = 0; i < nFreeInputs; i++) {
        free_outputs[i] <== free_inputs[i];
    }

    for (var k = 0; k < 4; k++) {
        recurser_publics[k] <== publics[2 * k] + publics[2 * k + 1] * 2**32;
    }

    // Unused slots zeroed; drain every input so circom sees them all used.
    for (var i = 4; i < ZISK_PUBLICS(); i++) {
        recurser_publics[i] <== 0;
    }
    for (var i = 0; i < ZISK_PUBLICS(); i++) {
        _ <== publics[i];
    }
}
