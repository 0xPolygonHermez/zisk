/-
  The program's ROM from the spec (Transpiler/Rom.lean), as the theorem's
  ROM: where each translation puts its instructions.
-/
import Transpiler.Rom

namespace Transpiler

open _root_.Zisk

/-- Where a translation puts its instructions: the first at pc, the k-th
    after it at the k-th internal address from ia, ia + 2k. -/
def Shape (pc ia : BitVec 64) (l : List (BitVec 64 × zisk_inst)) : Prop :=
  l.map Prod.fst = pc :: (List.range (l.length - 1)).map fun k => ia + BitVec.ofNat 64 (2 * k)

theorem shape_one (pc ia : BitVec 64) (i : zisk_inst) : Shape pc ia [(pc, i)] := by
  simp [Shape]

theorem shape_two (pc ia : BitVec 64) (i j : zisk_inst) : Shape pc ia [(pc, i), (ia, j)] := by
  simp [Shape]

theorem shape_three (pc ia : BitVec 64) (i j k : zisk_inst) : Shape pc ia [(pc, i), (ia, j), (ia + 2, k)] := by
  simp [Shape, List.range_succ]

theorem shape_four (pc ia : BitVec 64) (i j k m : zisk_inst) :
    Shape pc ia [(pc, i), (ia, j), (ia + 2, k), (ia + 4, m)] := by
  simp [Shape, List.range_succ]

theorem shape_csr {w : BitVec 32} {pc : BitVec 64} {next : Option (BitVec 32)} {ia : BitVec 64}
    {l : List (BitVec 64 × zisk_inst)} (h : csr w pc next ia = some l) : Shape pc ia l := by
  simp only [csr] at h
  split at h <;> (try split at h) <;> (try split at h) <;> (try split at h) <;>
    first
      | (cases h; first | exact shape_one _ _ _ | (simp only [callInsts]; split <;> first | exact shape_one _ _ _ | exact shape_two _ _ _ _))
      | (cases h)

theorem shape_csr3 {w : BitVec 32} {pc : BitVec 64} {next next2 : Option (BitVec 32)} {ia : BitVec 64}
    {l : List (BitVec 64 × zisk_inst)} (h : csr3 w pc next next2 ia = some l) : Shape pc ia l := by
  simp only [csr3] at h
  split at h
  · exact shape_csr h
  · split at h
    · split at h
      · cases h; simp only [callInsts]; split <;> first | exact shape_one _ _ _ | exact shape_two _ _ _ _
      · cases h; exact shape_one _ _ _
      · cases h
    · exact shape_csr h

theorem shape_jalr (w : BitVec 32) (pc ia : BitVec 64) : Shape pc ia (jalr w pc ia) := by
  simp only [jalr]; split
  · exact shape_one _ _ _
  · exact shape_two _ _ _ _

theorem shape_uw (w : BitVec 32) (op : Option ShAdd) (pc ia : BitVec 64) : Shape pc ia (uw w op pc ia) :=
  shape_two _ _ _ _

theorem shape_sc (w : BitVec 32) (d : Bool) (pc ia : BitVec 64) : Shape pc ia (sc w d pc ia) := by
  simp only [sc]; split
  · exact shape_one _ _ _
  · exact shape_two _ _ _ _

theorem shape_amo (w : BitVec 32) (op : AmoOp) (d : Bool) (pc ia : BitVec 64) : Shape pc ia (amo w op d pc ia) := by
  simp only [amo]
  split
  · split
    · exact shape_two _ _ _ _
    · exact shape_three _ _ _ _ _
  · split
    · exact shape_three _ _ _ _ _
    · exact shape_four _ _ _ _ _ _

theorem convert_shape {pc : BitVec 64} {w : BitVec 32} {next : Option (BitVec 32)} {ia : BitVec 64}
    {next2 : Option (BitVec 32)} {l : List (BitVec 64 × zisk_inst)}
    (h : convert pc w next ia next2 = some l) : Shape pc ia l := by
  simp only [convert] at h
  repeat' (split at h)
  all_goals first
    | exact shape_csr3 h
    | (cases h; first
        | exact shape_one _ _ _
        | exact shape_jalr _ _ _
        | exact shape_uw _ _ _ _
        | exact shape_sc _ _ _ _
        | exact shape_amo _ _ _ _ _)

