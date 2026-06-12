// Hashing NormalizePublics of the `chain` example aggregation (recurser e2e test).
//
// Exercises a non-identity preparation step: each leaf proof's publics are
// transformed before they reach AggregatePublics. We hash the four u32 values
// [1, 2, 3, freeInputs[0]] with Poseidon1_8 (the 8→8 GL permutation from
// circuits.gl/hash/poseidon1/poseidon8.circom) and stash the first 4 output
// field elements in publics slots [2, 3, 4, 5].
//
// The first two slots [0, 1] are the chain endpoints [old, new] and pass
// through untouched; slots [6, nPublics) also pass through (they stay zero).
//
// `freeInputs[0]` is supplied at prove time (`.with_free_inputs(vec![4])`), so
// the hashed tuple is [1, 2, 3, 4]. Both leaves hash the identical tuple, so
// both produce the SAME digest — which AggregatePublics asserts is equal
// across A and B and propagates up the fold tree.
//
// The four digest elements are FULL Goldilocks field elements (a hash output
// exceeds 32 bits). They round-trip across fold levels because ZisK's `Proof`
// now carries the untruncated u64 publics on the recursion path (see
// `ProofBody::Vadcop::publics_full` in common/src/proof.rs) instead of the u32
// projection used for leaf programs / Solidity.
//
// No `include` here on purpose: circom requires every `include` at the top of
// the file, but this body is injected mid-file into aggregator.circom.tera.
// `Poseidon1_8` is already in scope — the generated verifier includes
// `hash/poseidon1/pow.circom`, which includes `poseidon8.circom`.

template NormalizePublics(nPublics, nFreeInputs) {
    signal input publics[nPublics];
    signal input free_inputs[nFreeInputs];
    signal output recurser_publics[nPublics];

    // Poseidon1_8 permutation: in[8] -> out[8]. Hash [1, 2, 3, freeInputs[0]]
    // in the first four input slots; zero-pad the rest.
    signal hashIn[8];
    hashIn[0] <== 1;
    hashIn[1] <== 2;
    hashIn[2] <== 3;
    hashIn[3] <== free_inputs[0];
    for (var i = 4; i < 8; i++) {
        hashIn[i] <== 0;
    }

    signal digest[8] <== Poseidon1_8()(hashIn);

    // Endpoints pass through; the first 4 digest elements land in [2, 3, 4, 5];
    // the rest of the publics passes through unchanged.
    recurser_publics[0] <== publics[0];
    recurser_publics[1] <== publics[1];
    for (var i = 0; i < 4; i++) {
        recurser_publics[i + 2] <== digest[i];
    }
    for (var i = 6; i < nPublics; i++) {
        recurser_publics[i] <== publics[i];
    }

    // Drain the unused digest tail and any remaining free inputs so Circom
    // doesn't complain about unused signals.
    for (var i = 4; i < 8; i++) {
        _ <== digest[i];
    }
    for (var i = 1; i < nFreeInputs; i++) {
        _ <== free_inputs[i];
    }
}
