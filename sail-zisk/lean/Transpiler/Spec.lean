/-
  The transpiler (transpilers/riscv), as a function in Lean: what ZisK
  instructions it emits for a RISC-V instruction. A hand-written spec,
  kept in step with the Rust by `make transpiler-check`, which runs both on
  the same programs and compares every instruction the spec describes
  (TranspilerCheck/).

  It follows the Rust function by function, so that a change there has an
  obvious place here:
  - `riscv_interpreter.rs`, `riscv_decoder.rs`: the fields and the name of
    the instruction a 32-bit word encodes (`Word`, `decode`);
  - `zisk_inst_builder.rs`: what the builder makes of its arguments, x0
    among them: a source register x0 is the immediate 0, and a store to x0
    is no store (`srcA`, `srcB`, `storeReg`);
  - `riscv2zisk_context.rs`: one function per kind of translation
    (`registerOp`, `immediateOp`, `copyb`, `loadOp`, `storeOp`, ...), and
    `convert`, which picks one per instruction.

  So far: RV64I but for the CSRs; Zifencei; M; A; Zba; Zbs;
  Zbb; and Zbkb. An instruction it does not describe is `none`.

  Internal addresses (the odd ROM addresses a translation of more than one
  ZisK instruction continues at) come, in the Rust, from a counter global to
  the whole ROM. Here a translation takes the first one it may use, `ia`,
  and the k-th is `ia + 2k`; the counter itself, which only has to hand
  out distinct odd addresses, is left out.

  Where the Rust panics on an instruction (an auipc-jalr pair jumping out
  of the ROM, say), the spec says nothing: it describes the transpiler's
  output on programs it accepts.
-/
import Zisk
import ZiskRiscv.Decode

namespace Transpiler

open _root_.Zisk

/-! ## The fields of an instruction word (riscv_interpreter.rs) -/

section Word
variable (w : BitVec 32)

def opcode : BitVec 7 := w.extractLsb 6 0
def funct3 : BitVec 3 := w.extractLsb 14 12
def funct7 : BitVec 7 := w.extractLsb 31 25
def funct6 : BitVec 6 := w.extractLsb 31 26
def rd : Nat := (w.extractLsb 11 7).toNat
def rs1 : Nat := (w.extractLsb 19 15).toNat
def rs2 : Nat := (w.extractLsb 24 20).toNat

