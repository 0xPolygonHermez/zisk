/-
  The machine's steps the simulation covers, on ZisK's memory without the
  execute-only bytes (precondition X): ZisK's steps there are its steps on
  the whole memory, or the RISC-V step breaks X.
-/
import Equiv.TotalityX
import Equiv.ZiskCall

namespace Equiv.Final

open Equiv.Riscv Equiv.Sim Equiv.Machine Equiv.Top Equiv.Sparse Equiv.Zisk
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- X's bytes are in the ROM. -/
def RomX (X : Nat → Bool) : Prop := ∀ k, X k = true → 0x9000_0000 ≤ k ∧ k < 0x9800_0000

theorem RomX.offRam {X : Nat → Bool} (h : RomX X) : ∀ k, X k = true → OffRam k := by
  intro k hk; have := h k hk; unfold OffRam; omega

/-- An a source `srcA r` reads register r. -/
theorem source_a_srcA (inst : _root_.Zisk.zisk_inst) (r : Nat) (hr : r < 32)
    (h1 : inst.a_src = (Transpiler.srcA r).1) (h2 : inst.a_imm = (Transpiler.srcA r).2) (σ : ZState)
    (xs : Vector (BitVec 64) 40) (hx : σ.regs.get? _root_.Zisk.Register.Xs = some xs) :
    (_root_.Zisk.Functions.source_a inst).run σ = .ok (regv xs r) σ := by
  unfold _root_.Zisk.Functions.source_a
  rw [h1]
  by_cases h0 : r = 0
  · subst h0
    simp only [Transpiler.srcA, if_true] at h2 ⊢
    rw [h2]; simp [regv]
  · simp only [Transpiler.srcA, h0, if_false] at h2 ⊢
    have e1 : r ≠ 0 ∧ (r : Int) < 40 := ⟨h0, by omega⟩
    simp (disch := omega) [_root_.Zisk.Functions.rX, Equiv.Zisk.reg_index, _root_.Zisk.assert,
      Sail.ConcurrencyInterfaceV1.PreSail.assert, *]
    rw [Equiv.Zisk.readReg_run σ _root_.Zisk.Register.Xs xs hx]
    simp only [Equiv.Zisk.run_pure', EStateM.Result.ok.injEq, and_true, regv, h0, if_false]
    rw [getElem!_pos xs ((r : Nat) : Int) (by simp; omega)]

theorem rs1_lt (w : BitVec 32) : Transpiler.rs1 w < 32 := by
  unfold Transpiler.rs1; omega

theorem srcA_ne_mem (r : Nat) : (Transpiler.srcA r).1 ≠ .A_MEM := by
  unfold Transpiler.srcA; split <;> nofun

/-- A step at pc on the instruction a word's translation reads its access
    through: as on the whole memory, or the access reads one of X's bytes. -/
theorem own_miss {X : Nat → Bool} (hX : RomX X) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {rs : List PMA_Region}
    {s : RVState} {z σ z' : ZState} (hrel : Rel s z rs) (h : FRX X σ z) {pc : BitVec 64}
    (hpc : s.regs.get? Register.PC = some pc) {w : BitVec 32} (hro : ReadsOwn w pc (pc, zrom pc)) {b : Bool}
    (ht : (zstep zrom).run z = .ok b z') :
    (∃ σ', (zstep zrom).run σ = .ok b σ' ∧ FRX X σ' z') ∨ BreakX X s w := by
  obtain ⟨-, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, -, -⟩ := hrel
  rw [hpc] at hpc'; cases hpc'
  obtain ⟨-, ha1, ha2, hbi, hmo, kd, hkd, hacc⟩ := hro
  rcases zstep_x hX.offRam h hzpc ht with hs | ⟨k, hk, ⟨hA, -⟩ | ⟨a, σ1, hsa, hb⟩ | hm⟩
  · exact .inl hs
  · exact absurd (ha1.symm.trans hA) (srcA_ne_mem _)
  · have hxσ : σ.regs.get? _root_.Zisk.Register.Xs = some xs := by rw [h.regs]; exact hx
    rw [source_a_srcA _ _ (rs1_lt w) ha1 ha2 σ xs hxσ] at hsa
    cases hsa
    rcases hb with ⟨hB, -⟩ | ⟨-, hk1, hk2⟩
    · exact absurd (hbi.symm.trans hB) (by nofun)
    · right
      have hreg : regOf s (Transpiler.rs1 w) = regv xs (Transpiler.rs1 w) := by
        simp [regOf, getX_regv agree (rs1_lt w)]
      refine ⟨kd, _, _, hacc s, hkd, k, hk, ?_, ?_⟩
      · rw [hreg]; exact hk1
      · rw [hreg]; omega
  · rw [hmo] at hm; cases hm

/-- The word at pc of an instruction the simulation covers in one step: in
    memory, its translation in the ZisK program, not a CSR instruction. -/
theorem StepOkT.wt {s : RVState} {rs : List PMA_Region} {pc : BitVec 64}
    {zrom : BitVec 64 → _root_.Zisk.zisk_inst} : ∀ o, StepOkT s rs pc zrom o →
      ∃ e, ZiskRiscv.readWord s.mem pc.toNat = some e ∧ Transpiled zrom pc e (nextWord s pc) ∧ NotMach e
  | .r i, hi => ⟨_, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, hi.tr, notSys_r i⟩
  | .i i, hi => ⟨_, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, hi.tr, notSys_i i⟩
  | .sh i, hi => ⟨_, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, hi.tr, notSys_sh i⟩
  | .u i, hi => ⟨_, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, hi.tr, notSys_u i⟩
  | .ld i, ⟨hi, _⟩ => ⟨_, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, hi.tr, notSys_ld i⟩
  | .lw i, ⟨hi, _⟩ => ⟨_, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, hi.tr, notSys_lw i⟩
  | .sd i, ⟨hi, _⟩ => ⟨_, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, hi.tr, notSys_sd i⟩
  | .sw i, ⟨hi, _⟩ => ⟨_, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, hi.tr, notSys_sw i⟩
  | .br i, hi => ⟨_, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, hi.tr, notSys_br i⟩
  | .j i, hi => ⟨_, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, hi.tr, notSys_j i⟩
  | .jr i, ⟨hi, _⟩ => ⟨_, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, hi.tr, notSys_jr i⟩
  | .rw i, hi => ⟨_, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, hi.tr, notSys_rw i⟩
  | .iw i, hi => ⟨_, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, hi.tr, notSys_iw i⟩
  | .siw i, hi => ⟨_, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, hi.tr, notSys_siw i⟩
  | .m i, hi => ⟨_, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, hi.tr, notSys_m i⟩
  | .fence i, hi => ⟨_, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, hi.tr, notSys_fence i⟩
  | .zba i, hi => ⟨_, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, hi.tr, notSys_zba i⟩
  | .slliuw i, hi => ⟨_, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, hi.tr, notSys_slliuw i⟩
  | .zbs i, hi => ⟨_, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, hi.tr, notSys_zbs i⟩
  | .zbsi i, hi => ⟨_, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, hi.tr, notSys_zbsi i⟩
  | .zbb i, hi => ⟨_, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, hi.tr, notSys_zbb i⟩
  | .zbbw i, hi => ⟨_, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, hi.tr, notSys_zbbw i⟩
  | .rori i, hi => ⟨_, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, hi.tr, notSys_rori i⟩
  | .roriw i, hi => ⟨_, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, hi.tr, notSys_roriw i⟩
  | .pack i, hi => ⟨_, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, hi.tr, notSys_pack i⟩
  | .packw i, hi => ⟨_, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, hi.tr, notSys_packw i⟩
  | .sext i, hi => ⟨_, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, hi.tr, notSys_sext i⟩
  | .zexth i, hi => ⟨_, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, hi.tr, notSys_zexth i⟩
  | .byte i, hi => ⟨_, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, hi.tr, notSys_byte i⟩
  | .count i, hi => ⟨_, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, hi.tr, notSys_count i⟩

/-- X broken at the step: by the word at pc. -/
def BreakXAt (X : Nat → Bool) (s : RVState) : Prop :=
  ∃ pc w, s.regs.get? Register.PC = some pc ∧ ZiskRiscv.readWord s.mem pc.toNat = some w ∧ BreakX X s w

/-- A ZisK step at pc on the head of the translation of the word there, not
    a CSR instruction: as on the whole memory, or the word breaks X. -/
theorem plain_x {X : Nat → Bool} (hX : RomX X) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {rs : List PMA_Region}
    {s : RVState} {z σ z' : ZState} (hrel : Rel s z rs) (h : FRX X σ z) {pc : BitVec 64}
    (hpc : s.regs.get? Register.PC = some pc) {e : BitVec 32} (hw : ZiskRiscv.readWord s.mem pc.toNat = some e)
    {next : Option (BitVec 32)} (htr : Transpiled zrom pc e next) (hd : Transpiler.decode e ≠ some .csr) {b : Bool}
    (ht : (zstep zrom).run z = .ok b z') :
    (∃ σ', (zstep zrom).run σ = .ok b σ' ∧ FRX X σ' z') ∨ BreakXAt X s := by
  obtain ⟨ia, l, hl, hz⟩ := htr
  obtain ⟨i, hi⟩ := convert_head hl hd
  have hzi : zrom pc = i := hz _ hi
  have hzpc : z.regs.get? _root_.Zisk.Register.PC = some pc := by
    obtain ⟨pc', hpc', hz'⟩ := hrel.pc; rw [hpc] at hpc'; cases hpc'; exact hz'
  rcases convert_reads hl hd _ hi with hn | hro
  · exact .inl (zstep_noreads hX.offRam h hzpc (hzi ▸ hn) ht)
  · rw [← hzi] at hro
    rcases own_miss hX hrel h hpc hro ht with hs | hb
    · exact .inl hs
    · exact .inr ⟨pc, e, hpc, hw, hb⟩

/-- Each instruction of the chain neither sets the pc from c nor ends, its
    offsets the step to the next address. -/
def ChainOk (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (a b : BitVec 64) : Prop :=
  (zrom a).set_pc = false ∧ (zrom a).is_end = false ∧ (zrom a).jmp_off1 = b - a ∧ (zrom a).jmp_off2 = b - a

/-- A ZisK block running a translation list from pc, its instructions in
    the ZisK program, chained, the first reading nothing or the access, the
    rest nothing: as on the whole memory, or the word at pc breaks X. -/
theorem list_x {X : Nat → Bool} (hX : RomX X) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {rs : List PMA_Region}
    {s : RVState} {z σ z' : ZState} (hrel : Rel s z rs) (h : FRX X σ z) {pc : BitVec 64}
    (hpc : s.regs.get? Register.PC = some pc) {e : BitVec 32} (hw : ZiskRiscv.readWord s.mem pc.toNat = some e)
    (rest : List (BitVec 64 × _root_.Zisk.zisk_inst)) (hzr : ∀ p ∈ rest, zrom p.1 = p.2)
    (hnr : ∀ p ∈ rest, NoReads p.2) (h0 : NoReads (zrom pc) ∨ ReadsOwn e pc (pc, zrom pc))
    (hc : ChainR (ChainOk zrom) pc (rest.map Prod.fst)) (ht : ZRun zrom (rest.length + 1) z z') :
    (∃ σ', ZRun zrom (rest.length + 1) σ σ' ∧ FRX X σ' z') ∨ BreakXAt X s := by
  have hzpc : z.regs.get? _root_.Zisk.Register.PC = some pc := by
    obtain ⟨pc', hpc', hz'⟩ := hrel.pc; rw [hpc] at hpc'; cases hpc'; exact hz'
  cases ht with
  | step ht1 hr =>
  rename_i z1
  have first : (∃ σ1, (zstep zrom).run σ = .ok false σ1 ∧ FRX X σ1 z1) ∨ BreakXAt X s := by
    rcases h0 with hn | hro
    · exact .inl (zstep_noreads hX.offRam h hzpc hn ht1)
    · rcases own_miss hX hrel h hpc hro ht1 with hs | hb
      · exact .inl hs
      · exact .inr ⟨pc, e, hpc, hw, hb⟩
  rcases first with ⟨σ1, hs1, h1⟩ | hb
  · cases rest with
    | nil => cases hr; exact .inl ⟨σ1, .step hs1 (.done _), h1⟩
    | cons p1 rest' =>
      obtain ⟨⟨hs, he, hj1, hj2⟩, hc'⟩ := hc
      obtain ⟨-, hp1⟩ := zstep_pc hzpc hs he hj1 hj2 ht1
      rw [show pc + (p1.1 - pc) = p1.1 by rw [BitVec.add_comm, BitVec.sub_add_cancel]] at hp1
      have hnq : ∀ q ∈ (p1.1 :: rest'.map Prod.fst).take (rest'.length + 1), NoReads (zrom q) := by
        intro q hq
        have hq' : q ∈ p1.1 :: rest'.map Prod.fst := List.mem_of_mem_take hq
        simp only [List.mem_cons, List.mem_map] at hq'
        rcases hq' with rfl | ⟨p', hp', rfl⟩
        · rw [hzr p1 (by simp)]; exact hnr p1 (by simp)
        · rw [hzr p' (by simp [hp'])]; exact hnr p' (by simp [hp'])
      obtain ⟨σ', hr', h'⟩ := zrun_chain hX.offRam (rest'.map Prod.fst) p1.1 (by simp) hp1 h1 hnq hc' hr
      exact .inl ⟨σ', .step hs1 hr', h'⟩
  · exact .inr hb

/-- The instruction at pc of the translation of the word there, not a CSR
    instruction: reading nothing, or the access. -/
theorem head_x {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64} {e : BitVec 32}
    {next : Option (BitVec 32)} (htr : Transpiled zrom pc e next) (hd : Transpiler.decode e ≠ some .csr) :
    NoReads (zrom pc) ∨ ReadsOwn e pc (pc, zrom pc) := by
  obtain ⟨ia, l, hl, hz⟩ := htr
  obtain ⟨i, hi⟩ := convert_head hl hd
  rw [hz _ hi]
  exact convert_reads hl hd _ hi

/-- A list of instructions chained: each but the last neither sets the pc
    from c nor ends, its offsets the step to the next one's address. -/
def ChainL : List (BitVec 64 × _root_.Zisk.zisk_inst) → Prop
  | [] => True
  | [_] => True
  | a :: b :: l => a.2.set_pc = false ∧ a.2.is_end = false ∧ a.2.jmp_off1 = b.1 - a.1 ∧
      a.2.jmp_off2 = b.1 - a.1 ∧ ChainL (b :: l)

theorem chainL_R {zrom : BitVec 64 → _root_.Zisk.zisk_inst} :
    ∀ (l : List (BitVec 64 × _root_.Zisk.zisk_inst)) (a : BitVec 64 × _root_.Zisk.zisk_inst),
    (∀ p ∈ a :: l, zrom p.1 = p.2) → ChainL (a :: l) → ChainR (ChainOk zrom) a.1 (l.map Prod.fst)
  | [], _, _, _ => trivial
  | b :: l, a, hz, ⟨h1, h2, h3, h4, hc⟩ => by
    have ha := hz a (by simp)
    refine ⟨⟨by rw [ha]; exact h1, by rw [ha]; exact h2, by rw [ha]; exact h3, by rw [ha]; exact h4⟩, ?_⟩
    exact chainL_R l b (fun p hp => hz p (List.mem_cons_of_mem _ hp)) hc

/-- A block running a whole translation list from pc: as on the whole
    memory, or the word at pc breaks X. -/
theorem block_x {X : Nat → Bool} (hX : RomX X) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {rs : List PMA_Region}
    {s : RVState} {z σ z' : ZState} (hrel : Rel s z rs) (h : FRX X σ z) {pc : BitVec 64}
    (hpc : s.regs.get? Register.PC = some pc) {e : BitVec 32} (hw : ZiskRiscv.readWord s.mem pc.toNat = some e)
    (i0 : _root_.Zisk.zisk_inst) (rest : List (BitVec 64 × _root_.Zisk.zisk_inst))
    (hz : ∀ p ∈ (pc, i0) :: rest, zrom p.1 = p.2) (hc : ChainL ((pc, i0) :: rest))
    (hnr : ∀ p ∈ rest, NoReads p.2) (h0 : NoReads (zrom pc) ∨ ReadsOwn e pc (pc, zrom pc)) {k : Nat}
    (hk : k = ((pc, i0) :: rest).length) (ht : ZRun zrom k z z') :
    (∃ σ', ZRun zrom k σ σ' ∧ FRX X σ' z') ∨ BreakXAt X s := by
  subst hk
  exact list_x hX hrel h hpc hw rest (fun p hp => hz p (List.mem_cons_of_mem _ hp)) hnr h0
    (chainL_R rest (pc, i0) hz hc) ht

theorem sc_shape (w : BitVec 32) (d : Bool) (pc ia : BitVec 64) :
    ∃ i0 rest, Transpiler.sc w d pc ia = (pc, i0) :: rest ∧ ChainL ((pc, i0) :: rest) ∧
      ∀ p ∈ rest, NoReads p.2 := by
  unfold Transpiler.sc
  split
  · exact ⟨_, [], rfl, trivial, by simp⟩
  · refine ⟨_, _, rfl, ⟨rfl, rfl, rfl, rfl, trivial⟩, ?_⟩
    intro p hp; simp only [List.mem_singleton] at hp; subst hp
    exact ⟨nofun, nofun, nofun, rfl⟩

theorem amo_shape (w : BitVec 32) (op : Transpiler.AmoOp) (d : Bool) (pc ia : BitVec 64) :
    ∃ i0 rest, Transpiler.amo w op d pc ia = (pc, i0) :: rest ∧ ChainL ((pc, i0) :: rest) ∧
      ∀ p ∈ rest, NoReads p.2 := by
  unfold Transpiler.amo
  cases op <;> dsimp only <;> (repeat' split) <;>
    refine ⟨_, _, rfl, ?_, ?_⟩ <;>
    simp only [ChainL, Transpiler.mk, true_and, and_true, List.mem_cons, List.mem_singleton,
      List.not_mem_nil, or_false, forall_eq_or_imp, forall_eq] <;>
    (try simp only [BitVec.sub_add_cancel, BitVec.add_sub_cancel])
  all_goals first
    | (constructor <;> rw [BitVec.add_comm, BitVec.add_sub_cancel])
    | (refine ⟨?_, ?_, ?_, ?_⟩ <;> (try simp only [Transpiler.srcA, Transpiler.srcB]) <;> (repeat' split) <;>
        first | (intro h; cases h) | rfl | (simp [zop_nomem]; done))
    | (refine ⟨⟨?_, ?_, ?_, ?_⟩, ⟨?_, ?_, ?_, ?_⟩⟩ <;> (try simp only [Transpiler.srcA, Transpiler.srcB]) <;>
        (repeat' split) <;> first | (intro h; cases h) | rfl | (simp [zop_nomem]; done))
    | (refine ⟨⟨?_, ?_, ?_, ?_⟩, ⟨?_, ?_, ?_, ?_⟩, ⟨?_, ?_, ?_, ?_⟩⟩ <;>
        (try simp only [Transpiler.srcA, Transpiler.srcB]) <;> (repeat' split) <;>
        first | (intro h; cases h) | rfl | (simp [zop_nomem]; done))

theorem not_csr_2f {w : BitVec 32} (h : w.extractLsb' 0 7 = 0x2F#7) : Transpiler.decode w ≠ some .csr := by
  intro hc; have := decode_csr_opcode hc; rw [h] at this; cases this

/-- A ZisK step at pc on the `marchid` read: its cell is in the RAM, so as
    on the whole memory. -/
theorem marchid_x {X : Nat → Bool} (hX : RomX X) {zrom : BitVec 64 → _root_.Zisk.zisk_inst}
    {z σ z' : ZState} (h : FRX X σ z) {pc : BitVec 64} (hzpc : z.regs.get? _root_.Zisk.Register.PC = some pc)
    {rd : Nat} (hz : zrom pc = Transpiler.mk (.A_IMM, 0) (.B_MEM, MARCHID_CELL) (.ZOP_COPYB ())
      (Transpiler.storeReg rd) 4 4) {b : Bool} (ht : (zstep zrom).run z = .ok b z') :
    ∃ σ', (zstep zrom).run σ = .ok b σ' ∧ FRX X σ' z' := by
  rcases zstep_x hX.offRam h hzpc ht with hs | ⟨k, hk, ⟨hA, -⟩ | ⟨a, σ1, -, hb⟩ | hm⟩
  · exact hs
  · rw [hz] at hA; cases hA
  · have hr := hX k hk
    rcases hb with ⟨-, h1, h2⟩ | ⟨hB, -⟩
    · rw [hz] at h1 h2
      simp [Transpiler.mk, MARCHID_CELL, Transpiler.CSR_ADDR] at h1 h2
      omega
    · rw [hz] at hB; cases hB
  · rw [hz] at hm; cases hm

set_option maxHeartbeats 4000000 in
/-- `one_stepT`, ZisK on its memory without X's bytes alongside: its steps
    there are its steps on the whole memory, or the step breaks X. -/
theorem one_stepTX (s : RVState) (z : ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (h : OneOkT rs zrom s) (n : Nat) {X : Nat → Bool} (hX : RomX X) :
    ∃ s' z' k', MStep rs n s s' ∧ ZRun zrom k' z z' ∧ Rel s' z' rs ∧ RamOnly s.mem s'.mem ∧ FKeep z z' ∧
      ∀ σ, FRX X σ z → (∃ σ', ZRun zrom k' σ σ' ∧ FRX X σ' z') ∨ BreakXAt X s := by
  obtain ⟨pc0, hpc0, hzpc0⟩ := hrel.pc
  obtain ⟨pc, hpc, ⟨o, ho⟩ | ⟨i, hi, hodd, v1, hv1, t0, t1⟩ | ⟨i, hi⟩ | ⟨i, hi, v1, hv1, hok⟩ |
    (⟨i, hi, v1, hv1, hok⟩ | ⟨i, hi, v1, v2, hv1, hv2, hok⟩) | ⟨i, hi, hrd, hcell⟩⟩ := h
  · obtain ⟨s', z', h1, h2, h3, h4⟩ := step_relT s z rs hrel pc hpc zrom o ho n
    obtain ⟨-, e, he, hop⟩ := StepOkT.word o ho
    obtain ⟨e', ht, hnm⟩ := StepOkT.tr o ho
    obtain ⟨e2, hw2, ht2, hnm2⟩ := StepOkT.wt o ho
    have hzpc : z.regs.get? _root_.Zisk.Register.PC = some pc := by rw [hpc] at hpc0; cases hpc0; exact hzpc0
    refine ⟨s', z', 1, .plain pc e hpc he hop h1, .step h2 (.done _), h3, h4,
      zstep_fkeep hzpc (transpiled_keep ht hnm.not_csr) h2, fun σ hσ => ?_⟩
    rcases plain_x hX hrel hσ hpc hw2 ht2 hnm2.not_csr h2 with ⟨σ', hs, h'⟩ | hb
    · exact .inl ⟨σ', .step hs (.done _), h'⟩
    · exact .inr hb
  · obtain ⟨ia, hjo⟩ := hi.toJoAt hodd
    obtain ⟨s', z', h1, h2, h3, -, hm, hk⟩ := jo_rel s z rs hrel pc hpc zrom i ia hjo v1 hv1 ⟨t0, t1⟩ n
    refine ⟨s', z', 2, .plain pc _ hpc (readWord_of hi.h0 hi.h1 hi.h2 hi.h3) (notSys_jr i).plain h1, h2, h3,
      RamOnly.of_eq hm, hk, fun σ hσ => ?_⟩
    refine block_x hX hrel hσ hpc (readWord_of hi.h0 hi.h1 hi.h2 hi.h3) (zrom pc) [(ia, zrom ia)]
      (by simp) ?_ ?_ ?_ rfl h2
    · rw [hjo.rom1]; exact ⟨rfl, rfl, rfl, rfl, trivial⟩
    · intro p hp; simp only [List.mem_singleton] at hp; subst hp
      rw [hjo.rom2]; exact ⟨(by intro h; cases h), (by intro h; cases h), (by intro h; cases h), rfl⟩
    · left; rw [hjo.rom1]
      refine ⟨(by intro h; cases h), ?_, ?_, rfl⟩ <;> simp only [Equiv.Zisk.jalrAddInst, Equiv.Zisk.srcBsrc] <;> split <;>
        (intro h; cases h)
  · obtain ⟨s', z', h1, h2, h3, -, hm, hk⟩ := uwstep_rel s z rs hrel pc hpc zrom i _ hi n
    refine ⟨s', z', 2, .plain pc _ hpc (readWord_of hi.h0 hi.h1 hi.h2 hi.h3) (notSys_uw i).plain h1, h2, h3,
      RamOnly.of_eq hm, hk, fun σ hσ => ?_⟩
    obtain ⟨ia, r1, r2⟩ := i.rom_of_spec hi.tr
    refine block_x hX hrel hσ hpc (readWord_of hi.h0 hi.h1 hi.h2 hi.h3) (zrom pc) [(ia, zrom ia)]
      (by simp) ?_ ?_ ?_ rfl h2
    · rw [r1]; exact ⟨rfl, rfl, rfl, rfl, trivial⟩
    · intro p hp; simp only [List.mem_singleton] at hp; subst hp
      rw [r2]
      split <;> refine ⟨?_, ?_, ?_, ?_⟩ <;> simp only [Transpiler.mk, Transpiler.srcA, Transpiler.srcB] <;>
        (repeat' split) <;> first | (intro h; cases h) | rfl | (simp [zop_nomem]; done)
    · left; rw [r1]
      refine ⟨?_, ?_, ?_, rfl⟩ <;> simp only [Transpiler.mk, Transpiler.srcA] <;> (repeat' split) <;>
        (intro h; cases h)
  · obtain ⟨s', z', k, h1, h2, h3, -, hm, hk, ia, hrom, hkl⟩ := amostep_rel s z rs hrel pc hpc zrom i _ hi v1 hv1 hok n
    refine ⟨s', z', k, .plain pc _ hpc (readWord_of hi.h0 hi.h1 hi.h2 hi.h3) (notSys_amo i).plain h1, h2, h3, hm, hk,
      fun σ hσ => ?_⟩
    obtain ⟨i0, rest, heq, hc, hnr⟩ := amo_shape i.encode i.op i.d pc ia
    rw [heq] at hrom hkl
    exact block_x hX hrel hσ hpc (readWord_of hi.h0 hi.h1 hi.h2 hi.h3) i0 rest hrom hc hnr
      (head_x hi.tr (notSys_amo i).not_csr) hkl h2
  · obtain ⟨s', z', h1, h2, h3, h4, hk⟩ := lrstep_m s z rs hrel pc hpc zrom i _ hi v1 hv1 hok n
    refine ⟨s', z', 1, h1, h2, h3, h4, hk, fun σ hσ => ?_⟩
    have hd : Transpiler.decode i.encode ≠ some .csr := not_csr_2f (by simp only [LrInstr.encode]; bv_decide)
    exact block_x hX hrel hσ hpc (readWord_of hi.h0 hi.h1 hi.h2 hi.h3) (zrom pc) [] (by simp) trivial
      (by simp) (head_x hi.tr hd) rfl h2
  · obtain ⟨s', z', k, h1, h2, h3, h4, hk, ia, hrom, hkl⟩ := scstep_m s z rs hrel pc hpc zrom i _ hi v1 v2 hv1 hv2 hok n
    refine ⟨s', z', k, h1, h2, h3, h4, hk, fun σ hσ => ?_⟩
    have hd : Transpiler.decode i.encode ≠ some .csr := not_csr_2f (by simp only [ScInstr.encode]; bv_decide)
    obtain ⟨i0, rest, heq, hc, hnr⟩ := sc_shape i.encode i.d pc ia
    rw [heq] at hrom hkl
    exact block_x hX hrel hσ hpc (readWord_of hi.h0 hi.h1 hi.h2 hi.h3) i0 rest hrom hc hnr (head_x hi.tr hd) hkl h2
  · obtain ⟨s', z', h1, h2, h3, h4, hk⟩ := marchidstep_m s z rs hrel pc hpc zrom i hrd _ hi hcell n
    refine ⟨s', z', 1, h1, h2, h3, h4, hk, fun σ hσ => .inl ?_⟩
    have hzpc : z.regs.get? _root_.Zisk.Register.PC = some pc := by rw [hpc] at hpc0; cases hpc0; exact hzpc0
    cases h2 with
    | step hs hr =>
      cases hr
      obtain ⟨σ', hs', h'⟩ := marchid_x hX hσ hzpc (i.spec hrd hi.tr) hs
      exact ⟨σ', .step hs' (.done _), h'⟩

end Equiv.Final
