/-
  sail-riscv's trap path, in ZisK's profile: an instruction whose execution
  traps (`ExecutionResult.Trap`) leaves the step at the trap vector, mtvec
  at 4 — outside ZisK's memory, which the machine sees as a trap
  (`try_step_trap`). And the traps the run's conditions meet: a load or a
  store at an address no PMA region permits, an access fault
  (`execute_load_fault`, `execute_store_fault`).
-/
import Equiv.RiscvLoad
import Equiv.RiscvStore

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

section ME
open Sail.ArchSem.PreSail
/-- Running a lifted SailM computation alone. -/
theorem me_run_lift (x : SailM α) : LeanZiskRV.SailME.run (monadLift x : SailME α α) = x := by
  simp only [LeanZiskRV.SailME.run]
  induction x with
  | pure a => rfl
  | impure e k ih =>
    show PreSailME.run (instMonadLiftPreSailMPreSailME.lift (FreeM.impure e k)) = _
    simp only [instMonadLiftPreSailMPreSailME.lift, PreSailME.run]
    show FreeM.impure e _ = FreeM.impure e _
    congr 1; funext r; exact ih r
end ME

set_option maxRecDepth 100000 in
theorem cnw_mstatus (s : RVState) (v : BitVec 64) :
    (interpretSailEffects (csr_name_write_callback "mstatus" v)).run s = .ok () s := rfl

set_option maxRecDepth 100000 in
theorem cnw_mcause (s : RVState) (v : BitVec 64) :
    (interpretSailEffects (csr_name_write_callback "mcause" v)).run s = .ok () s := rfl

set_option maxRecDepth 100000 in
theorem cnw_mtval (s : RVState) (v : BitVec 64) :
    (interpretSailEffects (csr_name_write_callback "mtval" v)).run s = .ok () s := rfl

set_option maxRecDepth 100000 in
theorem cnw_mepc (s : RVState) (v : BitVec 64) :
    (interpretSailEffects (csr_name_write_callback "mepc" v)).run s = .ok () s := rfl

set_option maxRecDepth 100000 in
theorem lcw_mstatus (s : RVState) (v : BitVec 64) :
    (interpretSailEffects (long_csr_write_callback "mstatus" "mstatush" v)).run s = .ok () s := rfl

/-- mtvec at TRAP_VEC, direct mode: every trap's vector. -/
theorem tvec4 (c : BitVec 64) : tvec_addr 0x4#64 c = some 0x4#64 := rfl

set_option maxHeartbeats 4000000 in
/-- sail-riscv's exception handler, in machine mode with mtvec at 4 (direct
    mode): nothing delegates (no S mode), and the trap vector is 4. -/
