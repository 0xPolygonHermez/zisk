/-
  A ZisK call sequence the transpiler refuses (`malformed`: a call port
  without the instruction after it the call needs, the memset port with an
  immediate other than 0 or 2, ...): the machine's step is the run's
  failure, and the spec's translation, when it describes the word, the halt.
-/
import Equiv.CallRun

namespace Equiv.Machine

open ZiskRiscv

/-- A sequence the transpiler refuses comes from a word of `csrrs` or `csrrsi`. -/
theorem decode_malformed_kind {fetch : Nat → Option (BitVec 32)} {a : Nat} {why : String}
    (h : decode fetch a = .malformed why) :
    ∃ w, fetch a = some w ∧ (kind (fields w) = .csrrs ∨ kind (fields w) = .csrrsi) := by
  simp only [decode] at h
  split at h
  · cases h
  · next w hw =>
    refine ⟨w, hw, ?_⟩
    split at h <;> simp_all [decodeCsrrwi]
    split at h <;> cases h

/-- The word after the next one matters to the decoder only for memset's
    three words, `csrrsi` on 0x816 with 2 (the shape `csr3` looks for). -/
theorem decode_fetch3_2 {pc : BitVec 64} {w : BitVec 32} {next : Option (BitVec 32)} (n2 : Option (BitVec 32))
    (hg : ¬ (kind (fields w) == .csrrsi && (fields w).csr == 0x816 && (fields w).rs1 == 2 && next.isSome) = true) :
    decode (Transpiler.fetch3 pc w next n2) pc.toNat = decode (Transpiler.fetch2 pc w next) pc.toNat := by
  have f0 : Transpiler.fetch3 pc w next n2 pc.toNat = some w := by simp [Transpiler.fetch3]
  have f4 : Transpiler.fetch3 pc w next n2 (pc.toNat + 4) = next := by simp [Transpiler.fetch3]
  have g0 : Transpiler.fetch2 pc w next pc.toNat = some w := by simp [Transpiler.fetch2]
  have g4 : Transpiler.fetch2 pc w next (pc.toNat + 4) = next := by simp [Transpiler.fetch2]
  simp only [decode, f0, f4, g0, g4]
  cases hk : kind (fields w) <;> try rfl
  simp only [decodeCsrrsi]
  split <;> try rfl
  next hc =>
  have hr : next = none ∨ (fields w).rs1 ≠ 2 := by
    cases next with
    | none => exact .inl rfl
    | some _ => right; intro e; simp_all; revert hg; decide
  generalize (Transpiler.fetch3 pc w next n2 (pc.toNat + 8)).map fields = m1
  generalize (Transpiler.fetch2 pc w next (pc.toNat + 8)).map fields = m2
  rcases hr with rfl | hr
  · simp only [Option.map_none]
    generalize (fields w).rs1 = r
    rcases r with _ | _ | _ | r <;> rfl
  · generalize hr' : (fields w).rs1 = r at hr
    rcases r with _ | _ | _ | r
    · rfl
    · rfl
    · exact absurd rfl hr
    · rfl

/-- The decoder reads the word at pc and the two after it, no others. -/
theorem decode_fetch3 (fetch : Nat → Option (BitVec 32)) {pc : BitVec 64} {w : BitVec 32}
    {next next2 : Option (BitVec 32)} (h0 : fetch pc.toNat = some w) (h4 : fetch (pc.toNat + 4) = next)
    (h8 : fetch (pc.toNat + 8) = next2) :
    decode fetch pc.toNat = decode (Transpiler.fetch3 pc w next next2) pc.toNat := by
  have f0 : Transpiler.fetch3 pc w next next2 pc.toNat = some w := by simp [Transpiler.fetch3]
  have f4 : Transpiler.fetch3 pc w next next2 (pc.toNat + 4) = next := by simp [Transpiler.fetch3]
  have f8 : Transpiler.fetch3 pc w next next2 (pc.toNat + 8) = next2 := by simp [Transpiler.fetch3]
  simp only [decode, h0, h4, h8, f0, f4, f8]

/-- A sequence the transpiler refuses, that the spec describes: its
    translation, the halt. -/
theorem convert_malformed3 {pc : BitVec 64} {w : BitVec 32} {next next2 : Option (BitVec 32)} {why : String}
    (h : decode (Transpiler.fetch3 pc w next next2) pc.toNat = .malformed why) {ia : BitVec 64}
    {l : List (BitVec 64 × Zisk.zisk_inst)} (hl : Transpiler.convert pc w next ia next2 = some l) :
    l = [(pc, Transpiler.halt)] := by
  obtain ⟨w', hw', hk⟩ := decode_malformed_kind h
  have : w' = w := by simp [Transpiler.fetch3] at hw'; exact hw'.symm
  subst this
  have hd := spec_decode_csr w' (by rcases hk with hk | hk <;> simp [hk])
  simp only [Transpiler.convert, hd] at hl
  cases next2 with
  | none =>
    rw [fetch3_none] at h
    simp only [Transpiler.csr3, Transpiler.csr, h] at hl
    split at hl
    · cases hl
    · cases hl; rfl
  | some n2 =>
    simp only [Transpiler.csr3] at hl
    split at hl
    · simp only [h] at hl; cases hl; rfl
    · next hg =>
      rw [decode_fetch3_2 _ hg] at h
      simp only [Transpiler.csr, h] at hl
      split at hl
      · cases hl
      · cases hl; rfl

/-- The machine's step on a sequence the transpiler refuses: the run's failure. -/
theorem step_malformed (S : ZiskRiscv.State model) {why : String} (hx : model.getPC S.rv ≠ model.exitAddr)
    (hd : ZiskRiscv.decode (ZiskRiscv.readWord (model.mem S.rv)) (model.getPC S.rv).toNat = .malformed why) :
    ∃ m, ZiskRiscv.step S = .ok (S, .failure m) := by
  have e1 : ZiskRiscv.tryStep S = .ok (S, .failure s!"a ZisK call the transpiler refuses: {why}") := by
    simp only [ZiskRiscv.tryStep, hd]
    simp [hx, pure, Except.pure]; rfl
  exact ⟨_, by rw [ZiskRiscv.step, ZiskRiscv.step.go, e1]⟩

end Equiv.Machine
