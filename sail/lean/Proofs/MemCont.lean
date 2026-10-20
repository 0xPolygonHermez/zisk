/-
  The continuation buses, for the RAM, InputData and RomData.

  Bus 11 links the RAM's segments and InputData's, each area under its own
  tag — its base address — and bus 12 RomData's. Their senders are the
  areas' two arguments a segment (`contSenders`, from the census of the
  buses), and their global updates open each area's chain (`GlobalCont`,
  written from `mem.pil` and `rom_data.pil`, as the global updates are not
  extracted). On sound buses, `ContBal.chain` orders each area's segments
  into a chain: what `RamChain`, `InWorld` and `RdWorld` take.
-/
import Proofs.Continuation
import Proofs.MemFromBus

namespace Correspondence

open Pil.Zisk MemSort
open Classical

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

set_option linter.unusedSectionVars false

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-! ### Sums over the senders -/

theorem foldr_sum_append (l1 l2 : List F) :
    (l1 ++ l2).foldr (· + ·) 0 = l1.foldr (· + ·) 0 + l2.foldr (· + ·) 0 := by
  induction l1 with
  | nil => simp only [List.nil_append, List.foldr_nil, Lean.Grind.Semiring.zero_add]
  | cons a l ih => simp only [List.cons_append, List.foldr_cons, ih, Lean.Grind.Semiring.add_assoc]

theorem provedCount_flatMap {α : Type _} (l : List α) (f : α → List (Sender F)) (τ : List F) :
    provedCount (l.flatMap f) τ = (l.map fun a => provedCount (f a) τ).foldr (· + ·) 0 := by
  induction l with
  | nil => rfl
  | cons a l ih => rw [List.flatMap_cons, provedCount_append, ih]; rfl

theorem assumedCount_flatMap {α : Type _} (l : List α) (f : α → List (Sender F)) (τ : List F) :
    assumedCount (l.flatMap f) τ = (l.map fun a => assumedCount (f a) τ).foldr (· + ·) 0 := by
  induction l with
  | nil => rfl
  | cons a l ih => rw [List.flatMap_cons, assumedCount_append, ih]; rfl

theorem sum_ind {α : Type _} (l : List α) (c : α → Bool) :
    (l.map fun a => if c a = true then (1 : F) else 0).foldr (· + ·) 0 = ((l.countP c : Nat) : F) := by
  induction l with
  | nil => simp [Lean.Grind.Semiring.natCast_zero]
  | cons a l ih =>
    simp only [List.map_cons, List.foldr_cons, ih, List.countP_cons]
    rw [Lean.Grind.Semiring.natCast_add]
    cases c a
    · simp [Lean.Grind.Semiring.natCast_zero, Lean.Grind.Semiring.zero_add, Lean.Grind.Semiring.add_zero]
    · simp only [if_true, Lean.Grind.Semiring.natCast_one]
      generalize ((l.countP c : Nat) : F) = x
      grind

theorem sum_zero {α : Type _} (l : List α) (g : α → F) (h : ∀ a ∈ l, g a = 0) :
    (l.map g).foldr (· + ·) 0 = 0 := by
  induction l with
  | nil => rfl
  | cons a l ih =>
    simp only [List.map_cons, List.foldr_cons, h a (List.mem_cons_self ..),
      ih (fun b hb => h b (List.mem_cons_of_mem _ hb)), Lean.Grind.Semiring.add_zero]

theorem count_once {s : Sender F} (ho : s.p.arg.once = true) (τ : List F) :
    s.count τ = if tupleOf s.p 0 = τ then s.p.arg.reps s.p.x s.p.t 0 else 0 := by
  simp only [Sender.count, Sender.at, ho, if_true, List.map_cons, List.map_nil, List.foldr_cons,
    List.foldr_nil, Lean.Grind.Semiring.add_zero]

/-- A count of `1 - last`, `last` a bit, is an indicator. -/
theorem live_ind {last : F} (h : last * (1 - last) = 0) (c : Bool) :
    (if c = true then 1 - last else 0) = if (decide (last = 0) && c) = true then (1 : F) else 0 := by
  obtain ⟨b, hb⟩ := bool_of_idem h
  cases b <;> cases c <;> simp [hb, bit, Lean.Grind.Field.zero_ne_one.symm] <;> grind

