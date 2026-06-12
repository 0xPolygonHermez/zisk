//! Per-AIR unit tests for the binary state machines: Binary, BinaryAdd and
//! BinaryExtension. The SMs compute the result themselves, so an honest input
//! is just an opcode plus operands.

use zisk_core::zisk_ops::ZiskOp;
use zisk_prover_backend::{
    inputs::BinaryInput, testing::with_prover, BinaryAddSm, BinaryExtensionSm, BinarySm,
};

#[test]
#[ignore = "requires ~/.zisk/provingKey"]
fn binary_honest_input_verifies() {
    with_prover(|prover| {
        let result = prover
            .input::<BinarySm>(BinaryInput { op: ZiskOp::And.code(), a: 0b1100, b: 0b1010 })
            .input::<BinarySm>(BinaryInput { op: ZiskOp::Ltu.code(), a: 5, b: 7 })
            .run()
            .expect("verification run failed");

        assert!(result.valid, "honest Binary inputs should satisfy all constraints");
    });
}

/// BinaryAdd's input is the raw operand pair `[a, b]`; the SM splits them
/// into 32-bit chunks and computes the carried sum itself.
#[test]
#[ignore = "requires ~/.zisk/provingKey"]
fn binary_add_honest_input_verifies() {
    with_prover(|prover| {
        let result = prover
            .input::<BinaryAddSm>([0x0000_0003_0000_0005u64, 0x0000_0002_0000_0001u64])
            .run()
            .expect("verification run failed");

        assert!(result.valid, "honest BinaryAdd input should satisfy all constraints");
    });
}

#[test]
#[ignore = "requires ~/.zisk/provingKey"]
fn binary_extension_honest_input_verifies() {
    with_prover(|prover| {
        let result = prover
            .input::<BinaryExtensionSm>(BinaryInput { op: ZiskOp::Sll.code(), a: 3, b: 2 })
            .input::<BinaryExtensionSm>(BinaryInput {
                op: ZiskOp::SignExtendB.code(),
                a: 0,
                b: 0xFF,
            })
            .run()
            .expect("verification run failed");

        assert!(result.valid, "honest BinaryExtension inputs should satisfy all constraints");
    });
}
