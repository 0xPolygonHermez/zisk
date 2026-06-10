// Chain-fold AggregatePublics for the recurser e2e test.
//
// Combines two contiguous segments A=[a.old, a.new] and B=[b.old, b.new]
// (with a.new == b.old enforced by CheckPublics) into the merged segment
// [a.old, b.new]. The merged proof attests the whole span A.old -> B.new.
//
// Unused slots [2, nPublics) are forced to zero so the merged proof keeps the
// same well-formed shape its inputs had — essential because an aggregated
// proof is fed straight back into the next fold level (the unused range must
// stay zero up the whole tree, see recurser/docs/aggregator-flow.md §13).
template AggregatePublics(nPublics, nPrivateInputs) {
    signal output aggregated_publics[nPublics];
    signal input a_publics[nPublics];
    signal input b_publics[nPublics];
    signal input private_inputs[nPrivateInputs];

    aggregated_publics[0] <== a_publics[0]; // older endpoint comes from A
    aggregated_publics[1] <== b_publics[1]; // newer endpoint comes from B
    for (var i = 2; i < nPublics; i++) {
        aggregated_publics[i] <== 0;
    }

    // Drain every input so Circom doesn't complain about unused signals.
    for (var i = 0; i < nPublics; i++) {
        _ <== a_publics[i];
        _ <== b_publics[i];
    }
    for (var i = 0; i < nPrivateInputs; i++) {
        _ <== private_inputs[i];
    }
}
