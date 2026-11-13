/-
  The contract's CSR sequences (doc/riscv_contract.md, §§ 7–8), recognised
  in RISC-V instruction words.

  ZisK calls its precompiles, DMA and free input through `csrs`/`csrsi`/
  `csrwi`/`csrr` on CSR numbers 0x800–0x8FF and 0xFFE, sometimes followed by
  an `add`/`addi` that carries more operands. `decode` finds such a sequence
  at an address and says which sail-zisk operation it is, with which
  operands, and how many instructions it takes; or that the instruction is
  the `ecall` the contract treats apart (§ 5); or that it is `lr` or `sc`,
  which the contract gives its own meaning (§ 1: a plain load, an
  unconditional store); or that it is plain RISC-V, left to sail-riscv.

  It follows the transpiler's dispatch exactly
  (transpilers/riscv/src/riscv2zisk_context.rs: `csrrs`, `csrrsi`, `csrrwi`
  and the `transpile_*_pattern` functions), in its default build. What the
  transpiler refuses — a sequence of the wrong shape, or a SYSTEM
  instruction other than `ecall`, `ebreak` and the CSR accesses (`mret`,
  `wfi`, `sfence.vma`, ..., § 1) — is `malformed`: the transpiler makes it a
  halt, so a run fails if it gets there. sail-riscv would run `mret`, `wfi`
  and `sfence.vma` in machine mode; the machine departs from it there. CSR
  accesses outside these forms are not ZisK calls; by precondition C a
  program in the contract makes none but reading `marchid`, and they are
  left to sail-riscv.

  Pure and computable: it reads instruction words through `fetch`, and
  nothing else.
-/
import Zisk

namespace ZiskRiscv

open Zisk (zisk_op)

/-- The fields of an instruction word the contract reads. -/
structure Fields where
  opcode : Nat
  rd     : Nat
  funct3 : Nat
  rs1    : Nat  -- also the 5-bit immediate of the CSR immediate forms
  rs2    : Nat
  funct7 : Nat
  csr    : Nat
  immI   : BitVec 64  -- the I-type immediate, sign-extended
deriving Repr

