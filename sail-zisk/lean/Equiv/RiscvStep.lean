/-
  One step of sail-riscv, `try_step`, in ZisK's profile: the whole step —
  interrupt check, fetch, decode, execute, pc update, retired-instruction
  count — on an R-type instruction (`try_step_rtype`).

  The step reads many registers whose values do not matter (interrupt and
  counter state); a well-formed state (`WF`: every register present, as
  sail-riscv's `initializeRegisters` leaves it) gives each read a value,
  `regVal`. What the step needs of the rest: no interrupt pending (mip and
  mie 0: ZisK has no interrupt sources), the hart active, no landing pad
  expected, the memory hypotheses of Equiv/RiscvMem.lean, and the
  instruction's four bytes at pc.

  The proof steps through sail-riscv's code with `rv_step`, by `rw` rather
  than simp: simp does not rewrite with lean-sail's interpreter lemmas when
  the state is a pattern variable, and `rw` takes the outermost occurrence,
  the one at a concrete state. At fetch, decode and execute it applies the
  lemmas of Equiv/RiscvMem, Equiv/RiscvDecode and Equiv/Riscv.
-/
import Equiv.RiscvDecode

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- Every register is present: what sail-riscv's `initializeRegisters`
    leaves, and what lean-sail's interpreter needs to read one. -/
structure WF (s : RVState) : Prop where
  all : ∀ r, (s.regs.get? r).isSome

/-- A register's value in a well-formed state. -/
def regVal {s : RVState} (h : WF s) (r : Register) : RegisterType r := (s.regs.get? r).get (h.all r)

theorem WF.get? {s : RVState} (h : WF s) (r : Register) : s.regs.get? r = some (regVal h r) := by
  simp [regVal]

/-- A read of a register: its value in the register map. Generic in the
    register, so for `rw`: simp does not take it (nor its instances). -/
theorem readReg_match (s : RVState) (r : Register) :
    (interpretSailEffects (readReg r : SailM _)).run s = match s.regs.get? r with
      | some v => .ok v s
      | none => .error .Unreachable s := by
  cases h : s.regs.get? r with
  | some v => exact readReg_run s r v h
  | none => simp [interpretSailEffects, LeanZiskRV.readReg, Sail.ArchSem.PreSail.readReg, FreeM.liftM,
      interpretInstructionEffect, h]; rfl

/-- Whether a register is one of x1..x31. -/
def isXReg : Register → Bool
  | .x1 => true
  | .x2 => true
  | .x3 => true
  | .x4 => true
  | .x5 => true
  | .x6 => true
  | .x7 => true
  | .x8 => true
  | .x9 => true
  | .x10 => true
  | .x11 => true
  | .x12 => true
  | .x13 => true
  | .x14 => true
  | .x15 => true
  | .x16 => true
  | .x17 => true
  | .x18 => true
  | .x19 => true
  | .x20 => true
  | .x21 => true
  | .x22 => true
  | .x23 => true
  | .x24 => true
  | .x25 => true
  | .x26 => true
  | .x27 => true
  | .x28 => true
  | .x29 => true
  | .x30 => true
  | .x31 => true
  | _ => false

theorem xreg_isX (k : Nat) : isXReg (xreg k) = true := by
  unfold xreg; split <;> rfl

/-- An x register is no other register. -/
theorem xreg_beq_false (k : Nat) (r : Register) (hr : isXReg r = false) :
    (@BEq.beq Register instBEqOfDecidableEq (xreg k) r) = false := by
  cases e : (@BEq.beq Register instBEqOfDecidableEq (xreg k) r)
  · rfl
  · simp at e; subst e; simp [xreg_isX] at hr

theorem and_zero' (x : BitVec w) : x &&& 0 = 0 := by simp
theorem zero_and' (x : BitVec w) : 0 &&& x = 0 := by simp
theorem zero_ne' : ((0 : BitVec w) != 0#w) = false := by simp

/-- One step through sail-riscv's code, in a well-formed state `h`: a read
    (whose value is the map's, through the writes before it), a write, a
    bind, a pure. -/
syntax "rv_step " term : tactic
macro_rules
  | `(tactic| rv_step $h) => `(tactic| first
    | (rw [readReg_match]
       try simp (config := { decide := true }) only [Std.ExtDHashMap.get?_insert, cast_eq,
         beq_self_eq_true, if_true, dite_true, Bool.false_eq_true, if_false, dite_false]
       try simp only [*]
       try simp only [WF.get? $h])
    | (rw [writeReg_run]; try dsimp only)
    | (simp only [LeanZiskRV.assert, Sail.ArchSem.PreSail.assert, ↓reduceIte, interp_freePure, run_pure']
       try dsimp only)
    | (simp (disch := decide) only [xreg_beq_false, Bool.false_eq_true, if_false, dite_false]
       try simp only [*]
       try simp only [WF.get? $h])
    | (simp (config := { decide := true }) only [↓reduceIte, if_false, if_true, Bool.false_eq_true,
         Bool.false_and, Bool.and_false, Bool.true_and, Bool.and_true, Bool.false_or, Bool.or_false,
         BitVec.and_zero, BitVec.zero_and, zeros, bne_self_eq_false, Mk_Minterrupts, BitVec.zero_eq,
         and_zero', zero_and', zero_ne'])
    | (rw [interp_bind, run_bind']; try dsimp only)
    | (rw [interp_pure, run_pure']; try dsimp only)
    | (rw [run_pure']; try dsimp only)
    | (rw [me_run_lift_bind]; try dsimp only)
    | (rw [me_run_pure]; try dsimp only)
    | (rw [me_bind_pure]; try dsimp only)
    | (rw [bind_assoc]; try dsimp only))

/-- Writing a register the value it has changes nothing. -/
theorem insert_same (s : RVState) (r : Register) (v : RegisterType r) (hv : s.regs.get? r = some v) :
    { s with regs := s.regs.insert r v } = s := by
  cases s; simp only [SequentialState.mk.injEq, and_true]
  apply Std.ExtDHashMap.ext_get?; intro k
  rw [Std.ExtDHashMap.get?_insert]; split
  · next e => simp at e; subst e; simpa using hv.symm
  · rfl

/-- update_mip, with no interrupt pending (mip 0, as ZisK, with no
    interrupt sources, keeps it): nothing changes. -/
theorem update_mip_run (s : RVState) (h : WF s) (hmip : s.regs.get? mip = some 0) :
    (interpretSailEffects (update_mip ())).run s = .ok () s := by
  simp only [update_mip, get_mtime, get_hart_mtimecmp, machine_software_interrupt_pending, plat_have_clint,
    currentlyEnabled, hartSupports]
  repeat rv_step h
  have hz : supported_interrupts_mask (Sail.BitVec.updateSubrange (0 : BitVec 64) 3 3 (0#1 ||| 0#1)) = 0 := by
    rfl
  rw [hz, insert_same s mip 0 hmip]

theorem execute_rtype_eq (a b c : regidx) (op : rop) :
    execute (.RTYPE (a, b, c, op)) = execute_RTYPE a b c op := by
  simp [execute]

theorem no_lp : (landing_pad_bits_backwards .NO_LP_EXPECTED == landing_pad_bits_backwards .LP_EXPECTED) = false := by
  rfl

theorem beq_xreg_false (r : Register) (k : Nat) (hr : isXReg r = false) :
    (@BEq.beq Register instBEqOfDecidableEq r (xreg k)) = false := by
  cases e : (@BEq.beq Register instBEqOfDecidableEq r (xreg k))
  · rfl
  · simp at e; subst e; simp [xreg_isX] at hr

theorem isSome_insert (m : Std.ExtDHashMap Register RegisterType) (k : Register) (v : RegisterType k)
    (hm : ∀ r, (m.get? r).isSome) : ∀ r, ((m.insert k v).get? r).isSome := by
  intro r; simp only [Std.ExtDHashMap.get?_insert]; split <;> simp [hm r]

/-- A register MemReady reads. -/
def memRegs : List Register := [cur_privilege, mstatus, misa, senvcfg, menvcfg, htif_tohost_base, pma_regions]

theorem MemReady.congr {s t : RVState} {rs : List PMA_Region} (hs : MemReady s rs)
    (he : ∀ r ∈ memRegs, t.regs.get? r = s.regs.get? r) : MemReady t rs := by
  obtain ⟨hp, hm, hmi, hse, hme, hh, hr⟩ := hs
  simp only [memRegs, List.mem_cons, List.not_mem_nil, or_false, forall_eq_or_imp, forall_eq] at he
  obtain ⟨e1, e2, e3, e4, e5, e6, e7⟩ := he
  exact ⟨e1 ▸ hp, e2 ▸ hm, e3 ▸ hmi, e4 ▸ hse, e5 ▸ hme, e6 ▸ hh, e7 ▸ hr⟩

/-- What a step needs of the state, whatever the instruction: well formed,
    no interrupt pending, the hart active, no landing pad expected, and
    memory accesses as RiscvMem states them. Every step here keeps it. -/
structure StepReady (s : RVState) (rs : List PMA_Region) : Prop where
  wf : WF s
  mip : s.regs.get? mip = some 0
  mie : s.regs.get? mie = some 0
  hart : s.regs.get? hart_state = some (.HART_ACTIVE ())
  elp : s.regs.get? elp = some (landing_pad_bits_backwards .NO_LP_EXPECTED)
  mem : MemReady s rs
  /-- No pointer masking in machine mode (ZisK's profile has no Smmpm). -/
  pmm : ∃ v, s.regs.get? mseccfg = some v ∧ _get_Seccfg_PMM v = 0#2
  /-- M enabled: misa's M bit set (ZisK's profile has M, and a program in the
      contract writes no misa). -/
  misaM : ∃ v, s.regs.get? misa = some v ∧ _get_Misa_M v = 1#1
  /-- Zba, Zbb, Zbs enabled: misa's B bit set (ZisK's profile has them, and a
      program in the contract writes no misa). -/
  misaB : ∃ v, s.regs.get? misa = some v ∧ _get_Misa_B v = 1#1
  /-- A's AMOs enabled (Zaamo): misa's A bit set (ZisK's profile has A, and a
      program in the contract writes no misa). -/
  misaA : ∃ v, s.regs.get? misa = some v ∧ _get_Misa_A v = 1#1

/-- An instruction word at pc that sail-riscv fetches as such: aligned, in
    executable memory, not compressed. -/
def ExecAt (rs : List PMA_Region) (a : BitVec 64) : Prop :=
  ∃ r, matching_pma_region rs (.Physaddr a) 2 = some r ∧ r.attributes.executable = true

structure WordAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64) (b0 b1 b2 b3 : BitVec 8) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some b0
  h1 : s.mem.get? (pc.toNat + 1) = some b1
  h2 : s.mem.get? (pc.toNat + 2) = some b2
  h3 : s.mem.get? (pc.toNat + 3) = some b3
  c : b0.extractLsb 1 0 = 3#2

set_option maxHeartbeats 4000000 in
/-- One step of sail-riscv, in ZisK's profile, on any instruction that
    writes one register: the word at pc decodes to `ins` (in machine mode),
    and `ins` writes `val` to rd ≠ x0 and does nothing else (from any state
    whose x registers are s's). Then pc moves to pc + 4, rd gets val, and no
    register but PC, nextPC, minstret, minstret_increment and rd changes. -/
theorem try_step_alu (s : RVState) (rs : List PMA_Region) (hr : StepReady s rs)
    (pc : BitVec 64) (hpc : s.regs.get? PC = some pc)
    (b0 b1 b2 b3 : BitVec 8) (hw : WordAt s rs pc b0 b1 b2 b3) (ins : instruction)
    (hdec : ∀ t : RVState, t.regs.get? cur_privilege = some .Machine →
      (∃ v, t.regs.get? mseccfg = some v) →
      (∃ v, t.regs.get? misa = some v ∧ _get_Misa_M v = 1#1) →
      (∃ v, t.regs.get? misa = some v ∧ _get_Misa_B v = 1#1) →
      (interpretSailEffects (ext_decode (b3 ++ b2 ++ (b1 ++ b0)))).run t = .ok ins t)
    (rd : Nat) (hrd : rd ≠ 0) (val : BitVec 64)
    (hexec : ∀ t : RVState, (∀ k, k < 32 → getX t k = getX s k) → t.mem = s.mem → MemReady t rs →
      t.regs.get? mseccfg = s.regs.get? mseccfg → WF t →
      (interpretSailEffects (execute ins)).run t = .ok RETIRE_SUCCESS (setX t rd val)) (n : Nat) :
    ∃ s', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      s'.mem = s.mem ∧ WF s' ∧ s'.regs.get? PC = some (pc + 4) ∧
      (∀ k, k < 32 → getX s' k = getX (setX s rd val) k) ∧
      (∀ r, isXReg r = false → r ≠ PC → r ≠ nextPC → r ≠ minstret → r ≠ minstret_increment →
        s'.regs.get? r = s.regs.get? r) := by
  obtain ⟨h, hmip, hmie, hhs, help, hs, -, ⟨mv, hmv, hmM⟩, ⟨bv, hbv, hbB⟩, -⟩ := hr
  obtain ⟨hal, ⟨r1, hm1, hx1⟩, ⟨r2, hm2, hx2⟩, h0, h1, h2, h3, hc⟩ := hw
  have hp := hs.priv
  simp only [try_step, run_hart_active, should_inc_minstret, counter_priv_filter_bit, dispatchInterrupt,
    getPendingSet, read_mip, is_landing_pad_expected, hart_is_active, hart_is_waiting, tick_pc,
    get_config_rvfi, get_config_print_instr, currentlyEnabled, hartSupports, external_interrupts_pending,
    supervisor_software_interrupt_pending, plat_have_sig]
  rw [interp_bind, run_bind', update_mip_run s h hmip]; dsimp only
  repeat rv_step h
  rw [fetch_word _ rs ?hs1 pc ?hpc1 hal r1 r2 hm1 hm2 hx1 hx2 b0 b1 b2 b3 ?h0 ?h1 ?h2 ?h3 hc]
  rotate_left
  · exact hs.congr (by
      intro r hr
      simp only [memRegs, List.mem_cons, List.not_mem_nil, or_false] at hr
      rcases hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [Std.ExtDHashMap.get?_insert])
  · simp only [Std.ExtDHashMap.get?_insert]; simpa using hpc
  · exact h0
  · exact h1
  · exact h2
  · exact h3
  dsimp only [ext_fetch_hook]
  repeat rv_step h
  rw [hdec _ ?hp1 ?hsec1 ?hm1 ?hb1]
  rotate_left
  · simp only [Std.ExtDHashMap.get?_insert]; simpa using hp
  · exact ⟨regVal h mseccfg, by simp only [Std.ExtDHashMap.get?_insert]; simpa using h.get? mseccfg⟩
  · exact ⟨mv, by simp only [Std.ExtDHashMap.get?_insert]; simpa using hmv, hmM⟩
  · exact ⟨bv, by simp only [Std.ExtDHashMap.get?_insert]; simpa using hbv, hbB⟩
  repeat rv_step h
  simp only [no_lp, Bool.false_and, Bool.false_eq_true, if_false]
  repeat rv_step h
  rw [hexec _ ?gx ?gm ?gr ?gs ?gw]
  rotate_left
  · intro k _
    simp (disch := decide) only [getX, Std.ExtDHashMap.get?_insert, beq_xreg_false, Bool.false_eq_true,
      dite_false]
  · rfl
  · exact hs.congr (by
      intro r hr
      simp only [memRegs, List.mem_cons, List.not_mem_nil, or_false] at hr
      rcases hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [Std.ExtDHashMap.get?_insert])
  · simp [Std.ExtDHashMap.get?_insert]
  · exact ⟨isSome_insert _ _ _ (isSome_insert _ _ _ h.all)⟩
  simp only [setX, RETIRE_SUCCESS]
  rw [if_neg]; rotate_left; · exact hrd
  repeat rv_step h
  split
  all_goals (repeat rv_step h)
  all_goals refine ⟨_, rfl, rfl, ⟨?_⟩, ?_, ?_, ?_⟩
  all_goals try ((repeat apply isSome_insert); exact h.all)
  all_goals try (simp [Std.ExtDHashMap.get?_insert, BitVec.addInt]; done)
  all_goals try (intro r hx hpc' hn hm hmi
                 simp only [Std.ExtDHashMap.get?_insert, xreg_beq_false _ r hx, beq_iff_eq, Ne.symm hpc',
                   Ne.symm hn, Ne.symm hm, Ne.symm hmi, Bool.false_eq_true, dite_false, WF.get? h]
                 done)
  all_goals intro k hk32
  all_goals simp only [getX]
  all_goals split
  all_goals try rfl
  all_goals simp (disch := decide) only [Std.ExtDHashMap.get?_insert, beq_xreg_false, Bool.false_eq_true,
                     dite_false]
  all_goals rfl

/-- getX agreement gives the source registers' values. -/
theorem getX_of_agree {s t : RVState} (he : ∀ k, k < 32 → getX t k = getX s k) {k : Nat} (hk : k < 32)
    {v : BitVec 64} (hv : getX s k = some v) : getX t k = some v := (he k hk).trans hv

/-- One step of sail-riscv on an R-type instruction with rd ≠ x0. -/
theorem try_step_rtype (s : RVState) (rs : List PMA_Region) (hr : StepReady s rs)
    (pc : BitVec 64) (hpc : s.regs.get? PC = some pc)
    (b0 b1 b2 b3 : BitVec 8) (hw : WordAt s rs pc b0 b1 b2 b3) (op : rop)
    (hop : (b3 ++ b2 ++ (b1 ++ b0)).extractLsb 6 0 = 0x33#7)
    (hf3 : (b3 ++ b2 ++ (b1 ++ b0)).extractLsb 14 12 = rtypeF3 op)
    (hf7 : (b3 ++ b2 ++ (b1 ++ b0)).extractLsb 31 25 = rtypeF7 op)
    (hrd : (b3 ++ b2 ++ (b1 ++ b0)).extractLsb 11 7 ≠ 0#5)
    (v1 v2 : BitVec 64)
    (hv1 : getX s ((b3 ++ b2 ++ (b1 ++ b0)).extractLsb 19 15).toNat = some v1)
    (hv2 : getX s ((b3 ++ b2 ++ (b1 ++ b0)).extractLsb 24 20).toNat = some v2) (n : Nat) :
    ∃ s', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      s'.mem = s.mem ∧ WF s' ∧ s'.regs.get? PC = some (pc + 4) ∧
      (∀ k, k < 32 → getX s' k =
        getX (setX s ((b3 ++ b2 ++ (b1 ++ b0)).extractLsb 11 7).toNat (rtypeVal op v1 v2)) k) ∧
      (∀ r, isXReg r = false → r ≠ PC → r ≠ nextPC → r ≠ minstret → r ≠ minstret_increment →
        s'.regs.get? r = s.regs.get? r) := by
  have hk : ((b3 ++ b2 ++ (b1 ++ b0)).extractLsb 11 7).toNat ≠ 0 := by
    intro e; apply hrd; apply BitVec.eq_of_toNat_eq; simpa using e
  refine try_step_alu s rs hr pc hpc b0 b1 b2 b3 hw
    (.RTYPE (.Regidx ((b3 ++ b2 ++ (b1 ++ b0)).extractLsb 24 20), .Regidx ((b3 ++ b2 ++ (b1 ++ b0)).extractLsb 19 15),
      .Regidx ((b3 ++ b2 ++ (b1 ++ b0)).extractLsb 11 7), op)) ?_ _ hk _ ?_ n
  · intro t htp ⟨v, hv⟩ _ _
    exact decode_rtype t _ op hop hf3 hf7 hrd htp v hv
  · intro t he _ _ _ _
    rw [execute_rtype_eq]
    have l1 : ((b3 ++ b2 ++ (b1 ++ b0)).extractLsb 19 15 : RegBits).toNat < 32 := RegBits.lt _
    have l2 : ((b3 ++ b2 ++ (b1 ++ b0)).extractLsb 24 20 : RegBits).toNat < 32 := RegBits.lt _
    exact execute_rtype t op _ _ _ v1 v2 (getX_of_agree he l1 hv1) (getX_of_agree he l2 hv2)

end Equiv.Riscv
