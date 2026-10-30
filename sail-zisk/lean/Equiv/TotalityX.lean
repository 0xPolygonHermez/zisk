/-
  Precondition X (doc/riscv_contract.md § 2): the program reads no byte of
  an execute-only segment, which ZisK's memory lacks. Broken at a step by
  an access other than a store whose bytes include one (`BreakX`), or by a
  ZisK call whose operation fails on one (`BreakXC`).

  What the spec's translations read (`convert_reads`): nothing, but the
  instruction at pc of a load, an `lr` and an AMO, which reads the access's
  own bytes, through rs1.
-/
import Equiv.Totality
import Equiv.XStep

namespace Equiv.Final

open Equiv.Riscv Equiv.Sim Equiv.Machine Equiv.Top Equiv.Sparse
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- m without X's bytes. -/
def eraseX (X : Nat → Bool) (m : Mem) : Mem := m.filter fun k _ => !X k

theorem eraseX_get (X : Nat → Bool) (m : Mem) (k : Nat) :
    (eraseX X m).get? k = if X k then none else m.get? k := by
  simp only [eraseX, Std.ExtHashMap.get?_eq_getElem?, Std.ExtHashMap.getElem?_filter']
  cases X k <;> cases m[k]? <;> rfl

/-- X: an access but a store, its bytes including one of X's. -/
def BreakX (X : Nat → Bool) (s : RVState) (w : BitVec 32) : Prop :=
  ∃ kd a n, access s w = some (kd, a, n) ∧ kd ≠ .store ∧ ∃ k, X k = true ∧ a.toNat ≤ k ∧ k < a.toNat + n

/-- X, on a ZisK call: the call at pc, its operation failing on one of X's
    bytes, on the memory without them. -/
def BreakXC (X : Nat → Bool) (S : ZiskRiscv.State model) : Prop :=
  ∃ pc w c, S.rv.regs.get? Register.PC = some pc ∧ ZiskRiscv.readWord S.rv.mem pc.toNat = some w ∧
    ZiskRiscv.decode (Transpiler.fetch3 pc w (nextWord S.rv pc) (nextWord2 S.rv pc)) pc.toNat = .call c ∧
    ∃ k σ', (_root_.Zisk.Functions.zisk_exec_mem c.op (S.argValue c.a) (S.argValue c.b)
      (if _root_.Zisk.Functions.zisk_op_is_precompiled c.op then c.ext else 0)).run
      (ZiskRiscv.ziskState (eraseX X (Equiv.normIn (callMem S c))) S.fcall) = .error (.OutOfMemoryRange k) σ' ∧
      X k = true

/-- The instruction at pc of the word w's translation that reads memory:
    through rs1, the access's own bytes. -/
def ReadsOwn (w : BitVec 32) (pc : BitVec 64) (p : BitVec 64 × _root_.Zisk.zisk_inst) : Prop :=
  p.1 = pc ∧ p.2.a_src = (Transpiler.srcA (Transpiler.rs1 w)).1 ∧ p.2.a_imm = (Transpiler.srcA (Transpiler.rs1 w)).2 ∧
    p.2.b_src = .B_IND ∧ Equiv.Zisk.memOp p.2.op = false ∧
    ∃ kd, kd ≠ AccKind.store ∧ ∀ s, access s w = some (kd, regOf s (Transpiler.rs1 w) + p.2.b_imm, p.2.ind_width.toNat)