/-- Whether the spec describes a word does not depend on the internal
    address it is given. -/
theorem csr_isSome (w : BitVec 32) (pc : BitVec 64) (next : Option (BitVec 32)) (ia ia' : BitVec 64) :
    (csr w pc next ia).isSome = (csr w pc next ia').isSome := by
  simp only [csr]; repeat' split <;> simp_all

theorem csr3_isSome (w : BitVec 32) (pc : BitVec 64) (next next2 : Option (BitVec 32)) (ia ia' : BitVec 64) :
    (csr3 w pc next next2 ia).isSome = (csr3 w pc next next2 ia').isSome := by
  simp only [csr3]
  split
  · exact csr_isSome _ _ _ _ _
  · split
    · split <;> simp
    · exact csr_isSome _ _ _ _ _

theorem convert_isSome (pc : BitVec 64) (w : BitVec 32) (next : Option (BitVec 32)) (ia ia' : BitVec 64)
    (next2 : Option (BitVec 32)) :
    (convert pc w next ia next2).isSome = (convert pc w next ia' next2).isSome := by
  simp only [convert]
  split <;> (try exact csr3_isSome _ _ _ _ _ _) <;> (repeat' split) <;> simp

/-! ## The internal addresses -/

theorem rom_addr_toNat : ROM_ADDR.toNat = 0x9000_0000 := rfl

theorem lsb_eq (x : BitVec 64) : x.getLsbD 0 = decide (x.toNat % 2 = 1) := by
  simp [BitVec.getLsbD, Nat.testBit_zero]

theorem internalAddr_odd (n : Nat) : (internalAddr n).getLsbD 0 = true := by
  rw [lsb_eq]; simp only [internalAddr, BitVec.toNat_add, BitVec.toNat_ofNat, rom_addr_toNat, decide_eq_true_eq]
  omega

theorem internalAddr_add (n k : Nat) : internalAddr n + BitVec.ofNat 64 (2 * k) = internalAddr (n + k) := by
  simp only [internalAddr]; apply BitVec.eq_of_toNat_eq
  simp only [BitVec.toNat_add, BitVec.toNat_ofNat]; omega

/-- Distinct counts below 2^63, distinct internal addresses. -/
theorem internalAddr_inj {m n : Nat} (hm : m < 2 ^ 63) (hn : n < 2 ^ 63) (h : internalAddr m = internalAddr n) :
    m = n := by
  have := congrArg BitVec.toNat h
  simp only [internalAddr, BitVec.toNat_add, BitVec.toNat_ofNat, rom_addr_toNat] at this
  omega

/-- A translation at an even pc, from an internal address: its internal
    addresses the counter's next ones, as many as its instructions after
    the first. -/
theorem shape_internals {pc : BitVec 64} {n : Nat} {l : List (BitVec 64 × zisk_inst)}
    (hs : Shape pc (internalAddr n) l) (hpc : pc.getLsbD 0 = false) : internals l = l.length - 1 := by
  have hm : (l.filter (·.1.getLsbD 0)).length = ((l.map Prod.fst).filter (·.getLsbD 0)).length := by
    rw [List.filter_map, List.length_map]; rfl
  unfold internals; rw [hm, hs, List.filter_cons, hpc]
  simp only [Bool.false_eq_true, if_false]
  rw [List.filter_eq_self.2]
  · simp
  · intro a ha
    simp only [List.mem_map, List.mem_range] at ha
    obtain ⟨k, -, rfl⟩ := ha
    rw [internalAddr_add]; exact internalAddr_odd _

/-! ## The ROM's entries -/

/-- A code word: its address, the word, the two words after it. -/
abbrev CodeWord := BitVec 64 × BitVec 32 × Option (BitVec 32) × Option (BitVec 32)

/-- The counter after the code words, from n. -/
def romCount : List CodeWord → Nat → Nat
  | [], n => n
  | (pc, w, next, next2) :: rest, n =>
    match convert pc w next (internalAddr n) next2 with
    | some l => romCount rest (n + internals l)
    | none => romCount rest n

theorem le_romCount (ws : List CodeWord) (n : Nat) : n ≤ romCount ws n := by
  induction ws generalizing n with
  | nil => simp [romCount]
  | cons y rest ih =>
    obtain ⟨pc, w, nx, nx2⟩ := y
    simp only [romCount]; split
    · exact Nat.le_trans (Nat.le_add_right _ _) (ih _)
    · exact ih _

/-- Every word the spec describes has its translation in the ROM, from the
    counter's internal address. -/
theorem romFrom_mem (ws : List CodeWord) (n : Nat) {x : CodeWord} (hx : x ∈ ws) {ia : BitVec 64}
    {l : List (BitVec 64 × zisk_inst)} (h : convert x.1 x.2.1 x.2.2.1 ia x.2.2.2 = some l) :
    ∃ m l', convert x.1 x.2.1 x.2.2.1 (internalAddr m) x.2.2.2 = some l' ∧ ∀ p ∈ l', p ∈ romFrom ws n := by
  induction ws generalizing n with
  | nil => cases hx
  | cons y rest ih =>
    obtain ⟨pc, w, nx, nx2⟩ := y
    simp only [romFrom]
    rcases List.mem_cons.1 hx with rfl | hx'
    · have hs := convert_isSome pc w nx ia (internalAddr n) nx2
      rw [h] at hs
      obtain ⟨l', hl'⟩ := Option.isSome_iff_exists.1 hs.symm
      refine ⟨n, l', hl', ?_⟩
      rw [hl']; intro p hp; exact List.mem_append_left _ hp
    · split
      · obtain ⟨m, l', h1, h2⟩ := ih _ hx'
        exact ⟨m, l', h1, fun p hp => List.mem_append_right _ (h2 p hp)⟩
      · exact ih n hx'

/-- A translation's addresses, at an even pc, from the counter at n: pc,
    then the counter's next internal addresses. -/
theorem shape_addrs {pc : BitVec 64} {n : Nat} {l : List (BitVec 64 × zisk_inst)}
    (hs : Shape pc (internalAddr n) l) :
    l.map Prod.fst = pc :: (List.range (l.length - 1)).map fun k => internalAddr (n + k) := by
  rw [hs]; simp [internalAddr_add]

/-- Where the ROM's entries are: at a code word's address, or at an
    internal address the counter handed out. -/
theorem romFrom_addr (ws : List CodeWord) (n : Nat) (hev : ∀ x ∈ ws, x.1.getLsbD 0 = false) :
    ∀ p ∈ romFrom ws n, (∃ x ∈ ws, p.1 = x.1) ∨ ∃ m, n ≤ m ∧ m < romCount ws n ∧ p.1 = internalAddr m := by
  induction ws generalizing n with
  | nil => simp [romFrom]
  | cons y rest ih =>
    obtain ⟨pc, w, nx, nx2⟩ := y
    intro p hp
    simp only [romFrom, romCount] at hp ⊢
    have ih' := fun n => ih n (fun x hx => hev x (List.mem_cons_of_mem _ hx))
    split at hp
    · next l hl =>
      rw [hl]; dsimp only
      have hs := convert_shape hl
      have hi := shape_internals hs (hev _ List.mem_cons_self)
      rcases List.mem_append.1 hp with hp | hp
      · have hm : p.1 ∈ l.map Prod.fst := List.mem_map_of_mem hp
        rw [shape_addrs hs] at hm
        rcases List.mem_cons.1 hm with e | hm
        · exact .inl ⟨_, List.mem_cons_self, e⟩
        · simp only [List.mem_map, List.mem_range] at hm
          obtain ⟨k, hk, e⟩ := hm
          refine .inr ⟨n + k, by omega, ?_, e.symm⟩
          have := le_romCount rest (n + internals l); omega
      · rcases ih' _ p hp with ⟨x, hx, e⟩ | ⟨m, h1, h2, e⟩
        · exact .inl ⟨x, List.mem_cons_of_mem _ hx, e⟩
        · exact .inr ⟨m, by omega, h2, e⟩
    · next hl =>
      rw [hl]; dsimp only
      rcases ih' _ p hp with ⟨x, hx, e⟩ | ⟨m, h1, h2, e⟩
      · exact .inl ⟨x, List.mem_cons_of_mem _ hx, e⟩
      · exact .inr ⟨m, h1, h2, e⟩

/-- No two entries of the ROM at one address: the code words at distinct,
    even addresses, the counter below 2^63. -/
theorem romFrom_nodup (ws : List CodeWord) (n : Nat) (hev : ∀ x ∈ ws, x.1.getLsbD 0 = false)
    (hpcs : (ws.map (·.1)).Nodup) (hb : romCount ws n < 2 ^ 63) : ((romFrom ws n).map (·.1)).Nodup := by
  induction ws generalizing n with
  | nil => simp [romFrom]
  | cons y rest ih =>
    obtain ⟨pc, w, nx, nx2⟩ := y
    have hev' : ∀ x ∈ rest, x.1.getLsbD 0 = false := fun x hx => hev x (List.mem_cons_of_mem _ hx)
    simp only [List.map_cons, List.nodup_cons] at hpcs
    obtain ⟨hpc, hpcs'⟩ := hpcs
    have hpe : pc.getLsbD 0 = false := hev _ List.mem_cons_self
    simp only [romFrom, romCount] at hb ⊢
    split at hb
    · next l hl =>
      rw [hl]; dsimp only
      have hs := convert_shape hl
      have hi := shape_internals hs hpe
      have hle := le_romCount rest (n + internals l)
      rw [List.map_append, List.nodup_append]
      refine ⟨?_, ih _ hev' hpcs' hb, ?_⟩
      · rw [shape_addrs hs, List.nodup_cons]
        refine ⟨fun hm => ?_, ?_⟩
        · simp only [List.mem_map, List.mem_range] at hm
          obtain ⟨k, -, e⟩ := hm
          have := internalAddr_odd (n + k); rw [e, hpe] at this; cases this
        · unfold List.Nodup; rw [List.pairwise_map]
          refine List.Pairwise.imp_of_mem (fun {a b} ha hb' hab e => hab ?_) List.nodup_range
          simp only [List.mem_range] at ha hb'
          exact Nat.add_left_cancel (internalAddr_inj (by omega) (by omega) e)
      · intro a ha b hb' e
        subst e
        rw [shape_addrs hs] at ha
        obtain ⟨q, hq, rfl⟩ := List.mem_map.1 hb'
        rcases romFrom_addr rest _ hev' q hq with ⟨x, hx, ex⟩ | ⟨m, h1, h2, em⟩
        · rcases List.mem_cons.1 ha with ea | ea
          · exact hpc (by rw [← ea, ex]; exact List.mem_map_of_mem hx)
          · simp only [List.mem_map, List.mem_range] at ea
            obtain ⟨k, -, e⟩ := ea
            have := internalAddr_odd (n + k); rw [e, ex, hev' x hx] at this; cases this
        · rcases List.mem_cons.1 ha with ea | ea
          · have := internalAddr_odd m; rw [← em, ea, hpe] at this; cases this
          · simp only [List.mem_map, List.mem_range] at ea
            obtain ⟨k, hk, e⟩ := ea
            have := internalAddr_inj (by omega) (by omega) (e.trans em); omega
    · next hl =>
      rw [hl]; exact ih n hev' hpcs' hb

/-- The ROM as a function: an address's entry. -/
def romLookup (r : List (BitVec 64 × zisk_inst)) (a : BitVec 64) : Option zisk_inst :=
  (r.find? (·.1 == a)).map (·.2)

theorem romLookup_of_mem {r : List (BitVec 64 × zisk_inst)} (hn : (r.map (·.1)).Nodup) {p : BitVec 64 × zisk_inst}
    (hp : p ∈ r) : romLookup r p.1 = some p.2 := by
  induction r with
  | nil => cases hp
  | cons q rest ih =>
    simp only [List.map_cons, List.nodup_cons] at hn
    simp only [romLookup, List.find?_cons] at ih ⊢
    rcases List.mem_cons.1 hp with rfl | hp'
    · simp
    · have hne : (q.1 == p.1) = false := by
        simp only [beq_eq_false_iff_ne]; intro e
        exact hn.1 (e ▸ List.mem_map_of_mem hp')
      rw [hne]; exact ih hn.2 hp'

end Transpiler