theorem tag_ne {τ τ' : List F} : ((1073741824 : F) :: τ) ≠ (2684354560 : F) :: τ' := by
  intro h
  simp only [List.cons.injEq] at h
  have e : ((1073741824 : Nat) : F) = ((2684354560 : Nat) : F) := by
    rw [← natCast_ofNat' (F := F) 1073741824, ← natCast_ofNat' (F := F) 2684354560]; exact h.1
  have := natCast_inj33 (F := F) (by omega) (by omega) e
  omega

/-! ### The continuation buses' senders and global updates -/

/-- The RAM's arguments on bus 11. -/
def MemOnMem.cont (I : MemOnMem F) : List (Sender F) :=
  [⟨⟨Mem.bus7, I.x, I.t⟩, Mem.numRows⟩, ⟨⟨Mem.bus8, I.x, I.t⟩, Mem.numRows⟩]

/-- InputData's arguments on bus 11. -/
def InputDataOnMem.cont (I : InputDataOnMem F) : List (Sender F) :=
  [⟨⟨InputData.bus14, I.x, I.t⟩, InputData.numRows⟩, ⟨⟨InputData.bus15, I.x, I.t⟩, InputData.numRows⟩]

/-- **Bus 11's senders**: the RAM's and InputData's, by the census of the bus
(no other AIR sends on it). -/
def contSenders (S : MemOthers F) : List (Sender F) :=
  S.mem.flatMap MemOnMem.cont ++ S.inputData.flatMap InputDataOnMem.cont

/-- The tuple opening the RAM's chain: segment 0, first, from the RAM's base
word, at step 0, with value 0. -/
def ramG : List F := [0, 1, 335544320, 0, 0, 0]

/-- The tuple opening InputData's chain. -/
def inG : List F := [0, 1, 134217728, 0, 0, 0]

/-- **The global updates on bus 11**: each area's opens its chain, its
`enable_flag` times — a bit — and none assumes one of their tuples. Written
from `mem.pil`. -/
structure GlobalCont (gp ga : List (List F)) (eR eI : Nat) : Prop where
  eR1 : eR ≤ 1
  eI1 : eI ≤ 1
  ram : ∀ τ, times gp ((2684354560 : F) :: τ) = ((eR * (if τ = ramG then 1 else 0) : Nat) : F) ∧
    times ga ((2684354560 : F) :: τ) = 0
  input : ∀ τ, times gp ((1073741824 : F) :: τ) = ((eI * (if τ = inG then 1 else 0) : Nat) : F) ∧
    times ga ((1073741824 : F) :: τ) = 0

/-! ### The RAM's chain -/

/-- What a RAM segment assumes, after the tag. -/
def ramA (I : MemOnMem F) : List F :=
  [I.x.Mem_segment_id, I.x.Mem_is_first_segment, I.x.Mem_previous_segment_addr,
    I.x.Mem_previous_segment_step, I.x.Mem_previous_segment_value 0, I.x.Mem_previous_segment_value 1]

/-- What a RAM segment proves, after the tag. -/
def ramP (I : MemOnMem F) : List F :=
  [I.x.Mem_segment_id + 1, 0, I.x.Mem_segment_last_addr, I.x.Mem_segment_last_step,
    I.x.Mem_segment_last_value 0, I.x.Mem_segment_last_value 1]

theorem ram_cont_counts (I : MemOnMem F) (τ : List F) :
    provedCount I.cont ((2684354560 : F) :: τ) =
        (if (decide (I.x.Mem_is_last_segment = 0) && decide (ramP I = τ)) = true then 1 else 0) ∧
      assumedCount I.cont ((2684354560 : F) :: τ) = if decide (ramA I = τ) = true then 1 else 0 := by
  have h7 : tupleOf (⟨Mem.bus7, I.x, I.t⟩ : Prover F) 0 = (2684354560 : F) :: ramA I := by
    simp [tupleOf, Mem.bus7, ramA, Lean.Grind.Semiring.add_zero]
  have h8 : tupleOf (⟨Mem.bus8, I.x, I.t⟩ : Prover F) 0 = (2684354560 : F) :: ramP I := by
    simp [tupleOf, Mem.bus8, ramP, Lean.Grind.Semiring.add_zero]
  have hbit := (I.rows 0).c1
  simp only [Mem.c1] at hbit
  have p7 : (⟨⟨Mem.bus7, I.x, I.t⟩, Mem.numRows⟩ : Sender F).proving = false := rfl
  have p8 : (⟨⟨Mem.bus8, I.x, I.t⟩, Mem.numRows⟩ : Sender F).proving = true := rfl
  constructor
  · simp only [provedCount, MemOnMem.cont, List.filter_cons, List.filter_nil, p7, p8, Bool.not_false,
      if_true, Bool.false_eq_true, if_false, List.map_cons, List.map_nil, List.foldr_cons, List.foldr_nil,
      Lean.Grind.Semiring.add_zero]
    rw [count_once rfl, h8]
    simp only [List.cons.injEq, true_and]
    rw [← live_ind hbit (decide (ramP I = τ))]
    simp [Mem.bus8]
  · simp only [assumedCount, MemOnMem.cont, List.filter_cons, List.filter_nil, p7, p8, Bool.not_false,
      Bool.not_true, if_true, Bool.false_eq_true, if_false, List.map_cons, List.map_nil, List.foldr_cons,
      List.foldr_nil, Lean.Grind.Semiring.add_zero]
    rw [count_once rfl, h7]
    simp [Mem.bus7]

theorem in_cont_ram (I : InputDataOnMem F) (τ : List F) :
    provedCount I.cont ((2684354560 : F) :: τ) = 0 ∧ assumedCount I.cont ((2684354560 : F) :: τ) = 0 := by
  have hne : ∀ τ' : List F, ((1073741824 : F) :: τ') ≠ (2684354560 : F) :: τ :=
    fun τ' => tag_ne
  have h14 : tupleOf (⟨InputData.bus14, I.x, I.t⟩ : Prover F) 0 ≠ (2684354560 : F) :: τ := by
    simp only [tupleOf, InputData.bus14, List.map_cons, List.map_nil]; exact hne _
  have h15 : tupleOf (⟨InputData.bus15, I.x, I.t⟩ : Prover F) 0 ≠ (2684354560 : F) :: τ := by
    simp only [tupleOf, InputData.bus15, List.map_cons, List.map_nil]; exact hne _
  have p14 : (⟨⟨InputData.bus14, I.x, I.t⟩, InputData.numRows⟩ : Sender F).proving = false := rfl
  have p15 : (⟨⟨InputData.bus15, I.x, I.t⟩, InputData.numRows⟩ : Sender F).proving = true := rfl
  constructor
  · simp only [provedCount, InputDataOnMem.cont, List.filter_cons, List.filter_nil, p14, p15,
      if_true, Bool.false_eq_true, if_false, List.map_cons, List.map_nil, List.foldr_cons, List.foldr_nil,
      Lean.Grind.Semiring.add_zero]
    rw [count_once rfl, if_neg h15]
  · simp only [assumedCount, InputDataOnMem.cont, List.filter_cons, List.filter_nil, p14, p15,
      Bool.not_false, Bool.not_true, if_true, Bool.false_eq_true, if_false, List.map_cons, List.map_nil,
      List.foldr_cons, List.foldr_nil, Lean.Grind.Semiring.add_zero]
    rw [count_once rfl, if_neg h14]

