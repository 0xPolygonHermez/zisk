/* The ZisK instruction record.
 *
 * Unlike RISC-V there is no encdec mapping here: a ZisK instruction is
 * never decoded from a bit pattern. The transpiler (or the ziskasm
 * assembler) emits it already decoded and the ROM stores it as a record
 * keyed by address. Mirrors `ZiskInst` in core/src/zisk_inst.rs.
 */

struct zisk_inst = {
  a_src     : a_src,
  a_imm     : bits(64),        /* immediate, address, or register index */
  b_src     : b_src,
  b_imm     : bits(64),
  ind_width : {1, 2, 4, 8},  /* legal indirect widths, pinned statically */
  op        : zisk_op,
  store     : c_store,
  store_off : bits(64),
  store_pc  : bool,            /* store pc + jmp_off2 instead of c (JAL) */
  set_pc    : bool,
  jmp_off1  : bits(64),        /* taken / indirect target                */
  jmp_off2  : bits(64),        /* not taken / fall through               */
  is_end    : bool
}
