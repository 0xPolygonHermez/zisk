/* The ROM as data: the C emulator's rom_fetch, for any program.
 *
 * Compiling a program's ROM into the emulator as one `match` on the pc makes
 * every program its own emulator, and a large one slow to build: a RISC-V
 * test transpiles to thousands of instructions. So the emulator reads the
 * ROM instead, from a table tools/sail-difftest writes into memory beside
 * the program's own sections (`-b addr,file`), and one emulator runs every
 * program. Proofs do not use this file: zisk_execute takes the instruction
 * as an argument, and the Lean build keeps the default ROM.
 *
 * The table holds a 64-byte entry for the instruction at pc, at
 * ROM_TABLE + 64 * pc, far above every address ZisK uses. Eight
 * little-endian words:
 *
 *   word 0   bit 0 valid; bits 8..15 a_src, 16..23 b_src, 24..31 store,
 *            32..39 ind_width, 40 store_pc, 41 set_pc, 42 is_end,
 *            48..55 the opcode
 *   word 1   a_imm        word 2   b_imm        word 3   store_off
 *   word 4   jmp_off1     word 5   jmp_off2     words 6, 7   zero
 *
 * a_src is A_C 0, A_MEM 1, A_IMM 2, A_STEP 3, A_REG 4; b_src is B_C 0,
 * B_MEM 1, B_IMM 2, B_IND 3, B_REG 4; store is ST_NONE 0, ST_MEM 1,
 * ST_IND 2, ST_REG 3.
 */

let ROM_TABLE : bits(64) = 0x0001_0000_0000_0000

val rom_word : (bits(64), int) -> bits(64)
function rom_word(entry, k) = __read_ram(64, 8, zeros(), entry + to_bits(64, 8 * k))

val decode_a_src : bits(8) -> a_src
function decode_a_src(x) = match x {
  0x00 => A_C, 0x01 => A_MEM, 0x02 => A_IMM, 0x03 => A_STEP, 0x04 => A_REG,
  _ => { assert(false, "ROM table: bad a_src"); A_C }
}

val decode_b_src : bits(8) -> b_src
function decode_b_src(x) = match x {
  0x00 => B_C, 0x01 => B_MEM, 0x02 => B_IMM, 0x03 => B_IND, 0x04 => B_REG,
  _ => { assert(false, "ROM table: bad b_src"); B_C }
}

val decode_store : bits(8) -> c_store
function decode_store(x) = match x {
  0x00 => ST_NONE, 0x01 => ST_MEM, 0x02 => ST_IND, 0x03 => ST_REG,
  _ => { assert(false, "ROM table: bad store"); ST_NONE }
}

val decode_width : bits(8) -> {1, 2, 4, 8}
function decode_width(x) = match x {
  0x01 => 1, 0x02 => 2, 0x04 => 4, 0x08 => 8,
  _ => { assert(false, "ROM table: bad ind_width"); 8 }
}

val rom_fetch : bits(64) -> zisk_inst
function rom_fetch(pc) = {
  let entry = ROM_TABLE + (pc << 6);
  let w0 = rom_word(entry, 0);
  assert(w0[0] == bitone, "no instruction at this pc");
  let op : zisk_op = match zisk_op_of_code(w0[55 .. 48]) {
    Some(op) => op,
    None()   => { assert(false, "an opcode the model does not specify"); ZOP_FLAG() }
  };
  struct {
    a_src     = decode_a_src(w0[15 .. 8]),
    a_imm     = rom_word(entry, 1),
    b_src     = decode_b_src(w0[23 .. 16]),
    b_imm     = rom_word(entry, 2),
    ind_width = decode_width(w0[39 .. 32]),
    op        = op,
    store     = decode_store(w0[31 .. 24]),
    store_off = rom_word(entry, 3),
    store_pc  = w0[40] == bitone,
    set_pc    = w0[41] == bitone,
    jmp_off1  = rom_word(entry, 4),
    jmp_off2  = rom_word(entry, 5),
    is_end    = w0[42] == bitone
  }
}
