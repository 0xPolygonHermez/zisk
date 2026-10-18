/-
  What the generated Main AIR can prove.

  These are not facts anyone needs yet. They exist to answer a question the
  generator could not: are the definitions `pilout-constraints` emits actually
  usable for reasoning, or only well formed? Each one is proved from the
  generated constraint and nothing else, and the section at the end records
  where the abstraction stops.
-/
import Pil.Main

namespace Pil.Zisk.Main

variable {F : Type} [PilField F] (x : Ctx F) (t : Trace F) (i : Int)

/-! ### Reasoning straight from a constraint -/

/-- `main.pil:108` makes the segment flag idempotent, which is the usable form
of a booleanity constraint: it is what rewrites `m * m` to `m` in a later step. -/
theorem main_last_segment_idempotent (h : c0 x t i) :
    x.Main_main_last_segment * x.Main_main_last_segment = x.Main_main_last_segment := by
  unfold c0 at h; grind

/-- `main.pil:224` determines `addr1`, so the column can be eliminated. A
constraint states a polynomial vanishes; this is the same fact as an equation,
and going between the two is what a correspondence proof does constantly. -/
theorem addr1_eq (h : c1 x t i) :
    (t i).addr1 0 = (t i).b_offset_imm0 0 + (t i).b_src_ind 0 * (t i).a 0 0 := by
  unfold c1 at h; grind

/-- Two constraints combined. `std_tools.pil:56` makes `a_src_reg` idempotent
and bounds `a_src_mem + a_src_reg`; together they force the product of the two
selectors to be its own negation — one step short of the mutual exclusion the
PIL intends. Completing it needs `2 ≠ 0`, which a commutative ring does not
give; see below and `sources_exclusive_twice`. -/
theorem sources_almost_exclusive (h9 : c9 x t i) (h10 : c10 x t i) :
    (t i).a_src_mem 0 * (t i).a_src_mem 0 - (t i).a_src_mem 0
      = -((t i).a_src_mem 0 * (t i).a_src_reg 0)
        - ((t i).a_src_mem 0 * (t i).a_src_reg 0) := by
  unfold c9 c10 Zisk_direct_gsum_e_0 at *; grind

/-- With `main.pil:553`, the booleanity of `a_src_mem`, the two selectors are
exclusive up to a factor of two: `2·(a_src_mem·a_src_reg) = 0`. That factor is
as far as ring laws go. -/
theorem sources_exclusive_twice (h9 : c9 x t i) (h10 : c10 x t i) (h110 : c110 x t i) :
    2 * ((t i).a_src_mem 0 * (t i).a_src_reg 0) = 0 := by
  unfold c9 c10 c110 Zisk_direct_gsum_e_0 Zisk_direct_gsum_e_41 at *; grind

/-! ### Reasoning from the whole AIR

`holds` is the AIR as one proposition. A proof about a single constraint should
be reachable from it without unfolding 713 definitions: each constraint has an
accessor on `everyRow`, `h.c1` for `h : everyRow x t i`. -/

/-- Any constraint of the AIR follows from `holds`, at every row. -/
theorem addr1_eq_of_holds (h : holds x t) :
    ∀ i : Int, (t i).addr1 0 = (t i).b_offset_imm0 0 + (t i).b_src_ind 0 * (t i).a 0 0 := by
  intro i
  exact addr1_eq x t i (h.2.1 i).c1

/-- The packed row is four instruction slots, and the same `.pil` line
constrains each. Slot 3's copy of `main.pil:224` is `c7`, and it mentions only
slot 3's columns — which is what makes a per-slot correspondence possible. -/
theorem addr1_eq_slot3 (h : c7 x t i) :
    (t i).addr1 3 = (t i).b_offset_imm0 3 + (t i).b_src_ind 3 * (t i).a 3 0 := by
  unfold c7 at h; grind

/-! ### Where the abstraction stops

What the constraints alone do not give, worth knowing before anyone plans a
larger proof.

**Booleanity is a constraint; range is not.** Main asserts each one-bit
selector boolean explicitly (`main.pil:544`–`567`, e.g. `c110` for
`a_src_mem[0]`), so those facts are available here. Wider declared ranges, such
as a `bits(32)` limb, are lookup arguments: they live in the pilout's hints,
which this generator does not extract, so a limb is not known here to be below
`2^32`.

**Ring laws are not field laws.** `PilField` is `Lean.Grind.CommRing`, so there
is no inverse and no characteristic. A proof needing either instantiates `F` at
the concrete field: `basePrime` is `18446744069414584321`, which the generated
AIR records for exactly this purpose.
-/

end Pil.Zisk.Main
