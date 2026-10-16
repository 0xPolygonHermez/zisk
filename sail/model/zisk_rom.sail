/* The ROM: the program, as instruction records keyed by address.
 *
 * A ZisK instruction is never fetched as bytes (see zisk_inst.sail), so
 * the ROM is not memory: it is a function from pc to zisk_inst, and which
 * function depends on the program. This file is the default, a ROM with
 * no instructions, so the model builds with no program at all. A program
 * replaces it with a file of its own defining rom_fetch, selected by ROM
 * in the Makefile.
 *
 * Proofs do not depend on the choice: zisk_execute takes the instruction
 * as an argument, and the AIR's own ROM lookup is what ties a row to it.
 */

val rom_fetch : bits(64) -> zisk_inst
function rom_fetch(pc) = {
  assert(false, "no program loaded: rom_fetch has no instructions");
  /* Unreachable; Sail needs a value of the return type. */
  struct {
    a_src = A_IMM, a_imm = zeros(), b_src = B_IMM, b_imm = zeros(),
    ind_width = 8, op = ZOP_COPYB(), store = ST_NONE, store_off = zeros(),
    store_pc = false, set_pc = false, jmp_off1 = zeros(), jmp_off2 = zeros(),
    is_end = true
  }
}
