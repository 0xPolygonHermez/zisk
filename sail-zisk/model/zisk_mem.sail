/* ZisK memory: byte-addressed and little-endian, in three regions, apart
 * from the register file (zisk_regs.sail), which no memory access may
 * overlap.
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

/* --- The memory map ---------------------------------------------------- *
 * Three regions, as the circuits prove memory (pil/zisk.pil: one Mem air,
 * InputData and RomData) and core/src/mem.rs lays it out:
 *
 *   input  [INPUT_ADDR, +2 GB)  read-only: the free input, then the input
 *   ROM    [ROM_ADDR, +128 MB)  read-only: the program's code and constants
 *   RAM    [RAM_ADDR, +512 MB)  read-write: stack, registers, CSRs, output,
 *                               data, heap
 *
 * An access outside them, or a write outside RAM, is one no circuit
 * proves; the emulator panics on it, and the model asserts. An access must
 * lie wholly inside one region. The emulator is stricter on the ROM: it
 * reads only the bytes the program's data sections define, where RomData
 * proves any address of the region; the model follows the circuits. */

enum mem_region = {MR_INPUT, MR_ROM, MR_RAM, MR_NONE}


/* The region [addr, addr + n) lies in, or MR_NONE. In int, so that an
 * access that wraps the address space cannot fit. */
val mem_region_of : forall 'n, 'n > 0. (bits(64), int('n)) -> mem_region
function mem_region_of(addr, n) = {
  let a = unsigned(addr);
  if      268435456  <= a & a + n <= 2415919104 then MR_INPUT  /* 0x1000_0000 + 0x8000_0000 */
  else if 2415919104 <= a & a + n <= 2550136832 then MR_ROM    /* 0x9000_0000 + 0x0800_0000 */
  else if 2684354560 <= a & a + n <= 3221225472 then MR_RAM    /* 0xA000_0000 + 0x2000_0000 */
  else MR_NONE
}

val readable : forall 'n, 'n > 0. (bits(64), int('n)) -> bool
function readable(addr, n) = match mem_region_of(addr, n) { MR_NONE => false, _ => true }

val writable : forall 'n, 'n > 0. (bits(64), int('n)) -> bool
function writable(addr, n) = match mem_region_of(addr, n) { MR_RAM => true, _ => false }

val read_mem : forall 'n, 'n in {1, 2, 4, 8}. (bits(64), int('n)) -> bits(8 * 'n)
function read_mem(addr, n) = {
  assert(readable(addr, n), "memory read outside ZisK's memory map");
  assert(not_bool(hits_registers(addr, n)), "memory read overlaps the register file");
  if addr == INPUT_ADDR & n == 8 then FreeInput else __read_ram(64, n, zeros(), addr)
}

val write_mem : forall 'n, 'n in {1, 2, 4, 8}. (bits(64), int('n), bits(8 * 'n)) -> unit
function write_mem(addr, n, v) = {
  assert(writable(addr, n), "memory write outside RAM: the ROM and the input are read-only");
  assert(not_bool(hits_registers(addr, n)), "memory write overlaps the register file");
  __write_ram(64, n, zeros(), addr, v)
}
