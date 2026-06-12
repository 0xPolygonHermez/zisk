// AggregatePublics of the `chain` example aggregation (recurser e2e test).
//
// Each proof attests a transition [old, new] = [publics[0], publics[1]].
// The template first enforces that the two segments are contiguous, then
// combines A=[a.old, a.new] and B=[b.old, b.new] into the merged segment
// [a.old, b.new]. The merged proof attests the whole span A.old -> B.new.
//
// Consistency constraints (a fold that violates them aborts):
//   - The stitch: A's `new` endpoint must equal B's `old` endpoint.
//   - Slots [2, 3, 4, 5] hold the NormalizePublics Poseidon1 digest of
//     [1, 2, 3, freeInputs[0]]. Both leaves hash the same tuple, so their
//     digests must be element-wise equal.
//
// We PROPAGATE the digest into the merged proof so it survives every fold
// level: an aggregated proof is in no normalization group, so the mux takes
// the identity path and the carried-up digest re-appears unchanged at the
// next level, where the same equality constraint holds again (see
// recurser/docs/aggregator-flow.md §5).
//
// The remaining unused slots [6, nPublics) are forced to zero so the merged
// proof keeps the same well-formed shape its inputs had — essential because an
// aggregated proof is fed straight back into the next fold level (the unused
// range must stay zero up the whole tree).
template AggregatePublics(nPublics) {
    signal output aggregated_publics[nPublics];
    signal input a_publics[nPublics];
    signal input b_publics[nPublics];

    // The stitch: A.new == B.old.
    a_publics[1] === b_publics[0];

    // The hash digest must match across A and B.
    for (var k = 2; k < 6; k++) {
        a_publics[k] === b_publics[k];
    }

    aggregated_publics[0] <== a_publics[0]; // older endpoint comes from A
    aggregated_publics[1] <== b_publics[1]; // newer endpoint comes from B
    // Propagate the digest in [2..6) so it survives up the fold tree.
    for (var i = 2; i < 6; i++) {
        aggregated_publics[i] <== a_publics[i];
    }
    for (var i = 6; i < nPublics; i++) {
        aggregated_publics[i] <== 0;
    }

    // Drain every input so Circom doesn't complain about unused signals.
    // (`_ <== x` binds an anonymous signal; harmless for signals also used above.)
    for (var i = 0; i < nPublics; i++) {
        _ <== a_publics[i];
        _ <== b_publics[i];
    }
}
