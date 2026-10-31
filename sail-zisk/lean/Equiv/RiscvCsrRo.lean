/-
  sail-riscv's CSR instructions on a write to a read-only CSR (number
  0xC00 and above), in machine mode: an illegal instruction, the state as
  it was (`execute_csrreg_ro`, `execute_csrimm_ro`), for any such CSR.

  The CSR checks' monadic parts run whatever the access check finds (Lean's
  `do` lifts each `(← …)` ahead of the `&&` around it), so the proof needs
  them all to run, for a CSR number not known: `Quiet`, code that on any
  well-formed state succeeds and leaves it as it was, through binds,
  branches and register reads, the extension checks and their mutual block
  by induction on its termination measure (`quietAt`).
-/
import Equiv.RiscvStep
namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- Code that, on any well-formed state, succeeds and leaves the state as it was. -/
structure Quiet {α : Type} (m : SailM α) : Prop where
  run : ∀ t : RVState, WF t → ∃ b, (interpretSailEffects m).run t = .ok b t

theorem Quiet.pure {α : Type} (a : α) : Quiet (pure a : SailM α) := ⟨fun _ _ => ⟨a, rfl⟩⟩

theorem Quiet.bind {α β : Type} {m : SailM α} {f : α → SailM β} (hm : Quiet m) (hf : ∀ a, Quiet (f a)) :
    Quiet (m >>= f) := by
  refine ⟨fun t ht => ?_⟩
  obtain ⟨a, ha⟩ := hm.run t ht
  obtain ⟨b, hb⟩ := (hf a).run t ht
  exact ⟨b, by rw [interp_bind, run_bind', ha]; exact hb⟩

theorem Quiet.readReg (r : Register) : Quiet (readReg r : SailM _) :=
  ⟨fun t ht => ⟨regVal ht r, readReg_run t r _ (ht.get? r)⟩⟩

theorem Quiet.ite {α : Type} {c : Prop} [Decidable c] {a b : SailM α} (ha : Quiet a) (hb : Quiet b) :
    Quiet (if c then a else b) := by
  split <;> assumption

/-- The mutual block of `currentlyEnabled`, at measure n. -/
def QuietAt (n : Nat) : Prop :=
  (∀ e, (currentlyEnabled_measure e).toNat = n → Quiet (currentlyEnabled e)) ∧
  (n = 7 → ∀ p b r, Quiet (check_stateen_bit p b r)) ∧
  (n = 6 → ∀ i, Quiet (get_hstateen i)) ∧
  (n = 3 → ∀ i, Quiet (get_sstateen i)) ∧
  (n = 2 → ∀ p, Quiet (get_xLPE p)) ∧
  (n = 5 → Quiet (is_hstateen_accessible ())) ∧
  (n = 2 → Quiet (is_sstateen_accessible ())) ∧
  (n = 8 → Quiet (is_zfinx_enabled_by_stateen ()))

syntax "quiet_ih " term : tactic
macro_rules
  | `(tactic| quiet_ih $ih) => `(tactic| repeat (first
      | exact Quiet.pure _
      | exact Quiet.readReg _
      | (apply Quiet.bind; (try intro))
      | (intro)
      | split
      | exact ($ih _ (by decide) |>.1 _ rfl)
      | exact ($ih 7 (by decide) |>.2.1 rfl _ _ _)
      | exact ($ih 6 (by decide) |>.2.2.1 rfl _)
      | exact ($ih 3 (by decide) |>.2.2.2.1 rfl _)
      | exact ($ih 2 (by decide) |>.2.2.2.2.1 rfl _)
      | exact ($ih 5 (by decide) |>.2.2.2.2.2.1 rfl)
      | exact ($ih 2 (by decide) |>.2.2.2.2.2.2.1 rfl)
      | exact ($ih 8 (by decide) |>.2.2.2.2.2.2.2 rfl)
      | (unfold get_mstateen)
      | (unfold read_senvcfg)
      | (unfold read_henvcfg)
      | (unfold inVirtualPrivilege)))

set_option maxHeartbeats 0 in
set_option maxRecDepth 100000 in
theorem quietAt (n : Nat) : QuietAt n := by
  induction n using Nat.strongRecOn with
  | _ n ih =>
  refine ⟨fun e hn => ?_, fun hn p b r => ?_, fun hn i => ?_, fun hn i => ?_, fun hn p => ?_, fun hn => ?_,
    fun hn => ?_, fun hn => ?_⟩
  · rw [currentlyEnabled.eq_def]
    cases e <;> (simp only []; subst hn; quiet_ih ih)
  · subst hn; rw [check_stateen_bit.eq_def]; quiet_ih ih
  · subst hn; rw [get_hstateen.eq_def]; quiet_ih ih
  · subst hn; rw [get_sstateen.eq_def]; quiet_ih ih
  · subst hn; rw [get_xLPE.eq_def]; quiet_ih ih
  · subst hn; rw [is_hstateen_accessible.eq_def]; quiet_ih ih
  · subst hn; rw [is_sstateen_accessible.eq_def]; quiet_ih ih
  · subst hn; rw [is_zfinx_enabled_by_stateen.eq_def]; quiet_ih ih

theorem quiet_ce (e : extension) : Quiet (currentlyEnabled e) := (quietAt _).1 e rfl
theorem quiet_csb (p b r) : Quiet (check_stateen_bit p b r) := (quietAt 7).2.1 rfl p b r
theorem quiet_xlpe (p) : Quiet (get_xLPE p) := (quietAt 2).2.2.2.2.1 rfl p
theorem quiet_hsa : Quiet (is_hstateen_accessible ()) := (quietAt 5).2.2.2.2.2.1 rfl
theorem quiet_ssa : Quiet (is_sstateen_accessible ()) := (quietAt 2).2.2.2.2.2.2.1 rfl
theorem quiet_hst (i) : Quiet (get_hstateen i) := (quietAt 6).2.2.1 rfl i
theorem quiet_sst (i) : Quiet (get_sstateen i) := (quietAt 3).2.2.2.1 rfl i

/-- Through binds, pures, register reads, branches, the extension checks
    and the helpers they call. -/
syntax "quiet" : tactic
macro_rules
  | `(tactic| quiet) => `(tactic| repeat' (first
      | exact Quiet.pure _
      | exact Quiet.readReg _
      | exact quiet_ce _
      | exact quiet_csb _ _ _
      | exact quiet_xlpe _
      | exact quiet_hsa
      | exact quiet_ssa
      | exact quiet_hst _
      | exact quiet_sst _
      | (apply Quiet.bind; (try intro))
      | (apply Quiet.ite)
      | (intro)
      | split
      | (unfold get_mstateen)
      | (unfold read_senvcfg)
      | (unfold read_henvcfg)
      | (unfold inVirtualPrivilege)
      | (unfold is_satp_accessible)
      | (unfold counter_enabled)
      | (unfold feature_enabled_for_priv)
      | (unfold privLevel_to_CSR_privbits)))

