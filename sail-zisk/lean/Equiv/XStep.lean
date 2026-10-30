/-
  A ZisK step on the memory without the execute-only bytes (precondition
  X): where the step on the whole memory succeeds, the step without them
  does the same, or reads one of them — where the instruction's sources
  read, or in its operation (`execute_miss`). An instruction that reads no
  memory (`NoReads`) never does.
-/
import Equiv.ZMiss
import Equiv.ZFrameStep

namespace Equiv.Sim

open Equiv.Zisk

/-- An instruction whose sources read no memory and whose operation has no
    body. -/
def NoReads (inst : _root_.Zisk.zisk_inst) : Prop :=
  inst.a_src ≠ .A_MEM ∧ inst.b_src ≠ .B_MEM ∧ inst.b_src ≠ .B_IND ∧ memOp inst.op = false

/-- A step at p, on τ and on σ, τ without X's bytes: as on τ, or the
    instruction at p reads one of them. -/
theorem zstep_x {X : Nat → Bool} (hX : ∀ k, X k → OffRam k) {zrom : BitVec 64 → _root_.Zisk.zisk_inst}
    {σ τ τ' : ZState} (h : FRX X σ τ) {p : BitVec 64} (hp : τ.regs.get? _root_.Zisk.Register.PC = some p)
    {b : Bool} (ht : (zstep zrom).run τ = .ok b τ') :
    (∃ σ', (zstep zrom).run σ = .ok b σ' ∧ FRX X σ' τ') ∨
      ∃ k, X k ∧ (((zrom p).a_src = .A_MEM ∧ (zrom p).a_imm.toNat ≤ k ∧ k < (zrom p).a_imm.toNat + 8) ∨
        (∃ a σ1, (_root_.Zisk.Functions.source_a (zrom p)).run σ = .ok a σ1 ∧ bReads (zrom p) a k) ∨
        memOp (zrom p).op = true) := by
  rcases zstep_sub hX h ht with hs | ⟨k, σ', hs, hk⟩
  · exact .inl hs
  · have hpσ : σ.regs.get? _root_.Zisk.Register.PC = some p := by rw [h.regs]; exact hp
    rw [zstep_run zrom σ p hpσ] at hs
    exact .inr ⟨k, hk, execute_miss _ σ k σ' hs⟩

/-- A step at p on an instruction that reads no memory: as on τ. -/
theorem zstep_noreads {X : Nat → Bool} (hX : ∀ k, X k → OffRam k) {zrom : BitVec 64 → _root_.Zisk.zisk_inst}
    {σ τ τ' : ZState} (h : FRX X σ τ) {p : BitVec 64} (hp : τ.regs.get? _root_.Zisk.Register.PC = some p)
    (hn : NoReads (zrom p)) {b : Bool} (ht : (zstep zrom).run τ = .ok b τ') :
    ∃ σ', (zstep zrom).run σ = .ok b σ' ∧ FRX X σ' τ' := by
  obtain ⟨ha, hbm, hbi, hm⟩ := hn
  rcases zstep_x hX h hp ht with hs | ⟨k, -, ⟨e, -⟩ | ⟨a, σ1, -, ⟨e, -⟩ | ⟨e, -⟩⟩ | e⟩
  · exact hs
  · exact absurd e ha
  · exact absurd e hbm
  · exact absurd e hbi
  · rw [hm] at e; cases e

/-! ## Where a step leaves the pc -/

/-- A run of P leaves the pc as it was. -/
def KeepPC {α : Type} (P : _root_.Zisk.SailM α) : Prop :=
  ∀ σ a σ', P.run σ = .ok a σ' → σ'.regs.get? _root_.Zisk.Register.PC = σ.regs.get? _root_.Zisk.Register.PC

theorem keepPC_of_frame {α : Type} {P : _root_.Zisk.SailM α} (h : Frame P) : KeepPC P := by
  intro σ a σ' hr
  obtain ⟨σ'', hs, -, hk, -⟩ := h.1 σ σ ⟨fun _ _ x => x, fun _ _ => rfl⟩ a σ' hr
  rw [hr] at hs; cases hs
  exact hk _ rfl

theorem KeepPC.pure {α : Type} (a : α) : KeepPC (pure a : _root_.Zisk.SailM α) := (keepPC_of_frame (Frame.pure a))

theorem KeepPC.bind {α β : Type} {P : _root_.Zisk.SailM α} {f : α → _root_.Zisk.SailM β} (hP : KeepPC P)
    (hf : ∀ a, KeepPC (f a)) : KeepPC (P >>= f) := by
  intro σ b σ' hr
  rw [run_bind'] at hr
  cases h1 : P.run σ with
  | error e σ1 => rw [h1] at hr; cases hr
  | ok a σ1 => rw [h1] at hr; rw [hf a σ1 b σ' hr, hP σ a σ1 h1]

theorem KeepPC.readReg (r : _root_.Zisk.Register) : KeepPC (_root_.Zisk.readReg r) := by
  intro σ a σ' hr
  rw [readReg_run'] at hr
  split at hr
  · cases hr; rfl
  · cases hr

theorem KeepPC.writeReg (r : _root_.Zisk.Register) (v : _root_.Zisk.RegisterType r)
    (hr : r ≠ _root_.Zisk.Register.PC) : KeepPC (_root_.Zisk.writeReg r v) := by
  intro σ a σ' hrun
  rw [writeReg_run] at hrun; cases hrun
  simp only [Std.ExtDHashMap.get?_insert]
  split
  · next e => simp only [beq_iff_eq] at e; exact absurd e hr
  · rfl

theorem KeepPC.assert (p : Bool) (msg : String) :
    KeepPC (Sail.ConcurrencyInterfaceV1.PreSail.assert p msg : _root_.Zisk.SailM Unit) := (keepPC_of_frame (Frame.assert p msg))

theorem KeepPC.ite {α : Type} {c : Prop} [Decidable c] {P Q : _root_.Zisk.SailM α} (hP : KeepPC P)
    (hQ : KeepPC Q) : KeepPC (if c then P else Q) := by
  split
  · exact hP
  · exact hQ

theorem KeepPC.rX (r : BitVec 6) : KeepPC (_root_.Zisk.Functions.rX r) := by
  unfold _root_.Zisk.Functions.rX
  exact KeepPC.bind (KeepPC.assert _ _) (fun _ => KeepPC.bind (KeepPC.assert _ _) (fun _ =>
    KeepPC.bind (KeepPC.readReg _) (fun _ => KeepPC.pure _)))

theorem KeepPC.wX (r : BitVec 6) (v : BitVec 64) : KeepPC (_root_.Zisk.Functions.wX r v) := by
  unfold _root_.Zisk.Functions.wX
  exact KeepPC.bind (KeepPC.assert _ _) (fun _ => KeepPC.bind (KeepPC.assert _ _) (fun _ =>
    KeepPC.bind (KeepPC.readReg _) (fun _ => KeepPC.writeReg _ _ (by decide))))

theorem KeepPC.source_a (inst : _root_.Zisk.zisk_inst) : KeepPC (_root_.Zisk.Functions.source_a inst) := by
  unfold _root_.Zisk.Functions.source_a
  split
  · exact KeepPC.readReg _
  · exact KeepPC.rX _
  · exact (keepPC_of_frame (Frame.read_mem _ _))
  · exact KeepPC.pure _
  · exact KeepPC.readReg _

theorem KeepPC.source_b (inst : _root_.Zisk.zisk_inst) (a : BitVec 64) :
    KeepPC (_root_.Zisk.Functions.source_b inst a) := by
  unfold _root_.Zisk.Functions.source_b
  split
  · exact KeepPC.readReg _
  · exact KeepPC.rX _
  · exact (keepPC_of_frame (Frame.read_mem _ _))
  · exact KeepPC.pure _
  · exact KeepPC.bind (keepPC_of_frame (Frame.read_mem _ _)) (fun _ => KeepPC.pure _)

theorem KeepPC.store_c (inst : _root_.Zisk.zisk_inst) (a c : BitVec 64) :
    KeepPC (_root_.Zisk.Functions.store_c inst a c) := by
  unfold _root_.Zisk.Functions.store_c _root_.Zisk.Functions.value_to_store
  refine KeepPC.bind (KeepPC.ite (KeepPC.bind (KeepPC.readReg _) (fun _ => KeepPC.pure _)) (KeepPC.pure _))
    (fun _ => ?_)
  split
  · exact KeepPC.pure _
  · exact KeepPC.wX _ _
  · exact (keepPC_of_frame (Frame.write_mem _ _ _))
  · exact (keepPC_of_frame (Frame.write_mem _ _ _))

/-- Push `KeepPC` through code that writes no pc. -/
syntax "keeppc" : tactic
macro_rules
  | `(tactic| keeppc) => `(tactic| first
      | (refine KeepPC.bind ?_ (fun _ => ?_) <;> keeppc)
      | (refine KeepPC.ite ?_ ?_ <;> keeppc)
      | exact KeepPC.pure _
      | exact KeepPC.readReg _
      | exact KeepPC.assert _ _
      | exact KeepPC.writeReg _ _ (by decide)
      | (dsimp only; keeppc))

/-- A precompile's prelude (blake2b's chain) keeps the pc. -/
theorem keepPC_prelude (op : _root_.Zisk.zisk_op) (e : BitVec 64) :
    KeepPC (_root_.Zisk.Functions.precompiled_prelude op e) := by
  unfold _root_.Zisk.Functions.precompiled_prelude
  split
  · unfold _root_.Zisk.Functions.blake2b_chain
    keeppc
  · exact KeepPC.pure _

theorem run_bind_ok {α β : Type} {P : _root_.Zisk.SailM α} {f : α → _root_.Zisk.SailM β} {σ σ' : ZState} {b : β}
    (h : (P >>= f).run σ = .ok b σ') : ∃ a σ1, P.run σ = .ok a σ1 ∧ (f a).run σ1 = .ok b σ' := by
  rw [run_bind'] at h
  cases h1 : P.run σ with
  | error e σ1 => rw [h1] at h; cases h
  | ok a σ1 => rw [h1] at h; exact ⟨a, σ1, rfl, h⟩

/-- The step's end, the pc at p, the instruction not setting the pc from c:
    the pc moves on by one of its offsets. -/
theorem next_pc_tail (inst : _root_.Zisk.zisk_inst) (hs : inst.set_pc = false) (c : BitVec 64) (f : Bool)
    (σ : ZState) (p : BitVec 64) (hp : σ.regs.get? _root_.Zisk.Register.PC = some p) {b : Bool} {σ' : ZState}
    (h : (do
      let x ← _root_.Zisk.Functions.next_pc inst c f
      _root_.Zisk.writeReg _root_.Zisk.Register.PC x
      pure false : _root_.Zisk.SailM Bool).run σ = .ok b σ') :
    b = false ∧ (σ'.regs.get? _root_.Zisk.Register.PC = some (p + inst.jmp_off1) ∨
      σ'.regs.get? _root_.Zisk.Register.PC = some (p + inst.jmp_off2)) := by
  obtain ⟨x, σ1, h1, h2⟩ := run_bind_ok h
  obtain ⟨u, σ2, h3, h4⟩ := run_bind_ok h2
  rw [run_pure'] at h4; cases h4
  rw [writeReg_run] at h3; cases h3
  refine ⟨rfl, ?_⟩
  simp only [Std.ExtDHashMap.get?_insert_self]
  unfold _root_.Zisk.Functions.next_pc at h1
  rw [hs] at h1
  simp only [Bool.false_eq_true, if_false] at h1
  split at h1
  · obtain ⟨v, σ3, h5, h6⟩ := run_bind_ok h1
    rw [readReg_run'] at h5; rw [hp] at h5; cases h5
    rw [run_pure'] at h6; cases h6
    exact .inl rfl
  · obtain ⟨v, σ3, h5, h6⟩ := run_bind_ok h1
    rw [readReg_run'] at h5; rw [hp] at h5; cases h5
    rw [run_pure'] at h6; cases h6
    exact .inr rfl

set_option maxHeartbeats 0 in
set_option maxRecDepth 100000 in
/-- A step on an instruction that neither sets the pc from c nor ends: the
    pc moves on by one of its two offsets. -/
theorem execute_pc (inst : _root_.Zisk.zisk_inst) (hs : inst.set_pc = false) (he : inst.is_end = false)
    (σ : ZState) (p : BitVec 64) (hp : σ.regs.get? _root_.Zisk.Register.PC = some p) {b : Bool} {σ' : ZState}
    (h : (_root_.Zisk.Functions.zisk_execute inst).run σ = .ok b σ') :
    b = false ∧ (σ'.regs.get? _root_.Zisk.Register.PC = some (p + inst.jmp_off1) ∨
      σ'.regs.get? _root_.Zisk.Register.PC = some (p + inst.jmp_off2)) := by
  unfold _root_.Zisk.Functions.zisk_execute at h
  rw [run_bind'] at h
  cases ha : (_root_.Zisk.Functions.source_a inst).run σ with
  | error e σ1 => rw [ha] at h; cases h
  | ok a σ1 =>
  rw [ha] at h; dsimp only at h
  have p1 := (KeepPC.source_a inst σ a σ1 ha).trans hp
  rw [run_bind'] at h
  cases hb : (_root_.Zisk.Functions.source_b inst a).run σ1 with
  | error e σ2 => rw [hb] at h; cases h
  | ok bv σ2 =>
  rw [hb] at h; dsimp only at h
  have p2 := (KeepPC.source_b inst a σ1 bv σ2 hb).trans p1
  -- the prelude (blake2b's chain, for a precompile), which keeps the pc
  try dsimp only at h
  split at h
  all_goals rw [run_bind'] at h
  all_goals split at h
  all_goals (try (cases h; done))
  all_goals rename_i u σq hq
  all_goals have p2q : σq.regs.get? _root_.Zisk.Register.PC = some p := (by
    first | exact (keepPC_prelude _ _ σ2 u σq hq).trans p2 | (rw [run_pure'] at hq; cases hq; exact p2))
  all_goals clear p2 hb
  all_goals
    rw [run_bind'] at h
    split at h
    case h_2 => cases h
    next r σ3 hc =>
    have p3 : σ3.regs.get? _root_.Zisk.Register.PC = some p := by
      refine (KeepPC.bind (keepPC_of_frame (Frame.exec_mem _ _ _ _)) (fun o => ?_) σq r σ3 hc).trans p2q
      split <;> exact KeepPC.pure _
    rw [run_bind'] at h
    cases hst : (_root_.Zisk.Functions.store_c inst a r.1).run σ3 with
    | error e σ4 => rw [hst] at h; cases h
    | ok u σ4 =>
    rw [hst] at h; dsimp only at h
    have p4 := (KeepPC.store_c inst a r.1 σ3 u σ4 hst).trans p3
    have tail : ∀ (σ5 : ZState), σ5.regs.get? _root_.Zisk.Register.PC = some p →
        (do
          _root_.Zisk.writeReg _root_.Zisk.Register.Zc r.1
          _root_.Zisk.writeReg _root_.Zisk.Register.Zflag r.2
          let st ← _root_.Zisk.readReg _root_.Zisk.Register.Step
          _root_.Zisk.writeReg _root_.Zisk.Register.Step (Sail.BitVec.addInt st 1)
          if inst.is_end = true then pure true
          else do
            let x ← _root_.Zisk.Functions.next_pc inst r.1 r.2
            _root_.Zisk.writeReg _root_.Zisk.Register.PC x
            pure false : _root_.Zisk.SailM Bool).run σ5 = .ok b σ' →
        b = false ∧ (σ'.regs.get? _root_.Zisk.Register.PC = some (p + inst.jmp_off1) ∨
          σ'.regs.get? _root_.Zisk.Register.PC = some (p + inst.jmp_off2)) := by
      intro σ5 hp5 h5
      obtain ⟨_, σ6, h6, h7⟩ := run_bind_ok h5
      obtain ⟨_, σ7, h8, h9⟩ := run_bind_ok h7
      obtain ⟨st, σ8, h10, h11⟩ := run_bind_ok h9
      obtain ⟨_, σ9, h12, h13⟩ := run_bind_ok h11
      have q := (KeepPC.writeReg _ _ (by decide) σ8 _ σ9 h12).trans ((KeepPC.readReg _ σ7 st σ8 h10).trans
        ((KeepPC.writeReg _ _ (by decide) σ6 _ σ7 h8).trans ((KeepPC.writeReg _ _ (by decide) σ5 _ σ6 h6).trans hp5)))
      rw [he] at h13
      simp only [Bool.false_eq_true, if_false] at h13
      exact next_pc_tail inst hs r.1 r.2 σ9 p q h13
    split at h
    · obtain ⟨_, σ5, h5, h6⟩ := run_bind_ok h
      obtain ⟨_, σ6, h7, h8⟩ := run_bind_ok h5
      have q := (KeepPC.writeReg _ _ (by decide) σ6 _ σ5 h8).trans ((KeepPC.writeReg _ _ (by decide) σ4 _ σ6 h7).trans p4)
      exact tail σ5 q h6
    · obtain ⟨_, σ5, h5, h6⟩ := run_bind_ok h
      rw [run_pure'] at h5; cases h5
      exact tail σ4 p4 h6

/-- A ZisK step at p on an instruction that neither sets the pc from c nor
    ends, both offsets d: on to p + d. -/
theorem zstep_pc {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {τ τ' : ZState} {p d : BitVec 64}
    (hp : τ.regs.get? _root_.Zisk.Register.PC = some p) (hs : (zrom p).set_pc = false)
    (he : (zrom p).is_end = false) (h1 : (zrom p).jmp_off1 = d) (h2 : (zrom p).jmp_off2 = d) {b : Bool}
    (ht : (zstep zrom).run τ = .ok b τ') :
    b = false ∧ τ'.regs.get? _root_.Zisk.Register.PC = some (p + d) := by
  rw [zstep_run zrom τ p hp] at ht
  obtain ⟨hb, h | h⟩ := execute_pc _ hs he τ p hp ht
  · exact ⟨hb, by rw [h, h1]⟩
  · exact ⟨hb, by rw [h, h2]⟩

/-- R between each address and the next, from a along l. -/
def ChainR (R : BitVec 64 → BitVec 64 → Prop) : BitVec 64 → List (BitVec 64) → Prop
  | _, [] => True
  | a, b :: l => R a b ∧ ChainR R b l

/-- A run of k ZisK steps from p along a chain of instructions — each but
    the last neither setting the pc nor ending, its offsets the step to the
    next address — on τ and on σ, τ without X's bytes: each step on σ is
    the one on τ, unless the instruction there reads one of X's bytes. -/
theorem zrun_chain {X : Nat → Bool} (hX : ∀ k, X k → OffRam k) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} :
    ∀ (l : List (BitVec 64)) {k : Nat} {σ τ τ' : ZState} (p : BitVec 64), k ≤ l.length + 1 →
    τ.regs.get? _root_.Zisk.Register.PC = some p → FRX X σ τ →
    (∀ q ∈ (p :: l).take k, NoReads (zrom q)) →
    ChainR (fun a b => (zrom a).set_pc = false ∧ (zrom a).is_end = false ∧
      (zrom a).jmp_off1 = b - a ∧ (zrom a).jmp_off2 = b - a) p l →
    ZRun zrom k τ τ' → ∃ σ', ZRun zrom k σ σ' ∧ FRX X σ' τ'
  | _, _, σ, _, _, _, _, hp, h, _, _, .done _ => ⟨σ, .done _, h⟩
  | l, _, σ, τ, τ', p, hk, hp, h, hn, hc, .step (z' := τ1) ht hr => by
    obtain ⟨σ1, hs1, h1⟩ := zstep_noreads hX h hp (hn p (by simp)) ht
    cases l with
    | nil =>
      cases hr with
      | done _ => exact ⟨σ1, .step hs1 (.done _), h1⟩
      | step _ _ => simp at hk
    | cons q l =>
      obtain ⟨⟨hs, he, hj1, hj2⟩, hc'⟩ := hc
      obtain ⟨-, hp1⟩ := zstep_pc hp hs he hj1 hj2 ht
      rw [show p + (q - p) = q by rw [BitVec.add_comm, BitVec.sub_add_cancel]] at hp1
      obtain ⟨σ', hr', h'⟩ := zrun_chain hX l q (by simp at hk ⊢; omega) hp1 h1
        (fun r hr => hn r (by simp only [List.take_succ_cons, List.mem_cons] at hr ⊢; right; exact hr)) hc' hr
      exact ⟨σ', .step hs1 hr', h'⟩

end Equiv.Sim