/-- **The RAM's continuation bus balances, segment by segment.** -/
theorem ram_contBal {S : MemOthers F} {gc ac : List (List F)} {eR eI : Nat}
    (hsum : SumHolds 11 (contSenders S) gc ac) (hg : GlobalCont gc ac eR eI) (hsmall : S.mem.length < 2 ^ 32) :
    ContBal S.mem.length (fun i => (S.mem[i]?).elim [] ramA) (fun i => (S.mem[i]?).elim [] ramP)
      (fun i => (S.mem[i]?).elim false fun I => decide (I.x.Mem_is_last_segment = 0)) eR ramG := by
  apply contBal_of_counts _ _ _ _ _ _ hsmall hg.eR1
  · intro τ
    have e := hsum.2 ((2684354560 : F) :: τ)
    obtain ⟨hgp, hga⟩ := hg.ram τ
    rw [hgp, hga, Lean.Grind.Semiring.add_zero] at e
    unfold contSenders at e
    rw [provedCount_append, assumedCount_append, provedCount_flatMap, assumedCount_flatMap,
      provedCount_flatMap, assumedCount_flatMap] at e
    rw [sum_zero _ _ (fun I _ => (in_cont_ram I τ).1), sum_zero _ _ (fun I _ => (in_cont_ram I τ).2),
      Lean.Grind.Semiring.add_zero, Lean.Grind.Semiring.add_zero] at e
    have e1 : (S.mem.map fun I => provedCount I.cont ((2684354560 : F) :: τ)) =
        S.mem.map fun I => if (decide (I.x.Mem_is_last_segment = 0) && decide (ramP I = τ)) = true
          then (1 : F) else 0 := List.map_congr_left fun I _ => (ram_cont_counts I τ).1
    have e2 : (S.mem.map fun I => assumedCount I.cont ((2684354560 : F) :: τ)) =
        S.mem.map fun I => if decide (ramA I = τ) = true then (1 : F) else 0 :=
      List.map_congr_left fun I _ => (ram_cont_counts I τ).2
    rw [e1, e2, sum_ind, sum_ind] at e
    exact e
  · rfl
  · intro I _
    simp only [ramA, ramP, el, List.getD_cons_zero]