theorem exception_handler_run (s : RVState) (h : WF s) (hp : s.regs.get? cur_privilege = some .Machine)
    (hmt : s.regs.get? mtvec = some 0x4#64) (ctx : ExceptionContext) (pc : BitVec 64) :
    ∃ s', (interpretSailEffects (exception_handler .Machine ctx pc)).run s = .ok 0x4#64 s' ∧ WF s' ∧
      s'.regs.get? hart_state = s.regs.get? hart_state := by
  simp only [exception_handler, exception_delegatee, trap_handler, machine_trap_set_mdt, trap_handler_inner,
    currentlyEnabled, hartSupports, inVirtualPrivilege, get_config_print_exception, get_config_print_interrupt,
    track_trap, read_mstatus, prepare_trap_vector]
  repeat (first
    | rv_step h
    | (rw [me_run_lift]; try dsimp only)
    | (rw [lcw_mstatus]; try dsimp only)
    | (rw [cnw_mstatus]; try dsimp only)
    | (rw [cnw_mcause]; try dsimp only)
    | (rw [cnw_mtval]; try dsimp only)
    | (rw [cnw_mepc]; try dsimp only)
    | (rw [tvec4]; try dsimp only))
  exact ⟨_, rfl, ⟨by (repeat apply isSome_insert); exact h.all⟩, by simp [Std.ExtDHashMap.get?_insert, WF.get? h]⟩

/-- The state the exception handler leaves. -/
noncomputable def excState (s : RVState) (ctx : ExceptionContext) (pc : BitVec 64) : RVState :=
  match (interpretSailEffects (exception_handler .Machine ctx pc)).run s with
  | .ok _ s' => s'
  | .error _ s' => s'

theorem exception_handler_eq (s : RVState) (h : WF s) (hp : s.regs.get? cur_privilege = some .Machine)
    (hmt : s.regs.get? mtvec = some 0x4#64) (ctx : ExceptionContext) (pc : BitVec 64) :
    (interpretSailEffects (exception_handler .Machine ctx pc)).run s = .ok 0x4#64 (excState s ctx pc) ∧
      WF (excState s ctx pc) ∧ (excState s ctx pc).regs.get? hart_state = s.regs.get? hart_state := by
  obtain ⟨s', he, hw, hh⟩ := exception_handler_run s h hp hmt ctx pc
  simp only [excState, he, true_and]; exact ⟨hw, hh⟩

set_option maxHeartbeats 4000000 in
/-- One step of sail-riscv on an instruction whose execution traps, in
    machine mode, mtvec at 4 (direct): the step ends at the trap vector. -/
theorem try_step_trap_fn (s : RVState) (rs : List PMA_Region) (hr : StepReady s rs)
    (hmt : s.regs.get? mtvec = some 0x4#64)
    (pc : BitVec 64) (hpc : s.regs.get? PC = some pc)
    (b0 b1 b2 b3 : BitVec 8) (hw : WordAt s rs pc b0 b1 b2 b3) (ins : instruction)
    (hdec : ∀ t : RVState, t.regs.get? cur_privilege = some .Machine →
      (∃ v, t.regs.get? mseccfg = some v) →
      (∃ v, t.regs.get? misa = some v ∧ _get_Misa_M v = 1#1) →
      (∃ v, t.regs.get? misa = some v ∧ _get_Misa_B v = 1#1) →
      (interpretSailEffects (ext_decode (b3 ++ b2 ++ (b1 ++ b0)))).run t = .ok ins t)
    (ctx : RVState → ExceptionContext) (pc' : RVState → BitVec 64)
    (hexec : ∀ t : RVState, (∀ k, k < 32 → getX t k = getX s k) → t.mem = s.mem → MemReady t rs →
      t.regs.get? mseccfg = s.regs.get? mseccfg → WF t →
      (interpretSailEffects (execute ins)).run t = .ok (.Trap (.Machine, ctx t, pc' t)) t) (n : Nat) :
    ∃ s', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧ s'.regs.get? PC = some 0x4#64 := by
  obtain ⟨h, hmip, hmie, hhs, help, hs, -, ⟨mv, hmv, hmM⟩, ⟨bv, hbv, hbB⟩, -⟩ := hr
  obtain ⟨hal, ⟨r1, hm1, hx1⟩, ⟨r2, hm2, hx2⟩, h0, h1, h2, h3, hc⟩ := hw
  have hp := hs.priv
  simp only [try_step, run_hart_active, should_inc_minstret, counter_priv_filter_bit, dispatchInterrupt,
    getPendingSet, read_mip, is_landing_pad_expected, hart_is_active, hart_is_waiting, tick_pc,
    get_config_rvfi, get_config_print_instr, currentlyEnabled, hartSupports, external_interrupts_pending,
    supervisor_software_interrupt_pending, plat_have_sig, transform_exception_context]
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
  repeat rv_step h
  rw [(exception_handler_eq _ ?hw ?hp2 ?hmt2 _ _).1]
  rotate_left
  · exact ⟨by (repeat apply isSome_insert); exact h.all⟩
  · simp [Std.ExtDHashMap.get?_insert, hp]
  · simp [Std.ExtDHashMap.get?_insert, hmt]
  generalize hE : excState _ _ _ = E
  have hw2 : WF E := by
    rw [← hE]
    exact (exception_handler_eq _ ⟨by (repeat apply isSome_insert); exact h.all⟩
      (by simp [Std.ExtDHashMap.get?_insert, hp]) (by simp [Std.ExtDHashMap.get?_insert, hmt]) _ _).2.1
  have hhs2 : E.regs.get? hart_state = some (.HART_ACTIVE ()) := by
    rw [← hE, (exception_handler_eq _ ⟨by (repeat apply isSome_insert); exact h.all⟩
      (by simp [Std.ExtDHashMap.get?_insert, hp]) (by simp [Std.ExtDHashMap.get?_insert, hmt]) _ _).2.2]
    simp [Std.ExtDHashMap.get?_insert, hhs]
  simp only [set_next_pc]
  repeat (first | rv_step hw2 | (simp only [WF.get? hw2]; try dsimp only))
  exact ⟨_, rfl, by simp [Std.ExtDHashMap.get?_insert]⟩

set_option maxHeartbeats 4000000 in
/-- One step of sail-riscv on an instruction its execution finds illegal
    (`Illegal_Instruction`), in machine mode, mtvec at 4: the step ends at
    the trap vector. -/
theorem try_step_illegal (s : RVState) (rs : List PMA_Region) (hr : StepReady s rs)
    (hmt : s.regs.get? mtvec = some 0x4#64)
    (pc : BitVec 64) (hpc : s.regs.get? PC = some pc)
    (b0 b1 b2 b3 : BitVec 8) (hw : WordAt s rs pc b0 b1 b2 b3) (ins : instruction)
    (hdec : ∀ t : RVState, t.regs.get? cur_privilege = some .Machine →
      (∃ v, t.regs.get? mseccfg = some v) →
      (∃ v, t.regs.get? misa = some v ∧ _get_Misa_M v = 1#1) →
      (∃ v, t.regs.get? misa = some v ∧ _get_Misa_B v = 1#1) →
      (∃ v, t.regs.get? misa = some v ∧ _get_Misa_A v = 1#1) →
      (interpretSailEffects (ext_decode (b3 ++ b2 ++ (b1 ++ b0)))).run t = .ok ins t)
    (hexec : ∀ t : RVState, (∀ k, k < 32 → getX t k = getX s k) → t.mem = s.mem → MemReady t rs →
      t.regs.get? mseccfg = s.regs.get? mseccfg → WF t →
      (interpretSailEffects (execute ins)).run t = .ok (.Illegal_Instruction ()) t) (n : Nat) :
    ∃ s', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧ s'.regs.get? PC = some 0x4#64 := by
  obtain ⟨av, hav, haA⟩ := hr.misaA
  obtain ⟨h, hmip, hmie, hhs, help, hs, -, ⟨mv, hmv, hmM⟩, ⟨bv, hbv, hbB⟩, -⟩ := hr
  obtain ⟨hal, ⟨r1, hm1, hx1⟩, ⟨r2, hm2, hx2⟩, h0, h1, h2, h3, hc⟩ := hw
  have hp := hs.priv
  simp only [try_step, run_hart_active, should_inc_minstret, counter_priv_filter_bit, dispatchInterrupt,
    getPendingSet, read_mip, is_landing_pad_expected, hart_is_active, hart_is_waiting, tick_pc,
    get_config_rvfi, get_config_print_instr, currentlyEnabled, hartSupports, external_interrupts_pending,
    supervisor_software_interrupt_pending, plat_have_sig, handle_exception]
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
  rw [hdec _ ?hp1 ?hsec1 ?hm1 ?hb1 ?ha1]
  rotate_left
  · simp only [Std.ExtDHashMap.get?_insert]; simpa using hp
  · exact ⟨regVal h mseccfg, by simp only [Std.ExtDHashMap.get?_insert]; simpa using h.get? mseccfg⟩
  · exact ⟨mv, by simp only [Std.ExtDHashMap.get?_insert]; simpa using hmv, hmM⟩
  · exact ⟨bv, by simp only [Std.ExtDHashMap.get?_insert]; simpa using hbv, hbB⟩
  · exact ⟨av, by simp only [Std.ExtDHashMap.get?_insert]; simpa using hav, haA⟩
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
  repeat rv_step h
  rw [(exception_handler_eq _ ?hw ?hp2 ?hmt2 _ _).1]
  rotate_left
  · exact ⟨by (repeat apply isSome_insert); exact h.all⟩
  · simp [Std.ExtDHashMap.get?_insert, hp]
  · simp [Std.ExtDHashMap.get?_insert, hmt]
  generalize hE : excState _ _ _ = E
  have hw2 : WF E := by
    rw [← hE]
    exact (exception_handler_eq _ ⟨by (repeat apply isSome_insert); exact h.all⟩
      (by simp [Std.ExtDHashMap.get?_insert, hp]) (by simp [Std.ExtDHashMap.get?_insert, hmt]) _ _).2.1
  have hhs2 : E.regs.get? hart_state = some (.HART_ACTIVE ()) := by
    rw [← hE, (exception_handler_eq _ ⟨by (repeat apply isSome_insert); exact h.all⟩
      (by simp [Std.ExtDHashMap.get?_insert, hp]) (by simp [Std.ExtDHashMap.get?_insert, hmt]) _ _).2.2]
    simp [Std.ExtDHashMap.get?_insert, hhs]
  simp only [set_next_pc]
  repeat (first | rv_step hw2 | (simp only [WF.get? hw2]; try dsimp only))
  exact ⟨_, rfl, by simp [Std.ExtDHashMap.get?_insert]⟩

/-- `try_step_trap_fn`, the trap's context and pc given by the execution. -/
theorem try_step_trap (s : RVState) (rs : List PMA_Region) (hr : StepReady s rs)
    (pc : BitVec 64) (hpc : s.regs.get? PC = some pc)
    (b0 b1 b2 b3 : BitVec 8) (hw : WordAt s rs pc b0 b1 b2 b3) (ins : instruction)
    (hdec : ∀ t : RVState, t.regs.get? cur_privilege = some .Machine →
      (∃ v, t.regs.get? mseccfg = some v) →
      (∃ v, t.regs.get? misa = some v ∧ _get_Misa_M v = 1#1) →
      (∃ v, t.regs.get? misa = some v ∧ _get_Misa_B v = 1#1) →
      (interpretSailEffects (ext_decode (b3 ++ b2 ++ (b1 ++ b0)))).run t = .ok ins t)
    (hexec : ∀ t : RVState, (∀ k, k < 32 → getX t k = getX s k) → t.mem = s.mem → MemReady t rs →
      t.regs.get? mseccfg = s.regs.get? mseccfg → WF t →
      ∃ cp : ExceptionContext × BitVec 64,
        (interpretSailEffects (execute ins)).run t = .ok (.Trap (.Machine, cp.1, cp.2)) t) (n : Nat) :
    ∃ s', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧ s'.regs.get? PC = some 0x4#64 := by
  classical
  let C : RVState → ExceptionContext × BitVec 64 := fun t =>
    if h : ∃ cp : ExceptionContext × BitVec 64,
        (interpretSailEffects (execute ins)).run t = .ok (.Trap (.Machine, cp.1, cp.2)) t
    then Classical.choose h else (make_exception_context (.E_Illegal_Instr ()) 0 false none, 0)
  refine try_step_trap_fn s rs hr hr.mtvec pc hpc b0 b1 b2 b3 hw ins hdec (fun t => (C t).1) (fun t => (C t).2)
    (fun t a b c d e => ?_) n
  have hx := hexec t a b c d e
  show _ = EStateM.Result.ok (ExecutionResult.Trap (.Machine, (C t).1, (C t).2)) t
  simp only [C, dif_pos hx]
  exact Classical.choose_spec hx

theorem me_run_throw_bind (e : β) (f : α → SailME β β) :
    LeanZiskRV.SailME.run ((LeanZiskRV.SailME.throw e : SailME β α) >>= f) = pure e := rfl

theorem me_run_throw (e : β) : LeanZiskRV.SailME.run (LeanZiskRV.SailME.throw e : SailME β β) = pure e := rfl

/-- The PMA check of an access no region permits: an access fault. -/
theorem pmaCheck_fault (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (a : BitVec 64) (w : Nat) (acc : MemoryAccessType mem_payload) (hacc : PlainAccess acc)
    (hno : matching_pma_region rs (.Physaddr a) w = none ∨
      ∃ r, matching_pma_region rs (.Physaddr a) w = some r ∧ permits r.attributes acc = false) :
    ∃ e, (interpretSailEffects (pmaCheck (.Physaddr a) w acc .PBMT_PMA false)).run s = .ok (.Err e) s := by
  rcases hno with hm | ⟨⟨base, size, attrs, dt⟩, hm, hperm⟩
  · rcases hacc with rfl | rfl | rfl <;>
    · simp [pmaCheck, hm, me_run_lift_bind, readReg_run _ _ _ hr, accessFaultFromAccessType,
        bind_assoc, me_run_throw_bind]
  · rcases hacc with rfl | rfl | rfl <;>
    · simp only [permits] at hperm
      simp [pmaCheck, hm, override_PMA, hperm, me_run_lift_bind, readReg_run _ _ _ hr,
        accessFaultFromAccessType, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert, bind_assoc, LeanZiskRV.not,
        me_run_map_lift]

/-- A read no region permits: the PMA check's fault, at the address. -/
theorem mem_read_fault (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (a : BitVec 64) (w : Nat) (acc : MemoryAccessType mem_payload) (hacc : PlainAccess acc)
    (hno : matching_pma_region rs (.Physaddr a) w = none ∨
      ∃ r, matching_pma_region rs (.Physaddr a) w = some r ∧ permits r.attributes acc = false) :
    ∃ e, (interpretSailEffects (mem_read acc .PBMT_PMA .Machine (.Physaddr a) w false false false)).run s =
      .ok (.Err (.Physaddr a, e)) s := by
  obtain ⟨e, he⟩ := pmaCheck_fault s rs hr a w acc hacc hno
  refine ⟨e, ?_⟩
  simp [mem_read, mem_read_priv_meta, checked_mem_read, check_pma_with_pmp_priority, he, pmpCheck_run,
    me_run_lift_bind, bind_assoc, me_run_throw_bind, MemoryOpResult_drop_meta]

set_option maxHeartbeats 4000000 in
/-- A load no region permits, of 1, 2, 4 or 8 bytes, in machine mode with
    no pointer masking: a trap, the state as it was. -/
theorem execute_load_fault (t : RVState) (_h : WF t) (rs : List PMA_Region) (hs : MemReady t rs)
    (sec : RegisterType mseccfg) (hsec : t.regs.get? mseccfg = some sec) (hpm : _get_Seccfg_PMM sec = 0#2)
    (rs1 rd : RegBits) (imm : BitVec 12) (v1 : BitVec 64) (h1 : getX t rs1.toNat = some v1)
    (w : Nat) (hw : w = 1 ∨ w = 2 ∨ w = 4 ∨ w = 8) (u : Bool)
    (hno : matching_pma_region rs (.Physaddr (v1 + sign_extend (m := 64) imm)) w = none ∨
      ∃ r, matching_pma_region rs (.Physaddr (v1 + sign_extend (m := 64) imm)) w = some r ∧
        permits r.attributes (.Load .Data) = false) :
    ∃ ctx p, (interpretSailEffects (execute_LOAD imm (.Regidx rs1) (.Regidx rd) u w)).run t
      = .ok (.Trap (.Machine, ctx, p)) t := by
  have hs' := hs
  obtain ⟨hp, ⟨ms, hms, hmprv⟩, ⟨mi, hmi⟩, ⟨se, hse⟩, ⟨me, hme⟩, hh, hr⟩ := hs
  have r1 := rX_run t rs1.toNat rs1.lt v1 h1
  have hacc : PlainAccess (.Load .Data) := Or.inr (Or.inl rfl)
  have ht := translateAddr_run t (v1 + sign_extend (m := 64) imm) _ hacc mi hmi hp se hse me hme
  obtain ⟨e, hrd'⟩ := mem_read_fault t rs hr _ w _ hacc hno
  obtain ⟨sp, hsp⟩ := split_any (v1 + sign_extend (m := 64) imm) w hw
  simp only [execute_LOAD, vmem_read, vmem_read_explicit, default_access_info, get_pmlen,
    get_transformed_data_addr, ext_data_get_addr, transform_effective_address, vmem_read_addr,
    rX_bits, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert, is_pmm_applicable, pm_transform_PA,
    Functions.xlen_bytes, get_pmm, pmlen_of_mode, toNatInt_toNat, Functions.satp_mode, currentlyEnabled,
    hartSupports, LeanZiskRV.not, translate_and_read_value, wX_bits, extend_value, memory_exception, trap]
  rw [interp_bind, run_bind', if_pos ?hw8]
  rotate_left
  · rcases hw with rfl | rfl | rfl | rfl <;> rfl
  repeat (first
    | rv_step _h
    | (rw [effectivePrivilege_run _ _ _ _ hmprv]; try dsimp only)
    | (rw [r1]; try dsimp only)
    | (simp only [pmm_mode_backwards, pm_id, plat_ld, bits_of_virtaddr, ite_self]; try dsimp only)
    | (rw [hsp]; try dsimp only)
    | (rw [me_run_throw_bind]; try dsimp only)
    | (simp only [bare_ne, Bool.false_and, Bool.and_false, Bool.false_eq_true, if_false]; try dsimp only)
    | (rw [ht]; try dsimp only)
    | (rw [hrd']; try dsimp only)
    | (erw [hrd']; try dsimp only))
  exact ⟨_, _, rfl⟩

/-- A write no region permits: the PMA check's fault, at the address. -/
theorem mem_write_ea_fault (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (a : BitVec 64) (w : Nat)
    (hno : matching_pma_region rs (.Physaddr a) w = none ∨
      ∃ r, matching_pma_region rs (.Physaddr a) w = some r ∧ permits r.attributes (.Store .Data) = false) :
    ∃ e, (interpretSailEffects (mem_write_ea (.Physaddr a) w .Machine (.Store .Data) .PBMT_PMA false false false)).run s =
      .ok (.Err (.Physaddr a, e)) s := by
  obtain ⟨e, he⟩ := pmaCheck_fault s rs hr a w _ (Or.inr (Or.inr rfl)) hno
  refine ⟨e, ?_⟩
  simp [mem_write_ea, check_pma_with_pmp_priority, he, pmpCheck_run, me_run_lift_bind, bind_assoc, me_run_throw_bind]

set_option maxHeartbeats 4000000 in
/-- A store no region permits, of 1, 2, 4 or 8 bytes, in machine mode with
    no pointer masking: a trap, the state as it was. -/
theorem execute_store_fault (t : RVState) (_h : WF t) (rs : List PMA_Region) (hs : MemReady t rs)
    (sec : RegisterType mseccfg) (hsec : t.regs.get? mseccfg = some sec) (hpm : _get_Seccfg_PMM sec = 0#2)
    (rs1 rs2 : RegBits) (imm : BitVec 12) (v1 v2 : BitVec 64)
    (h1 : getX t rs1.toNat = some v1) (h2 : getX t rs2.toNat = some v2)
    (w : Nat) (hw : w = 1 ∨ w = 2 ∨ w = 4 ∨ w = 8)
    (hno : matching_pma_region rs (.Physaddr (v1 + sign_extend (m := 64) imm)) w = none ∨
      ∃ r, matching_pma_region rs (.Physaddr (v1 + sign_extend (m := 64) imm)) w = some r ∧
        permits r.attributes (.Store .Data) = false) :
    ∃ ctx p, (interpretSailEffects (execute_STORE imm (.Regidx rs2) (.Regidx rs1) w)).run t
      = .ok (.Trap (.Machine, ctx, p)) t := by
  have hs' := hs
  obtain ⟨hp, ⟨ms, hms, hmprv⟩, ⟨mi, hmi⟩, ⟨se, hse⟩, ⟨me, hme⟩, hh, hr⟩ := hs
  have r1 := rX_run t rs1.toNat rs1.lt v1 h1
  have r2 := rX_run t rs2.toNat rs2.lt v2 h2
  have hacc : PlainAccess (.Store .Data) := Or.inr (Or.inr rfl)
  have ht := translateAddr_run t (v1 + sign_extend (m := 64) imm) _ hacc mi hmi hp se hse me hme
  obtain ⟨e, hea⟩ := mem_write_ea_fault t rs hr _ w hno
  obtain ⟨sp, hsp⟩ := split_any (v1 + sign_extend (m := 64) imm) w hw
  simp only [execute_STORE, vmem_write, vmem_write_explicit, default_access_info, get_pmlen,
    get_transformed_data_addr, ext_data_get_addr, transform_effective_address, vmem_write_addr,
    rX_bits, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert, is_pmm_applicable, pm_transform_PA,
    Functions.xlen_bytes, get_pmm, pmlen_of_mode, toNatInt_toNat, Functions.satp_mode, currentlyEnabled,
    hartSupports, LeanZiskRV.not, translate_and_write_value, memory_exception, trap]
  rw [interp_bind, run_bind', if_pos ?hw8]
  rotate_left
  · rcases hw with rfl | rfl | rfl | rfl <;> rfl
  repeat (first
    | rv_step _h
    | (rw [effectivePrivilege_run _ _ _ _ hmprv]; try dsimp only)
    | (rw [r1]; try dsimp only)
    | (rw [r2]; try dsimp only)
    | (simp only [pmm_mode_backwards, pm_id, plat_st, bits_of_virtaddr, ite_self]; try dsimp only)
    | (rw [hsp]; try dsimp only)
    | (simp only [bare_ne, Bool.false_and, Bool.and_false, Bool.false_eq_true, if_false]; try dsimp only)
    | (rw [me_run_throw_bind]; try dsimp only)
    | (rw [ht]; try dsimp only)
    | (erw [hea]; try dsimp only))
  exact ⟨_, _, rfl⟩

end Equiv.Riscv
