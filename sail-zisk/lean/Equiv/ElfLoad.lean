/-
  The ELF as ZisK loads it (ZiskRiscv/Elf.lean), what the theorems take of
  it: each segment's checks (`SegFacts`), the segments pairwise disjoint.
-/
import ZiskRiscv.Elf
import Equiv.TotalityMainX

namespace Equiv.Final

open ZiskRiscv.Elf

/-- A segment in the ROM. -/
def InRom (s : Seg) : Prop := ROM_ADDR ≤ s.base ∧ s.base + s.memsz ≤ ROM_ADDR + ROM_SIZE

/-- A segment in the RAM. -/
def InRam (s : Seg) : Prop := RAM_ADDR ≤ s.base ∧ s.base + s.memsz ≤ RAM_ADDR + RAM_SIZE

/-- What the loader's checks give of a segment. -/
structure SegFacts (s : Seg) : Prop where
  body : s.data.size ≤ s.memsz
  region : InRom s ∨ InRam s
  code : s.exec = true → InRom s ∧ s.memsz = s.data.size ∧ s.base % 4 = 0

/-- Two segments share no byte. -/
def Disj (a b : Seg) : Prop := a.base + a.memsz ≤ b.base ∨ b.base + b.memsz ≤ a.base

theorem segValid_facts {loaded : List Seg} {start memsz filesz : Nat} {body : ByteArray} {exec write : Bool}
    (h : segValid loaded start memsz filesz body.size exec write = true) (read : Bool) :
    SegFacts { base := start, data := body, memsz, exec, read, write } ∧
      ∀ o ∈ loaded, Disj o { base := start, data := body, memsz, exec, read, write } := by
  simp only [segValid, Bool.and_eq_true, Bool.not_eq_true', Bool.or_eq_true, decide_eq_true_eq,
    beq_iff_eq, List.any_eq_false, Bool.and_eq_false_iff, decide_eq_false_iff_not] at h
  obtain ⟨⟨⟨⟨⟨⟨⟨⟨-, -⟩, hal⟩, hreg⟩, hov⟩, hfm⟩, hbody⟩, hcode⟩, -⟩ := h
  refine ⟨⟨by simp; omega, ?_, fun hx => ?_⟩, fun o ho => ?_⟩
  · simp only [InRom, InRam]; exact hreg
  · subst hx
    rcases hcode with hc | ⟨⟨⟨a, b⟩, c⟩, -⟩
    · cases hc
    · simp only [if_true] at hal
      exact ⟨⟨a, b⟩, by simp; omega, hal⟩
  · have := hov o ho; simp only [Disj]; omega

theorem loadSegs_ok : ∀ (l : List (ELF64ProgramHeaderTableEntry × InterpretedSegment)) (acc res : List Seg),
    loadSegs l acc = .ok res → (∀ s ∈ acc, SegFacts s) → acc.Pairwise Disj →
    (∀ s ∈ res, SegFacts s) ∧ res.Pairwise Disj
  | [], acc, res, h, h1, h2 => by cases h; exact ⟨h1, h2⟩
  | (ph, seg) :: rest, acc, res, h, h1, h2 => by
    simp only [loadSegs] at h
    split at h
    · cases h
    · next _ s hc =>
      unfold checkSeg at hc
      dsimp only at hc
      split at hc
      · cases hc
      split at hc
      · next hv =>
        cases hc
        obtain ⟨hf, hd⟩ := segValid_facts hv (flag (ProgramHeaderTableEntry.p_flags ph) PF_R)
        refine loadSegs_ok rest _ res h (fun t ht => ?_) ?_
        · rcases List.mem_append.1 ht with ht | ht
          · exact h1 t ht
          · simp at ht; subst ht; exact hf
        · rw [List.pairwise_append]
          exact ⟨h2, List.pairwise_singleton _ _, fun a ha b hb => by simp at hb; subst hb; exact hd a ha⟩
      · cases hc
    · exact loadSegs_ok rest acc res h h1 h2

/-- The segments of an ELF the loader accepts: each checked, pairwise
    disjoint. -/
theorem segments_ok {elf : ELF64File} {segs : List Seg} (h : segments elf = .ok segs) :
    (∀ s ∈ segs, SegFacts s) ∧ segs.Pairwise Disj := by
  unfold segments at h
  dsimp only at h
  split at h
  · cases h
  split at h
  · cases h
  split at h
  · cases h
  split at h
  · cases h
  · next _ res hl =>
    split at h
    · cases h
    split at h
    · cases h
    · cases h
      exact loadSegs_ok _ [] _ hl (by simp) List.Pairwise.nil

end Equiv.Final
