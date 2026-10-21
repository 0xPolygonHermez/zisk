/* ZisK architectural state: pc, the c and flag of the previous step, the
 * step counter, how the program ended, and the register file.
 *
 * ZisK maps r0..r39 at REG_FIRST = SYS_ADDR (core/src/zisk_registers.rs): the
 * 32 RISC-V registers, then r32..r39, which RISC-V code does not see — the
 * transpiler uses them as scratch. An instruction never reaches them through
 * memory: the encoder (ZiskInstBuilder::src_a, src_b, store) turns
 *
 *   reg 0          into SRC_IMM 0, or no store at all,
 *   reg 1..39      into SRC_REG / STORE_REG with the index as offset,
 *   reg 40..63     into SRC_MEM / STORE_MEM at REG_FIRST + 8 * index.
 *
 * So the "virtual registers" 40..63 are plain memory above REG_LAST, and
 * the register file is the 39 registers the main trace holds
 * (REGS_IN_MAIN_FROM..REGS_IN_MAIN_TO). The emulator keeps those in an array
 * apart from memory; the model does the same.
 */

register PC    : bits(64)   /* program counter                         */
register Zc    : bits(64)   /* c register: result of the previous step */
register Zflag : bool       /* flag register: one bit, not 64          */
register Step  : bits(64)   /* execution step counter                  */
register Zerror      : bool      /* the program ended by `halt`        */
register Zerror_code : bits(64)  /* and its exit code, halt's a        */

/* 40 registers, r0 included: REGS_IN_MAIN_TOTAL_NUMBER. r0 is never read or
 * written (see above); its slot exists only so that a register index indexes
 * the vector directly. */
register Xs : vector(40, bits(64))

val rX : regidx -> bits(64)
function rX(r) = {
  let i = unsigned(r);
  assert(i != 0, "r0 is encoded as an immediate, never as A_REG/B_REG");
  assert(i < 40, "a register beyond r39 is encoded as memory, never as A_REG/B_REG");
  Xs[i]
}

val wX : (regidx, bits(64)) -> unit
function wX(r, v) = {
  let i = unsigned(r);
  assert(i != 0, "a store to r0 is encoded as ST_NONE, never as ST_REG");
  assert(i < 40, "a register beyond r39 is encoded as memory, never as ST_REG");
  Xs[i] = v
}
