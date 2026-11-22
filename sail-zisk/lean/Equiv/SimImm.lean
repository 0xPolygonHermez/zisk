/-
  The values of the immediate instructions: sail-zisk's op on rs1 and the
  transpiler's immediate is what sail-riscv computes. The I-type ops are the
  R-type ones on a sign-extended immediate; the immediate shifts the R-type
  shifts on the shift amount, zero-extended (the transpiler takes the low
  six bits of the immediate as the amount: riscv_interpreter.rs).
-/
import Equiv.Sim
import Equiv.RiscvImm
import Equiv.ZiskImm

namespace Equiv.Sim

open Equiv.Riscv (itypeVal shiftVal rtypeVal)
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- The R-type op whose ZisK op the transpiler emits for each I-type op. -/
def iopR : iop → rop
  | .ADDI => .ADD | .SLTI => .SLT | .SLTIU => .SLTU | .XORI => .XOR | .ORI => .OR | .ANDI => .AND

def sopR : sop → rop
  | .SLLI => .SLL | .SRLI => .SRL | .SRAI => .SRA

theorem ziskIVal (op : iop) (a : BitVec 64) (imm : BitVec 12) :
    (_root_.Zisk.Functions.zisk_exec (ziskOp (iopR op)) a (imm.signExtend 64)).1 = itypeVal op a imm := by
  rw [ziskVal]; cases op <;> rfl

theorem ziskSVal (op : sop) (a : BitVec 64) (sh : BitVec 6) :
    (_root_.Zisk.Functions.zisk_exec (ziskOp (sopR op)) a (sh.zeroExtend 64)).1 = shiftVal op a sh := by
  rw [ziskVal]
  have e : Sail.BitVec.extractLsb (sh.zeroExtend 64) (Functions.log2_xlen -i 1).toNat 0
      = Sail.BitVec.extractLsb sh (Functions.log2_xlen -i 1).toNat 0 := by
    simp only [Sail.BitVec.extractLsb, Functions.log2_xlen]
    apply BitVec.eq_of_getLsbD_eq; intro i hi; simp at hi ⊢
  cases op <;> simp only [sopR, rtypeVal, shiftVal, e]

end Equiv.Sim
