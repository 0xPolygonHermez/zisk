/* The register file and memory.
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
 * keeps those in an array apart from memory, and Mem debug_asserts that no
 * memory access lands on them; the model makes both explicit.
 */

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

/* --- Memory ----------------------------------------------------------- *
 * Byte-addressed and little-endian, through the read_ram/write_ram
 * builtins: rts.c implements them for the C emulator, and the Lean
 * runtime over SequentialState.mem. The third argument is unused by both.
 *
 * One difference between the two backends to know about: an unwritten
 * byte reads as 0 in C, but throws OutOfMemoryRange in Lean. A proof
 * therefore states which bytes the initial memory defines.
 */

val __read_ram = impure {c: "read_ram", lean: "read_ram", _: "read_ram"} :
  forall 'n 'm, 'n >= 0. (int('m), int('n), bits('m), bits('m)) -> bits(8 * 'n)

val __write_ram = impure {c: "write_ram", lean: "write_ram", _: "write_ram"} :
  forall 'n 'm, 'n >= 0. (int('m), int('n), bits('m), bits('m), bits(8 * 'n)) -> unit

let REG_FIRST : bits(64) = 0x0000_0000_a040_0000   /* SYS_ADDR             */
let REG_END   : bits(64) = 0x0000_0000_a040_0100   /* REG_LAST + 8         */

/* True when [addr, addr + n) overlaps x0..x31. The emulator only checks
 * aligned addresses; the model rejects any overlap. Computed in int so
 * an access that wraps the address space cannot slip under the check. */
val hits_registers : forall 'n, 'n > 0. (bits(64), int('n)) -> bool
function hits_registers(addr, n) =
  unsigned(addr) < unsigned(REG_END) & unsigned(addr) + n > unsigned(REG_FIRST)

val read_mem : forall 'n, 'n in {1, 2, 4, 8}. (bits(64), int('n)) -> bits(8 * 'n)
function read_mem(addr, n) = {
  assert(not_bool(hits_registers(addr, n)), "memory read overlaps the register file");
  __read_ram(64, n, zeros(), addr)
}

val write_mem : forall 'n, 'n in {1, 2, 4, 8}. (bits(64), int('n), bits(8 * 'n)) -> unit
function write_mem(addr, n, v) = {
  assert(not_bool(hits_registers(addr, n)), "memory write overlaps the register file");
  __write_ram(64, n, zeros(), addr, v)
}