/-- **The RAM's chain, from its continuation bus**: its instances, in the
order the bus chains them, make a `RamChain`. -/
theorem MemBusWorld.ram_chain {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) (hinst : ∀ I ∈ S.mem, RamInst I) {gc ac : List (List F)} {eR eI : Nat}
    (hsum : SumHolds 11 (contSenders S) gc ac) (hg : GlobalCont gc ac eR eI)
    (hsmall : S.mem.length < 2 ^ 32) :
    ∃ hram : RamSegs S, RamChain (W.segs hram) := by
  obtain ⟨order, hsub, hall, h0, hnext⟩ := chain_list (ram_contBal hsum hg hsmall)
  let hram : RamSegs S := ⟨hinst, order.attach.map fun I => ⟨I.1, hsub I.1 I.2⟩, fun I => by
    refine List.mem_map.mpr ⟨⟨I.1, hall I.1 I.2⟩, List.mem_attach _ _, rfl⟩⟩
  have hlen : (W.segs hram).length = order.length := by simp [MemBusWorld.segs, hram]
  have hx : ∀ k (hk : k < (W.segs hram).length), (W.segs hram)[k].x = (order[k]'(by omega)).x := by
    intro k hk
    simp only [MemBusWorld.segs, hram, List.getElem_map, List.getElem_attach]
    rfl
  refine ⟨hram, ⟨fun hpos => ?_, fun k hk => ?_⟩⟩
  · have := h0 (by omega)
    rw [hx 0 hpos]
    simp only [ramA, ramG, List.cons.injEq] at this
    exact this.2.1
  · obtain ⟨-, hp⟩ := hnext k (by omega)
    rw [hx k (by omega), hx (k + 1) hk]
    simp only [ramA, ramP, List.cons.injEq] at hp
    obtain ⟨-, h1, h2, h3, h4, h5, -⟩ := hp
    exact ⟨h1.symm, h2.symm, h3.symm, h4.symm, h5.symm⟩

/-! ### InputData's chain -/

/-- What an InputData instance needs beyond its rows: the trace's period,
`__L1__` as the setup fixes it, and its range checks sound. -/
structure InInst (I : InputDataOnMem F) : Prop where
  cyclic : InputData.cyclic I.t
  first : (I.t 0).__L1__ = 1
  inner : ∀ i : Int, 0 < i → i < InputData.numRows → (I.t i).__L1__ = 0
  range : RangeHolds InputData.ranges InputData.bus25 I.x I.t
  base0 : RangeHolds InputData.ranges InputData.bus17 I.x I.t
  base1 : RangeHolds InputData.ranges InputData.bus19 I.x I.t
  end0 : RangeHolds InputData.ranges InputData.bus21 I.x I.t
  end1 : RangeHolds InputData.ranges InputData.bus23 I.x I.t

/-- The instance as `MemInput.lean` takes it. -/
def InInst.trace {I : InputDataOnMem F} (h : InInst I) : InTrace F :=
  ⟨I.x, I.t, I.rows, h.cyclic, h.first, h.inner, h.range, h.base0, h.base1, h.end0, h.end1⟩

/-- What an InputData segment assumes, after the tag. -/
def inA (I : InputDataOnMem F) : List F :=
  [I.x.InputData_segment_id, I.x.InputData_is_first_segment, I.x.InputData_previous_segment_addr,
    I.x.InputData_previous_segment_step, I.x.InputData_previous_segment_value 0,
    I.x.InputData_previous_segment_value 1]

/-- What an InputData segment proves, after the tag. -/
def inP (I : InputDataOnMem F) : List F :=
  [I.x.InputData_segment_id + 1, 0, I.x.InputData_segment_last_addr, I.x.InputData_segment_last_step,
    I.x.InputData_segment_last_value 0, I.x.InputData_segment_last_value 1]

theorem in_cont_counts (I : InputDataOnMem F) (τ : List F) :
    provedCount I.cont ((1073741824 : F) :: τ) =
        (if (decide (I.x.InputData_is_last_segment = 0) && decide (inP I = τ)) = true then 1 else 0) ∧
      assumedCount I.cont ((1073741824 : F) :: τ) = if decide (inA I = τ) = true then 1 else 0 := by
  have h14 : tupleOf (⟨InputData.bus14, I.x, I.t⟩ : Prover F) 0 = (1073741824 : F) :: inA I := by
    simp [tupleOf, InputData.bus14, inA, Lean.Grind.Semiring.add_zero]
  have h15 : tupleOf (⟨InputData.bus15, I.x, I.t⟩ : Prover F) 0 = (1073741824 : F) :: inP I := by
    simp [tupleOf, InputData.bus15, inP, Lean.Grind.Semiring.add_zero]
  have hbit := (I.rows 0).c1
  simp only [InputData.c1] at hbit
  have p14 : (⟨⟨InputData.bus14, I.x, I.t⟩, InputData.numRows⟩ : Sender F).proving = false := rfl
  have p15 : (⟨⟨InputData.bus15, I.x, I.t⟩, InputData.numRows⟩ : Sender F).proving = true := rfl
  constructor
  · simp only [provedCount, InputDataOnMem.cont, List.filter_cons, List.filter_nil, p14, p15,
      Bool.not_false, if_true, Bool.false_eq_true, if_false, List.map_cons, List.map_nil, List.foldr_cons,
      List.foldr_nil, Lean.Grind.Semiring.add_zero]
    rw [count_once rfl, h15]
    simp only [List.cons.injEq, true_and]
    rw [← live_ind hbit (decide (inP I = τ))]
    simp [InputData.bus15]
  · simp only [assumedCount, InputDataOnMem.cont, List.filter_cons, List.filter_nil, p14, p15,
      Bool.not_false, Bool.not_true, if_true, Bool.false_eq_true, if_false, List.map_cons, List.map_nil,
      List.foldr_cons, List.foldr_nil, Lean.Grind.Semiring.add_zero]
    rw [count_once rfl, h14]
    simp [InputData.bus14]

theorem ram_cont_in (I : MemOnMem F) (τ : List F) :
    provedCount I.cont ((1073741824 : F) :: τ) = 0 ∧ assumedCount I.cont ((1073741824 : F) :: τ) = 0 := by
  have hne : ∀ τ' : List F, ((2684354560 : F) :: τ') ≠ (1073741824 : F) :: τ :=
    fun τ' h => tag_ne h.symm
  have h7 : tupleOf (⟨Mem.bus7, I.x, I.t⟩ : Prover F) 0 ≠ (1073741824 : F) :: τ := by
    simp only [tupleOf, Mem.bus7, List.map_cons, List.map_nil]; exact hne _
  have h8 : tupleOf (⟨Mem.bus8, I.x, I.t⟩ : Prover F) 0 ≠ (1073741824 : F) :: τ := by
    simp only [tupleOf, Mem.bus8, List.map_cons, List.map_nil]; exact hne _
  have p7 : (⟨⟨Mem.bus7, I.x, I.t⟩, Mem.numRows⟩ : Sender F).proving = false := rfl
  have p8 : (⟨⟨Mem.bus8, I.x, I.t⟩, Mem.numRows⟩ : Sender F).proving = true := rfl
  constructor
  · simp only [provedCount, MemOnMem.cont, List.filter_cons, List.filter_nil, p7, p8, if_true,
      Bool.false_eq_true, if_false, List.map_cons, List.map_nil, List.foldr_cons, List.foldr_nil,
      Lean.Grind.Semiring.add_zero]
    rw [count_once rfl, if_neg h8]
  · simp only [assumedCount, MemOnMem.cont, List.filter_cons, List.filter_nil, p7, p8, Bool.not_false,
      Bool.not_true, if_true, Bool.false_eq_true, if_false, List.map_cons, List.map_nil, List.foldr_cons,
      List.foldr_nil, Lean.Grind.Semiring.add_zero]
    rw [count_once rfl, if_neg h7]

