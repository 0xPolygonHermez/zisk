//! Per-AIR unit test for the Add256 big-int precompile. The input carries
//! only the operands and carry-in; the SM computes the carried sum itself.

use zisk_prover_backend::{inputs::Add256Input, testing::with_prover, Add256Sm};

#[test]
#[ignore = "requires ~/.zisk/provingKey"]
fn add256_honest_input_verifies() {
    with_prover(|prover| {
        let result = prover
            .input::<Add256Sm>(Add256Input {
                step_main: 1,
                addr_main: 0xa000_0000,
                addr_a: 0xa000_0100,
                addr_b: 0xa000_0200,
                addr_c: 0xa000_0300,
                cin: 0, // must be 0 or 1
                a: [1, 0, 0, 0],
                b: [2, 0, 0, 0],
            })
            .run()
            .expect("verification run failed");

        assert!(result.valid, "honest Add256 input should satisfy all constraints");
    });
}