set_option maxHeartbeats 0 in
set_option maxRecDepth 100000 in
theorem quiet_acc (c p a) : Quiet (is_CSR_accessible c p a) := by
  unfold is_CSR_accessible; dsimp only; quiet

set_option maxHeartbeats 0 in
set_option maxRecDepth 100000 in
theorem quiet_stateen (c p a) : Quiet (stateen_allows_CSR_access c p a) := by
  unfold stateen_allows_CSR_access; quiet

theorem quiet_priv (c : BitVec 12) (p : Privilege) : Quiet (check_CSR_priv c p) := by
  unfold check_CSR_priv; quiet

theorem quiet_check (c p a) : Quiet (check_CSR c p a) := by
  unfold check_CSR
  repeat' (first
    | exact quiet_priv _ _
    | exact quiet_acc _ _ _
    | exact quiet_stateen _ _ _
    | exact Quiet.pure _
    | (intro)
    | (apply Quiet.bind; (try intro)))

set_option maxRecDepth 100000 in
theorem quiet_virt (c p a) : Quiet (is_CSR_exception_virtual c p a) := by
  unfold is_CSR_exception_virtual; dsimp only
  repeat' (first | exact quiet_check _ _ _ | exact Quiet.pure _ | split)

/-- The CSR checks on an access their CSR's number forbids (a write to a
    read-only CSR): false, the state as it was. -/