/-- **InputData's continuation bus balances, segment by segment.** -/
theorem in_contBal {S : MemOthers F} {gc ac : List (List F)} {eR eI : Nat}
    (hsum : SumHolds 11 (contSenders S) gc ac) (hg : GlobalCont gc ac eR eI)
    (hsmall : S.inputData.length < 2 ^ 32) :
    ContBal S.inputData.length (fun i => (S.inputData[i]?).elim [] inA) (fun i => (S.inputData[i]?).elim [] inP)
      (fun i => (S.inputData[i]?).elim false fun I => decide (I.x.InputData_is_last_segment = 0)) eI inG := by
  apply contBal_of_counts _ _ _ _ _ _ hsmall hg.eI1
  · intro τ
    have e := hsum.2 ((1073741824 : F) :: τ)
    obtain ⟨hgp, hga⟩ := hg.input τ
    rw [hgp, hga, Lean.Grind.Semiring.add_zero] at e
    unfold contSenders at e
    rw [provedCount_append, assumedCount_append, provedCount_flatMap, assumedCount_flatMap,
      provedCount_flatMap, assumedCount_flatMap] at e
    rw [sum_zero _ _ (fun I _ => (ram_cont_in I τ).1), sum_zero _ _ (fun I _ => (ram_cont_in I τ).2),
      Lean.Grind.Semiring.zero_add, Lean.Grind.Semiring.zero_add] at e
    have e1 : (S.inputData.map fun I => provedCount I.cont ((1073741824 : F) :: τ)) =
        S.inputData.map fun I => if (decide (I.x.InputData_is_last_segment = 0) && decide (inP I = τ)) = true
          then (1 : F) else 0 := List.map_congr_left fun I _ => (in_cont_counts I τ).1
    have e2 : (S.inputData.map fun I => assumedCount I.cont ((1073741824 : F) :: τ)) =
        S.inputData.map fun I => if decide (inA I = τ) = true then (1 : F) else 0 :=
      List.map_congr_left fun I _ => (in_cont_counts I τ).2
    rw [e1, e2, sum_ind, sum_ind] at e
    exact e
  · rfl
  · intro I _
    simp only [inA, inP, el, List.getD_cons_zero]

