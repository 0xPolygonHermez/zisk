/* ZisK memory: byte-addressed and little-endian, apart from the register
 * file (zisk_regs.sail), which no memory access may overlap.
 */

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
let REG_END   : bits(64) = 0x0000_0000_a040_0140   /* REG_LAST + 8: r0..r39 */

/* True when [addr, addr + n) overlaps r0..r39. The emulator only checks
 * aligned addresses; the model rejects any overlap. Computed in int so
 * an access that wraps the address space cannot slip under the check. */
val hits_registers : forall 'n, 'n > 0. (bits(64), int('n)) -> bool
function hits_registers(addr, n) =
  unsigned(addr) < unsigned(REG_END) & unsigned(addr) + n > unsigned(REG_FIRST)

/* The free input's address: an 8-byte read there returns FreeInput, not
 * memory (Mem::read). The input data itself starts 8 bytes later. */
let INPUT_ADDR : bits(64) = 0x0000_0000_1000_0000

val read_mem : forall 'n, 'n in {1, 2, 4, 8}. (bits(64), int('n)) -> bits(8 * 'n)
function read_mem(addr, n) = {
  assert(not_bool(hits_registers(addr, n)), "memory read overlaps the register file");
  if addr == INPUT_ADDR & n == 8 then FreeInput else __read_ram(64, n, zeros(), addr)
}

val write_mem : forall 'n, 'n in {1, 2, 4, 8}. (bits(64), int('n), bits(8 * 'n)) -> unit
function write_mem(addr, n, v) = {
  assert(not_bool(hits_registers(addr, n)), "memory write overlaps the register file");
  __write_ram(64, n, zeros(), addr, v)
}
