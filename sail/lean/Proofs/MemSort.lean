/-
  Sorted memory checking, in the abstract.

  ZisK's Mem AIR checks memory as a log of accesses sorted by address, then by
  step. Each lane holds one access — an address, a step, a value, and whether
  it writes — and, with `dual_mem`, possibly a second read of the same address
  and value at a later step (`step_dual`). Against the lane before it, a lane
  either keeps the address, with a step no lower than the previous lane's last
  one (strictly higher for a write), a read returning the previous value; or
  moves to a higher address, a read there returning the initial value.

  `reads_last_write`: then every read the log emits returns the value of the
  latest write to its address at or before its step, or, if there is none,
  the initial value. The proof walks the log lane by lane, keeping three
  facts: every access so far is at or below the last lane's address, and at
  that address at or below its last step; the last lane's value is the
  address's current one; and every read so far returns the latest write.
-/

namespace Correspondence.MemSort

variable {V : Type}

/-- One lane of the sorted log: an access, and possibly a second read at
`dual`. -/
structure Lane (V : Type) where
  addr : Nat
  step : Nat
  wr : Bool
  val : V
  dual : Option Nat

/-- The step of the lane's last access. -/
def Lane.last (q : Lane V) : Nat := q.dual.getD q.step

/-- An access the log emits: an address, a step, a value. -/
structure Emit (V : Type) where
  addr : Nat
  step : Nat
  val : V

/-- The reads a lane emits: its access if it is not a write, and its dual
read. -/
def Lane.reads (q : Lane V) : List (Emit V) :=
  (if q.wr then [] else [⟨q.addr, q.step, q.val⟩]) ++
    (match q.dual with | some sd => [⟨q.addr, sd, q.val⟩] | none => [])

/-- The write a lane emits, if it writes. -/
def Lane.writes (q : Lane V) : List (Emit V) := if q.wr then [⟨q.addr, q.step, q.val⟩] else []

/-- A lane's dual read comes no earlier than its access, and strictly later
after a write. -/
def Lane.Ok (q : Lane V) : Prop := ∀ sd, q.dual = some sd → q.step + (if q.wr then 1 else 0) ≤ sd

/-- Lane `q` follows lane `p` in the sorted log. -/
def Follows (v₀ : V) (p q : Lane V) : Prop :=
  (q.addr = p.addr ∧ p.last + (if q.wr then 1 else 0) ≤ q.step ∧ (q.wr = false → q.val = p.val)) ∨
    (p.addr < q.addr ∧ (q.wr = false → q.val = v₀))

/-- A sorted log, built lane by lane. -/
inductive Sorted (v₀ : V) : List (Lane V) → Prop
  | one (q : Lane V) : q.Ok → (q.wr = false → q.val = v₀) → Sorted v₀ [q]
  | snoc (l : List (Lane V)) (p q : Lane V) : Sorted v₀ (l ++ [p]) → q.Ok → Follows v₀ p q →
      Sorted v₀ (l ++ [p] ++ [q])

/-- Every read the log emits. -/
def reads (l : List (Lane V)) : List (Emit V) := l.flatMap Lane.reads

/-- Every write the log emits. -/
def writes (l : List (Lane V)) : List (Emit V) := l.flatMap Lane.writes

