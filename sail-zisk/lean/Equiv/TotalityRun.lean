/-
  Totality over the run: from the start, step by step, the run is covered
  (`RunOkM`, at code addresses, marchid's cell holding the ID) as far as
  one likes, or it stops at a step where `step_case` finds one of the ends,
  the return to the initial ra, or a precondition broken — among them
  marchid's cell changed by the step before (`BreakCell`).
-/
import Equiv.TotalityRej

namespace Equiv.Sim

open Equiv.Riscv Equiv.Machine
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-! ## Runs, extended at their end -/

theorem sIter_succ : ∀ (k n : Nat) (S : ZiskRiscv.State model), sIter n (k + 1) S = sNext (n + k) (sIter n k S)
  | 0, n, S => rfl
  | k + 1, n, S => by
    rw [show sIter n (k + 1 + 1) S = sIter (n + 1) (k + 1) (sNext n S) from rfl, sIter_succ k (n + 1)]
    rw [show n + 1 + k = n + (k + 1) by omega]; rfl

theorem RunOkM.snoc {rs : List PMA_Region} {zr : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop} :
    ∀ (k : Nat) (S : ZiskRiscv.State model) (n : Nat), RunOkM rs zr C k S n → OneOkM rs zr C (sIter n k S) →
      RunOkM rs zr C (k + 1) S n
  | 0, _, _, _, h => h
  | 1, _, _, h1, h => .inl ⟨h1, h⟩
  | k + 2, _, _, .inl ⟨h1, hr⟩, h => .inl ⟨h1, RunOkM.snoc (k + 1) _ _ hr h⟩
  | k + 2, _, _, .inr ⟨hf, hr⟩, h => .inr ⟨hf, RunOkM.snoc k _ _ hr h⟩

theorem RunOkM.snoc2 {rs : List PMA_Region} {zr : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop} :
    ∀ (k : Nat) (S : ZiskRiscv.State model) (n : Nat), RunOkM rs zr C k S n →
      (∃ pc i, (sIter n k S).rv.regs.get? Register.PC = some pc ∧ FtAt (sIter n k S).rv rs pc zr i) →
      RunOkM rs zr C (k + 2) S n
  | 0, _, _, _, h => .inr ⟨h, trivial⟩
  | 1, _, _, h1, h => .inl ⟨h1, .inr ⟨h, trivial⟩⟩
  | k + 2, _, _, .inl ⟨h1, hr⟩, h => .inl ⟨h1, RunOkM.snoc2 (k + 1) _ _ hr h⟩
  | k + 2, _, _, .inr ⟨hf, hr⟩, h => .inr ⟨hf, RunOkM.snoc2 k _ _ hr h⟩

theorem RunInM.snoc {C : BitVec 64 → Prop} {k : Nat} {S : ZiskRiscv.State model} {n : Nat}
    (h : RunInM C k S n) (hc : ∀ pc, (sIter n k S).rv.regs.get? Register.PC = some pc → C pc) :
    RunInM C (k + 1) S n := by
  intro i hi pc hpc
  rcases Nat.lt_succ_iff_lt_or_eq.1 hi with hi | rfl
  · exact h i hi pc hpc
  · exact hc pc hpc

end Equiv.Sim

namespace Equiv

theorem memRead_memWrite8 (m : Mem) (a : Nat) (v : BitVec 64) :
    memRead (memWrite (n := 8) m a v) 8 a = some v := by
  simp only [memWrite, byteList, List.ofFn_succ, List.ofFn_zero, List.foldl_cons, List.foldl_nil, memRead,
    Fin.val_succ, Fin.val_zero]
  simp [Nat.add_assoc, Std.ExtHashMap.get?_eq_getElem?, Std.ExtHashMap.getElem_insert]
  bv_decide

end Equiv

namespace Equiv.Top

open Equiv.Riscv Equiv.Sim Equiv.Machine
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- `covered_run`, and memory changed only in the RAM and the input's
    first 8 bytes since the start. -/
theorem covered_run_ram (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (rs : List PMA_Region) (hx : ¬ ExecAt rs EXIT_ADDR) (ht : ¬ ExecAt rs TRAP_VEC) (C : BitVec 64 → Prop)
    (hnx : NoCodeRam C) (hni : NoCodeIn C)
    (m0 : Mem) (hspec : SpecRom zrom C m0) (s0 : RVState) (hs0 : Start s0 rs entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (hzf : Equiv.Zisk.FcallRel fc z0)
    (hmir : Equiv.memRead m0 8 Equiv.IN_LO = some fc.freeInput) (zr : BitVec 64 → _root_.Zisk.zisk_inst) (k : Nat)
    (hrun : RunOkM rs zr C k ⟨s0, fc⟩ 0) (hin : RunInM C k ⟨s0, fc⟩ 0) :
    ∃ z', Rel ((sIter 0 k ⟨s0, fc⟩).rv) z' rs ∧
      SpecRom zrom C ((sIter 0 k ⟨s0, fc⟩).rv).mem ∧ Grows s0.mem ((sIter 0 k ⟨s0, fc⟩).rv).mem ∧
      RamOnlyX s0.mem ((sIter 0 k ⟨s0, fc⟩).rv).mem := by
  obtain ⟨st, hst⟩ := hzst
  obtain ⟨z1, r1, hrel1, hk1⟩ := start_rel zrom entry hb m0 z0 st hzx hzpc hst hze hzm s0 rs hs0
  have hs : SpecRom zrom C s0.mem := by
    rw [hs0.mem]
    exact hspec.keep ((memWrite_ramOnly _ _ _ arch_writable.1).trans (memWrite_ramOnly _ _ _ mtvec_writable.1)) hnx
  have hmir0 : Mirror ⟨s0, fc⟩ := by
    show Equiv.memRead s0.mem 8 Equiv.IN_LO = some fc.freeInput
    rw [hs0.mem, startMem, mirror_ramOnly ((memWrite_ramOnly _ _ _ arch_writable.1).trans
      (memWrite_ramOnly _ _ _ mtvec_writable.1))]
    exact hmir
  obtain ⟨z', k', hzr, hrel, hfr, hro, hg, -, hm⟩ := prog_romM zr zrom rs C hx ht hnx hni k ⟨s0, fc⟩ z1 0 hrel1
    (fun r hr => (hk1 r hr).trans (hzf r hr)) hmir0 hrun hin hs
  exact ⟨z', hrel, hs.keepX hro hnx hni, hg, hro⟩

end Equiv.Top

namespace Equiv.Final

open Equiv.Riscv Equiv.Sim Equiv.Machine Equiv.Top
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-! ## The code -/

/-- The program's code addresses: its code words'. -/
def CodeAt (segs : List (BitVec 64 × Array (BitVec 32))) (pc : BitVec 64) : Prop :=
  ∃ x ∈ Transpiler.codeWords segs, x.1 = pc

/-- A code word whose translation reads the word after it, or the one after
    that, has them in its segment (`tailOk`): code addresses too. -/
theorem code_next {segs : List (BitVec 64 × Array (BitVec 32))} (ht : Transpiler.tailOk segs = true)
    {x : BitVec 64 × BitVec 32 × Option (BitVec 32) × Option (BitVec 32)} (hx : x ∈ Transpiler.codeWords segs) :
    (Transpiler.usesNext x.2.1 = true → CodeAt segs (x.1 + 4)) ∧
      (Transpiler.usesNext2 x.2.1 = true → CodeAt segs (x.1 + 8)) := by
  simp only [Transpiler.codeWords, List.mem_flatMap, List.mem_map, List.mem_range] at hx
  obtain ⟨⟨base, ws⟩, hseg, k, hk, rfl⟩ := hx
  have hall := List.all_eq_true.1 ht _ hseg
  simp only [List.all_eq_true, List.mem_range, Bool.or_eq_true, decide_eq_true_eq, Bool.and_eq_true,
    Bool.not_eq_true'] at hall
  have hk' := hall k hk
  have mk : ∀ j, j < ws.size → CodeAt segs (base + BitVec.ofNat 64 (4 * j)) := fun j hj =>
    ⟨_, List.mem_flatMap.2 ⟨_, hseg, List.mem_map.2 ⟨j, List.mem_range.2 hj, rfl⟩⟩, rfl⟩
  have e : ∀ d : Nat, base + BitVec.ofNat 64 (4 * k) + BitVec.ofNat 64 d = base + BitVec.ofNat 64 (4 * k + d) := by
    intro d; apply BitVec.eq_of_toNat_eq; simp [BitVec.toNat_add]; omega
  refine ⟨fun hu => ?_, fun hu => ?_⟩
  · have : k + 1 < ws.size := by
      rcases hk' with h | ⟨-, h | h⟩
      · omega
      · exact h
      · simp [hu] at h
    have := mk (k + 1) this
    rw [show 4 * (k + 1) = 4 * k + 4 by omega, ← e 4] at this; exact this
  · have : k + 2 < ws.size := by
      rcases hk' with h | ⟨h, -⟩
      · exact h
      · simp [hu] at h
    have := mk (k + 2) this
    rw [show 4 * (k + 2) = 4 * k + 8 by omega, ← e 8] at this; exact this

/-- A code word, through a run that changed memory only in the RAM and the
    input's first 8 bytes since the start: the word the program has. -/
theorem code_word {C : BitVec 64 → Prop} (hnx : NoCodeRam C) (hni : NoCodeIn C) {m0 m : Mem} {s0 : RVState}
    (hs0 : s0.mem = startMem m0) (hro : RamOnlyX s0.mem m) {pc : BitVec 64} (hc : C pc) :
    ZiskRiscv.readWord m pc.toNat = ZiskRiscv.readWord m0 pc.toNat := by
  have hst : RamOnly m0 (startMem m0) :=
    (memWrite_ramOnly _ _ _ arch_writable.1).trans (memWrite_ramOnly _ _ _ mtvec_writable.1)
  refine readWord_congr fun i hi => ?_
  have h1 := (hnx pc hc).1 i hi
  rw [hro _ h1 ((hni pc hc).1 i hi), hs0]
  exact hst _ h1

/-- marchid's cell at the start: the ID, which the BIOS writes. -/
theorem start_marchid (m0 : Mem) : Equiv.memRead (startMem m0) 8 MARCHID_CELL.toNat = some ARCH_ID := by
  have e : MARCHID_CELL.toNat = Transpiler.ARCH_ID_CSR_ADDR.toNat := by decide
  rw [e, startMem, memRead_frame _ _ 8 _ (fun k h1 h2 => memWrite_get_off _ _ _ _ (by
    simp only [Transpiler.ARCH_ID_CSR_ADDR, Transpiler.MTVEC_ADDR] at h1 h2 ⊢; simp at h1 h2 ⊢; omega)),
    memRead_memWrite8]
  rfl

theorem auipc_decode (i : FInstr) : Transpiler.decode i.auipc.encode = some .auipc := by
  have ho : (Transpiler.opcode i.auipc.encode).toNat = 23 := by
    have : Transpiler.opcode i.auipc.encode = 23#7 := by
      simp only [Transpiler.opcode, FInstr.auipc, UInstr.encode, uopEnc]; bv_decide
    rw [this]; rfl
  simp only [Transpiler.decode, ho]

/-! ## The run -/

/-- The run's first k steps covered, at code addresses, marchid's cell
    holding the ID where they end. -/
def Covered (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (C : BitVec 64 → Prop) (S0 : ZiskRiscv.State model)
    (k : Nat) : Prop :=
  RunOkM ziskRegions zrom C k S0 0 ∧ RunInM C k S0 0 ∧
    Equiv.memRead (sIter 0 k S0).rv.mem 8 MARCHID_CELL.toNat = some ARCH_ID

/-- C, on memory, as a run breaks it: a step, covered, that changes
    marchid's cell (contract § 6: the program stores nothing there). -/
def BreakCell (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (C : BitVec 64 → Prop) (n : Nat)
    (S : ZiskRiscv.State model) : Prop :=
  OneOkM ziskRegions zrom C S ∧ Equiv.memRead (sNext n S).rv.mem 8 MARCHID_CELL.toNat ≠ some ARCH_ID

/-- Where a run stops, at step n: the return to the initial ra, one of the
    ends, or a precondition broken (J, one at the word at pc, I by a ZisK
    call, or marchid's cell changed). -/
def StopAt (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (C : BitVec 64 → Prop) (n : Nat)
    (S : ZiskRiscv.State model) : Prop :=
  S.rv.regs.get? Register.PC = some Transpiler.OUTPUT_BLOCK ∨ EndAt zrom C S ∨ BreakJ C S.rv ∨
  (∃ pc w, S.rv.regs.get? Register.PC = some pc ∧ ZiskRiscv.readWord S.rv.mem pc.toNat = some w ∧
    Broken S.rv pc w) ∨
  BreakIC S ∨ BreakCell zrom C n S

section
variable (entry : BitVec 64) (segs : List (BitVec 64 × Array (BitVec 32))) (m0 : Mem)
  (hp : ProgramOk segs (CodeAt segs) m0) (ht : Transpiler.tailOk segs = true)
  (hcx : ∀ pc, CodeAt segs pc → ExecAt ziskRegions pc) (hfull : Full m0)
  (s0 : RVState) (hs0 : Start s0 ziskRegions entry m0)
  (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
  (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
  (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
  (fc : ZiskRiscv.Fcall) (hzf : Equiv.Zisk.FcallRel fc z0)
  (hmir : Equiv.memRead m0 8 Equiv.IN_LO = some fc.freeInput)
include hp ht hcx hfull hs0 hzx hzpc hzst hze hzm hzf hmir

set_option maxHeartbeats 4000000 in
/-- From a covered run, one step more: covered further, or stopped. -/
theorem cover_step (k : Nat) (hcov : Covered (zromOf segs entry) (CodeAt segs) ⟨s0, fc⟩ k) :
    (∃ k', k < k' ∧ Covered (zromOf segs entry) (CodeAt segs) ⟨s0, fc⟩ k') ∨
      ∃ j, Covered (zromOf segs entry) (CodeAt segs) ⟨s0, fc⟩ j ∧
        StopAt (zromOf segs entry) (CodeAt segs) j (sIter 0 j ⟨s0, fc⟩) := by
  obtain ⟨hrun, hin, hcell⟩ := hcov
  obtain ⟨hb, hspec, hhalt⟩ := hp.rom entry
  have hnx := noCodeRam_of_exec hcx
  have hni := noCodeIn_of_exec hcx
  obtain ⟨z', hrel, hsp, hg, hro⟩ := covered_run_ram (zromOf segs entry) entry hb ziskRegions exit_not_exec
    trap_not_exec (CodeAt segs) hnx hni m0 hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf hmir _ k hrun hin
  generalize hS : sIter 0 k ⟨s0, fc⟩ = S at hrel hsp hg hro hcell
  obtain ⟨pc, hpc, -⟩ := hrel.pc
  have stop : StopAt (zromOf segs entry) (CodeAt segs) k S →
      (∃ k', k < k' ∧ Covered (zromOf segs entry) (CodeAt segs) ⟨s0, fc⟩ k') ∨
      ∃ j, Covered (zromOf segs entry) (CodeAt segs) ⟨s0, fc⟩ j ∧
        StopAt (zromOf segs entry) (CodeAt segs) j (sIter 0 j ⟨s0, fc⟩) :=
    fun h => .inr ⟨k, ⟨hrun, hin, hS ▸ hcell⟩, hS ▸ h⟩
  by_cases hC : CodeAt segs pc
  rotate_left
  · by_cases hr : pc = Transpiler.OUTPUT_BLOCK
    · subst hr; exact stop (.inl hpc)
    · exact stop (.inr (.inr (.inl ⟨pc, hpc, hC, hr⟩)))
  have hcw : ∀ p, CodeAt segs p → ZiskRiscv.readWord S.rv.mem p.toNat = ZiskRiscv.readWord m0 p.toNat :=
    fun p hp' => code_word hnx hni hs0.mem hro hp'
  have hcode : ∀ p, CodeAt segs p → ∃ w, ZiskRiscv.readWord S.rv.mem p.toNat = some w := by
    intro p hp'
    obtain ⟨x, hx, rfl⟩ := hp'
    exact ⟨_, (hcw _ ⟨x, hx, rfl⟩).trans (hp.words x hx)⟩
  have hfull0 : Full s0.mem := by
    rw [hs0.mem, startMem]; exact hfull.grow ((memWrite_grows' _ _ _).trans (memWrite_grows' _ _ _))
  have hword : ∀ w, ZiskRiscv.readWord S.rv.mem pc.toNat = some w →
      ∃ x ∈ Transpiler.codeWords segs, x.1 = pc ∧ x.2.1 = w := by
    intro w hw
    obtain ⟨x, hx, rfl⟩ := hC
    rw [hcw _ ⟨x, hx, rfl⟩, hp.words x hx] at hw
    exact ⟨x, hx, rfl, Option.some.inj hw⟩
  have hctx : ∀ w, CodeAt segs pc → ZiskRiscv.readWord S.rv.mem pc.toNat = some w →
      Ctx (zromOf segs entry) (CodeAt segs) S pc w := by
    intro w hc hw
    obtain ⟨x, hx, hx1, hx2⟩ := hword w hw
    exact {
      atPc := hpc, code := hc,
      aligned := fun p ⟨y, hy, e⟩ => e ▸ hp.ok.aligned y hy,
      word := hw,
      next1 := fun hu => by rw [← hx1]; rw [← hx2] at hu; exact (code_next ht hx).1 hu,
      next2 := fun hu => by rw [← hx1]; rw [← hx2] at hu; exact (code_next ht hx).2 hu,
      exec := hcx, ready := hrel.ready, spec := hsp, full := hfull0.grow hg, halt := hhalt,
      marchid := hcell }
  have hsk : ∀ j, sIter 0 (j + 1) ⟨s0, fc⟩ = sNext j (sIter 0 j ⟨s0, fc⟩) := fun j => by
    rw [sIter_succ]; simp
  have hinS : RunInM (CodeAt segs) (k + 1) ⟨s0, fc⟩ 0 :=
    RunInM.snoc hin (fun p hp' => by rw [hS, hpc] at hp'; cases hp'; exact hC)
  rcases step_case (kind_cases _ _) S pc hpc hcode hctx with h | ⟨pc', i, hpc', hft⟩ | h | h | h | h | h
  · by_cases hcell' : Equiv.memRead (sNext k S).rv.mem 8 MARCHID_CELL.toNat = some ARCH_ID
    · exact .inl ⟨k + 1, by omega, RunOkM.snoc k _ 0 hrun (hS ▸ h), hinS, by rw [hsk, hS]; exact hcell'⟩
    · exact stop (.inr (.inr (.inr (.inr (.inr ⟨h, hcell'⟩)))))
  · rw [hpc] at hpc'; cases hpc'
    obtain ⟨s2, z1, hrunF, -, -, -, hr1, hpc1, hmem1, hmem2, -⟩ :=
      fused_rel S.rv z' ziskRegions hrel pc hpc _ i hft.toFAt k
    obtain ⟨t1, h1, -, m1, m2, -, -⟩ := fused_mnext hpc hft hrunF hpc1 hmem1
    have q1 : sNext k S = { S with rv := t1 } := m1.snext_eq
    have q2 : sNext (k + 1) { S with rv := t1 } = { S with rv := s2 } := m2.snext_eq (S := { S with rv := t1 })
    rw [rvNext_of h1] at hpc1
    have hw : ZiskRiscv.readWord S.rv.mem pc.toNat = some i.auipc.encode := readWord_of hft.h0 hft.h1 hft.h2 hft.h3
    have hc4 : CodeAt segs (pc + 4) :=
      (hctx _ hC hw).next1 (by simp [Transpiler.usesNext, auipc_decode])
    have e1 : sIter 0 (k + 1) ⟨s0, fc⟩ = { S with rv := t1 } := by rw [hsk, hS, q1]
    have e2 : sIter 0 (k + 2) ⟨s0, fc⟩ = { S with rv := s2 } := by rw [hsk, e1, q2]
    refine .inl ⟨k + 2, by omega, RunOkM.snoc2 k _ 0 hrun ⟨pc, i, hS ▸ hpc, hS ▸ hft⟩,
      RunInM.snoc hinS (fun p hp' => by rw [e1] at hp'; rw [hpc1] at hp'; cases hp'; exact hc4), ?_⟩
    rw [e2]; show Equiv.memRead s2.mem 8 _ = _; rw [hmem2]; exact hcell
  · exact stop (.inl h)
  · exact stop (.inr (.inl h))
  · exact stop (.inr (.inr (.inl h)))
  · exact stop (.inr (.inr (.inr (.inl h))))
  · exact stop (.inr (.inr (.inr (.inr (.inl h)))))

/-- Totality: from the start, the run is covered as far as one likes — or
    it stops, at a step after which its steps were covered, at one of the
    ends or at a precondition broken. -/
theorem run_cases :
    ∀ N, (∃ k, N ≤ k ∧ Covered (zromOf segs entry) (CodeAt segs) ⟨s0, fc⟩ k) ∨
      ∃ j, Covered (zromOf segs entry) (CodeAt segs) ⟨s0, fc⟩ j ∧
        StopAt (zromOf segs entry) (CodeAt segs) j (sIter 0 j ⟨s0, fc⟩)
  | 0 => .inl ⟨0, Nat.le_refl _, trivial, fun _ hi => absurd hi (Nat.not_lt_zero _), by
      show Equiv.memRead s0.mem 8 _ = _; rw [hs0.mem]; exact start_marchid m0⟩
  | N + 1 => by
    rcases run_cases N with ⟨k, hk, hcov⟩ | hstop
    · by_cases hlt : N + 1 ≤ k
      · exact .inl ⟨k, hlt, hcov⟩
      · rcases cover_step entry segs m0 hp ht hcx hfull s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf hmir k hcov with
          ⟨k', hk', hc'⟩ | h
        · exact .inl ⟨k', by omega, hc'⟩
        · exact .inr h
    · exact .inr hstop

end

end Equiv.Final
