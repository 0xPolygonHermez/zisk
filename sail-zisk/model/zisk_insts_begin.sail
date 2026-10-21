/* ZisK operations: (c, flag) = op(a, b).
 *
 * One `union clause`, its opcode, and one `function clause` per operation, in a file per
 * family, as sail-riscv spreads its instructions over riscv_insts_*.sail.
 * Mirrors the table in core/src/zisk_ops.rs and the semantics in
 * core/src/ops_core.rs and core/src/operations/.
 */

scattered union zisk_op
scattered function zisk_exec
scattered function zisk_op_of_code

val zisk_exec : (zisk_op, bits(64), bits(64)) -> (bits(64), bool)

/* An operation's opcode, as `zisk_ops.rs` numbers it: the clause after each
 * operation's `union clause`. `make ops` checks the numbers against Rust. */
val zisk_op_of_code : bits(8) -> option(zisk_op)