/-- The spec's operations have no `zisk_exec_mem` body. -/
theorem zop_nomem :
    (∀ op : Transpiler.ROp, Equiv.Zisk.memOp op.zop = false) ∧
    (∀ op : Transpiler.ShAdd, Equiv.Zisk.memOp op.zop = false) ∧
    (∀ op : Transpiler.ZbsOp, Equiv.Zisk.memOp op.zop = false) ∧
    (∀ op : Transpiler.ZbbOp, Equiv.Zisk.memOp op.zop = false) ∧
    (∀ op : Transpiler.ZbbWOp, Equiv.Zisk.memOp op.zop = false) ∧
    (∀ op : Transpiler.PackOp, Equiv.Zisk.memOp op.zop = false) ∧
    (∀ op : Transpiler.SextOp, Equiv.Zisk.memOp op.zop = false) ∧
    (∀ op : Transpiler.ByteOp, Equiv.Zisk.memOp op.zop = false) ∧
    (∀ op : Transpiler.CountOp, Equiv.Zisk.memOp op.zop = false) ∧
    (∀ op : Transpiler.MOp, Equiv.Zisk.memOp op.zop = false) ∧
    (∀ op : Transpiler.SOp, Equiv.Zisk.memOp op.zop = false) ∧
    (∀ d (op : Transpiler.AmoOp), Equiv.Zisk.memOp (op.zop d) = false) ∧
    (∀ d, Equiv.Zisk.memOp (Transpiler.aLoad d) = false) := by
  refine ⟨fun op => ?_, fun op => ?_, fun op => ?_, fun op => ?_, fun op => ?_, fun op => ?_, fun op => ?_,
    fun op => ?_, fun op => ?_, fun op => ?_, fun op => ?_, fun d op => ?_, fun d => ?_⟩
  all_goals (try cases d) <;> (try cases op) <;> rfl

set_option maxHeartbeats 0 in
set_option maxRecDepth 100000 in
/-- What the spec's translation of a word other than a CSR instruction
    reads: nothing, but the load's, `lr`'s or AMO's instruction at pc. -/
theorem convert_reads {pc : BitVec 64} {w : BitVec 32} {next : Option (BitVec 32)} {ia : BitVec 64}
    {next2 : Option (BitVec 32)} {l : List (BitVec 64 × _root_.Zisk.zisk_inst)}
    (h : Transpiler.convert pc w next ia next2 = some l) (hd : Transpiler.decode w ≠ some .csr) :
    ∀ p ∈ l, NoReads p.2 ∨ ReadsOwn w pc p := by
  unfold Transpiler.convert at h
  rcases e : Transpiler.decode w with _ | i
  · rw [e] at h; cases h
    intro p hp; simp only [List.mem_cons, List.not_mem_nil, or_false] at hp
    subst hp; left; exact ⟨nofun, nofun, nofun, rfl⟩
  · rw [e] at h hd
    cases i
    all_goals (try (simp at hd; done))
    all_goals (try (exfalso; exact hd rfl))
    all_goals (dsimp only at h)
    all_goals (try simp only [Transpiler.uw, Transpiler.amo, Transpiler.sc, Transpiler.jalr] at h)
    all_goals ((try split at h) <;> (try split at h) <;> (try split at h) <;> (try split at h) <;>
      (try split at h) <;> (try split at h))
    all_goals (cases h)
    all_goals (intro p hp; simp only [List.mem_cons, List.not_mem_nil, or_false] at hp)
    all_goals (rcases hp with rfl | rfl | rfl | rfl | rfl)
    all_goals (simp only [Transpiler.mk, Transpiler.registerOp, Transpiler.immediateOp,
      Transpiler.immediateOpOrCopyb, Transpiler.copyb, Transpiler.singleOp, Transpiler.zextH, Transpiler.nop,
      Transpiler.ecall, Transpiler.lui, Transpiler.auipc, Transpiler.jal, Transpiler.loadOp, Transpiler.storeOp,
      Transpiler.branchOp, Transpiler.hint, Transpiler.halt, Transpiler.auipc.plain])
    all_goals first
      | (left; refine ⟨?_, ?_, ?_, ?_⟩ <;> (try simp only [Transpiler.srcA, Transpiler.srcB]) <;>
          (repeat' split) <;> first | rfl | (simp [zop_nomem]; done) | (simp; done))
      | (right; refine ⟨rfl, rfl, rfl, rfl, by first | rfl | (simp [zop_nomem]; done), ?_⟩
         first
          | (refine ⟨.load, by decide, fun s => ?_⟩; simp [access, e, LOp.width]; done)
          | (refine ⟨.amo, by decide, fun s => ?_⟩; simp [access, e, Transpiler.aWidth]; split <;> rfl)
          | (refine ⟨.load, by decide, fun s => ?_⟩; simp [access, e, Transpiler.aWidth]; split <;> rfl))


end Equiv.Final