/-- Read `e` returns the latest write to its address at or before its step,
or the initial value if there is none. -/
def ReadsLatest (v₀ : V) (ws : List (Emit V)) (e : Emit V) : Prop :=
  (∃ w ∈ ws, w.addr = e.addr ∧ w.step ≤ e.step ∧ w.val = e.val ∧
      ∀ w' ∈ ws, w'.addr = e.addr → w'.step ≤ e.step → w'.step ≤ w.step) ∨
    (e.val = v₀ ∧ ∀ w' ∈ ws, w'.addr = e.addr → ¬ w'.step ≤ e.step)

/-- The invariant, for a log whose last lane is `p`. -/
structure Inv (v₀ : V) (l : List (Lane V)) (p : Lane V) : Prop where
  /-- Every access is at or below `p`'s address, and at it, at or below `p`'s
  last step. -/
  below : ∀ e ∈ reads l ++ writes l, e.addr ≤ p.addr ∧ (e.addr = p.addr → e.step ≤ p.last)
  /-- `p`'s value is its address's current one. -/
  cur : ReadsLatest v₀ (writes l) ⟨p.addr, p.last, p.val⟩
  /-- Every read returns the latest write. -/
  ok : ∀ e ∈ reads l, ReadsLatest v₀ (writes l) e

theorem reads_snoc (l : List (Lane V)) (q : Lane V) : reads (l ++ [q]) = reads l ++ q.reads := by
  simp [reads]

theorem writes_snoc (l : List (Lane V)) (q : Lane V) : writes (l ++ [q]) = writes l ++ q.writes := by
  simp [writes]

theorem Lane.step_le_last {q : Lane V} (h : q.Ok) : q.step ≤ q.last := by
  unfold Lane.last
  cases hd : q.dual with
  | none => exact Nat.le_refl _
  | some sd => have := h sd hd; simp only [Option.getD_some]; split at this <;> omega

theorem Lane.mem_reads {q : Lane V} (hq : q.Ok) {e : Emit V} (he : e ∈ q.reads) :
    e.addr = q.addr ∧ e.val = q.val ∧ q.step ≤ e.step ∧ e.step ≤ q.last ∧
      (q.wr = true → q.step < e.step) := by
  have hl := Lane.step_le_last hq
  cases hw : q.wr <;> cases hd : q.dual <;>
    simp only [Lane.reads, hw, hd, if_true, Bool.false_eq_true, if_false, List.nil_append,
      List.singleton_append, List.append_nil, List.mem_cons, List.mem_singleton,
      List.not_mem_nil, or_false] at he
  · subst he; exact ⟨rfl, rfl, Nat.le_refl _, hl, fun h => nomatch h⟩
  · rename_i sd
    have h' := hq sd hd
    simp only [hw, Bool.false_eq_true, if_false] at h'
    have hlast : q.last = sd := by simp [Lane.last, hd]
    rcases he with rfl | rfl
    · exact ⟨rfl, rfl, Nat.le_refl _, hl, fun h => nomatch h⟩
    · exact ⟨rfl, rfl, by simp only; omega, by simp only; omega, fun h => nomatch h⟩
  · rename_i sd
    have h' := hq sd hd
    simp only [hw, if_true] at h'
    have hlast : q.last = sd := by simp [Lane.last, hd]
    subst he
    exact ⟨rfl, rfl, by simp only; omega, by simp only; omega, fun _ => by simp only; omega⟩

theorem Lane.mem_writes {q : Lane V} {e : Emit V} (he : e ∈ q.writes) :
    q.wr = true ∧ e = ⟨q.addr, q.step, q.val⟩ := by
  unfold Lane.writes at he
  split at he
  · rename_i hw; exact ⟨hw, List.mem_singleton.mp he⟩
  · exact nomatch he

/-- Writes past a read's step, or at other addresses, do not change what it
reads. -/
theorem ReadsLatest.extend {v₀ : V} {ws extra : List (Emit V)} {e : Emit V}
    (h : ReadsLatest v₀ ws e) (hx : ∀ w ∈ extra, w.addr = e.addr → e.step < w.step) :
    ReadsLatest v₀ (ws ++ extra) e := by
  rcases h with ⟨w, hw, ha, hs, hv, hmax⟩ | ⟨hv, hnone⟩
  · refine .inl ⟨w, List.mem_append_left _ hw, ha, hs, hv, fun w' hw' ha' hs' => ?_⟩
    rcases List.mem_append.mp hw' with hw' | hw'
    · exact hmax w' hw' ha' hs'
    · have := hx w' hw' ha'; omega
  · refine .inr ⟨hv, fun w' hw' ha' hs' => ?_⟩
    rcases List.mem_append.mp hw' with hw' | hw'
    · exact hnone w' hw' ha' hs'
    · have := hx w' hw' ha'; omega

/-- When every write to the address is at or before both steps, a read at
either step returns the same. -/
theorem ReadsLatest.restep {v₀ : V} {ws : List (Emit V)} {a s s' : Nat} {v : V}
    (h : ReadsLatest v₀ ws ⟨a, s, v⟩) (hall : ∀ w ∈ ws, w.addr = a → w.step ≤ s ∧ w.step ≤ s') :
    ReadsLatest v₀ ws ⟨a, s', v⟩ := by
  rcases h with ⟨w, hw, ha, -, hv, hmax⟩ | ⟨hv, hnone⟩
  · exact .inl ⟨w, hw, ha, (hall w hw ha).2, hv, fun w' hw' ha' _ => hmax w' hw' ha' (hall w' hw' ha').1⟩
  · exact .inr ⟨hv, fun w' hw' ha' _ => hnone w' hw' ha' (hall w' hw' ha').1⟩

/-- **One lane on.** -/
theorem Inv.snoc {v₀ : V} {l : List (Lane V)} {p q : Lane V} (h : Inv v₀ l p) (hq : q.Ok)
    (hf : Follows v₀ p q) : Inv v₀ (l ++ [q]) q := by
  have hql := Lane.step_le_last hq
  -- Old accesses at `q`'s address are before `q`'s step, and below or at its address.
  have hold : ∀ e ∈ reads l ++ writes l, e.addr ≤ q.addr ∧
      (e.addr = q.addr → e.step + (if q.wr then 1 else 0) ≤ q.step) := by
    intro e he
    obtain ⟨h1, h2⟩ := h.below e he
    rcases hf with ⟨ha, hs, -⟩ | ⟨ha, -⟩
    · exact ⟨by omega, fun he' => by have := h2 (by omega); omega⟩
    · exact ⟨by omega, fun he' => by omega⟩
  have hwold : ∀ w ∈ writes l, w.addr = q.addr → w.step + (if q.wr then 1 else 0) ≤ q.step :=
    fun w hw => (hold w (List.mem_append_right _ hw)).2
  -- `q`'s write, if any, does not change old reads.
  have hext : ∀ e ∈ reads l, ∀ w ∈ q.writes, w.addr = e.addr → e.step < w.step := by
    intro e he w hw ha
    obtain ⟨hwr, rfl⟩ := Lane.mem_writes hw
    have := (hold e (List.mem_append_left _ he)).2 ha.symm
    simp only [hwr, if_true] at this
    simp only; omega
  have hW : writes (l ++ [q]) = writes l ++ q.writes := writes_snoc l q
  -- `q`'s value is its address's current one.
  have hcur : ReadsLatest v₀ (writes l ++ q.writes) ⟨q.addr, q.last, q.val⟩ := by
    cases hw : q.wr
    · have hqw : q.writes = [] := by simp [Lane.writes, hw]
      rw [hqw, List.append_nil]
      rcases hf with ⟨ha, hs, hv⟩ | ⟨ha, hv⟩
      · rw [ha, hv hw]
        apply h.cur.restep
        intro w hw' ha'
        have := (h.below w (List.mem_append_right _ hw')).2 ha'
        have : p.last ≤ q.last := by simp only [hw, Bool.false_eq_true, if_false] at hs; omega
        omega
      · refine .inr ⟨hv hw, fun w' hw' ha' _ => ?_⟩
        have := (hold w' (List.mem_append_right _ hw')).1
        have := (h.below w' (List.mem_append_right _ hw')).1
        simp only at ha'; omega
    · refine .inl ⟨⟨q.addr, q.step, q.val⟩, List.mem_append_right _ (by simp [Lane.writes, hw]),
        rfl, hql, rfl, fun w' hw' ha' _ => ?_⟩
      rcases List.mem_append.mp hw' with hw' | hw'
      · have := hwold w' hw' ha'; simp only [hw, if_true] at this; simp only; omega
      · obtain ⟨-, rfl⟩ := Lane.mem_writes hw'; exact Nat.le_refl _
  refine ⟨?_, hW ▸ hcur, ?_⟩
  · intro e he
    rw [reads_snoc, writes_snoc] at he
    simp only [List.mem_append] at he
    rcases he with (he | he) | (he | he)
    · obtain ⟨h1, h2⟩ := hold e (List.mem_append_left _ he)
      exact ⟨h1, fun ha => by have := h2 ha; split at this <;> omega⟩
    · obtain ⟨h1, -, -, h4, -⟩ := Lane.mem_reads hq he
      exact ⟨by omega, fun _ => h4⟩
    · obtain ⟨h1, h2⟩ := hold e (List.mem_append_right _ he)
      exact ⟨h1, fun ha => by have := h2 ha; split at this <;> omega⟩
    · obtain ⟨-, rfl⟩ := Lane.mem_writes he
      exact ⟨Nat.le_refl _, fun _ => hql⟩
  · intro e he
    rw [hW]
    rw [reads_snoc] at he
    rcases List.mem_append.mp he with he | he
    · exact (h.ok e he).extend (hext e he)
    · obtain ⟨h1, h2, h3, h4, h5⟩ := Lane.mem_reads hq he
      have : e = ⟨q.addr, e.step, q.val⟩ := by cases e; simp only at h1 h2; subst h1 h2; rfl
      rw [this]
      apply hcur.restep
      intro w hw ha
      rcases List.mem_append.mp hw with hw | hw
      · have := hwold w hw ha; split at this <;> omega
      · obtain ⟨hwr, rfl⟩ := Lane.mem_writes hw
        exact ⟨hql, h3⟩

/-- **The first lane.** -/
theorem Inv.first {v₀ : V} {q : Lane V} (hq : q.Ok) (hv : q.wr = false → q.val = v₀) :
    Inv v₀ [q] q := by
  have hql := Lane.step_le_last hq
  have hR : reads [q] = q.reads := by simp [reads]
  have hW : writes [q] = q.writes := by simp [writes]
  have hcur : ReadsLatest v₀ q.writes ⟨q.addr, q.last, q.val⟩ := by
    cases hw : q.wr
    · exact .inr ⟨hv hw, fun w' hw' => by have := (Lane.mem_writes hw').1; rw [hw] at this; exact nomatch this⟩
    · refine .inl ⟨⟨q.addr, q.step, q.val⟩, by simp [Lane.writes, hw], rfl, hql, rfl,
        fun w' hw' _ _ => ?_⟩
      obtain ⟨-, rfl⟩ := Lane.mem_writes hw'; exact Nat.le_refl _
  refine ⟨?_, hW ▸ hcur, ?_⟩
  · intro e he
    rw [hR, hW] at he
    rcases List.mem_append.mp he with he | he
    · obtain ⟨h1, -, -, h4, -⟩ := Lane.mem_reads hq he
      exact ⟨by omega, fun _ => h4⟩
    · obtain ⟨-, rfl⟩ := Lane.mem_writes he
      exact ⟨Nat.le_refl _, fun _ => hql⟩
  · intro e he
    rw [hR] at he
    rw [hW]
    obtain ⟨h1, h2, h3, h4, h5⟩ := Lane.mem_reads hq he
    have : e = ⟨q.addr, e.step, q.val⟩ := by cases e; simp only at h1 h2; subst h1 h2; rfl
    rw [this]
    apply hcur.restep
    intro w hw ha
    obtain ⟨hwr, rfl⟩ := Lane.mem_writes hw
    exact ⟨hql, h3⟩

/-- The invariant holds along a sorted log. -/
theorem Sorted.inv {v₀ : V} {L : List (Lane V)} (h : Sorted v₀ L) :
    ∃ p, L.getLast? = some p ∧ Inv v₀ L p := by
  induction h with
  | one q hq hv =>
    exact ⟨q, rfl, Inv.first hq hv⟩
  | snoc l p q _ hq hf ih =>
    obtain ⟨p', hp', hinv⟩ := ih
    simp at hp'
    subst hp'
    exact ⟨q, by simp, hinv.snoc hq hf⟩

/-- **Every read returns the last write.** Along a sorted log, every read
returns the value of the latest write to its address at or before its step,
or, if there is none, the initial value. -/
theorem reads_last_write {v₀ : V} {L : List (Lane V)} (h : Sorted v₀ L) :
    ∀ e ∈ reads L, ReadsLatest v₀ (writes L) e := by
  obtain ⟨p, -, hinv⟩ := h.inv
  exact hinv.ok

end Correspondence.MemSort