/-- **InputData's chain, from its continuation bus**: its instances, in the
order the bus chains them, make an `InWorld`. -/
theorem in_world {S : MemOthers F} (hinst : ∀ I ∈ S.inputData, InInst I) {gc ac : List (List F)}
    {eR eI : Nat} (hsum : SumHolds 11 (contSenders S) gc ac) (hg : GlobalCont gc ac eR eI)
    (hsmall : S.inputData.length < 2 ^ 32) (hne : S.inputData ≠ []) : Nonempty (InWorld S) := by
  obtain ⟨order, hsub, hall, -, hnext⟩ := chain_list (in_contBal hsum hg hsmall)
  have hpos : 0 < order.length := by
    obtain ⟨I, hI⟩ := List.exists_mem_of_ne_nil _ hne
    exact List.length_pos_of_mem (hall I hI)
  let seg : Nat → InTrace F := fun k =>
    if h : k < order.length then (hinst _ (hsub _ (List.getElem_mem h))).trace
    else (hinst _ (hsub _ (List.getElem_mem hpos))).trace
  have hseg : ∀ k (h : k < order.length), (seg k).x = order[k].x ∧ (seg k).t = order[k].t := by
    intro k h
    simp only [seg, dif_pos h]
    exact ⟨rfl, rfl⟩
  refine ⟨⟨seg, order.length, fun k hk => ?_, fun I hI => ?_⟩⟩
  · obtain ⟨-, hp⟩ := hnext k hk
    simp only [inA, inP, List.cons.injEq] at hp
    obtain ⟨-, -, h2, -, h4, h5, -⟩ := hp
    show (seg (k + 1)).x.InputData_previous_segment_addr = (seg k).x.InputData_segment_last_addr ∧
      (seg (k + 1)).x.InputData_previous_segment_value 0 = (seg k).x.InputData_segment_last_value 0 ∧
      (seg (k + 1)).x.InputData_previous_segment_value 1 = (seg k).x.InputData_segment_last_value 1
    rw [(hseg (k + 1) hk).1, (hseg k (by omega)).1]
    exact ⟨h2.symm, h4.symm, h5.symm⟩
  · obtain ⟨k, hk, hkI⟩ := List.getElem_of_mem (hall I hI)
    refine ⟨k, hk, ?_, ?_⟩
    · rw [(hseg k hk).1, hkI]
    · rw [(hseg k hk).2, hkI]

/-! ### RomData's chain -/

/-- What a RomData instance needs beyond its rows: the trace's period and
`__L1__` as the setup fixes it. -/
structure RdInst (I : RomDataOnMem F) : Prop where
  cyclic : RomData.cyclic I.t
  first : (I.t 0).__L1__ = 1
  inner : ∀ i : Int, 0 < i → i < RomData.numRows → (I.t i).__L1__ = 0

/-- RomData's arguments on bus 12. -/
def RomDataOnMem.cont (I : RomDataOnMem F) : List (Sender F) :=
  [⟨⟨RomData.bus6, I.x, I.t⟩, RomData.numRows⟩, ⟨⟨RomData.bus7, I.x, I.t⟩, RomData.numRows⟩]

/-- **Bus 12's senders**: RomData's, by the census of the bus. -/
def rdContSenders (S : MemOthers F) : List (Sender F) := S.romData.flatMap RomDataOnMem.cont

