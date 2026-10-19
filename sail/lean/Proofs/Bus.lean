/-
  What a bus guarantees.

  A pil2 bus is enforced jointly, by the gsum columns of every AIR on it and a
  global constraint that closes the sum; none of that is modeled here. What a
  proof needs is the guarantee a sound bus argument gives, stated over the
  bus arguments the generator extracts (`BusArg`), and taken as a hypothesis.

  * `LookupHolds a x t ps`: whenever the assuming side `a` sends its tuple, one
    of the provers `ps` proves the same tuple, at some row, with nonzero count,
    on the same bus. A prover is an argument together with its AIR's context
    and trace, so the provers of one bus can come from AIRs of different types
    — forty of them, on the operation bus.
  * Tuples are compared as the bus compares them: up to zero padding. Arith
    sends a 4-element tuple to a table that proves 8-element ones, and Main's
    11-element operation tuples meet 8-element ones from the FROPS tables.
  * A prover may serve several buses through one argument, choosing per row
    (a merged virtual table). The bus is compared per row.
  * `RangeHolds ranges a x t`: whenever the range check `a` sends its value,
    the value is a number in the range its bus serves, taken from the AIR's
    generated `ranges` rather than written in the proof.
  * `TableNever p b k`: no row of the table `p` on bus `b` has `k` as its first
    element. That is a fact about fixed data, and `sail/check_tables.py`
    checks it against the data the setup commits.

  Both `LookupHolds` and `TableNever` refuse arguments that are not what they
  claim: the assuming side must assume on a single bus, every prover must prove
  and must be able to serve that bus.
-/
import Pil.Prelude

namespace Correspondence

/-- One proving argument of one AIR instance: the argument, and the context and
trace it is evaluated on. -/
structure Prover (F : Type) where
  {C : Type}
  {T : Type}
  arg : BusArg F C T
  x : C
  t : T

variable {F : Type} [PilField F]

/-- Element `k` of a tuple at row `i`; zero past its end, as the bus pads. -/
def tupleAt {C T : Type} (tuple : List (C → T → Int → F)) (x : C) (t : T) (i : Int) (k : Nat) : F :=
  (tuple.getD k (fun _ _ _ => 0)) x t i

/-- What a sound lookup argument guarantees between the side `a` that assumes
and the provers `ps`. -/
def LookupHolds {C T : Type} (a : BusArg F C T) (x : C) (t : T) (ps : List (Prover F)) : Prop :=
  (∃ b, a.opids = [b] ∧ ∀ p ∈ ps, b ∈ p.arg.opids) ∧
    a.piop = .lookup ∧ a.side = .assumes ∧
    (∀ p ∈ ps, p.arg.piop = .lookup ∧ p.arg.side = .proves) ∧
    ∀ i, a.reps x t i ≠ 0 →
      ∃ p ∈ ps, ∃ j, p.arg.reps p.x p.t j ≠ 0 ∧ p.arg.bus p.x p.t j = a.bus x t i ∧
        ∀ k, tupleAt a.tuple x t i k = tupleAt p.arg.tuple p.x p.t j k

attribute [local instance] Lean.Grind.Semiring.natCast in
/-- What a sound range check guarantees: whenever `a` sends its value, the
value is a number in the range its bus serves. The range must be one of the
AIR's generated `ranges`, on the argument's single bus, with no negative bound. -/
def RangeHolds {C T : Type} (ranges : List RangeDef) (a : BusArg F C T) (x : C) (t : T) : Prop :=
  a.piop = .rangeCheck ∧ a.side = .assumes ∧
    ∃ b d, a.opids = [b] ∧ d ∈ ranges ∧ d.opid = b ∧ d.minNeg = false ∧ d.maxNeg = false ∧
      ∀ i, a.reps x t i ≠ 0 → ∀ f ∈ a.tuple, ∃ n : Nat, d.min ≤ n ∧ n ≤ d.max ∧ f x t i = (n : F)

/-- No row of the table `p` on bus `b` sends `k` as its first element: a fact
about the table's fixed data, which `check_tables.py` checks. -/
def TableNever (p : Prover F) (b k : F) : Prop :=
  ∀ j, p.arg.bus p.x p.t j = b → tupleAt p.arg.tuple p.x p.t j 0 ≠ k

/-- A count of 1 is not 0: what a lookup with a constant count needs to fire. -/
theorem one_ne_zero' {F : Type} [Lean.Grind.Field F] : (1 : F) ≠ 0 :=
  fun h => Lean.Grind.Field.zero_ne_one h.symm

end Correspondence