def fields (w : BitVec 32) : Fields where
  opcode := (w.extractLsb' 0 7).toNat
  rd     := (w.extractLsb' 7 5).toNat
  funct3 := (w.extractLsb' 12 3).toNat
  rs1    := (w.extractLsb' 15 5).toNat
  rs2    := (w.extractLsb' 20 5).toNat
  funct7 := (w.extractLsb' 25 7).toNat
  csr    := (w.extractLsb' 20 12).toNat
  immI   := (w.extractLsb' 20 12).signExtend 64

/-- The instructions the contract tells apart. -/
inductive Kind where
  | csrrw | csrrs | csrrc | csrrwi | csrrsi | csrrci
  | add | addi | ecall | lr | sc
  /-- SYSTEM with funct3 0, but `ecall` and `ebreak`: `mret`, `wfi`, ... -/
  | system
  | other
deriving Repr, BEq, DecidableEq

def kind (f : Fields) : Kind :=
  match f.opcode, f.funct3 with
  | 0x73, 1 => .csrrw  | 0x73, 2 => .csrrs  | 0x73, 3 => .csrrc
  | 0x73, 5 => .csrrwi | 0x73, 6 => .csrrsi | 0x73, 7 => .csrrci
  | 0x73, 0 =>
    if f.rs1 == 0 && f.rd == 0 && f.csr == 0 then .ecall
    else if f.rs1 == 0 && f.rd == 0 && f.csr == 1 then .other  -- ebreak: sail-riscv's trap
    else .system
  | 0x33, 0 => if f.funct7 == 0 then .add else .other
  | 0x13, 0 => .addi
  -- lr (funct5 2, its rs2 field 0: any other is reserved) and sc (funct5 3), .w or .d
  | 0x2F, 2 | 0x2F, 3 =>
    if f.funct7 >>> 2 == 2 && f.rs2 == 0 then .lr else if f.funct7 >>> 2 == 3 then .sc else .other
  | _, _ => .other

/-- An operand of a call: a register's value, a constant, or the free input
    (what an 8-byte load of INPUT_ADDR returns). -/
inductive Arg where
  | reg (r : Nat)
  | imm (v : BitVec 64)
  | freeInput
deriving Repr

/-- A ZisK call: sail-zisk's operation `op` on `a`, `b` and the extended
    argument `ext`. When `count` is given, its value is first stored at
    EXTRA_PARAMS_ADDR, where the operation reads its byte count. `rd`, when
    not 0, gets the operation's `c`. The call takes `len` instructions; the
    program continues after them. -/
structure Call where
  op    : zisk_op
  a     : Arg := .imm 0
  b     : Arg
  ext   : BitVec 64 := 0
  count : Option Nat := none
  rd    : Nat := 0
  len   : Nat

instance : Repr Call where
  reprPrec c _ := s!"Call(a={repr c.a}, b={repr c.b}, ext={c.ext}, count={c.count}, rd={c.rd}, len={c.len})"

inductive Decoded where
  /-- A ZisK call (§§ 7–8). -/
  | call (c : Call)
  /-- `ecall` (§ 5): an exit if a7 is 93; anything else is outside the contract. -/
  | ecall
  /-- An access to a ZisK CSR number that ZisK makes nothing of: `csrs ID,
      x0`. sail-riscv, which has no such CSR, would trap. -/
  | nop
  /-- A sequence the transpiler refuses, or a SYSTEM instruction it does not
      translate (`mret`, `wfi`, ...): a halt, the run's failure. -/
  | malformed (why : String)
  /-- `lr` or `sc` (§ 1): sail-riscv runs the plain load or store `w` in its
      place (`lw`/`ld rd, 0(rs1)`, `sw`/`sd rs2, 0(rs1)`); then, for `sc`,
      rd (when not x0) gets 0. -/
  | subst (w : BitVec 32) (zeroRd : Nat)
  /-- Plain RISC-V, sail-riscv's to run. -/
  | riscv

/-- The precompiles called on one pointer: `csrs ID, p` (§ 7). -/
def onePointer : Nat → Option zisk_op
  | 0x800 => some (.ZOP_KECCAK ())
  | 0x801 => some (.ZOP_ARITH256 ())
  | 0x802 => some (.ZOP_ARITH256_MOD ())
  | 0x804 => some (.ZOP_SECP256K1_DBL ())
  | 0x807 => some (.ZOP_BN254_CURVE_DBL ())
  | 0x80B => some (.ZOP_ARITH384_MOD ())
  | 0x80D => some (.ZOP_BLS12_381_CURVE_DBL ())
  | 0x812 => some (.ZOP_POSEIDON2 ())
  | 0x818 => some (.ZOP_SECP256R1_DBL ())
  | 0x81B => some (.ZOP_POSEIDON1 ())
  | 0x821 => some (.ZOP_POSEIDON2_KOALABEAR ())
  | _ => none

/-- The precompiles called on two pointers: `csrs ID, p1; add x0, p2, x0` (§ 7). -/
def twoPointers : Nat → Option zisk_op
  | 0x803 => some (.ZOP_SECP256K1_ADD ())
  | 0x805 => some (.ZOP_SHA256 ())
  | 0x806 => some (.ZOP_BN254_CURVE_ADD ())
  | 0x808 => some (.ZOP_BN254_COMPLEX_ADD ())
  | 0x809 => some (.ZOP_BN254_COMPLEX_SUB ())
  | 0x80A => some (.ZOP_BN254_COMPLEX_MUL ())
  | 0x80C => some (.ZOP_BLS12_381_CURVE_ADD ())
  | 0x80E => some (.ZOP_BLS12_381_COMPLEX_ADD ())
  | 0x80F => some (.ZOP_BLS12_381_COMPLEX_SUB ())
  | 0x810 => some (.ZOP_BLS12_381_COMPLEX_MUL ())
  | 0x817 => some (.ZOP_SECP256R1_ADD ())
  | 0x81D => some (.ZOP_BABYJUBJUB_ADD ())
  | 0x81E => some (.ZOP_BLAKE3 ())
  | 0x81F => some (.ZOP_BLAKE2S ())
  | 0x820 => some (.ZOP_ED25519_ADD ())
  | _ => none

/-- The words of the fcall parameter ports 0x8F0..0x8FF (`CSR_FCALL_PARAM_OFFSET_TO_WORDS`). -/
def paramWords : List Nat := [1, 2, 4, 8, 12, 16, 20, 24, 25, 32, 48, 64, 80, 96, 128, 256]

/-- The DMA copies and compares, and jump_dest: `csrrs rd, ID, src` with an
    `add` or `addi` after it (`transpile_dma_memcpy_memcmp_pattern`). -/
def dmaPair (csr rd src : Nat) (n0 : Fields) : Decoded :=
  match kind n0 with
  | .add =>
    let op : zisk_op := match csr with
      | 0x813 => .ZOP_DMA_MEMCPY () | 0x814 => .ZOP_DMA_MEMCMP () | _ => .ZOP_JUMP_DEST ()
    -- rd from the csrrs; in the deprecated form, from the add
    .call { op, a := .reg n0.rs1, b := .reg src, count := some n0.rs2,
            rd := if n0.rd == 0 then rd else n0.rd, len := 2 }
  | .addi =>
    if csr == 0x81C then .malformed "jump_dest with an immediate size"
    else
      let op : zisk_op := if csr == 0x813 then .ZOP_DMA_XMEMCPY () else .ZOP_DMA_XMEMCMP ()
      .call { op, a := .reg n0.rs1, b := .reg src, ext := n0.immI,
              rd := if n0.rd == 0 then rd else n0.rd, len := 2 }
  | _ => .malformed "memcpy/memcmp/jump_dest without an add or addi after it"

/-- A ZisK CSR number: the syscall, zkvmcall and fcall ranges, and the fcall result. -/
def isZiskCsr (csr : Nat) : Bool := (0x800 ≤ csr && csr ≤ 0x8FF) || csr == 0xFFE

/-- `csrs 0x816, dst; addi rd, n, v`: memset with the count in a register. -/
def memsetPair (dst : Nat) (n0 : Option Fields) : Decoded :=
  match n0 with
  | some n0 =>
    if kind n0 == .addi && n0.immI.toNat ≤ 0xFF then
      .call { op := .ZOP_DMA_XMEMSET (), a := .reg dst, b := .reg n0.rs1, ext := n0.immI,
              rd := n0.rd, len := 2 }
    else .malformed "memset without an addi after it"
  | none => .malformed "the sequence runs past the code"

/-- `csrrs rd, csr, rs1`, and what follows it (`Riscv2ZiskContext::csrrs`). -/
def decodeCsrrs (f : Fields) (n0 : Option Fields) : Decoded :=
  let need (n : Option Fields) (k : Decoded → Decoded) (fn : Fields → Decoded) : Decoded :=
    match n with | some n => fn n | none => k (.malformed "the sequence runs past the code")
  match f.csr with
  | 0x813 | 0x814 | 0x81C => need n0 id (dmaPair f.csr f.rd f.rs1)
  | 0x816 => memsetPair f.rs1 n0
  | 0x811 =>
    if f.rs1 != 0 then .call { op := .ZOP_ADD256 (), b := .reg f.rs1, rd := f.rd, len := 1 }
    else if f.rd == 0 then .nop else .riscv
  | csr =>
    if f.rd != 0 then
      -- `csrr rd, 0xFFE`: fcall_get; any other access with a destination is a cell access
      if f.rs1 == 0 && csr == 0xFFE then
        .call { op := .ZOP_FCALL_GET (), b := .freeInput, rd := f.rd, len := 1 }
      else .riscv
    else if f.rs1 == 0 then (if isZiskCsr csr then .nop else .riscv)  -- csrs ID, x0: nothing
    else match csr with
      | 0x815 =>
        need n0 id fun n0 => match kind n0 with
          | .add  => .call { op := .ZOP_DMA_INPUTCPY (), a := .reg n0.rs1, b := .reg n0.rs2,
                             rd := n0.rd, len := 2 }
          | .addi => .call { op := .ZOP_DMA_INPUTCPY (), a := .reg n0.rs1, b := .imm n0.immI,
                             rd := n0.rd, len := 2 }
          | _ => .malformed "inputcpy without an add or addi after it"
      | 0x81A =>
        need n0 id fun n0 =>
          if kind n0 == .addi && n0.rd == 0 && n0.rs1 == 0 then
            .call { op := .ZOP_PROFILE (), a := .reg f.rs1, b := .imm n0.immI, len := 2 }
          else .malformed "profile without an addi x0, x0 after it"
      | 0x819 =>
        need n0 id fun n0 =>
          if kind n0 == .addi && n0.rd == 0 then
            .call { op := .ZOP_BLAKE2B (), a := .reg f.rs1, b := .reg n0.rs1, ext := n0.immI, len := 2 }
          else .malformed "blake2b without an addi x0 after it"
      | csr =>
        match twoPointers csr, onePointer csr with
        | some op, _ =>
          need n0 id fun n0 =>
            if kind n0 == .add && n0.rd == 0 && n0.rs2 == 0 then
              .call { op, a := .reg f.rs1, b := .reg n0.rs1, len := 2 }
            else .malformed "a two-pointer precompile without an add x0, p2, x0 after it"
        | none, some op => .call { op, b := .reg f.rs1, len := 1 }
        | none, none =>
          if 0x8F0 ≤ csr && csr ≤ 0x8FF then
            .call { op := .ZOP_FCALL_PARAM (), a := .imm (BitVec.ofNat 64 (paramWords.getD (csr - 0x8F0) 0)),
                    b := .reg f.rs1, len := 1 }
          else .riscv

/-- `csrrsi rd, csr, uimm` (`Riscv2ZiskContext::csrrsi`): only the memset form. -/
def decodeCsrrsi (f : Fields) (n0 n1 : Option Fields) : Decoded :=
  if f.rd == 0 && f.csr == 0x816 then
    match f.rs1, n0, n1 with  -- rs1 is the immediate here
    | 2, some n0, some n1 =>
      if kind n0 == .addi && kind n1 == .addi && n1.immI.toNat ≤ 0xFF then
        .call { op := .ZOP_DMA_XMEMSET (), a := .reg n0.rs1, b := .imm n0.immI, ext := n1.immI,
                rd := n0.rd, len := 3 }
      else .malformed "memset without two addi after it"
    -- the register form's branch, which reads rs1: the immediate form has none, so x0
    | 0, n0, _ => memsetPair 0 n0
    | _, _, _ => .malformed "csrsi on the memset port with an immediate other than 0 or 2"
  else .riscv

/-- `csrrwi rd, csr, uimm` (`Riscv2ZiskContext::csrrwi`): fcall, on 0x8C0..0x8DF. -/
def decodeCsrrwi (f : Fields) : Decoded :=
  if f.rd == 0 && 0x8C0 ≤ f.csr && f.csr ≤ 0x8DF then
    .call { op := .ZOP_FCALL (), a := .imm (BitVec.ofNat 64 ((f.csr - 0x8C0) * 32 + f.rs1)),
            b := .imm 0, len := 1 }
  else .riscv

/-- What the instruction at `pc` starts, reading words through `fetch`. -/
def decode (fetch : Nat → Option (BitVec 32)) (pc : Nat) : Decoded :=
  match fetch pc with
  | none => .riscv
  | some w =>
    let f := fields w
    let n0 := (fetch (pc + 4)).map fields
    let n1 := (fetch (pc + 8)).map fields
    match kind f with
    | .ecall  => .ecall
    | .csrrs  => decodeCsrrs f n0
    | .csrrsi => decodeCsrrsi f n0 n1
    | .csrrwi => decodeCsrrwi f
    | .lr => .subst (0#12 ++ w.extractLsb' 15 5 ++ w.extractLsb' 12 3 ++ w.extractLsb' 7 5 ++ 0x03#7) 0
    | .sc => .subst (0#7 ++ w.extractLsb' 20 5 ++ w.extractLsb' 15 5 ++ w.extractLsb' 12 3 ++ 0#5 ++ 0x23#7) f.rd
    | .system => .malformed "a SYSTEM instruction other than ecall and ebreak (mret, wfi, sfence.vma, ...)"
    | _ => .riscv

end ZiskRiscv