/-- The tuple opening RomData's chain: segment 0, first, from address 0, with
value 0. -/
def rdG : List F := [0, 1, 0, 0, 0]

/-- **The global updates on bus 12**: one opens RomData's chain, its
`enable_flag` times — a bit — and none assumes. Written from
`rom_data.pil`. -/
structure GlobalRd (gp ga : List (List F)) (e : Nat) : Prop where
  e1 : e ≤ 1
  all : ∀ τ, times gp τ = ((e * (if τ = rdG then 1 else 0) : Nat) : F) ∧ times ga τ = 0

/-- What a RomData segment assumes. -/
def rdA (I : RomDataOnMem F) : List F :=
  [I.x.RomData_segment_id, I.x.RomData_is_first_segment, I.x.RomData_previous_segment_addr,
    I.x.RomData_previous_segment_value 0, I.x.RomData_previous_segment_value 1]

/-- What a RomData segment proves. -/
def rdP (I : RomDataOnMem F) : List F :=
  [I.x.RomData_segment_id + 1, 0, I.x.RomData_segment_last_addr, I.x.RomData_segment_last_value 0,
    I.x.RomData_segment_last_value 1]

theorem rd_cont_counts (I : RomDataOnMem F) (τ : List F) :
    provedCount I.cont τ =
        (if (decide (I.x.RomData_is_last_segment = 0) && decide (rdP I = τ)) = true then 1 else 0) ∧
      assumedCount I.cont τ = if decide (rdA I = τ) = true then 1 else 0 := by
  have h6 : tupleOf (⟨RomData.bus6, I.x, I.t⟩ : Prover F) 0 = rdA I := by
    simp [tupleOf, RomData.bus6, rdA, Lean.Grind.Semiring.add_zero]
  have h7 : tupleOf (⟨RomData.bus7, I.x, I.t⟩ : Prover F) 0 = rdP I := by
    simp [tupleOf, RomData.bus7, rdP, Lean.Grind.Semiring.add_zero]
  have hbit := (I.rows 0).c1
  simp only [RomData.c1] at hbit
  have p6 : (⟨⟨RomData.bus6, I.x, I.t⟩, RomData.numRows⟩ : Sender F).proving = false := rfl
  have p7 : (⟨⟨RomData.bus7, I.x, I.t⟩, RomData.numRows⟩ : Sender F).proving = true := rfl
  constructor
  · simp only [provedCount, RomDataOnMem.cont, List.filter_cons, List.filter_nil, p6, p7,
      Bool.not_false, if_true, Bool.false_eq_true, if_false, List.map_cons, List.map_nil, List.foldr_cons,
      List.foldr_nil, Lean.Grind.Semiring.add_zero]
    rw [count_once rfl, h7]
    rw [← live_ind hbit (decide (rdP I = τ))]
    simp [RomData.bus7]
  · simp only [assumedCount, RomDataOnMem.cont, List.filter_cons, List.filter_nil, p6, p7,
      Bool.not_false, Bool.not_true, if_true, Bool.false_eq_true, if_false, List.map_cons, List.map_nil,
      List.foldr_cons, List.foldr_nil, Lean.Grind.Semiring.add_zero]
    rw [count_once rfl, h6]
    simp [RomData.bus6]

/-- **RomData's continuation bus balances, segment by segment.** -/
theorem rd_contBal {S : MemOthers F} {gc ac : List (List F)} {e : Nat}
    (hsum : SumHolds 12 (rdContSenders S) gc ac) (hg : GlobalRd gc ac e) (hsmall : S.romData.length < 2 ^ 32) :
    ContBal S.romData.length (fun i => (S.romData[i]?).elim [] rdA) (fun i => (S.romData[i]?).elim [] rdP)
      (fun i => (S.romData[i]?).elim false fun I => decide (I.x.RomData_is_last_segment = 0)) e rdG := by
  apply contBal_of_counts _ _ _ _ _ _ hsmall hg.e1
  · intro τ
    have e' := hsum.2 τ
    obtain ⟨hgp, hga⟩ := hg.all τ
    rw [hgp, hga, Lean.Grind.Semiring.add_zero] at e'
    unfold rdContSenders at e'
    rw [provedCount_flatMap, assumedCount_flatMap] at e'
    have e1 : (S.romData.map fun I => provedCount I.cont τ) =
        S.romData.map fun I => if (decide (I.x.RomData_is_last_segment = 0) && decide (rdP I = τ)) = true
          then (1 : F) else 0 := List.map_congr_left fun I _ => (rd_cont_counts I τ).1
    have e2 : (S.romData.map fun I => assumedCount I.cont τ) =
        S.romData.map fun I => if decide (rdA I = τ) = true then (1 : F) else 0 :=
      List.map_congr_left fun I _ => (rd_cont_counts I τ).2
    rw [e1, e2, sum_ind, sum_ind] at e'
    exact e'
  · rfl
  · intro I _
    simp only [rdA, rdP, el, List.getD_cons_zero]