theorem check_CSR_ro (t : RVState) (ht : WF t) (c : BitVec 12) (p : Privilege) (a : CSRAccessType)
    (h : check_CSR_access c a = false) :
    (interpretSailEffects (check_CSR c p a)).run t = .ok false t := by
  obtain ⟨b1, h1⟩ := (quiet_priv c p).run t ht
  obtain ⟨b2, h2⟩ := (quiet_acc c p a).run t ht
  obtain ⟨b3, h3⟩ := (quiet_stateen c p a).run t ht
  simp only [check_CSR, interp_bind, run_bind', h1, h2, h3, h, Bool.false_and, Bool.and_false, interp_pure,
    run_pure']

theorem check_CSR_result_ro (t : RVState) (ht : WF t) (c : BitVec 12) (a : CSRAccessType)
    (h : check_CSR_access c a = false) :
    (interpretSailEffects (check_CSR_result c .Machine a)).run t = .ok (.CSR_Illegal ()) t := by
  obtain ⟨b, hb⟩ := (quiet_virt c .Machine a).run t ht
  simp only [check_CSR_result, interp_bind, run_bind', check_CSR_ro t ht c _ a h]
  simp only [Bool.false_eq_true, if_false, interp_bind, run_bind', hb]
  cases b <;> rfl

/-- `doCSR` in machine mode on an access its CSR's number forbids: an
    illegal instruction, the state as it was. -/
theorem doCSR_ro (t : RVState) (ht : WF t) (hp : t.regs.get? cur_privilege = some .Machine)
    (c : BitVec 12) (v : BitVec 64) (rd : regidx) (op : csrop) (a : CSRAccessType)
    (h : check_CSR_access c a = false) :
    (interpretSailEffects (doCSR c v rd op a)).run t = .ok (.Illegal_Instruction ()) t := by
  simp only [doCSR, interp_bind, run_bind']
  rw [readReg_run t _ _ hp]; dsimp only
  rw [check_CSR_result_ro t ht c a h]; rfl

theorem quiet_rX (r : regidx) : Quiet (rX_bits r) := by
  unfold rX_bits rX; quiet

/-- A CSR instruction, register form, in machine mode, on an access its
    CSR's number forbids: an illegal instruction, the state as it was. -/
theorem execute_csrreg_ro (t : RVState) (ht : WF t) (hp : t.regs.get? cur_privilege = some .Machine)
    (c : BitVec 12) (rs1 rd : regidx) (op : csrop)
    (h : check_CSR_access c (csr_access_type op (rd == zreg) (rs1 == zreg)) = false) :
    (interpretSailEffects (execute (.CSRReg (c, rs1, rd, op)))).run t = .ok (.Illegal_Instruction ()) t := by
  obtain ⟨v, hv⟩ := (quiet_rX rs1).run t ht
  simp only [execute, execute_CSRReg, interp_bind, run_bind', hv]
  exact doCSR_ro t ht hp c v rd op _ h

/-- The immediate form. -/
theorem execute_csrimm_ro (t : RVState) (ht : WF t) (hp : t.regs.get? cur_privilege = some .Machine)
    (c : BitVec 12) (imm : BitVec 5) (rd : regidx) (op : csrop)
    (h : check_CSR_access c (csr_access_type op (rd == zreg) (imm == zeros (n := 5))) = false) :
    (interpretSailEffects (execute (.CSRImm (c, imm, rd, op)))).run t = .ok (.Illegal_Instruction ()) t := by
  simp only [execute, execute_CSRImm]
  exact doCSR_ro t ht hp c _ rd op _ h

end Equiv.Riscv
