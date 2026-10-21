/* ZisK operations: (c, flag) = op(a, b).
 *
 * One `union clause`, its opcode, and one `function clause` per operation, in a file per
 * family, as sail-riscv spreads its instructions over riscv_insts_*.sail.
 * Mirrors the table in core/src/zisk_ops.rs and the semantics in
 * core/src/ops_core.rs and core/src/operations/.
 *
 * Most operations are pure functions of a and b, and are clauses of
 * zisk_exec. The ones that work on memory or on the fcall state — DMA,
 * fcall, the precompiles — are clauses of zisk_exec_mem instead, which also
 * gets the instruction's extended argument: jmp_off1, which the emulator
 * hands such an op as extended_arg (Emu::step), and which never serves as a
 * jump, since these ops leave flag false. zisk_execute asks zisk_exec_mem
 * first; None means the op is a pure one. Keeping the two apart keeps
 * zisk_exec pure, so a proof about an ALU op needs no state.
 */

scattered union zisk_op
scattered function zisk_exec
scattered function zisk_exec_mem
scattered function zisk_op_of_code

val zisk_exec : (zisk_op, bits(64), bits(64)) -> (bits(64), bool)

val zisk_exec_mem : (zisk_op, bits(64), bits(64), bits(64)) -> option((bits(64), bool))

/* An operation's opcode, as `zisk_ops.rs` numbers it: the clause after each
 * operation's `union clause`. `make ops` checks the numbers against Rust. */
val zisk_op_of_code : bits(8) -> option(zisk_op)