/-- **RomData's chain, from its continuation bus**: its instances, in the
order the bus chains them, make an `RdWorld`. -/
theorem rd_world {S : MemOthers F} (hinst : ∀ I ∈ S.romData, RdInst I) {gc ac : List (List F)} {e : Nat}
    (hsum : SumHolds 12 (rdContSenders S) gc ac) (hg : GlobalRd gc ac e)
    (hsmall : S.romData.length < 2 ^ 32) (hne : S.romData ≠ []) : Nonempty (RdWorld S) := by
  obtain ⟨order, hsub, hall, h0, hnext⟩ := chain_list (rd_contBal hsum hg hsmall)
  have hpos : 0 < order.length := by
    obtain ⟨I, hI⟩ := List.exists_mem_of_ne_nil _ hne
    exact List.length_pos_of_mem (hall I hI)
  let seg : Nat → RomDataOnMem F := fun k => if h : k < order.length then order[k] else order[0]
  have hseg : ∀ k (h : k < order.length), seg k = order[k] := fun k h => by simp only [seg, dif_pos h]
  have hmem : ∀ k < order.length, seg k ∈ S.romData := fun k h => by
    rw [hseg k h]; exact hsub _ (List.getElem_mem h)
  refine ⟨⟨seg, order.length, hmem, fun I hI => ?_, fun k hk => (hinst _ (hmem k hk)).cyclic,
    fun k hk => (hinst _ (hmem k hk)).first, fun k hk => (hinst _ (hmem k hk)).inner, fun hK => ?_,
    fun k hk => ?_⟩⟩
  · obtain ⟨k, hk, hkI⟩ := List.getElem_of_mem (hall I hI)
    exact ⟨k, hk, by rw [hseg k hk, hkI]⟩
  · have := h0 hK
    rw [hseg 0 hK]
    simp only [rdA, rdG, List.cons.injEq] at this
    exact this.2.1
  · obtain ⟨-, hp⟩ := hnext k hk
    rw [hseg (k + 1) hk, hseg k (by omega)]
    simp only [rdA, rdP, List.cons.injEq] at hp
    obtain ⟨-, -, h2, h3, h4, -⟩ := hp
    exact ⟨h2.symm, h3.symm, h4.symm⟩

/-! ### The continuation buses, together -/

/-- **The memory areas' instances and their continuation buses**: each
instance's fixed columns and range checks as the setup fixes them, buses 11
and 12 sound over their senders, with the global updates `GlobalCont` and
`GlobalRd` describe, and fewer than `2^32` instances of each. -/
structure ContWorld (S : MemOthers F) : Prop where
  ram : ∀ I ∈ S.mem, RamInst I
  input : ∀ I ∈ S.inputData, InInst I
  romd : ∀ I ∈ S.romData, RdInst I
  bus11 : ∃ (gc ac : List (List F)) (eR eI : Nat), SumHolds 11 (contSenders S) gc ac ∧ GlobalCont gc ac eR eI
  bus12 : ∃ (gc ac : List (List F)) (e : Nat), SumHolds 12 (rdContSenders S) gc ac ∧ GlobalRd gc ac e
  small : S.mem.length < 2 ^ 32 ∧ S.inputData.length < 2 ^ 32 ∧ S.romData.length < 2 ^ 32

/-- **`MemAnswers`, from the memory and continuation buses.** -/
theorem MemBusWorld.answers_cont {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) (CW : ContWorld S) (hrom : RomOk S) (hav : MainAvoids M)
    (hfirst : FirstNoLoad M) : MemAnswers M busD (busInit S) (mainStores M) := by
  obtain ⟨gc, ac, eR, eI, hsum, hg⟩ := CW.bus11
  obtain ⟨gc', ac', e, hsum', hg'⟩ := CW.bus12
  obtain ⟨hram, hc⟩ := W.ram_chain CW.ram hsum hg CW.small.1
  exact W.answers hram hc (in_world CW.input hsum hg CW.small.2.1) (rd_world CW.romd hsum' hg' CW.small.2.2)
    hrom hav hfirst

end Correspondence
