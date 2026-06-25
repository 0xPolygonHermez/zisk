//! Per-AIR unit tests for the hash precompiles: Keccakf, Sha256f, Poseidon2
//! and Blake2. All inputs carry the pre-image state only — each SM computes
//! the permutation itself, so any state is honest. Address fields feed the
//! memory bus (unchecked per-AIR in isolation) but use realistic RAM
//! addresses anyway.

use zisk_prover_backend::{
    inputs::{Blake2Input, KeccakfInput, Sha256fInput},
    testing::with_prover,
    Blake2Sm, KeccakfSm, Sha256fSm,
};

#[test]
#[ignore = "requires ~/.zisk/provingKey"]
fn keccakf_honest_input_verifies() {
    with_prover(|prover| {
        let result = prover
            .input::<KeccakfSm>(KeccakfInput {
                step_main: 1,
                addr_main: 0xa000_0000,
                state: [0u64; 25],
            })
            .run()
            .expect("verification run failed");

        assert!(result.valid, "honest Keccakf input should satisfy all constraints");
    });
}

#[test]
#[ignore = "requires ~/.zisk/provingKey"]
fn sha256f_honest_input_verifies() {
    with_prover(|prover| {
        let result = prover
            .input::<Sha256fSm>(Sha256fInput {
                step_main: 1,
                addr_main: 0xa000_0000,
                state_addr: 0xa000_0000,
                input_addr: 0xa000_0100,
                state: [0u64; 4],
                input: [0u64; 8],
            })
            .run()
            .expect("verification run failed");

        assert!(result.valid, "honest Sha256f input should satisfy all constraints");
    });
}

#[test]
#[ignore = "requires ~/.zisk/provingKey"]
fn blake2_honest_input_verifies() {
    with_prover(|prover| {
        let result = prover
            .input::<Blake2Sm>(Blake2Input {
                addr_main: 0xa000_0000,
                step_main: 1,
                index: 0, // Blake2 round index, must be < 10
                state_addr: 0xa000_0000,
                input_addr: 0xa000_0100,
                state: [0u64; 16],
                input: [0u64; 16],
            })
            .run()
            .expect("verification run failed");

        assert!(result.valid, "honest Blake2 input should satisfy all constraints");
    });
}
