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

/* --- Interfaces left abstract in this slice -------------------------- *
 * Deliberately undefined: the memory model and the register file are the
 * next milestone. Declaring them lets the instruction semantics below be
 * typechecked precisely while staying honest about what is not modeled.
 */

val rom_fetch : bits(64) -> zisk_inst

/* NOTE: ZisK has no hardwired-zero register. The assembler and the
 * transpiler both rewrite `r0` to SRC_IMM 0 at encode time
 * (see ZiskInstBuilder::src_a), so a_src = A_REG implies index != 0.
 */
val rX : regidx -> bits(64)
val wX : (regidx, bits(64)) -> unit

val read_mem  : forall 'n, 'n in {1, 2, 4, 8}. (bits(64), int('n)) -> bits(8 * 'n)
val write_mem : forall 'n, 'n in {1, 2, 4, 8}. (bits(64), int('n), bits(8 * 'n)) -> unit