/-- I-type: imm[11:0], sign-extended (`i.imm as u64`). -/
def immI : BitVec 64 := (w.extractLsb 31 20).signExtend 64
/-- A shift's amount: the I-type immediate's low 6 bits (`i.imm &= 0x3F`). -/
def shamt : BitVec 64 := (w.extractLsb 25 20).setWidth 64
/-- S-type: imm[11:5] ++ imm[4:0]. -/
def immS : BitVec 64 := (w.extractLsb 31 25 ++ w.extractLsb 11 7).signExtend 64
/-- B-type: imm[12|10:5|4:1|11], times 2. -/
def immB : BitVec 64 :=
  (w.extractLsb 31 31 ++ w.extractLsb 7 7 ++ w.extractLsb 30 25 ++ w.extractLsb 11 8 ++ 0#1).signExtend 64
/-- U-type: imm[31:12], shifted (`((inst & 0xFFFFF000) >> 12) << 12) as i32`). -/
def immU : BitVec 64 := (w.extractLsb 31 12 ++ 0#12).signExtend 64
/-- J-type: imm[20|10:1|11|19:12], times 2. -/
def immJ : BitVec 64 :=
  (w.extractLsb 31 31 ++ w.extractLsb 19 12 ++ w.extractLsb 20 20 ++ w.extractLsb 30 21 ++ 0#1).signExtend 64

end Word

/-! ## The instructions described (riscv_decoder.rs) -/

/-- The register-register ops, by their name in the Rust. -/
inductive ROp | add | sub | sll | slt | sltu | xor | srl | sra | or | and
  | addw | subw | sllw | srlw | sraw
  deriving BEq, Repr

/-- The register-immediate ops. -/
inductive IOp | addi | slti | sltiu | xori | ori | andi | addiw
  deriving BEq, Repr

/-- The shifts by an immediate. -/
inductive SOp | slli | srli | srai | slliw | srliw | sraiw
  deriving BEq, Repr

inductive BOp | beq | bne | blt | bge | bltu | bgeu
  deriving BEq, Repr

inductive LOp | lb | lh | lw | ld | lbu | lhu | lwu
  deriving BEq, Repr

inductive SzOp | sb | sh | sw | sd
  deriving BEq, Repr

/-- The M extension's ops. -/
inductive MOp | mul | mulh | mulhsu | mulhu | div | divu | rem | remu | mulw | divw | divuw | remw | remuw
  deriving BEq, Repr

/-- Zba's shift-and-adds: rs1 shifted by 1, 2 or 3, plus rs2. -/
inductive ShAdd | sh1 | sh2 | sh3
  deriving BEq, Repr

/-- Zbs's single-bit ops: clear, extract, invert, set. -/
inductive ZbsOp | bclr | bext | binv | bset
  deriving BEq, Repr

/-- Zbb's register ops (andn, orn and xnor are in Zbkb too). -/
inductive ZbbOp | andn | orn | xnor | max | maxu | min | minu | rol | ror
  deriving BEq, Repr

/-- Zbb's word rotations. -/
inductive ZbbWOp | rolw | rorw
  deriving BEq, Repr

/-- Zbkb's packs of two registers. -/
inductive PackOp | pack | packh
  deriving BEq, Repr

/-- Zbb's sign extensions of rs1. -/
inductive SextOp | sextb | sexth
  deriving BEq, Repr

/-- The byte ops on rs1: Zbb's rev8 and orc.b, Zbkb's brev8. -/
inductive ByteOp | rev8 | orcb | brev8
  deriving BEq, Repr

/-- A's read-modify-write ops (amoswap, amoadd, ...). -/
inductive AmoOp | swap | add | xor | and | or | min | max | minu | maxu
  deriving BEq, Repr

/-- Zbb's counts on rs1, and their word forms. -/
inductive CountOp | clz | ctz | cpop | clzw | ctzw | cpopw
  deriving BEq, Repr

/-- An instruction the spec describes, as `RiscvDecoder::decode_32` names it. -/
inductive Inst
  | r (op : ROp) | i (op : IOp) | shift (op : SOp) | branch (op : BOp)
  | load (op : LOp) | store (op : SzOp) | lui | auipc | jal | jalr | m (op : MOp) | ecall | fence
  | shadd (op : ShAdd) | slliuw | zbs (op : ZbsOp) | zbsi (op : ZbsOp) | zbb (op : ZbbOp)
  | zbbw (op : ZbbWOp) | rori | roriw | pack (op : PackOp) | packw | sext (op : SextOp) | zexth
  | byte (op : ByteOp) | count (op : CountOp) | adduw | shadduw (op : ShAdd)
  | amo (op : AmoOp) (d : Bool) | lr (d : Bool) | sc (d : Bool) | csr | ebreak
  deriving BEq, Repr

/-- What `decode_32` decodes a word to, among the instructions described.
    As in the Rust (and RISC-V), lui, auipc and jal are any word with their
    opcode. -/
def decode (w : BitVec 32) : Option Inst :=
  let f3 := (funct3 w).toNat
  let f7 := (funct7 w).toNat
  let f6 := (funct6 w).toNat
  match (opcode w).toNat with
  | 3 => match f3 with
    | 0 => some (.load .lb) | 1 => some (.load .lh) | 2 => some (.load .lw) | 3 => some (.load .ld)
    | 4 => some (.load .lbu) | 5 => some (.load .lhu) | 6 => some (.load .lwu) | _ => none
  | 19 => match f3 with
    | 0 => some (.i .addi)
    | 1 =>
      -- sext.b, sext.h, clz, ctz and cpop come first in the Rust; all have funct6 24,
      -- so here, after the rest.
      if f6 == 0 then some (.shift .slli) else if f6 == 10 then some (.zbsi .bset)
      else if f6 == 18 then some (.zbsi .bclr) else if f6 == 26 then some (.zbsi .binv)
      else if (w.extractLsb 31 20).toNat == 0x604 then some (.sext .sextb)
      else if (w.extractLsb 31 20).toNat == 0x605 then some (.sext .sexth)
      else if (w.extractLsb 31 20).toNat == 0x600 then some (.count .clz)
      else if (w.extractLsb 31 20).toNat == 0x601 then some (.count .ctz)
      else if (w.extractLsb 31 20).toNat == 0x602 then some (.count .cpop) else none
    | 2 => some (.i .slti) | 3 => some (.i .sltiu) | 4 => some (.i .xori)
    | 5 =>
      -- rev8, brev8 and orc.b come first in the Rust; none has funct6 0, 16, 18 or 24,
      -- so here, after the rest.
      if f6 == 0 then some (.shift .srli) else if f6 == 16 then some (.shift .srai)
      else if f6 == 18 then some (.zbsi .bext) else if f6 == 24 then some .rori
      else if (w.extractLsb 31 20).toNat == 0x6B8 then some (.byte .rev8)
      else if (w.extractLsb 31 20).toNat == 0x687 then some (.byte .brev8)
      else if (w.extractLsb 31 20).toNat == 0x287 then some (.byte .orcb) else none
    | 6 => some (.i .ori) | 7 => some (.i .andi) | _ => none
  -- fence (fence.tso among them) and fence.i, whatever their fields: a nop
  | 15 => if f3 == 0 || f3 == 1 then some .fence else none
  | 23 => some .auipc
  | 27 => match f3 with
    | 0 => some (.i .addiw)
    | 1 =>
      -- clzw, ctzw, cpopw (funct7 48) come first, then slli.uw (funct6 2).
      if (w.extractLsb 31 20).toNat == 0x600 then some (.count .clzw)
      else if (w.extractLsb 31 20).toNat == 0x601 then some (.count .ctzw)
      else if (w.extractLsb 31 20).toNat == 0x602 then some (.count .cpopw)
      else if f6 == 2 then some .slliuw
      else if f7 == 0 then some (.shift .slliw) else none
    | 5 =>
      if f7 == 0 then some (.shift .srliw) else if f7 == 32 then some (.shift .sraiw)
      else if f7 == 48 then some .roriw else none
    | _ => none
  | 35 => match f3 with
    | 0 => some (.store .sb) | 1 => some (.store .sh) | 2 => some (.store .sw) | 3 => some (.store .sd)
    | _ => none
  | 51 => match f3, f7 with
    | 0, 0 => some (.r .add) | 0, 32 => some (.r .sub) | 1, 0 => some (.r .sll) | 2, 0 => some (.r .slt)
    | 3, 0 => some (.r .sltu) | 4, 0 => some (.r .xor) | 5, 0 => some (.r .srl) | 5, 32 => some (.r .sra)
    | 6, 0 => some (.r .or) | 7, 0 => some (.r .and)
    | 0, 1 => some (.m .mul) | 1, 1 => some (.m .mulh) | 2, 1 => some (.m .mulhsu) | 3, 1 => some (.m .mulhu)
    | 4, 1 => some (.m .div) | 5, 1 => some (.m .divu) | 6, 1 => some (.m .rem) | 7, 1 => some (.m .remu)
    | 2, 16 => some (.shadd .sh1) | 4, 16 => some (.shadd .sh2) | 6, 16 => some (.shadd .sh3)
    | 1, 36 => some (.zbs .bclr) | 5, 36 => some (.zbs .bext) | 1, 52 => some (.zbs .binv)
    | 1, 20 => some (.zbs .bset)
    | 7, 32 => some (.zbb .andn) | 6, 32 => some (.zbb .orn) | 4, 32 => some (.zbb .xnor)
    | 6, 5 => some (.zbb .max) | 7, 5 => some (.zbb .maxu) | 4, 5 => some (.zbb .min) | 5, 5 => some (.zbb .minu)
    | 1, 48 => some (.zbb .rol) | 5, 48 => some (.zbb .ror)
    | 4, 4 => some (.pack .pack) | 7, 4 => some (.pack .packh)
    | _, _ => none
  -- A: funct3 2 (.w) or 3 (.d), funct5 the op; lr's rs2 field must be 0
  | 47 =>
    if f3 == 2 || f3 == 3 then
      let d := f3 == 3
      match (w.extractLsb 31 27).toNat with
      | 2 => if rs2 w = 0 then some (.lr d) else none
      | 3 => some (.sc d)
      | 1 => some (.amo .swap d) | 0 => some (.amo .add d) | 4 => some (.amo .xor d)
      | 12 => some (.amo .and d) | 8 => some (.amo .or d) | 16 => some (.amo .min d)
      | 20 => some (.amo .max d) | 24 => some (.amo .minu d) | 28 => some (.amo .maxu d)
      | _ => none
    else none
  | 55 => some .lui
  | 59 => match f3, f7 with
    | 0, 0 => some (.r .addw) | 0, 32 => some (.r .subw) | 1, 0 => some (.r .sllw)
    | 5, 0 => some (.r .srlw) | 5, 32 => some (.r .sraw) | 1, 48 => some (.zbbw .rolw) | 5, 48 => some (.zbbw .rorw)
    | 0, 4 => some .adduw
    | 2, 16 => some (.shadduw .sh1) | 4, 16 => some (.shadduw .sh2) | 6, 16 => some (.shadduw .sh3)
    -- zext.h is packw with rs2 x0, and comes first in the Rust
    | 4, 4 => if (rs2 w) = 0 then some .zexth else some .packw
    | 0, 1 => some (.m .mulw) | 4, 1 => some (.m .divw) | 5, 1 => some (.m .divuw) | 6, 1 => some (.m .remw)
    | 7, 1 => some (.m .remuw) | _, _ => none
  | 99 => match f3 with
    | 0 => some (.branch .beq) | 1 => some (.branch .bne) | 4 => some (.branch .blt)
    | 5 => some (.branch .bge) | 6 => some (.branch .bltu) | 7 => some (.branch .bgeu) | _ => none
  | 103 => if f3 == 0 then some .jalr else none
  -- ecall and ebreak are the one words 0x00000073 and 0x00100073 (riscv_interpreter.rs)
  | 115 => if w == 0x00000073#32 then some .ecall else if w == 0x00100073#32 then some .ebreak
      else if f3 == 0 then none else some .csr
  | 111 => some .jal
  | _ => none

/-! ## The instruction builder (zisk_inst_builder.rs) -/

/-- A register as an a source: x0 is the immediate 0. -/
def srcA (r : Nat) : a_src × BitVec 64 :=
  if r = 0 then (.A_IMM, 0) else (.A_REG, BitVec.ofNat 64 r)

/-- A register as a b source: x0 is the immediate 0. -/
def srcB (r : Nat) : b_src × BitVec 64 :=
  if r = 0 then (.B_IMM, 0) else (.B_REG, BitVec.ofNat 64 r)

/-- A store to a register (and whether pc + jmp_off2 is what is stored): to
    x0, no store at all, and store_pc stays false. -/
def storeReg (r : Nat) (storePc : Bool := false) : c_store × BitVec 64 × Bool :=
  if r = 0 then (.ST_NONE, 0, false) else (.ST_REG, BitVec.ofNat 64 r, storePc)

/-- An instruction from its parts, the builder's defaults for the rest. -/
def mk (a : a_src × BitVec 64) (b : b_src × BitVec 64) (op : zisk_op)
    (st : c_store × BitVec 64 × Bool := (.ST_NONE, 0, false)) (j1 j2 : BitVec 64)
    (setPc : Bool := false) (w : Int := 8) : zisk_inst :=
  { a_src := a.1, a_imm := a.2, b_src := b.1, b_imm := b.2, ind_width := w, op,
    store := st.1, store_off := st.2.1, store_pc := st.2.2, set_pc := setPc,
    jmp_off1 := j1, jmp_off2 := j2, is_end := false }

/-! ## The translations (riscv2zisk_context.rs) -/

/-- The BIOS's ecall handler, where `ecall` jumps (core/src/rom_layout.rs,
    default build: 0x1008 + 4 · (13 + 15 + 1)). -/
def ECALL_HANDLER_ADDR : BitVec 64 := 0x107C#64

/-- The ROM's mask for a jalr target: bit 0 cleared (`JALR_MASK`). -/
def JALR_MASK : BitVec 64 := 0xfffffffffffffffe#64

section Translations
variable (w : BitVec 32)

/-- `create_register_op`: op(r[rs1], r[rs2]) -> r[rd]. -/
def registerOp (op : zisk_op) : zisk_inst :=
  mk (srcA (rs1 w)) (srcB (rs2 w)) op (storeReg (rd w)) 4 4

/-- `immediate_op`: op(r[rs1], imm) -> r[rd]. -/
def immediateOp (op : zisk_op) (imm : BitVec 64) : zisk_inst :=
  mk (srcA (rs1 w)) (.B_IMM, imm) op (storeReg (rd w)) 4 4

/-- `immediate_op_or_x0_copyb`: as `immediate_op`, but copyb when rs1 is x0. -/
def immediateOpOrCopyb (op : zisk_op) : zisk_inst :=
  immediateOp w (if rs1 w = 0 then .ZOP_COPYB () else op) (immI w)

/-- `copyb`: r[rs1] or r[rs2] (rs = 1 or 2) -> r[rd]. -/
def copyb (rs : Nat) : zisk_inst :=
  mk (.A_IMM, 0) (srcB (if rs = 1 then rs1 w else rs2 w)) (.ZOP_COPYB ()) (storeReg (rd w)) 4 4

/-- `create_single_source_register_op` (rs = 1): op(0, r[rs1]) -> r[rd]. -/
def singleOp (op : zisk_op) : zisk_inst :=
  mk (.A_IMM, 0) (srcB (rs1 w)) op (storeReg (rd w)) 4 4

/-- `zero_extend_h`: and(r[rs1], 0xffff) -> r[rd]. -/
def zextH : zisk_inst :=
  mk (srcA (rs1 w)) (.B_IMM, 0xFFFF) (.ZOP_AND ()) (storeReg (rd w)) 4 4

/-- `nop`: flag(0, 0), on to pc + 4. -/
def nop : zisk_inst := mk (.A_IMM, 0) (.B_IMM, 0) (.ZOP_FLAG ()) (j1 := 4) (j2 := 4)

/-- `hint`: flag(r[rs1], imm), on to pc + 4. -/
def hint : zisk_inst := mk (srcA (rs1 w)) (.B_IMM, immI w) (.ZOP_FLAG ()) (j1 := 4) (j2 := 4)

/-- `create_branch_op`: op(r[rs1], r[rs2]); on its flag (or its negation)
    to pc + imm, else on to pc + 4. -/
def branchOp (op : zisk_op) (neg : Bool) : zisk_inst :=
  if neg then mk (srcA (rs1 w)) (srcB (rs2 w)) op (j1 := 4) (j2 := immB w)
  else mk (srcA (rs1 w)) (srcB (rs2 w)) op (j1 := immB w) (j2 := 4)

/-- `load_op`: op(r[rs1], mem[r[rs1] + imm], width bytes) -> r[rd]. -/
def loadOp (op : zisk_op) (width : Int) : zisk_inst :=
  mk (srcA (rs1 w)) (.B_IND, immI w) op (storeReg (rd w)) 4 4 (w := width)

/-- `store_op`: copyb(r[rs1], r[rs2]) -> mem[r[rs1] + imm], width bytes. -/
def storeOp (width : Int) : zisk_inst :=
  mk (srcA (rs1 w)) (srcB (rs2 w)) (.ZOP_COPYB ()) (.ST_IND, immS w, false) 4 4 (w := width)

/-- `lui`: copyb(0, imm) -> r[rd]. -/
def lui : zisk_inst := mk (.A_IMM, 0) (.B_IMM, immU w) (.ZOP_COPYB ()) (storeReg (rd w)) 4 4

/-- `jal`: flag(0, 0), pc + 4 -> r[rd], to pc + imm. -/
def jal : zisk_inst :=
  mk (.A_IMM, 0) (.B_IMM, 0) (.ZOP_FLAG ()) (storeReg (rd w) true) (immJ w) 4

/-- `jalr` (pc, the first internal address ia): with an even offset, one
    instruction, mask &&& r[rs1] to pc + imm; with an odd one, two: imm +
    r[rs1] at pc, then mask &&& c at ia, back to pc + 4 for the link. -/
def jalr (pc ia : BitVec 64) : List (BitVec 64 × zisk_inst) :=
  if (immI w).getLsbD 0 = false then
    [(pc, mk (.A_IMM, JALR_MASK) (srcB (rs1 w)) (.ZOP_AND ()) (storeReg (rd w) true) (immI w) 4 (setPc := true))]
  else
    [(pc, mk (.A_IMM, immI w) (srcB (rs1 w)) (.ZOP_ADD ()) (j1 := ia - pc) (j2 := ia - pc)),
     (ia, mk (.A_IMM, JALR_MASK) (.B_C, 0) (.ZOP_AND ()) (storeReg (rd w) true) 0 (pc + 4 - ia)
       (setPc := true))]

/-- Whether `auipc` fuses with the word after it: a jalr that jumps through
    its rd, not x0, and links to rd or x0. -/
def fuses (n : BitVec 32) : Bool :=
  decode n == some .jalr && rd w ≠ 0 && rs1 n = rd w && (rd n = rd w || rd n = 0)

/-- `auipc` (pc, the next word, if any): fused with a jalr after it that
    jumps through rd and links to rd or x0, one instruction, the constant to
    rd and the jump made at transpile time; else copyb(0, pc + imm) -> r[rd]. -/
def auipc (pc : BitVec 64) (next : Option (BitVec 32)) : zisk_inst :=
  match next with
  | some n =>
    if fuses w n then
      let auipcResult := pc + immU w
      let jumpPc := (auipcResult + immI n) &&& JALR_MASK
      let c := if rd n = 0 then auipcResult else pc + 8
      mk (.A_IMM, 0) (.B_IMM, c) (.ZOP_COPYB ()) (storeReg (rd w)) (jumpPc - pc) (jumpPc - pc)
    else plain
  | none => plain
where plain := mk (.A_IMM, 0) (.B_IMM, pc + immU w) (.ZOP_COPYB ()) (storeReg (rd w)) 4 4

/-- `ecall`: pc + 4 -> r[ra], on to the BIOS's ecall handler. -/
def ecall : zisk_inst :=
  mk (.A_IMM, 0) (.B_IMM, ECALL_HANDLER_ADDR) (.ZOP_COPYB ()) (storeReg 1 true) 0 4 (setPc := true)

end Translations

def ROp.zop : ROp → zisk_op
  | .add => .ZOP_ADD () | .sub => .ZOP_SUB () | .sll => .ZOP_SLL () | .slt => .ZOP_LT ()
  | .sltu => .ZOP_LTU () | .xor => .ZOP_XOR () | .srl => .ZOP_SRL () | .sra => .ZOP_SRA ()
  | .or => .ZOP_OR () | .and => .ZOP_AND () | .addw => .ZOP_ADD_W () | .subw => .ZOP_SUB_W ()
  | .sllw => .ZOP_SLL_W () | .srlw => .ZOP_SRL_W () | .sraw => .ZOP_SRA_W ()

/-- The op the transpiler emits for each Zba shift-and-add (riscv2zisk_context.rs). -/
def ShAdd.zop : ShAdd → zisk_op
  | .sh1 => .ZOP_SH1ADD () | .sh2 => .ZOP_SH2ADD () | .sh3 => .ZOP_SH3ADD ()

/-- The op the transpiler emits for each Zbs instruction, register or immediate. -/
def ZbsOp.zop : ZbsOp → zisk_op
  | .bclr => .ZOP_BCLR () | .bext => .ZOP_BEXT () | .binv => .ZOP_BINV () | .bset => .ZOP_BSET ()

/-- The op the transpiler emits for each Zbb register op. -/
def ZbbOp.zop : ZbbOp → zisk_op
  | .andn => .ZOP_ANDN () | .orn => .ZOP_ORN () | .xnor => .ZOP_XNOR () | .max => .ZOP_MAX ()
  | .maxu => .ZOP_MAXU () | .min => .ZOP_MIN () | .minu => .ZOP_MINU () | .rol => .ZOP_ROL () | .ror => .ZOP_ROR ()

def ZbbWOp.zop : ZbbWOp → zisk_op
  | .rolw => .ZOP_ROL_W () | .rorw => .ZOP_ROR_W ()

def PackOp.zop : PackOp → zisk_op
  | .pack => .ZOP_PACK () | .packh => .ZOP_PACK_H ()

def SextOp.zop : SextOp → zisk_op
  | .sextb => .ZOP_SIGNEXTEND_B () | .sexth => .ZOP_SIGNEXTEND_H ()

def ByteOp.zop : ByteOp → zisk_op
  | .rev8 => .ZOP_REV8 () | .orcb => .ZOP_ORC_B () | .brev8 => .ZOP_BREV8 ()

def CountOp.zop : CountOp → zisk_op
  | .clz => .ZOP_CLZ () | .ctz => .ZOP_CTZ () | .cpop => .ZOP_CPOP ()
  | .clzw => .ZOP_CLZ_W () | .ctzw => .ZOP_CTZ_W () | .cpopw => .ZOP_CPOP_W ()

/-- The op the transpiler emits for each M instruction (riscv2zisk_context.rs). -/
def MOp.zop : MOp → zisk_op
  | .mul => .ZOP_MUL () | .mulh => .ZOP_MULH () | .mulhsu => .ZOP_MULSUH () | .mulhu => .ZOP_MULUH ()
  | .div => .ZOP_DIV () | .divu => .ZOP_DIVU () | .rem => .ZOP_REM () | .remu => .ZOP_REMU ()
  | .mulw => .ZOP_MUL_W () | .divw => .ZOP_DIV_W () | .divuw => .ZOP_DIVU_W () | .remw => .ZOP_REM_W ()
  | .remuw => .ZOP_REMU_W ()

def SOp.zop : SOp → zisk_op
  | .slli => .ZOP_SLL () | .srli => .ZOP_SRL () | .srai => .ZOP_SRA ()
  | .slliw => .ZOP_SLL_W () | .srliw => .ZOP_SRL_W () | .sraiw => .ZOP_SRA_W ()

/-- `add_u_w` and `sh_add_u_w_native`: rs1's low word, and(r[rs1],
    0xffffffff), to ZisK's register 32, on to the internal address ia; there
    add(r[rs2], r[32]) (add.uw) or sh<n>add(r[32], r[rs2]) -> r[rd], on to
    pc + 4. -/
def uw (w : BitVec 32) (op : Option ShAdd) (pc ia : BitVec 64) : List (BitVec 64 × zisk_inst) :=
  [(pc, mk (srcA (rs1 w)) (.B_IMM, 0xFFFFFFFF) (.ZOP_AND ()) (storeReg 32) (ia - pc) (ia - pc)),
   (ia, match op with
    | none => mk (srcA (rs2 w)) (srcB 32) (.ZOP_ADD ()) (storeReg (rd w)) (pc + 4 - ia) (pc + 4 - ia)
    | some o => mk (srcA 32) (srcB (rs2 w)) o.zop (storeReg (rd w)) (pc + 4 - ia) (pc + 4 - ia))]

/-- The op an AMO applies to the loaded value (as a, last c) and rs2 (b). -/
def AmoOp.zop (d : Bool) : AmoOp → zisk_op
  | .swap => .ZOP_COPYB () | .add => if d then .ZOP_ADD () else .ZOP_ADD_W () | .xor => .ZOP_XOR ()
  | .and => .ZOP_AND () | .or => .ZOP_OR () | .min => if d then .ZOP_MIN () else .ZOP_MIN_W ()
  | .max => if d then .ZOP_MAX () else .ZOP_MAX_W () | .minu => if d then .ZOP_MINU () else .ZOP_MINU_W ()
  | .maxu => if d then .ZOP_MAXU () else .ZOP_MAXU_W ()

/-- The load an A instruction starts with: copyb (.d) or signextend_w (.w). -/
def aLoad (d : Bool) : zisk_op := if d then .ZOP_COPYB () else .ZOP_SIGNEXTEND_W ()

/-- The access width of an A instruction: 8 (.d) or 4 (.w). -/
def aWidth (d : Bool) : Int := if d then 8 else 4

/-- `create_atomic_op` and `create_atomic_swap`: the load, at r[rs1], to rd
    (or, when rd is rs1 or rs2, to ZisK's register 32), on to ia; for all
    but the swap, the op of last c and r[rs2], on to ia + 2; the store of
    last c (the swap: r[rs2]) at r[rs1]; when rd is rs1 or rs2, last, the
    copy of r[32] to rd. On to pc + 4. -/
def amo (w : BitVec 32) (op : AmoOp) (d : Bool) (pc ia : BitVec 64) : List (BitVec 64 × zisk_inst) :=
  let direct := rd w ≠ rs1 w ∧ rd w ≠ rs2 w
  let dst := if direct then rd w else 32
  let load (j : BitVec 64) := mk (srcA (rs1 w)) (.B_IND, 0) (aLoad d) (storeReg dst) (j - pc) (j - pc) (w := aWidth d)
  let store (a j : BitVec 64) (b : b_src × BitVec 64) :=
    (a, mk (srcA (rs1 w)) b (.ZOP_COPYB ()) (.ST_IND, 0, false) (j - a) (j - a) (w := aWidth d))
  let copy (a : BitVec 64) := (a, mk (.A_IMM, 0) (srcB 32) (.ZOP_COPYB ()) (storeReg (rd w)) (pc + 4 - a) (pc + 4 - a))
  match op with
  | .swap =>
    if direct then [(pc, load ia), store ia (pc + 4) (srcB (rs2 w))]
    else [(pc, load ia), store ia (ia + 2) (srcB (rs2 w)), copy (ia + 2)]
  | _ =>
    let mid := (ia, mk (.A_C, 0) (srcB (rs2 w)) (op.zop d) (.ST_NONE, 0, false) 2 2)
    if direct then [(pc, load ia), mid, store (ia + 2) (pc + 4) (.B_C, 0)]
    else [(pc, load ia), mid, store (ia + 2) (ia + 4) (.B_C, 0), copy (ia + 4)]

/-- `sc_w`, `sc_d`: the store of r[rs2] at r[rs1], always; then 0 to rd,
    when rd is not x0. -/
def sc (w : BitVec 32) (d : Bool) (pc ia : BitVec 64) : List (BitVec 64 × zisk_inst) :=
  if rd w = 0 then
    [(pc, mk (srcA (rs1 w)) (srcB (rs2 w)) (.ZOP_COPYB ()) (.ST_IND, 0, false) 4 4 (w := aWidth d))]
  else
    [(pc, mk (srcA (rs1 w)) (srcB (rs2 w)) (.ZOP_COPYB ()) (.ST_IND, 0, false) (ia - pc) (ia - pc) (w := aWidth d)),
     (ia, mk (.A_IMM, 0) (.B_IMM, 0) (.ZOP_COPYB ()) (storeReg (rd w)) (pc + 4 - ia) (pc + 4 - ia)
       (w := if d then 8 else 4))]


/-! ## The CSRs (contract §§ 6–8) -/

/-- The CSR cells (`CSR_ADDR`): CSR k's at CSR_ADDR + 8k. -/
def CSR_ADDR : BitVec 64 := 0xA0408000

/-- Where a call with a count operand puts it (`EXTRA_PARAMS_ADDR`). -/
def EXTRA_PARAMS_ADDR : BitVec 64 := 0xA0400F00

/-- `halt_with_error`: the halt, ending the run in error. -/
def halt : zisk_inst := { mk (.A_IMM, 0) (.B_IMM, 0) (.ZOP_HALT ()) (j1 := 4) (j2 := 4) with is_end := true }

/-- A call's operand as an a source. -/
def argA : ZiskRiscv.Arg → a_src × BitVec 64
  | .reg r => srcA r
  | .imm v => (.A_IMM, v)
  | .freeInput => (.A_MEM, 0x1000_0000)

/-- A call's operand as a b source; the free input is the 8 bytes at INPUT_ADDR. -/
def argB : ZiskRiscv.Arg → b_src × BitVec 64
  | .reg r => srcB r
  | .imm v => (.B_IMM, v)
  | .freeInput => (.B_MEM, 0x1000_0000)

/-- The ops whose jmp_off1 is the next instruction's offset, not an
    extended argument. -/
def fcallOp : zisk_op → Bool
  | .ZOP_FCALL_PARAM () | .ZOP_FCALL_GET () | .ZOP_FCALL () => true
  | _ => false

/-- A ZisK call as the transpiler emits it (`create_extended_precompiles_op`
    and its kin): the op on a and b, c to rd, the extended argument in
    jmp_off1, on past the sequence; with a count, the count first stored at
    EXTRA_PARAMS_ADDR, the op at ia (`create_precompiles_triple_param_op`). -/
def callInsts (c : ZiskRiscv.Call) (pc ia : BitVec 64) : List (BitVec 64 × zisk_inst) :=
  let next := BitVec.ofNat 64 (4 * c.len)
  match c.count with
  | none =>
    [(pc, mk (argA c.a) (argB c.b) c.op (storeReg c.rd) (if fcallOp c.op then next else c.ext) next)]
  | some r =>
    [(pc, mk (.A_IMM, 0) (srcB r) (.ZOP_COPYB ()) (.ST_MEM, EXTRA_PARAMS_ADDR, false) (ia - pc) (ia - pc)),
     (ia, mk (argA c.a) (argB c.b) c.op (storeReg c.rd) 0 (pc + next - ia))]

/-- The words the machine's decoder reads: w at pc, the next word after it. -/
def fetch2 (pc : BitVec 64) (w : BitVec 32) (next : Option (BitVec 32)) : Nat → Option (BitVec 32) :=
  fun a => if a = pc.toNat then some w else if a = pc.toNat + 4 then next else none

/-- A CSR instruction that writes a read-only CSR, its number's bits 11:10
    set (`writes_read_only_csr`): csrrw and csrrwi always write; csrrs and
    csrrc when rs1 is not x0; csrrsi and csrrci when their immediate (rs1's
    field) is not 0. -/
def writesReadOnly (f : ZiskRiscv.Fields) : Bool :=
  let writes := match ZiskRiscv.kind f with
    | .csrrw | .csrrwi => true
    | .csrrs | .csrrc | .csrrsi | .csrrci => f.rs1 != 0
    | _ => false
  writes && f.csr / 1024 % 4 == 3

/-- A CSR instruction, in the contract's forms: what the machine's decoder
    (ZiskRiscv/Decode.lean, the transpiler's dispatch) makes of it — a ZisK
    call, `csrs ID, x0` (a nop), a sequence the transpiler refuses (a halt)
    — reading `marchid` (a copy of its cell), and a write to a read-only
    CSR (`writesReadOnly`: a halt, the run's failure, as `unimp`'s
    `csrrw x0, cycle, x0`). Other CSR accesses, and
    the memset sequence of three words, which the next word does not
    settle (`csr3` describes it), are not described; nor are zkvmcalls
    (precondition Z). -/
def csr (w : BitVec 32) (pc : BitVec 64) (next : Option (BitVec 32)) (ia : BitVec 64) : Option (List (BitVec 64 × zisk_inst)) :=
  let f := ZiskRiscv.fields w
  match ZiskRiscv.decode (fetch2 pc w next) pc.toNat with
  | .call c => some (callInsts c pc ia)
  | .nop =>
    if 0x850 ≤ f.csr ∧ f.csr ≤ 0x8BF ∨ 0x8E0 ≤ f.csr ∧ f.csr ≤ 0x8E7 then none
    else some [(pc, mk (.A_IMM, 0) (.B_IMM, 0) (.ZOP_COPYB ()) (j1 := 4) (j2 := 4))]
  | .malformed _ =>
    if ZiskRiscv.kind f == .csrrsi && f.csr == 0x816 && f.rs1 == 2 then none else some [(pc, halt)]
  | .riscv =>
    if ZiskRiscv.kind f == .csrrs && f.csr == 0xF12 && f.rs1 == 0 && f.rd != 0 then
      some [(pc, mk (.A_IMM, 0) (.B_MEM, CSR_ADDR + 0xF12 * 8) (.ZOP_COPYB ()) (storeReg f.rd) 4 4)]
    else if writesReadOnly f then some [(pc, halt)]
    else none
  | _ => none

/-- The words the machine's decoder reads: w at pc, the two words after it. -/
def fetch3 (pc : BitVec 64) (w : BitVec 32) (next next2 : Option (BitVec 32)) : Nat → Option (BitVec 32) :=
  fun a => if a = pc.toNat then some w else if a = pc.toNat + 4 then next
    else if a = pc.toNat + 8 then next2 else none

/-- A CSR instruction, given the word after the next one too: `csr`, but
    for the memset sequence of three words (`csrsi 0x816, 2` and two
    `addi`), which the two words after it settle — a call, or a sequence
    the transpiler refuses. -/
def csr3 (w : BitVec 32) (pc : BitVec 64) (next next2 : Option (BitVec 32)) (ia : BitVec 64) :
    Option (List (BitVec 64 × zisk_inst)) :=
  match next2 with
  | none => csr w pc next ia
  | some n2 =>
    let f := ZiskRiscv.fields w
    if ZiskRiscv.kind f == .csrrsi && f.csr == 0x816 && f.rs1 == 2 && next.isSome then
      match ZiskRiscv.decode (fetch3 pc w next (some n2)) pc.toNat with
      | .call c => some (callInsts c pc ia)
      | .malformed _ => some [(pc, halt)]
      | _ => none
    else csr w pc next ia

/-- `convert`: the ZisK instructions for the instruction w at pc, followed
    by the word `next` (none at the end of the code, or before a 16-bit
    parcel), with the internal addresses from ia on; `next2`, the word
    after that, if known, settles the memset sequence of three words. -/
def convert (pc : BitVec 64) (w : BitVec 32) (next : Option (BitVec 32)) (ia : BitVec 64)
    (next2 : Option (BitVec 32) := none) : Option (List (BitVec 64 × zisk_inst)) :=
  let one (i : zisk_inst) := some [(pc, i)]
  match decode w with
  | none => none
  | some (.r op) =>
    match op with
    | .add => if rs1 w = 0 then one (copyb w 2) else if rs2 w = 0 then one (copyb w 1)
        else one (registerOp w op.zop)
    | .or => if rs1 w = 0 then one (copyb w 2) else if rs2 w = 0 then one (copyb w 1)
        else one (registerOp w op.zop)
    | _ => one (registerOp w op.zop)
  | some (.i op) =>
    match op with
    | .addi =>
      if rd w = 0 then (if rs1 w = 0 then one nop else one (hint w))
      else if immI w = 0 && rs1 w ≠ 0 then one (copyb w 1)
      else one (immediateOpOrCopyb w (.ZOP_ADD ()))
    | .slti => one (immediateOp w (.ZOP_LT ()) (immI w))
    | .sltiu => one (immediateOp w (.ZOP_LTU ()) (immI w))
    | .xori => one (immediateOpOrCopyb w (.ZOP_XOR ()))
    | .ori => one (immediateOpOrCopyb w (.ZOP_OR ()))
    | .andi => one (immediateOp w (.ZOP_AND ()) (immI w))
    | .addiw =>
      if rd w = 0 && rs1 w = 0 && immI w = 0 then one nop
      else one (immediateOp w (.ZOP_ADD_W ()) (immI w))
  | some (.shift op) => one (immediateOp w op.zop (shamt w))
  | some (.branch op) =>
    match op with
    | .beq => one (branchOp w (.ZOP_EQ ()) false) | .bne => one (branchOp w (.ZOP_EQ ()) true)
    | .blt => one (branchOp w (.ZOP_LT ()) false) | .bge => one (branchOp w (.ZOP_LT ()) true)
    | .bltu => one (branchOp w (.ZOP_LTU ()) false) | .bgeu => one (branchOp w (.ZOP_LTU ()) true)
  | some (.load op) =>
    match op with
    | .lb => one (loadOp w (.ZOP_SIGNEXTEND_B ()) 1) | .lbu => one (loadOp w (.ZOP_COPYB ()) 1)
    | .lh => one (loadOp w (.ZOP_SIGNEXTEND_H ()) 2) | .lhu => one (loadOp w (.ZOP_COPYB ()) 2)
    | .lw => one (loadOp w (.ZOP_SIGNEXTEND_W ()) 4) | .lwu => one (loadOp w (.ZOP_COPYB ()) 4)
    | .ld => one (loadOp w (.ZOP_COPYB ()) 8)
  | some (.store op) =>
    match op with
    | .sb => one (storeOp w 1) | .sh => one (storeOp w 2) | .sw => one (storeOp w 4) | .sd => one (storeOp w 8)
  | some .lui => one (lui w)
  | some .auipc => one (auipc w pc next)
  | some .jal => one (jal w)
  | some .jalr => some (jalr w pc ia)
  | some (.m op) => one (registerOp w op.zop)
  | some .ecall => one ecall
  -- a breakpoint: the run fails there (halt_with_error)
  | some .ebreak => one halt
  | some .fence => one nop
  | some (.shadd op) => one (registerOp w op.zop)
  | some .slliuw => one (immediateOp w (.ZOP_SLL_U_W ()) (shamt w))
  | some (.zbs op) => one (registerOp w op.zop)
  | some (.zbsi op) => one (immediateOp w op.zop (shamt w))
  | some (.zbb op) => one (registerOp w op.zop)
  | some (.zbbw op) => one (registerOp w op.zop)
  | some .rori => one (immediateOp w (.ZOP_ROR ()) (shamt w))
  | some .roriw => one (immediateOp w (.ZOP_ROR_W ()) (shamt w))
  | some (.pack op) => one (registerOp w op.zop)
  | some .packw => one (registerOp w (.ZOP_PACK_W ()))
  | some (.sext op) => one (singleOp w op.zop)
  | some .zexth => one (zextH w)
  | some (.byte op) => one (singleOp w op.zop)
  | some (.count op) => one (singleOp w op.zop)
  | some .adduw => some (uw w none pc ia)
  | some (.shadduw op) => some (uw w (some op) pc ia)
  | some (.amo op d) => some (amo w op d pc ia)
  | some (.lr d) => one (mk (srcA (rs1 w)) (.B_IND, 0) (aLoad d) (storeReg (rd w)) 4 4 (w := aWidth d))
  | some (.sc d) => some (sc w d pc ia)
  | some .csr => csr3 w pc next next2 ia

end Transpiler
