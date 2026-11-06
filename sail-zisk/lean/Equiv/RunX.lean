/-
  Whole runs on ZisK's memory without the execute-only bytes (precondition
  X): the BIOS's start, then the covered run (`covered_runX`), ZisK's run
  there its run on the whole memory, or a step of the machine's run breaks X.
-/
import Equiv.StepX
import Equiv.TotalityMain

namespace Equiv.Final

open Equiv.Riscv Equiv.Sim Equiv.Machine Equiv.Top Equiv.Sparse Equiv.Zisk
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- ZisK's runs are deterministic. -/
theorem zrun_det {zrom : BitVec 64 → _root_.Zisk.zisk_inst} :
    ∀ {k : Nat} {z a b : ZState}, ZRun zrom k z a → ZRun zrom k z b → a = b
  | _, _, _, _, .done _, .done _ => rfl
  | _, _, _, _, .step h1 r1, .step h2 r2 => by
    rw [h1] at h2; cases h2; exact zrun_det r1 r2

/-- A run on σ, τ's memory without X's bytes, is one on τ. -/
theorem zrun_sup {X : Nat → Bool} (hX : ∀ k, X k = true → OffRam k) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} :
    ∀ {k : Nat} {σ τ σ' : ZState}, FRX X σ τ → ZRun zrom k σ σ' → ∃ τ', ZRun zrom k τ τ' ∧ FRX X σ' τ'
  | _, _, τ, _, h, .done _ => ⟨τ, .done _, h⟩
  | _, σ, τ, _, h, .step hs hr => by
    obtain ⟨τ1, ht, hf, hk, hro⟩ := (sframe_zstep zrom).1 σ τ h.fra _ _ hs
    obtain ⟨τ', hr', h'⟩ := zrun_sup hX (h.step hX hf hk hro) hr
    exact ⟨τ', .step ht hr', h'⟩

/-- ZisK's start on m0 without X's bytes, else as on m0. -/
theorem start_x {X : Nat → Bool} (hX : RomX X) (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (entry : BitVec 64)
    (hb : BiosAt zrom entry) {z0 σ0 z1 : ZState} (h : FRX X σ0 z0)
    (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0)) (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY)
    {st : BitVec 64} (hst : z0.regs.get? .Step = some st) (hze : z0.regs.get? .Zerror = some false)
    (r1 : ZRun zrom 6 z0 z1) : ∃ σ1, ZRun zrom 6 σ0 σ1 ∧ FRX X σ1 z1 := by
  obtain ⟨σ1, hr, -⟩ := bios_start zrom entry hb σ0 _ st (by rw [h.regs]; exact hzx) (by rw [h.regs]; exact hzpc)
    (by rw [h.regs]; exact hst) (by rw [h.regs]; exact hze)
  obtain ⟨τ1, hτ, h1⟩ := zrun_sup hX.offRam h hr
  rw [zrun_det hτ r1] at h1
  exact ⟨σ1, hr, h1⟩

/-- `covered_run`, ZisK on m0 without X's bytes alongside: its run there is
    its run on m0, or a step of the machine's run breaks X. -/
theorem covered_runX (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (rs : List PMA_Region) (hx : ¬ ExecAt rs EXIT_ADDR) (ht : ¬ ExecAt rs TRAP_VEC) (C : BitVec 64 → Prop)
    (hnx : NoCodeRam C) (hni : NoCodeIn C)
    (m0 : Mem) (hspec : SpecRom zrom C m0) (s0 : RVState) (hs0 : Start s0 rs entry m0)
    (z0 : ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (hzf : Equiv.Zisk.FcallRel fc z0)
    (hmir : Equiv.memRead m0 8 Equiv.IN_LO = some fc.freeInput) (zr : BitVec 64 → _root_.Zisk.zisk_inst) (k : Nat)
    (hrun : RunOkM rs zr C k ⟨s0, fc⟩ 0) (hin : RunInM C k ⟨s0, fc⟩ 0) {X : Nat → Bool} (hX : RomX X) :
    ∃ z' n, ZRun zrom n z0 z' ∧ Rel ((sIter 0 k ⟨s0, fc⟩).rv) z' rs ∧
      Equiv.Zisk.FcallRel (sIter 0 k ⟨s0, fc⟩).fcall z' ∧
      SpecRom zrom C ((sIter 0 k ⟨s0, fc⟩).rv).mem ∧ Grows s0.mem ((sIter 0 k ⟨s0, fc⟩).rv).mem ∧
      (PcOk ((sIter 0 k ⟨s0, fc⟩).rv) → MRunR C k ⟨s0, fc⟩ (sIter 0 k ⟨s0, fc⟩)) ∧
      ∀ σ0, FRX X σ0 z0 → (∃ σ', ZRun zrom n σ0 σ' ∧ FRX X σ' z') ∨
        ∃ j, j < k ∧ RunOkM rs zr C j ⟨s0, fc⟩ 0 ∧ BreakXS X (sIter 0 j ⟨s0, fc⟩) := by
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
  obtain ⟨z', k', hzr, hrel, hfr, hro, hg, -, hm, hσ⟩ := prog_romMX zr zrom rs C hx ht hnx hni hX k ⟨s0, fc⟩ z1 0
    hrel1 (fun r hr => (hk1 r hr).trans (hzf r hr)) hmir0 hrun hin hs
  refine ⟨z', 6 + k', ZRun.append r1 hzr, hrel, hfr, hs.keepX hro hnx hni, hg, hm, fun σ0 h0 => ?_⟩
  obtain ⟨σ1, hr1, h1⟩ := start_x hX zrom entry hb h0 hzx hzpc hst hze r1
  rcases hσ σ1 h1 with ⟨σ', hr', h'⟩ | hb'
  · exact .inl ⟨σ', ZRun.append hr1 hr', h'⟩
  · exact .inr hb'

/-- Off the ROM, σ's bytes are τ's. -/
theorem memRead_frx {X : Nat → Bool} (hX : RomX X) {σ τ : ZState} (h : FRX X σ τ) (n a : Nat)
    (ha : a + n ≤ 0x9000_0000 ∨ 0x9800_0000 ≤ a) : Equiv.memRead σ.mem n a = Equiv.memRead τ.mem n a := by
  refine memRead_frame τ.mem σ.mem n a (fun k h1 h2 => ?_)
  rw [h.mem k]
  cases e : X k
  · rfl
  · have := hX k e; omega

theorem OutBytes.of_frx {X : Nat → Bool} (hX : RomX X) {σ τ : ZState} (h : FRX X σ τ) (ho : OutBytes τ.mem) :
    OutBytes σ.mem := fun j hj => by
  obtain ⟨v, hv⟩ := ho j hj
  have : (Transpiler.OUTPUT_ADDR + BitVec.ofNat 64 (8 * j)).toNat = 0xA041_0000 + 8 * j := by
    simp [Transpiler.OUTPUT_ADDR, BitVec.toNat_add]; omega
  exact ⟨v, by rw [memRead_frx hX h _ _ (by rw [this]; omega)]; exact hv⟩

theorem InBytes.of_frx {X : Nat → Bool} (hX : RomX X) {σ τ : ZState} (h : FRX X σ τ) (hi : InBytes τ.mem) :
    InBytes σ.mem := fun k hk => by
  obtain ⟨v, hv⟩ := hi k hk
  have hlo : (inAddr k).toNat + 8 ≤ 0x9000_0000 := by
    have hk1 : (BitVec.ofNat 64 (k + 1)).toNat = k + 1 := by rw [BitVec.toNat_ofNat]; omega
    simp only [inAddr]
    generalize BitVec.ofNat 64 (k + 1) = K at hk1
    have b2 : K ≤ 15#64 := by rw [BitVec.le_def]; simp; omega
    have : Transpiler.INPUT_ADDR + K * Transpiler.INPUT_ANCHOR_STEP ≤ 0x88000000#64 := by
      simp only [Transpiler.INPUT_ADDR, Transpiler.INPUT_ANCHOR_STEP]; bv_decide
    rw [BitVec.le_def] at this
    have e : (0x88000000#64).toNat = 0x88000000 := rfl
    omega
  exact ⟨v, by rw [memRead_frx hX h _ _ (.inl hlo)]; exact hv⟩

/-- normIn of σ's memory: τ's without X's bytes. -/
theorem normIn_frx {X : Nat → Bool} {σ τ : ZState} (h : FRX X σ τ) {m : Mem} (hm : Equiv.normIn τ.mem = Equiv.normIn m) :
    Equiv.normIn σ.mem = eraseX X (Equiv.normIn m) := by
  apply Std.ExtHashMap.ext_getElem?; intro k
  have e1 := Equiv.normIn_get σ.mem k
  have e2 := eraseX_get X (Equiv.normIn m) k
  have e3 := Equiv.normIn_get m k
  have e4 := Equiv.normIn_get τ.mem k
  rw [hm, e3] at e4
  have e5 := h.mem k
  simp only [Std.ExtHashMap.get?_eq_getElem?] at e1 e2 e3 e4 e5 ⊢
  rw [e1, e2, e3, e5]
  by_cases hi : IN_LO ≤ k ∧ k < IN_LO + 8
  · simp [hi]
  · simp only [hi, if_false] at e4 ⊢
    cases X k <;> simp [e4]

/-- sail-zisk's run that fails from τ is one from σ. -/
theorem ziskFails_x {X : Nat → Bool} (hX : RomX X) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {σ τ : ZState}
    (h : FRX X σ τ) (hf : ZiskFails zrom τ) : ZiskFails zrom σ := by
  rcases hf with ⟨τ', he, hz⟩ | ⟨n, τ1, e, τ', hr, hs⟩
  · rcases zends_sub hX.offRam h he with ⟨σ', he', h'⟩ | ⟨j, σj, k, σ', hj, hs', -⟩
    · exact .inl ⟨σ', he', by rw [h'.regs]; exact hz⟩
    · exact .inr ⟨j, σj, _, σ', hj, hs'⟩
  · obtain ⟨n', σ1, e', σ', hr', hs'⟩ := zfail_sub hX.offRam h hr hs
    exact .inr ⟨n', σ1, e', σ', hr', hs'⟩

/-- `rel_return_end`, from ZisK's memory without X's bytes. -/
theorem rel_return_end_x {X : Nat → Bool} (hX : RomX X) (zrom : BitVec 64 → _root_.Zisk.zisk_inst)
    (entry : BitVec 64) (hb : BiosAt zrom entry) {s : RVState} {z σ : ZState} {rs : List PMA_Region}
    (hrel : Rel s z rs) (h : FRX X σ z) (hpc : s.regs.get? Register.PC = some Transpiler.OUTPUT_BLOCK)
    (ho : OutBytes s.mem) (hi : InBytes s.mem) :
    ∃ σ', ZEnds zrom σ σ' ∧ σ'.regs.get? .Zerror = some false ∧
      Equiv.normIn σ'.mem = eraseX X (Equiv.normIn s.mem) := by
  obtain ⟨-, ⟨xs, hx, -⟩, ⟨pc, hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  rw [hpc] at hpc'; cases hpc'
  obtain ⟨σ', h1, h2, h3⟩ := output_end zrom entry hb σ.mem (OutBytes.of_frx hX h (ho.of_normIn hzm))
    (InBytes.of_frx hX h (hi.of_normIn hzm)) σ xs (by rw [h.regs]; exact hzpc) (by rw [h.regs]; exact hx)
    ⟨st, by rw [h.regs]; exact hst⟩ (by rw [h.regs]; exact hze) rfl
  exact ⟨σ', h1, h2, by rw [h3]; exact normIn_frx h hzm⟩

/-- `rel_exit_ok`, from ZisK's memory without X's bytes. -/
theorem rel_exit_ok_x {X : Nat → Bool} (hX : RomX X) (zrom : BitVec 64 → _root_.Zisk.zisk_inst)
    (entry : BitVec 64) (hb : BiosAt zrom entry) {s : RVState} {z σ : ZState} {rs : List PMA_Region}
    (hrel : Rel s z rs) (h : FRX X σ z) (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (hi : zrom pc = Transpiler.ecall) (h17 : getX s 17 = some 93) (h10 : getX s 10 = some 0)
    (ho : OutBytes s.mem) (hin : InBytes s.mem) :
    ∃ σ', ZEnds zrom σ σ' ∧ σ'.regs.get? .Zerror = some false ∧
      Equiv.normIn σ'.mem = eraseX X (Equiv.normIn s.mem) := by
  obtain ⟨-, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  rw [hpc] at hpc'; cases hpc'
  have a17 := agree 17 (by omega) (by omega)
  have a10 := agree 10 (by omega) (by omega)
  rw [h17] at a17; rw [h10] at a10
  obtain ⟨σ', h1, h2, h3⟩ := ecall_exit_ok zrom entry hb σ.mem (OutBytes.of_frx hX h (ho.of_normIn hzm))
    (InBytes.of_frx hX h (hin.of_normIn hzm)) σ pc (by rw [h.regs]; exact hzpc) hi xs (by rw [h.regs]; exact hx)
    (Option.some.inj a17).symm (Option.some.inj a10).symm ⟨st, by rw [h.regs]; exact hst⟩
    (by rw [h.regs]; exact hze) rfl
  exact ⟨σ', h1, h2, by rw [h3]; exact normIn_frx h hzm⟩

/-- `rel_exit_fail`, from ZisK's memory without X's bytes. -/
theorem rel_exit_fail_x {X : Nat → Bool} (zrom : BitVec 64 → _root_.Zisk.zisk_inst)
    (entry : BitVec 64) (hb : BiosAt zrom entry) {s : RVState} {z σ : ZState} {rs : List PMA_Region}
    (hrel : Rel s z rs) (h : FRX X σ z) (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (hi : zrom pc = Transpiler.ecall) (h17 : getX s 17 = some 93) (c : BitVec 64) (h10 : getX s 10 = some c)
    (hc : c ≠ 0) :
    ∃ σ', ZEnds zrom σ σ' ∧ σ'.regs.get? .Zerror = some true ∧ σ'.regs.get? .Zerror_code = some c := by
  obtain ⟨-, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, -⟩ := hrel
  rw [hpc] at hpc'; cases hpc'
  have a17 := agree 17 (by omega) (by omega)
  have a10 := agree 10 (by omega) (by omega)
  rw [h17] at a17; rw [h10] at a10
  have e10 : xs[10]! = c := (Option.some.inj a10).symm
  obtain ⟨σ', he, h1, h2, -⟩ := ecall_exit_fail zrom entry hb σ pc (by rw [h.regs]; exact hzpc) hi xs
    (by rw [h.regs]; exact hx) (Option.some.inj a17).symm (by rw [e10]; exact hc) ⟨st, by rw [h.regs]; exact hst⟩
    (by rw [h.regs]; exact hze)
  exact ⟨σ', he, h1, by rw [h2, e10]⟩

end Equiv.Final
