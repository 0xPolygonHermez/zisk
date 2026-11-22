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

/-- A segment's byte at offset i: its file byte, else 0. -/
def byteAt (s : Seg) (i : Nat) : BitVec 8 := if h : i < s.data.size then (s.data[i]'h).toBitVec else 0

theorem foldl_insert_range (b : Nat) (f : Nat → BitVec 8) (a : Nat) :
    ∀ (n : Nat) (m : Mem), ((List.range n).foldl (fun m i => m.insert (b + i) (f i)) m).get? a =
      if b ≤ a ∧ a < b + n then some (f (a - b)) else m.get? a
  | 0, m => by rw [if_neg (by omega)]; rfl
  | n + 1, m => by
    rw [List.range_succ, List.foldl_append, List.foldl_cons, List.foldl_nil]
    simp only [Std.ExtHashMap.get?_eq_getElem?, Std.ExtHashMap.getElem?_insert]
    have ih := foldl_insert_range b f a n m
    simp only [Std.ExtHashMap.get?_eq_getElem?] at ih
    rw [ih]
    by_cases e : b + n = a
    · subst e; simp
    · have e' : (b + n == a) = false := by simpa using e
      simp only [e', Bool.false_eq_true, if_false]
      by_cases h : b ≤ a ∧ a < b + n
      · simp [h]; omega
      · rw [if_neg h, if_neg (by omega)]

theorem load_get (m : Mem) (s : Seg) (a : Nat) :
    (load m s).get? a = if s.base ≤ a ∧ a < s.base + s.memsz then some (byteAt s (a - s.base)) else m.get? a := by
  unfold load
  exact foldl_insert_range s.base (byteAt s) a s.memsz m

/-- Loading disjoint segments: an address in one of them reads its byte;
    one in none, what it read before. -/
theorem loads_get : ∀ (segs : List Seg), segs.Pairwise Disj → ∀ (m : Mem) (a : Nat),
    ((∀ s ∈ segs, s.base ≤ a → a < s.base + s.memsz → (segs.foldl load m).get? a = some (byteAt s (a - s.base))) ∧
     ((∀ s ∈ segs, ¬ (s.base ≤ a ∧ a < s.base + s.memsz)) → (segs.foldl load m).get? a = m.get? a))
  | [], _, m, a => ⟨by simp, fun _ => rfl⟩
  | s :: rest, hp, m, a => by
    obtain ⟨hs, hr⟩ := List.pairwise_cons.1 hp
    obtain ⟨ih1, ih2⟩ := loads_get rest hr (load m s) a
    simp only [List.foldl_cons]
    refine ⟨fun t ht h1 h2 => ?_, fun hn => ?_⟩
    · rcases List.mem_cons.1 ht with rfl | ht
      · rw [ih2 (fun o ho ⟨o1, o2⟩ => by have := hs o ho; simp only [Disj] at this; omega), load_get,
          if_pos ⟨h1, h2⟩]
      · exact ih1 t ht h1 h2
    · rw [ih2 (fun o ho => hn o (List.mem_cons_of_mem _ ho)), load_get, if_neg (hn s List.mem_cons_self)]

theorem loadInput_get (m : Mem) (input : ByteArray) (a : Nat) :
    (loadInput m input).get? a = if INPUT_ADDR + 8 ≤ a ∧ a < INPUT_ADDR + 8 + input.size then
      some (input.get! (a - (INPUT_ADDR + 8))).toBitVec else m.get? a := by
  unfold loadInput
  exact foldl_insert_range (INPUT_ADDR + 8) (fun i => (input.get! i).toBitVec) a input.size m

/-- RISC-V's memory from the loader: an address in a segment reads its
    byte, one in the input the input's, any other none. -/
theorem riscvMem_get {segs : List Seg} (hf : ∀ s ∈ segs, SegFacts s) (hp : segs.Pairwise Disj)
    {input : ByteArray} (hin : INPUT_ADDR + 8 + input.size ≤ 0x9000_0000) (a : Nat) :
    (∀ s ∈ segs, s.base ≤ a → a < s.base + s.memsz → (riscvMem segs input).get? a = some (byteAt s (a - s.base))) ∧
    ((∀ s ∈ segs, ¬ (s.base ≤ a ∧ a < s.base + s.memsz)) → ¬ (INPUT_ADDR + 8 ≤ a ∧ a < INPUT_ADDR + 8 + input.size) →
      (riscvMem segs input).get? a = none) := by
  obtain ⟨h1, h2⟩ := loads_get segs hp ∅ a
  unfold riscvMem
  refine ⟨fun s hs b1 b2 => ?_, fun hn hi => ?_⟩
  · rw [loadInput_get, if_neg ?_]
    · exact h1 s hs b1 b2
    · have := (hf s hs).region
      simp only [InRom, InRam, ROM_ADDR, RAM_ADDR] at this; omega
  · rw [loadInput_get, if_neg hi, h2 hn]; simp

/-- The word of four bytes, little-endian, as `code` reads it and as
    `readWord` reads it. -/
theorem word_le (d0 d1 d2 d3 : UInt8) :
    BitVec.ofNat 32 (d0.toNat + 256 * d1.toNat + 65536 * d2.toNat + 16777216 * d3.toNat) =
      d3.toBitVec ++ d2.toBitVec ++ d1.toBitVec ++ d0.toBitVec := by
  apply BitVec.eq_of_toNat_eq
  have e0 := d0.toBitVec.isLt; have e1 := d1.toBitVec.isLt; have e2 := d2.toBitVec.isLt
  have e3 := d3.toBitVec.isLt
  rw [BitVec.toNat_append, ← Nat.shiftLeft_add_eq_or_of_lt e0, BitVec.toNat_append,
    ← Nat.shiftLeft_add_eq_or_of_lt e1, BitVec.toNat_append, ← Nat.shiftLeft_add_eq_or_of_lt e2]
  simp only [BitVec.toNat_ofNat, Nat.shiftLeft_eq, UInt8.toNat_toBitVec] at *
  omega

/-- A code word: in an executable segment, at a 4-byte offset within its
    file bytes. -/
theorem mem_codeWords {segs : List Seg} {x : BitVec 64 × BitVec 32 × Option (BitVec 32) × Option (BitVec 32)}
    (hx : x ∈ Transpiler.codeWords (code segs)) :
    ∃ s ∈ segs, s.exec = true ∧ ∃ k, 4 * k + 4 ≤ s.data.size ∧
      x.1 = BitVec.ofNat 64 s.base + BitVec.ofNat 64 (4 * k) ∧
      x.2.1 = BitVec.ofNat 32 (s.data[4 * k]!.toNat + 256 * s.data[4 * k + 1]!.toNat +
        65536 * s.data[4 * k + 2]!.toNat + 16777216 * s.data[4 * k + 3]!.toNat) := by
  simp only [Transpiler.codeWords, code, List.flatMap, List.map_map, List.mem_flatten, List.mem_map,
    List.mem_filter, Function.comp] at hx
  obtain ⟨l, ⟨s, ⟨hs, hex⟩, rfl⟩, hl⟩ := hx
  simp only [List.mem_map, List.mem_range] at hl
  obtain ⟨k, hk, rfl⟩ := hl
  simp only [Array.size_map, Array.size_range] at hk
  refine ⟨s, hs, hex, k, by omega, rfl, ?_⟩
  simp [Array.getElem!_eq_getD, Array.getD, hk]

theorem ba_get! (d : ByteArray) (i : Nat) (h : i < d.size) : d[i]! = d[i]'h := by
  simp [getElem!_def, h]

/-- The code words, in RISC-V's memory from the loader. -/
theorem code_readWord {segs : List Seg} (hf : ∀ s ∈ segs, SegFacts s) (hp : segs.Pairwise Disj)
    {input : ByteArray} (hin : INPUT_ADDR + 8 + input.size ≤ 0x9000_0000)
    {x : BitVec 64 × BitVec 32 × Option (BitVec 32) × Option (BitVec 32)}
    (hx : x ∈ Transpiler.codeWords (code segs)) :
    ZiskRiscv.readWord (riscvMem segs input) x.1.toNat = some x.2.1 := by
  obtain ⟨s, hs, hex, k, hk, h1, h2⟩ := mem_codeWords hx
  obtain ⟨⟨hr1, hr2⟩, hms, -⟩ := (hf s hs).code hex
  have ha : x.1.toNat = s.base + 4 * k := by
    rw [h1, BitVec.toNat_add, BitVec.toNat_ofNat, BitVec.toNat_ofNat]
    simp only [ROM_ADDR, ROM_SIZE] at hr1 hr2; omega
  have hb : ∀ i (hi : i < 4), (riscvMem segs input).get? (s.base + 4 * k + i) =
      some (s.data[4 * k + i]'(Nat.lt_of_lt_of_le (Nat.add_lt_add_left hi _) hk)).toBitVec := by
    intro i hi
    have l1 : s.base ≤ s.base + 4 * k + i := by omega
    have l2 : s.base + 4 * k + i < s.base + s.memsz := by rw [hms]; omega
    rw [(riscvMem_get hf hp hin _).1 s hs l1 l2]
    simp [byteAt, show s.base + 4 * k + i - s.base = 4 * k + i by omega, show 4 * k + i < s.data.size by omega]
  rw [ha, h2, ba_get! _ _ (by omega), ba_get! _ _ (by omega), ba_get! _ _ (by omega), ba_get! _ _ (by omega),
    word_le]
  have b0 := hb 0 (by omega); have b1 := hb 1 (by omega); have b2 := hb 2 (by omega); have b3 := hb 3 (by omega)
  simp only [Nat.add_zero, Std.ExtHashMap.get?_eq_getElem?] at b0 b1 b2 b3
  simp [ZiskRiscv.readWord, Std.ExtHashMap.get?_eq_getElem?, b0, b1, b2, b3]

theorem callInsts_len (c : ZiskRiscv.Call) (pc ia : BitVec 64) : (Transpiler.callInsts c pc ia).length ≤ 2 := by
  unfold Transpiler.callInsts; split <;> simp

theorem csr_len {w : BitVec 32} {pc : BitVec 64} {next : Option (BitVec 32)} {ia : BitVec 64}
    {l : List (BitVec 64 × _root_.Zisk.zisk_inst)} (h : Transpiler.csr w pc next ia = some l) : l.length ≤ 2 := by
  unfold Transpiler.csr at h
  (repeat' (first | split at h | (dsimp only at h; split at h)))
  all_goals first | (simp at h; done) | (rw [← Option.some.inj h]; exact callInsts_len _ _ _) |
    (rw [← Option.some.inj h]; simp)

theorem csr3_len {w : BitVec 32} {pc : BitVec 64} {next next2 : Option (BitVec 32)} {ia : BitVec 64}
    {l : List (BitVec 64 × _root_.Zisk.zisk_inst)} (h : Transpiler.csr3 w pc next next2 ia = some l) :
    l.length ≤ 2 := by
  unfold Transpiler.csr3 at h
  (repeat' (first | split at h | (dsimp only at h; split at h)))
  all_goals first | exact csr_len h | (simp at h; done) | (rw [← Option.some.inj h]; exact callInsts_len _ _ _) |
    (rw [← Option.some.inj h]; simp)

set_option maxHeartbeats 0 in
set_option maxRecDepth 100000 in
/-- A translation has at most four instructions. -/
theorem convert_len {pc : BitVec 64} {w : BitVec 32} {next : Option (BitVec 32)} {ia : BitVec 64}
    {next2 : Option (BitVec 32)} {l : List (BitVec 64 × _root_.Zisk.zisk_inst)}
    (h : Transpiler.convert pc w next ia next2 = some l) : l.length ≤ 4 := by
  unfold Transpiler.convert at h
  rcases e : Transpiler.decode w with _ | i
  · rw [e] at h; cases h; simp
  · rw [e] at h
    cases i
    all_goals (dsimp only at h)
    all_goals (try (exact Nat.le_trans (csr3_len h) (by omega)))
    all_goals (try simp only [Transpiler.uw, Transpiler.amo, Transpiler.sc, Transpiler.jalr] at h)
    all_goals ((try split at h) <;> (try split at h) <;> (try split at h) <;> (try split at h) <;>
      (try split at h) <;> (try split at h))
    all_goals (try (cases h; simp; done))
    all_goals (try (exact csr3_len h))

theorem romCount_le : ∀ (ws : List Transpiler.CodeWord) (n : Nat), Transpiler.romCount ws n ≤ n + 4 * ws.length
  | [], n => by simp [Transpiler.romCount]
  | (pc, w, next, next2) :: rest, n => by
    simp only [Transpiler.romCount]
    split
    · next l hl =>
      have h1 := romCount_le rest (n + Transpiler.internals l)
      have h2 : Transpiler.internals l ≤ 4 :=
        Nat.le_trans (List.length_filter_le _ _) (convert_len hl)
      simp only [List.length_cons]; omega
    · have := romCount_le rest n; simp only [List.length_cons]; omega

/-- A list of distinct numbers in [a, a + N) has at most N of them. -/
theorem nodup_len : ∀ (N a : Nat) (l : List Nat), l.Nodup → (∀ x ∈ l, a ≤ x ∧ x < a + N) → l.length ≤ N
  | 0, a, l, _, h => by
    cases l with
    | nil => simp
    | cons x _ => have := h x List.mem_cons_self; omega
  | N + 1, a, l, hn, h => by
    have ih := nodup_len N (a + 1) (l.erase a) (hn.erase a) (fun x hx => by
      obtain ⟨h1, h2⟩ := (hn.mem_erase_iff).1 hx
      have := h x h2; omega)
    by_cases ha : a ∈ l
    · rw [List.length_erase_of_mem ha] at ih; omega
    · have hl : l.erase a = l := List.erase_of_not_mem ha
      rw [hl] at ih; omega

theorem ofNat_add_toNat {b k : Nat} (h : b + k < 2 ^ 64) :
    (BitVec.ofNat 64 b + BitVec.ofNat 64 k).toNat = b + k := by
  rw [BitVec.toNat_add, BitVec.toNat_ofNat, BitVec.toNat_ofNat]; omega

/-- The code addresses, as numbers: each in an executable segment, distinct. -/
theorem codeAddrs {segs : List Seg} (hf : ∀ s ∈ segs, SegFacts s) (hp : segs.Pairwise Disj) :
    ((Transpiler.codeWords (code segs)).map (·.1)).Pairwise (fun x y => x.toNat ≠ y.toNat) := by
  have rom : ∀ s ∈ segs, s.exec = true → s.base + s.data.size ≤ ROM_ADDR + ROM_SIZE := by
    intro s hs hx; obtain ⟨⟨-, h⟩, hm, -⟩ := (hf s hs).code hx; omega
  unfold Transpiler.codeWords code
  rw [List.map_flatMap, List.flatMap_map, List.pairwise_flatMap]
  refine ⟨fun s hs => ?_, ?_⟩
  · obtain ⟨hs, hx⟩ := List.mem_filter.1 hs
    have hr := rom s hs hx
    simp only [List.map_map, Function.comp_def, Array.size_map, Array.size_range]
    rw [List.pairwise_map]
    refine List.pairwise_lt_range.imp_of_mem (fun {k1 k2} h1 h2 hlt => ?_)
    rw [List.mem_range] at h1 h2
    rw [ofNat_add_toNat (by simp only [ROM_ADDR, ROM_SIZE] at hr; omega),
      ofNat_add_toNat (by simp only [ROM_ADDR, ROM_SIZE] at hr; omega)]
    omega
  · refine (hp.filter _).imp_of_mem (fun {a b} ha hb hd x hx y hy => ?_)
    obtain ⟨ha, hax⟩ := List.mem_filter.1 ha
    obtain ⟨hb, hbx⟩ := List.mem_filter.1 hb
    have hra := rom a ha hax; have hrb := rom b hb hbx
    have hma := ((hf a ha).code hax).2.1; have hmb := ((hf b hb).code hbx).2.1
    simp only [List.mem_map, List.mem_range, Array.size_map, Array.size_range] at hx hy
    obtain ⟨_, ⟨k1, hk1, rfl⟩, rfl⟩ := hx
    obtain ⟨_, ⟨k2, hk2, rfl⟩, rfl⟩ := hy
    rw [ofNat_add_toNat (by simp only [ROM_ADDR, ROM_SIZE] at hra; omega),
      ofNat_add_toNat (by simp only [ROM_ADDR, ROM_SIZE] at hrb; omega)]
    simp only [Disj] at hd
    omega

/-- A code address: in its segment, in the ROM. -/
theorem code_addr {segs : List Seg} (hf : ∀ s ∈ segs, SegFacts s)
    {x : BitVec 64 × BitVec 32 × Option (BitVec 32) × Option (BitVec 32)}
    (hx : x ∈ Transpiler.codeWords (code segs)) :
    ∃ s ∈ segs, s.exec = true ∧ ∃ k, 4 * k + 4 ≤ s.data.size ∧ x.1.toNat = s.base + 4 * k ∧ s.base % 4 = 0 ∧
      ROM_ADDR ≤ s.base ∧ s.base + s.data.size ≤ ROM_ADDR + ROM_SIZE := by
  obtain ⟨s, hs, hex, k, hk, h1, -⟩ := mem_codeWords hx
  obtain ⟨⟨hr1, hr2⟩, hms, hal⟩ := (hf s hs).code hex
  refine ⟨s, hs, hex, k, hk, ?_, hal, hr1, by omega⟩
  rw [h1, ofNat_add_toNat (by simp only [ROM_ADDR, ROM_SIZE] at hr2; omega)]

/-- The loader's code, as the theorems take it (`SegsOk`). -/
theorem segsOk_code {segs : List Seg} (hf : ∀ s ∈ segs, SegFacts s) (hp : segs.Pairwise Disj) :
    Equiv.Sim.SegsOk (code segs) := by
  have ha := codeAddrs hf hp
  refine ⟨fun x hx => ?_, ?_, fun x hx => ?_, ?_⟩
  · obtain ⟨s, -, -, k, -, e, hal, -⟩ := code_addr hf hx; omega
  · exact ha.imp (fun h e => h (by rw [e]))
  · obtain ⟨s, -, -, k, -, e, -, hr, -⟩ := code_addr hf hx
    have e : Transpiler.ROM_ADDR.toNat = 0x9000_0000 := rfl
    simp only [ROM_ADDR] at hr; omega
  · have hn : ((Transpiler.codeWords (code segs)).map (·.1.toNat)).Nodup := by
      have := (List.pairwise_map (f := fun x : BitVec 64 × BitVec 32 × Option (BitVec 32) × Option (BitVec 32) => x.1)).1 ha
      exact List.pairwise_map.2 this
    have hlen := nodup_len ROM_SIZE ROM_ADDR _ hn (fun x hx => by
      obtain ⟨y, hy, rfl⟩ := List.mem_map.1 hx
      obtain ⟨s, -, -, k, hk, e, -, hr1, hr2⟩ := code_addr hf hy
      rw [e]; omega)
    have := romCount_le (Transpiler.codeWords (code segs)) 0
    simp only [List.length_map, ROM_SIZE] at hlen
    omega

open Equiv.Riscv Equiv.Sim Equiv.Machine Equiv.Top Equiv.Sparse
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- Bytes all 0 read as 0. -/
theorem memRead_zero (m : Mem) : ∀ (n a : Nat), (∀ i, i < n → m.get? (a + i) = some 0) → memRead m n a = some 0
  | 0, a, _ => rfl
  | 1, a, h => by simpa [memRead] using h 0 (by omega)
  | n + 2, a, h => by
    have h0 := h 0 (by omega)
    have ih := memRead_zero m (n + 1) (a + 1) (fun i hi => by rw [Nat.add_assoc, Nat.add_comm 1 i]; exact h (i + 1) (by omega))
    simp only [Nat.add_zero] at h0
    simp only [memRead, h0, ih, Option.bind_eq_bind, Option.bind_some, Option.pure_def, Option.some.injEq]
    apply BitVec.eq_of_toNat_eq
    simp [BitVec.append]
    exact (BitVec.toNat_zero (w := 8 * (n + 2))).symm

/-- A ROM address is executable (the converse of `exec_rom`). -/
theorem rom_exec {a : BitVec 64} (h1 : 0x9000_0000 ≤ a.toNat) (h2 : a.toNat + 2 ≤ 0x9800_0000) :
    ExecAt ziskRegions a := by
  unfold ExecAt
  simp only [matching_pma_region, matching_pma_region_bits_range, ziskRegions]
  have g : get_slice_int 64 (2 : Int) 0 = 2#64 := by rfl
  have z : Sail.BitVec.zeroExtend a 64 = a := by simp [Sail.BitVec.zeroExtend]
  have u : ∀ x y : BitVec 64, (Sail.BitVec.toNatInt x ≤ Sail.BitVec.toNatInt y) ↔ x ≤ y := by
    intro x y; simp [Sail.BitVec.toNatInt, BitVec.le_def]
  simp only [range_subset, zopz0zIzJ_u, zero_extend, bits_of_physaddr, to_bits, g, z, Bool.and_eq_true,
    decide_eq_true_eq, u]
  simp only [Sail.BitVec.zeroExtend, g, BitVec.setWidth_eq]
  have b1 : 0x90000000#64 ≤ a := by rw [BitVec.le_def]; simp; omega
  have b2 : a ≤ 0x97FFFFFE#64 := by rw [BitVec.le_def]; simp; omega
  rw [if_neg (by bv_decide), if_pos (by bv_decide)]
  exact ⟨_, rfl, rfl⟩

/-- M is L with D's bytes where L has none: so after the same writes. -/
theorem or_foldl_insert {M L : Mem} {D : Nat → Option (BitVec 8)} (h : ∀ a, M.get? a = (L.get? a).or (D a)) :
    ∀ (l : List (Nat × BitVec 8)) (a : Nat),
      (l.foldl (fun m p => m.insert p.1 p.2) M).get? a = ((l.foldl (fun m p => m.insert p.1 p.2) L).get? a).or (D a)
  | [], a => h a
  | p :: l, a => by
    simp only [List.foldl_cons]
    refine or_foldl_insert (fun b => ?_) l a
    simp only [Std.ExtHashMap.get?_eq_getElem?, Std.ExtHashMap.getElem?_insert]
    have hb := h b; simp only [Std.ExtHashMap.get?_eq_getElem?] at hb
    split <;> simp_all

theorem or_startMem {M L : Mem} {D : Nat → Option (BitVec 8)} (h : ∀ a, M.get? a = (L.get? a).or (D a)) (a : Nat) :
    (startMem M).get? a = ((startMem L).get? a).or (D a) := by
  unfold startMem Equiv.memWrite
  exact or_foldl_insert (or_foldl_insert h _) _ a

theorem readWord_or {M L : Mem} {D : Nat → Option (BitVec 8)} (h : ∀ a, M.get? a = (L.get? a).or (D a))
    {a : Nat} {w : BitVec 32} (hw : ZiskRiscv.readWord L a = some w) : ZiskRiscv.readWord M a = some w := by
  simp only [ZiskRiscv.readWord, Option.bind_eq_bind, Option.pure_def] at hw ⊢
  rw [h a, h (a + 1), h (a + 2), h (a + 3)]
  cases h0 : L.get? a <;> cases h1 : L.get? (a + 1) <;> cases h2 : L.get? (a + 2) <;> cases h3 : L.get? (a + 3) <;>
    simp_all

/-- Below the RAM, the start's memory is the loader's. -/
theorem readWord_startMem (L : Mem) (a : Nat) (ha : a + 4 ≤ 0xA000_0000) :
    ZiskRiscv.readWord (startMem L) a = ZiskRiscv.readWord L a := by
  have e : ∀ i, i < 4 → (startMem L).get? (a + i) = L.get? (a + i) := by
    intro i hi
    unfold startMem
    rw [memWrite_get_off _ _ _ _ (by left; simp [Transpiler.MTVEC_ADDR]; omega),
      memWrite_get_off _ _ _ _ (by left; simp [Transpiler.ARCH_ID_CSR_ADDR]; omega)]
  have e0 := e 0 (by omega); have e1 := e 1 (by omega); have e2 := e 2 (by omega); have e3 := e 3 (by omega)
  simp only [Nat.add_zero] at e0
  simp only [ZiskRiscv.readWord, e0, e1, e2, e3]

set_option maxHeartbeats 4000000 in
/-- The theorem from the ELF as ZisK loads it (`segments`): the code, the
    memory each side starts with — RISC-V's every segment and the input,
    with 0 elsewhere in the regions (m0, from `riscvMem`; the machine on the
    sparse one itself, `startMem` of it); ZisK's that without the
    execute-only bytes (`xOnly`) — and the starts as § 3 says. Its run is
    covered as far as one likes, ZisK's on its memory in step; or both exit
    with the same code; or both fail; or the program breaks a precondition,
    X among them. -/
theorem zisk_elf (elf : ELF64File) (segs : List Seg) (hsegs : segments elf = .ok segs) (input : ByteArray)
    (hin : INPUT_ADDR + 8 + input.size ≤ 0x9000_0000) (ht : Transpiler.tailOk (code segs) = true)
    (entry : BitVec 64) (m0 : Mem) (hm0 : ∀ a, m0.get? a = ((riscvMem segs input).get? a).or (zeroFill a))
    (s0 : RVState) (hs0 : Start s0 ziskRegions entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (hfc : fc.freeInput = 0) (hzf : Equiv.Zisk.FcallRel fc z0)
    (σ0 : Equiv.Zisk.ZState) (hσr : σ0.regs = z0.regs) (hσm : σ0.mem = eraseX (xOnly segs) m0) :
    (∀ N, ∃ k, N ≤ k ∧ Covered (zromOf (code segs) entry) (CodeAt (code segs)) ⟨s0, fc⟩ k ∧
      ∃ z' n σ', ZRun (zromOf (code segs) entry) n z0 z' ∧ Rel (sIter 0 k ⟨s0, fc⟩).rv z' ziskRegions ∧
        ZRun (zromOf (code segs) entry) n σ0 σ' ∧ FRX (xOnly segs) σ' z') ∨
    (∃ k c, MachineExits ⟨{ s0 with mem := startMem (riscvMem segs input) }, fc⟩ c
        (ZiskRiscv.output (sIter 0 k ⟨s0, fc⟩)) ∧
      ZiskExitsX (xOnly segs) (zromOf (code segs) entry) σ0 c (sIter 0 k ⟨s0, fc⟩).rv.mem) ∨
    (MachineFails ⟨{ s0 with mem := startMem (riscvMem segs input) }, fc⟩ ∧
      ZiskFails (zromOf (code segs) entry) σ0) ∨
    (∃ j, Covered (zromOf (code segs) entry) (CodeAt (code segs)) ⟨s0, fc⟩ j ∧
      PreBroken (zromOf (code segs) entry) (CodeAt (code segs)) j (sIter 0 j ⟨s0, fc⟩)) ∨
    ∃ j, RunOkM ziskRegions (zromOf (code segs) entry) (CodeAt (code segs)) j ⟨s0, fc⟩ 0 ∧
      RunInM (CodeAt (code segs)) j ⟨s0, fc⟩ 0 ∧ BreakXS (xOnly segs) (sIter 0 j ⟨s0, fc⟩) := by
  obtain ⟨hf, hp⟩ := segments_ok hsegs
  have hw : ∀ x ∈ Transpiler.codeWords (code segs), ZiskRiscv.readWord m0 x.1.toNat = some x.2.1 :=
    fun x hx => readWord_or hm0 (code_readWord hf hp hin hx)
  have hpo : ProgramOk (code segs) (CodeAt (code segs)) m0 :=
    ProgramOk.of_tail (segsOk_code hf hp) (fun pc h => h) hw ht
  have hcx : ∀ pc, CodeAt (code segs) pc → ExecAt ziskRegions pc := by
    intro pc ⟨x, hx, e⟩
    obtain ⟨s, -, -, k, hk, ha, -, hr1, hr2⟩ := code_addr hf hx
    rw [← e]; simp only [ROM_ADDR, ROM_SIZE] at hr1 hr2
    exact rom_exec (by omega) (by omega)
  have hfull : Full m0 := by
    intro a ha
    rw [hm0]; cases (riscvMem segs input).get? a <;> simp [zeroFill, ha]
  have hmir : Equiv.memRead m0 8 Equiv.IN_LO = some fc.freeInput := by
    rw [hfc]
    refine memRead_zero m0 8 Equiv.IN_LO (fun i hi => ?_)
    rw [hm0, (riscvMem_get hf hp hin _).2 (fun s hs ⟨h1, h2⟩ => by
      have := (hf s hs).region; simp only [InRom, InRam, ROM_ADDR, RAM_ADDR, Equiv.IN_LO] at this h1; omega)
      (by simp only [INPUT_ADDR, Equiv.IN_LO]; omega)]
    simp [zeroFill, ZiskRiscv.inRegions, Equiv.IN_LO]; omega
  have hsp : Ext zeroFill { s0 with mem := startMem (riscvMem segs input) } s0 :=
    ⟨rfl, rfl, rfl, rfl, fun a => by rw [hs0.mem]; exact or_startMem hm0 a⟩
  have hcode : ∀ pc, CodeAt (code segs) pc →
      ∃ w, ZiskRiscv.readWord (startMem (riscvMem segs input)) pc.toNat = some w := by
    intro pc ⟨x, hx, e⟩
    obtain ⟨s, -, -, k, hk, ha, -, hr1, hr2⟩ := code_addr hf hx
    rw [← e, readWord_startMem _ _ (by simp only [ROM_ADDR, ROM_SIZE] at hr2; omega)]
    exact ⟨_, code_readWord hf hp hin hx⟩
  have hX : RomX (xOnly segs) := by
    intro a ha
    simp only [xOnly, List.any_eq_true, Bool.and_eq_true, decide_eq_true_eq, Bool.not_eq_true'] at ha
    obtain ⟨s, hs, ⟨⟨⟨hx, -⟩, h1⟩, h2⟩⟩ := ha
    obtain ⟨⟨r1, r2⟩, -, -⟩ := (hf s hs).code hx
    simp only [ROM_ADDR, ROM_SIZE] at r1 r2; omega
  have hσ : FRX (xOnly segs) σ0 z0 := ⟨hσr, fun k => by rw [hσm, eraseX_get, hzm]⟩
  exact zisk_totalX entry (code segs) m0 hpo ht hcx hfull s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf hmir
    { s0 with mem := startMem (riscvMem segs input) } hsp hcode hX σ0 hσ

end Equiv.Final
