/-
  sail-zisk's semantics for a ZisK call as the transpiler emits it
  (`Transpiler.callInsts`): the sources read, the operation run
  (`zisk_exec_mem`, given here as its run), its c to rd, on past the
  sequence.
-/
import Equiv.ZiskSpec
import Equiv.ZFrameAll

set_option linter.unusedSimpArgs false

namespace Equiv.Zisk

open _root_.Zisk _root_.Zisk.Functions Sail Sail.ConcurrencyInterfaceV1
open Register

/-- What a call's operation gives: `zisk_exec_mem`'s result, or, for an
    operation without memory, `zisk_exec`'s. -/
def opResult (op : zisk_op) (a b : BitVec 64) : Option ((BitVec 64) × Bool) → (BitVec 64) × Bool
  | some p => p
  | none => zisk_exec op a b

set_option maxHeartbeats 4000000 in
/-- A ZisK call, its operation's run given: c to rd (none for x0), on to
    pc + j1 if its flag is set, else pc + j2. -/
theorem execute_zcall (s : ZState) (ra : a_src × BitVec 64) (rb : b_src × BitVec 64) (op : zisk_op)
    (rd : Nat) (hd : rd < 40) (j1 j2 : BitVec 64)
    (xs : Vector (BitVec 64) 40) (pc st : BitVec 64) (va vb : BitVec 64)
    (hsa : (source_a (Transpiler.mk ra rb op (Transpiler.storeReg rd) j1 j2)).run s = .ok va s)
    (hsb : (source_b (Transpiler.mk ra rb op (Transpiler.storeReg rd) j1 j2) va).run s = .ok vb s)
    (hh : ∀ u, op ≠ .ZOP_HALT u)
    (r : Option ((BitVec 64) × Bool)) (s1 : ZState)
    (hrun : (zisk_exec_mem op va vb (if zisk_op_is_precompiled op then j1 else 0)).run s = .ok r s1)
    (c : BitVec 64) (flag : Bool) (hcf : opResult op va vb r = (c, flag))
    (hx : s1.regs.get? Xs = some xs) (hpc : s1.regs.get? PC = some pc) (hst : s1.regs.get? Step = some st) :
    (zisk_execute (Transpiler.mk ra rb op (Transpiler.storeReg rd) j1 j2)).run s = .ok false
      { s1 with regs := ((((linkRegs s1.regs xs rd c).insert Zc c).insert Zflag flag).insert
          Step (BitVec.addInt st 1)).insert PC (if flag then pc + j1 else pc + j2) } := by
  have ld : (rd : Int) < 40 := by omega
  simp only [zisk_execute, run_bind', hsa, hsb]
  have hz : (zeros : BitVec 64) = 0 := rfl
  simp only [Transpiler.mk, hz]
  rw [hrun]
  dsimp only
  have hh' := hh ()
  cases r <;> simp only [opResult] at hcf <;> (try subst hcf) <;> (try rw [hcf]) <;> simp only [run_pure'] <;>
  by_cases h0 : rd = 0 <;>
  · simp (disch := omega) [Transpiler.storeReg, h0, store_c, value_to_store, next_pc, linkRegs, writeReg_run,
      ld, reg_index, rX, wX, Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert, run_pure', *]
    repeat (rw [readReg_run']; (try simp only [Std.ExtDHashMap.get?_insert]); simp [hst, hpc, hx, *])
    all_goals
      cases flag <;> simp only [if_true, if_false, Bool.false_eq_true, run_map'] <;>
        rw [readReg_run'] <;> simp [Std.ExtDHashMap.get?_insert, hpc]

set_option maxHeartbeats 4000000 in
/-- The a source of a call instruction: an immediate, or a register 1..39. -/
theorem source_a_run (s : ZState) (ra : a_src × BitVec 64) (rb : b_src × BitVec 64) (op : zisk_op)
    (st : c_store × BitVec 64 × Bool) (j1 j2 : BitVec 64) (xs : Vector (BitVec 64) 40)
    (hx : s.regs.get? Xs = some xs) (va : BitVec 64) (ha : ASrc ra xs va) :
    (source_a (Transpiler.mk ra rb op st j1 j2)).run s = .ok va s := by
  rcases ha with rfl | ⟨r1, h1a, h1b, rfl, hv1⟩
  · rfl
  · have e1 : r1 ≠ 0 ∧ (r1 : Int) < 40 := ⟨by omega, by omega⟩
    simp (disch := omega) [source_a, Transpiler.mk, rX, reg_index, Zisk.assert,
      Sail.ConcurrencyInterfaceV1.PreSail.assert, *]
    rw [readReg_run s Xs xs hx]
    simp only [run_pure', EStateM.Result.ok.injEq, and_true]
    rw [← hv1, getElem!_pos xs ((r1 : Nat) : Int) (by simp; omega)]

set_option maxHeartbeats 4000000 in
/-- The b source of a call instruction: an immediate, or a register 1..39. -/
theorem source_b_run (s : ZState) (ra : a_src × BitVec 64) (rb : b_src × BitVec 64) (op : zisk_op)
    (st : c_store × BitVec 64 × Bool) (j1 j2 : BitVec 64) (xs : Vector (BitVec 64) 40)
    (hx : s.regs.get? Xs = some xs) (va vb : BitVec 64) (hb : BSrc rb xs vb) :
    (source_b (Transpiler.mk ra rb op st j1 j2) va).run s = .ok vb s := by
  rcases hb with rfl | ⟨r2, h2a, h2b, rfl, hv2⟩
  · rfl
  · have e2 : r2 ≠ 0 ∧ (r2 : Int) < 40 := ⟨by omega, by omega⟩
    simp (disch := omega) [source_b, Transpiler.mk, rX, reg_index, Zisk.assert,
      Sail.ConcurrencyInterfaceV1.PreSail.assert, *]
    rw [readReg_run s Xs xs hx]
    simp only [run_pure', EStateM.Result.ok.injEq, and_true]
    rw [← hv2, getElem!_pos xs ((r2 : Nat) : Int) (by simp; omega)]

/-- The b source of `fcall_get`: the free input, an 8-byte read of INPUT_ADDR. -/
theorem source_b_free (s : ZState) (ra : a_src × BitVec 64) (op : zisk_op)
    (st : c_store × BitVec 64 × Bool) (j1 j2 : BitVec 64) (va v : BitVec 64)
    (hfi : s.regs.get? FreeInput = some v) :
    (source_b (Transpiler.mk ra (.B_MEM, 0x1000_0000) op st j1 j2) va).run s = .ok v s := by
  have h1 : readable (268435456#64) 8 = true := by decide
  have h2 : hits_registers (268435456#64) 8 = false := by decide
  have h3 : (268435456#64 : BitVec 64) = INPUT_ADDR := rfl
  simp [source_b, Transpiler.mk, read_mem, Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert, h1, h2, h3]
  exact readReg_run s FreeInput v hfi

set_option maxHeartbeats 4000000 in
/-- A copy of the b operand to the 8 bytes at a fixed address (a count's
    store to EXTRA_PARAMS_ADDR); on to pc + j. -/
theorem execute_store_mem (s : ZState) (rb : b_src × BitVec 64) (addr j : BitVec 64)
    (xs : Vector (BitVec 64) 40) (pc st : BitVec 64)
    (hx : s.regs.get? Xs = some xs) (hpc : s.regs.get? PC = some pc) (hst : s.regs.get? Step = some st)
    (vb : BitVec 64) (hb : BSrc rb xs vb)
    (hwr : writable addr 8 = true) (hnr : hits_registers addr 8 = false) :
    (zisk_execute (Transpiler.mk (.A_IMM, 0) rb (.ZOP_COPYB ()) (.ST_MEM, addr, false) j j)).run s = .ok false
      { s with
        regs := (((s.regs.insert Zc vb).insert Zflag false).insert Step (BitVec.addInt st 1)).insert PC (pc + j)
        mem := Equiv.memWrite (n := 8) s.mem addr.toNat vb } := by
  have hm' := copyb_pure.mem
  have hp := copyb_pure.notPre
  rcases hb with rfl | ⟨r2, h2a, h2b, rfl, hv2⟩ <;>
  · (try subst hv2)
    (try have e2 : r2 ≠ 0 ∧ (r2 : Int) < 40 := ⟨by omega, by omega⟩)
    simp (disch := omega) [zisk_execute, Transpiler.mk, source_a, source_b, rX, store_c, value_to_store,
      next_pc, hm', hp, copyb_val, Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert, writeReg_run,
      reg_index, *]
    repeat (first
      | (rw [readReg_run s Xs xs hx]; try dsimp only)
      | (rw [write_mem_run s _ 8 _ (by simpa (disch := omega) using hwr) (by simpa (disch := omega) using hnr)];
          try dsimp only)
      | (rw [readReg_run']; (try simp only [Std.ExtDHashMap.get?_insert]); simp [hst, hpc, hx, *]))
    all_goals simp (disch := omega) [BitVec.toNat_add, *]

/-- Every result a run of P succeeds with satisfies Q. -/
def Post {α : Type} (P : SailM α) (Q : α → Prop) : Prop := ∀ σ a σ', P.run σ = .ok a σ' → Q a

theorem Post.pure {α : Type} {Q : α → Prop} (a : α) (h : Q a) : Post (pure a : SailM α) Q := by
  intro σ b σ' hr; simp only [EStateM.run, Pure.pure, EStateM.pure, EStateM.Result.ok.injEq] at hr
  obtain ⟨rfl, rfl⟩ := hr; exact h

theorem Post.bind {α β : Type} {Q : β → Prop} {P : SailM α} {f : α → SailM β} (hf : ∀ a, Post (f a) Q) :
    Post (P >>= f) Q := by
  intro σ b σ' hr
  rw [run_bind'] at hr
  split at hr
  · exact hf _ _ _ _ hr
  · cases hr

theorem Post.ite {α : Type} {Q : α → Prop} {c : Prop} [Decidable c] {P R : SailM α} (hP : Post P Q) (hR : Post R Q) :
    Post (if c then P else R) Q := by
  split
  · exact hP
  · exact hR

open Lean Elab Tactic Meta in
/-- The rule for the program's head, for `Post`. -/
elab "post_head" : tactic => withMainContext do
  let tgt ← instantiateMVars (← getMainTarget)
  let P := (tgt.getArg! 1).headBeta
  match P.getAppFn.constName? with
  | some ``Bind.bind => evalTactic (← `(tactic| refine Post.bind (fun _ => ?_)))
  | some ``Pure.pure => evalTactic (← `(tactic| refine Post.pure _ ?_))
  | some ``ite => evalTactic (← `(tactic| refine Post.ite ?_ ?_))
  | _ => throwError "post_head: no rule for the head"

/-- Push `Post` to the program's tail. -/
syntax "post" : tactic
macro_rules
  | `(tactic| post) => `(tactic| first
      | (post_head <;> post)
      | (split <;> post)
      | (dsimp only; post)
      | skip)

/-- An operation with memory leaves its flag clear: the step goes on past
    the sequence. -/
def FlagClear (r : Option ((BitVec 64) × Bool)) : Prop := ∀ c f, r = some (c, f) → f = false

set_option maxRecDepth 1000000 in
set_option maxHeartbeats 0 in
theorem exec_mem_flag (op : zisk_op) (a b ext : BitVec 64) : Post (zisk_exec_mem op a b ext) FlagClear := by
  cases op <;> simp only [zisk_exec_mem] <;> post
  all_goals (intro c f h; simp only [Option.some.injEq, Prod.mk.injEq, reduceCtorEq] at h; try exact h.2.symm)

end Equiv.Zisk
