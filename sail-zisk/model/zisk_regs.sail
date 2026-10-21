/* ZisK architectural state: pc, the c and flag of the previous step, the
 * step counter, how the program ended, and the register file.
 *
 * ZisK maps x0..x31 at REG_FIRST = SYS_ADDR (core/src/zisk_registers.rs),
 * but an instruction never reaches them through memory: the encoder
 * (ZiskInstBuilder::src_a, src_b, store) turns
 *
 *   reg 0          into SRC_IMM 0, or no store at all,
 *   reg 1..31      into SRC_REG / STORE_REG with the index as offset,
 *   reg 32..63     into SRC_MEM / STORE_MEM at REG_FIRST + 8 * index.
 *
 * So the "virtual registers" 32..63 are plain memory above REG_LAST, and
 * the register file is the 31 registers the main trace holds. The emulator
 * keeps those in an array apart from memory; the model does the same.
 */

register PC    : bits(64)   /* program counter                         */
register Zc    : bits(64)   /* c register: result of the previous step */
register Zflag : bool       /* flag register: one bit, not 64          */
register Step  : bits(64)   /* execution step counter                  */
register Zerror      : bool      /* the program ended by `halt`        */
register Zerror_code : bits(64)  /* and its exit code, halt's a        */

/* x0 is never read or written (see above); its slot exists only so that
 * a regidx indexes the vector directly. */
register Xs : vector(32, bits(64))

val rX : regidx -> bits(64)
function rX(r) = {
  assert(r != 0b00000, "x0 is encoded as an immediate, never as A_REG/B_REG");
  Xs[unsigned(r)]
}

val wX : (regidx, bits(64)) -> unit
function wX(r, v) = {
  assert(r != 0b00000, "a store to x0 is encoded as ST_NONE, never as ST_REG");
  Xs[unsigned(r)] = v
}
