/-
  The word instructions' values: ZisK's _w ops, as the transpiler emits them,
  compute what sail-riscv's RTYPEW, ADDIW and SHIFTIWOP do.
-/
import Equiv.RiscvW
import Equiv.Sim

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

def ziskOpW : ropw → _root_.Zisk.zisk_op
  | .ADDW => .ZOP_ADD_W () | .SUBW => .ZOP_SUB_W () | .SLLW => .ZOP_SLL_W ()
  | .SRLW => .ZOP_SRL_W () | .SRAW => .ZOP_SRA_W ()

theorem ziskWVal (op : ropw) (a b : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec (ziskOpW op) a b).1 = rtypewVal op a b := by
  cases op <;> simp [ziskOpW, rtypewVal, _root_.Zisk.Functions.zisk_exec, _root_.Zisk.Functions.sx32,
    _root_.Zisk.Functions.sign_extend, Sail.BitVec.signExtend, sign_extend, Sail.BitVec.extractLsb,
    shift_bits_left, shift_bits_right, shift_bits_right_arith, _root_.Zisk.Functions.sra_bits,
    Sail.BitVec.toNatInt, shl_int, shr_int]
  case ADDW | SUBW => bv_decide
  all_goals
    have h32 : ((b.toNat : Int) % 32) = ((b.toNat % 32 : Nat) : Int) := by omega
    have h5 : b.toNat % 32 = (b.setWidth 5).toNat := by simp [BitVec.toNat_setWidth]
  case SLLW => rw [h32, ← Int.ofNat_eq_natCast, shl_int]
  case SRLW => rw [h32, ← Int.ofNat_eq_natCast, shr_int]
  case SRAW =>
    rw [h32]
    generalize hk : b.toNat % 32 = k
    have hk5 : k = (b.setWidth 5).toNat := by rw [← hk]; simp [BitVec.toNat_setWidth]
    rw [← Int.ofNat_eq_natCast]
    erw [shr_int]
    simp only [Int.ofNat_eq_natCast, Int.toNat_natCast, hk5]
    rw [← BitVec.ushiftRight_eq', ← BitVec.sshiftRight_eq']
    bv_decide

end Equiv.Sim
