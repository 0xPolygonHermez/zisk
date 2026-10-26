/-
  Checks of the decoder and of the bridge to sail-zisk's operations, which
  Lean evaluates when it builds this module.
-/
import ZiskRiscv.Machine

namespace ZiskRiscv.Tests

/-- An I-type or CSR word: `imm/csr | rs1 | funct3 | rd | opcode`. -/
def itype (opcode funct3 rd rs1 imm : Nat) : BitVec 32 :=
  BitVec.ofNat 32 ((imm % 4096) <<< 20 ||| rs1 <<< 15 ||| funct3 <<< 12 ||| rd <<< 7 ||| opcode)

def csrrs (rd csr rs1 : Nat) := itype 0x73 2 rd rs1 csr
def csrrsi (rd csr imm : Nat) := itype 0x73 6 rd imm csr
def csrrwi (rd csr imm : Nat) := itype 0x73 5 rd imm csr
def addi (rd rs1 imm : Nat) := itype 0x13 0 rd rs1 imm
def add (rd rs1 rs2 : Nat) : BitVec 32 :=
  BitVec.ofNat 32 (rs2 <<< 20 ||| rs1 <<< 15 ||| rd <<< 7 ||| 0x33)
def ecall : BitVec 32 := 0x73

/-- A program of words from address 0. -/
def prog (ws : List (BitVec 32)) : Nat → Option (BitVec 32) :=
  fun a => if a % 4 == 0 then ws[a / 4]? else none

def describe : Decoded → String
  | .call c => s!"call len={c.len} rd={c.rd} count={c.count} {repr c.a} {repr c.b} ext={c.ext.toNat}"
  | .ecall => "ecall" | .nop => "nop" | .riscv => "riscv"
  | .malformed _ => "malformed"
  | .subst w r => s!"subst {w.toNat} rd={r}"

-- keccak on a0: one instruction
#guard describe (decode (prog [csrrs 0 0x800 10]) 0) == "call len=1 rd=0 count=none ZiskRiscv.Arg.imm 0x0000000000000000#64 ZiskRiscv.Arg.reg 10 ext=0"
-- sha256: csrs 0x805, a0; add x0, a1, x0
#guard describe (decode (prog [csrrs 0 0x805 10, add 0 11 0]) 0) == "call len=2 rd=0 count=none ZiskRiscv.Arg.reg 10 ZiskRiscv.Arg.reg 11 ext=0"
-- and with anything else after it, refused
#guard describe (decode (prog [csrrs 0 0x805 10, addi 0 11 0]) 0) == "malformed"
-- memcpy, count in a register: csrrs a3, 0x813, a1; add x0, a0, a2
#guard describe (decode (prog [csrrs 13 0x813 11, add 0 10 12]) 0) == "call len=2 rd=13 count=(some 12) ZiskRiscv.Arg.reg 10 ZiskRiscv.Arg.reg 11 ext=0"
-- xmemset, three instructions: csrsi 0x816, 2; addi a5, a0, 64; addi x0, a0, 0xab
#guard describe (decode (prog [csrrsi 0 0x816 2, addi 15 10 64, addi 0 10 0xab]) 0) == "call len=3 rd=15 count=none ZiskRiscv.Arg.reg 10 ZiskRiscv.Arg.imm 0x0000000000000040#64 ext=171"
-- fcall of function 33: csrwi 0x8C1, 1
#guard describe (decode (prog [csrrwi 0 0x8C1 1]) 0) == "call len=1 rd=0 count=none ZiskRiscv.Arg.imm 0x0000000000000021#64 ZiskRiscv.Arg.imm 0x0000000000000000#64 ext=0"
-- fcall_get: csrr a0, 0xFFE
#guard describe (decode (prog [csrrs 10 0xFFE 0]) 0) == "call len=1 rd=10 count=none ZiskRiscv.Arg.imm 0x0000000000000000#64 ZiskRiscv.Arg.freeInput ext=0"
-- csrs 0x800, x0 does nothing; ecall is apart; marchid is RISC-V's
#guard describe (decode (prog [csrrs 0 0x800 0]) 0) == "nop"
#guard describe (decode (prog [ecall]) 0) == "ecall"
#guard describe (decode (prog [csrrs 10 0xF12 0]) 0) == "riscv"
-- lr.w a0, (a1): lw a0, 0(a1) in its place
#guard describe (decode (prog [BitVec.ofNat 32 (2 <<< 27 ||| 11 <<< 15 ||| 2 <<< 12 ||| 10 <<< 7 ||| 0x2F)]) 0)
  == s!"subst {11 <<< 15 ||| 2 <<< 12 ||| 10 <<< 7 ||| 0x03} rd=0"
-- sc.d a0, a2, (a1): sd a2, 0(a1), then a0 = 0
#guard describe (decode (prog [BitVec.ofNat 32 (3 <<< 27 ||| 12 <<< 20 ||| 11 <<< 15 ||| 3 <<< 12 ||| 10 <<< 7 ||| 0x2F)]) 0)
  == s!"subst {12 <<< 20 ||| 11 <<< 15 ||| 3 <<< 12 ||| 0x23} rd=10"
-- lr.w with rs2 not x0 is reserved, and amoadd.w is RISC-V's
#guard describe (decode (prog [BitVec.ofNat 32 (2 <<< 27 ||| 1 <<< 20 ||| 11 <<< 15 ||| 2 <<< 12 ||| 10 <<< 7 ||| 0x2F)]) 0)
  == "riscv"
#guard describe (decode (prog [BitVec.ofNat 32 (12 <<< 20 ||| 11 <<< 15 ||| 2 <<< 12 ||| 10 <<< 7 ||| 0x2F)]) 0) == "riscv"

-- The bridge runs sail-zisk's keccak on the bytes given: keccak-f of the
-- all-zero state, whose first word is 0xF1258F7940E1DDE7.
def zeroState : Mem := (List.range 200).foldl (fun m i => m.insert (0xA000_0000 + i) 0) {}
#guard match runOp (.ZOP_KECCAK ()) 0 0xA000_0000 0 zeroState {} with
  | .ok (_, m, _) => readDword m 0xA000_0000 == 0xF1258F7940E1DDE7
  | .error _ => false

end ZiskRiscv.Tests
