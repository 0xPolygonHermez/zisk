/-
  The ROM's BIOS (core/src/rom_layout.rs, default build): the ZisK code that
  starts every program and ends it, around the transpiled RISC-V code.

  0x1000  jump over the end instruction                      add_end_and_lib
  0x1004  ROM_EXIT: the end
  0x1008  marchid and mtvec to their CSR cells, a0 and a1   add_entry_exit_jmp
          the input and output addresses, call the entry with ra = 0x101C
  0x101C  the output block: publish the 32 output words (pubout), read the
          input at every 128 MB, jump to ROM_EXIT
  0x107C  the ecall handler: a7 = 93 is the exit, with code a0: 0 goes to
          the output block, anything else halts with it; any other ecall
          returns to ra

  Checked against the Rust's ROM by `make transpiler-check`, as the spec.
-/
import Transpiler.Spec

namespace Transpiler

open _root_.Zisk

def ROM_ENTRY : BitVec 64 := 0x1000
def ROM_EXIT : BitVec 64 := 0x1004
/-- Where a program returns to end in success: the output block. -/
def OUTPUT_BLOCK : BitVec 64 := 0x101C
def ARCH_ID_ZISK : BitVec 64 := 0x0FFFEEEE
def ARCH_ID_CSR_ADDR : BitVec 64 := 0xA040F890
def MTVEC_ADDR : BitVec 64 := 0xA0409828
def INPUT_ADDR : BitVec 64 := 0x1000_0000
def OUTPUT_ADDR : BitVec 64 := 0xA041_0000
/-- The step between the BIOS's input reads: 128 MB. -/
def INPUT_ANCHOR_STEP : BitVec 64 := 0x800_0000

/-- copyb(0, b), to the register or memory cell `st`. -/
def copyImm (b : BitVec 64) (st : c_store × BitVec 64 × Bool) (j1 j2 : BitVec 64) : zisk_inst :=
  mk (.A_IMM, 0) (.B_IMM, b) (.ZOP_COPYB ()) st j1 j2

/-- The BIOS, for a program whose entry point is `entry`. -/
def bios (entry : BitVec 64) : List (BitVec 64 × zisk_inst) :=
  [(0x1000, copyImm 0 (.ST_NONE, 0, false) 8 8),
   (0x1004, { copyImm 0 (.ST_NONE, 0, false) 0 0 with is_end := true }),
   (0x1008, copyImm ARCH_ID_ZISK (.ST_MEM, ARCH_ID_CSR_ADDR, false) 4 4),
   (0x100C, copyImm 0x107C (.ST_MEM, MTVEC_ADDR, false) 4 4),
   (0x1010, copyImm INPUT_ADDR (.ST_REG, 10, false) 0 4),
   (0x1014, copyImm OUTPUT_ADDR (.ST_REG, 11, false) 0 4),
   (0x1018, mk (.A_IMM, 0) (.B_IMM, entry) (.ZOP_COPYB ()) (.ST_REG, 1, true) 0 4 (setPc := true)),
   (0x101C, mk (.A_IMM, 0) (.B_IMM, 32) (.ZOP_COPYB ()) (.ST_REG, 11, false) 0 4 (w := 4)),
   (0x1020, copyImm 0 (.ST_REG, 12, false) 0 4),
   (0x1024, copyImm OUTPUT_ADDR (.ST_REG, 13, false) 0 4),
   (0x1028, mk (.A_REG, 11) (.B_REG, 12) (.ZOP_EQ ()) (.ST_NONE, 0, false) 0x14 4),
   (0x102C, mk (.A_REG, 13) (.B_IND, 0) (.ZOP_COPYB ()) (.ST_NONE, 0, false) 0 4),
   (0x1030, mk (.A_REG, 12) (.B_C, 0) (.ZOP_PUBOUT ()) (.ST_NONE, 0, false) 0 4),
   (0x1034, mk (.A_REG, 13) (.B_IMM, 8) (.ZOP_ADD ()) (.ST_REG, 13, false) 0 4),
   (0x1038, mk (.A_REG, 12) (.B_IMM, 1) (.ZOP_ADD ()) (.ST_REG, 12, false) 4 (-16))] ++
  (List.range 15).map (fun k =>
    (0x103C + 4 * BitVec.ofNat 64 k,
     mk (.A_IMM, INPUT_ADDR + BitVec.ofNat 64 (k + 1) * INPUT_ANCHOR_STEP) (.B_IND, 0) (.ZOP_COPYB ())
       (.ST_NONE, 0, false) 4 4)) ++
  [(0x1078, mk (.A_IMM, 0) (.B_IMM, ROM_EXIT) (.ZOP_COPYB ()) (.ST_NONE, 0, false) 0 0 (setPc := true)),
   (0x107C, mk (.A_REG, 17) (.B_IMM, 93) (.ZOP_EQ ()) (.ST_NONE, 0, false) 8 4),
   (0x1080, mk (.A_IMM, 0) (.B_REG, 1) (.ZOP_COPYB ()) (.ST_NONE, 0, false) 0 4 (setPc := true)),
   (0x1084, mk (.A_REG, 10) (.B_IMM, 0) (.ZOP_EQ ()) (.ST_NONE, 0, false) (-0x68) 4),
   (0x1088, { mk (.A_REG, 10) (.B_IMM, 0) (.ZOP_HALT ()) (.ST_NONE, 0, false) 0 0 with is_end := true })]

end Transpiler
