/-
  The words the spec decodes to nothing, for each major opcode the profile
  uses (but SYSTEM's): what bits they have, each class one of
  Equiv/Decode/Rej/'s lemmas (RiscvDecodeRej.lean) takes. Each by the
  spec's decode split to its cases, and the fields back to bits.
-/
import Transpiler.Spec

-- One simp set for every opcode's cases: each uses part of it.
set_option linter.unusedSimpArgs false
namespace Transpiler

set_option maxHeartbeats 0 in
/-- A load the spec decodes to nothing: its bits. -/
theorem rej_3 {e : BitVec 32} (h : decode e = none) (ho : (opcode e).toNat = 3) :
    (e.extractLsb 6 0 = 0x03#7 ∧ e.extractLsb 14 12 = 7#3) := by
  have h3 := (funct3 e).isLt
  have h7 := (funct7 e).isLt
  simp only [decode, ho] at h
  generalize hk : (funct3 e).toNat = k at h h3
  generalize hk7 : (funct7 e).toNat = k7 at h h7
  simp only [opcode] at ho
  simp only [funct3] at hk
  simp only [funct7] at hk7
  try simp only [funct6, rs2] at h
  repeat' split at h
  all_goals (first | (simp at h; done) | skip)
  all_goals (subst_vars; simp only [BitVec.toNat_eq_nat, Bool.or_eq_true, beq_iff_eq, Nat.reducePow, Nat.reduceLT,
    true_and, Nat.reduceAdd, Nat.reduceSub, decide_eq_true_eq, ne_eq] at *; bv_decide)

set_option maxHeartbeats 0 in
/-- A MISC-MEM word the spec decodes to nothing: its bits. -/
theorem rej_15 {e : BitVec 32} (h : decode e = none) (ho : (opcode e).toNat = 15) :
    (e.extractLsb 6 0 = 0x0F#7 ∧ e.extractLsb 14 12 ≠ 0#3 ∧ e.extractLsb 14 12 ≠ 1#3) := by
  have h3 := (funct3 e).isLt
  have h7 := (funct7 e).isLt
  simp only [decode, ho] at h
  generalize hk : (funct3 e).toNat = k at h h3
  generalize hk7 : (funct7 e).toNat = k7 at h h7
  simp only [opcode] at ho
  simp only [funct3] at hk
  simp only [funct7] at hk7
  try simp only [funct6, rs2] at h
  repeat' split at h
  all_goals (first | (simp at h; done) | skip)
  all_goals (subst_vars; simp only [BitVec.toNat_eq_nat, Bool.or_eq_true, beq_iff_eq, Nat.reducePow, Nat.reduceLT,
    true_and, Nat.reduceAdd, Nat.reduceSub, decide_eq_true_eq, ne_eq] at *; bv_decide)

set_option maxHeartbeats 0 in
/-- An OP-IMM word the spec decodes to nothing: its bits. -/
theorem rej_19 {e : BitVec 32} (h : decode e = none) (ho : (opcode e).toNat = 19) :
    (e.extractLsb 6 0 = 0x13#7 ∧ e.extractLsb 14 12 = 1#3 ∧ e.extractLsb 31 26 ≠ 0#6 ∧ e.extractLsb 31 26 ≠ 10#6 ∧ e.extractLsb 31 26 ≠ 18#6 ∧ e.extractLsb 31 26 ≠ 26#6 ∧ e.extractLsb 31 20 ≠ 0x604#12 ∧ e.extractLsb 31 20 ≠ 0x605#12 ∧ e.extractLsb 31 20 ≠ 0x600#12 ∧ e.extractLsb 31 20 ≠ 0x601#12 ∧ e.extractLsb 31 20 ≠ 0x602#12) ∨
    (e.extractLsb 6 0 = 0x13#7 ∧ e.extractLsb 14 12 = 5#3 ∧ e.extractLsb 31 26 ≠ 0#6 ∧ e.extractLsb 31 26 ≠ 16#6 ∧ e.extractLsb 31 26 ≠ 18#6 ∧ e.extractLsb 31 26 ≠ 24#6 ∧ e.extractLsb 31 20 ≠ 0x6B8#12 ∧ e.extractLsb 31 20 ≠ 0x687#12 ∧ e.extractLsb 31 20 ≠ 0x287#12) := by
  have h3 := (funct3 e).isLt
  have h7 := (funct7 e).isLt
  simp only [decode, ho] at h
  generalize hk : (funct3 e).toNat = k at h h3
  generalize hk7 : (funct7 e).toNat = k7 at h h7
  simp only [opcode] at ho
  simp only [funct3] at hk
  simp only [funct7] at hk7
  try simp only [funct6, rs2] at h
  repeat' split at h
  all_goals (first | (simp at h; done) | skip)
  all_goals (subst_vars; simp only [BitVec.toNat_eq_nat, Bool.or_eq_true, beq_iff_eq, Nat.reducePow, Nat.reduceLT,
    true_and, Nat.reduceAdd, Nat.reduceSub, decide_eq_true_eq, ne_eq] at *; bv_decide)

set_option maxHeartbeats 0 in
/-- An OP-IMM-32 word the spec decodes to nothing: its bits. -/
theorem rej_27 {e : BitVec 32} (h : decode e = none) (ho : (opcode e).toNat = 27) :
    (e.extractLsb 6 0 = 0x1B#7 ∧ e.extractLsb 14 12 = 1#3 ∧ e.extractLsb 31 20 ≠ 0x600#12 ∧ e.extractLsb 31 20 ≠ 0x601#12 ∧ e.extractLsb 31 20 ≠ 0x602#12 ∧ e.extractLsb 31 26 ≠ 2#6 ∧ e.extractLsb 31 25 ≠ 0#7) ∨
    (e.extractLsb 6 0 = 0x1B#7 ∧ e.extractLsb 14 12 = 5#3 ∧ e.extractLsb 31 25 ≠ 0#7 ∧ e.extractLsb 31 25 ≠ 32#7 ∧ e.extractLsb 31 25 ≠ 48#7) ∨
    (e.extractLsb 6 0 = 0x1B#7 ∧ (e.extractLsb 14 12 = 2#3 ∨ e.extractLsb 14 12 = 3#3 ∨ e.extractLsb 14 12 = 4#3 ∨ e.extractLsb 14 12 = 6#3 ∨ e.extractLsb 14 12 = 7#3)) := by
  have h3 := (funct3 e).isLt
  have h7 := (funct7 e).isLt
  simp only [decode, ho] at h
  generalize hk : (funct3 e).toNat = k at h h3
  generalize hk7 : (funct7 e).toNat = k7 at h h7
  simp only [opcode] at ho
  simp only [funct3] at hk
  simp only [funct7] at hk7
  try simp only [funct6, rs2] at h
  repeat' split at h
  all_goals (first | (simp at h; done) | skip)
  all_goals (subst_vars; simp only [BitVec.toNat_eq_nat, Bool.or_eq_true, beq_iff_eq, Nat.reducePow, Nat.reduceLT,
    true_and, Nat.reduceAdd, Nat.reduceSub, decide_eq_true_eq, ne_eq] at *; bv_decide)

set_option maxHeartbeats 0 in
/-- A store the spec decodes to nothing: its bits. -/
theorem rej_35 {e : BitVec 32} (h : decode e = none) (ho : (opcode e).toNat = 35) :
    (e.extractLsb 6 0 = 0x23#7 ∧ (e.extractLsb 14 12 = 4#3 ∨ e.extractLsb 14 12 = 5#3 ∨ e.extractLsb 14 12 = 6#3 ∨ e.extractLsb 14 12 = 7#3)) := by
  have h3 := (funct3 e).isLt
  have h7 := (funct7 e).isLt
  simp only [decode, ho] at h
  generalize hk : (funct3 e).toNat = k at h h3
  generalize hk7 : (funct7 e).toNat = k7 at h h7
  simp only [opcode] at ho
  simp only [funct3] at hk
  simp only [funct7] at hk7
  try simp only [funct6, rs2] at h
  repeat' split at h
  all_goals (first | (simp at h; done) | skip)
  all_goals (subst_vars; simp only [BitVec.toNat_eq_nat, Bool.or_eq_true, beq_iff_eq, Nat.reducePow, Nat.reduceLT,
    true_and, Nat.reduceAdd, Nat.reduceSub, decide_eq_true_eq, ne_eq] at *; bv_decide)

set_option maxHeartbeats 0 in
/-- An AMO the spec decodes to nothing: its bits. -/
theorem rej_47 {e : BitVec 32} (h : decode e = none) (ho : (opcode e).toNat = 47) :
    (e.extractLsb 6 0 = 0x2F#7 ∧ e.extractLsb 14 12 = 0#3) ∨
    (e.extractLsb 6 0 = 0x2F#7 ∧ e.extractLsb 14 12 = 1#3) ∨
    (e.extractLsb 6 0 = 0x2F#7 ∧ e.extractLsb 14 12 = 4#3) ∨
    (e.extractLsb 6 0 = 0x2F#7 ∧ e.extractLsb 14 12 = 5#3) ∨
    (e.extractLsb 6 0 = 0x2F#7 ∧ e.extractLsb 14 12 = 6#3) ∨
    (e.extractLsb 6 0 = 0x2F#7 ∧ e.extractLsb 14 12 = 7#3) ∨
    (e.extractLsb 6 0 = 0x2F#7 ∧ (e.extractLsb 14 12 = 2#3 ∨ e.extractLsb 14 12 = 3#3) ∧ e.extractLsb 31 27 = 2#5 ∧ e.extractLsb 24 20 ≠ 0#5) ∨
    (e.extractLsb 6 0 = 0x2F#7 ∧ (e.extractLsb 14 12 = 2#3 ∨ e.extractLsb 14 12 = 3#3) ∧ e.extractLsb 31 27 ≠ 2#5 ∧ e.extractLsb 31 27 ≠ 3#5 ∧ e.extractLsb 31 27 ≠ 1#5 ∧ e.extractLsb 31 27 ≠ 0#5 ∧ e.extractLsb 31 27 ≠ 4#5 ∧ e.extractLsb 31 27 ≠ 12#5 ∧ e.extractLsb 31 27 ≠ 8#5 ∧ e.extractLsb 31 27 ≠ 16#5 ∧ e.extractLsb 31 27 ≠ 20#5 ∧ e.extractLsb 31 27 ≠ 24#5 ∧ e.extractLsb 31 27 ≠ 28#5 ∧ e.extractLsb 31 27 ≠ 5#5) ∨
    (e.extractLsb 6 0 = 0x2F#7 ∧ e.extractLsb 14 12 = 2#3 ∧ e.extractLsb 31 27 = 5#5) ∨
    (e.extractLsb 6 0 = 0x2F#7 ∧ e.extractLsb 14 12 = 3#3 ∧ e.extractLsb 31 27 = 5#5) := by
  have h3 := (funct3 e).isLt
  have h7 := (funct7 e).isLt
  simp only [decode, ho] at h
  generalize hk : (funct3 e).toNat = k at h h3
  generalize hk7 : (funct7 e).toNat = k7 at h h7
  simp only [opcode] at ho
  simp only [funct3] at hk
  simp only [funct7] at hk7
  try simp only [funct6, rs2] at h
  repeat' split at h
  all_goals (first | (simp at h; done) | skip)
  all_goals (subst_vars; simp only [BitVec.toNat_eq_nat, Bool.or_eq_true, beq_iff_eq, Nat.reducePow, Nat.reduceLT,
    true_and, Nat.reduceAdd, Nat.reduceSub, decide_eq_true_eq, ne_eq] at *; bv_decide)

set_option maxHeartbeats 0 in
/-- An OP word the spec decodes to nothing: its bits. -/
theorem rej_51 {e : BitVec 32} (h : decode e = none) (ho : (opcode e).toNat = 51) :
    (e.extractLsb 6 0 = 0x33#7 ∧ ¬(e.extractLsb 14 12 = 0#3 ∧ e.extractLsb 31 25 = 0#7) ∧ ¬(e.extractLsb 14 12 = 0#3 ∧ e.extractLsb 31 25 = 32#7) ∧ ¬(e.extractLsb 14 12 = 1#3 ∧ e.extractLsb 31 25 = 0#7) ∧ ¬(e.extractLsb 14 12 = 2#3 ∧ e.extractLsb 31 25 = 0#7) ∧ ¬(e.extractLsb 14 12 = 3#3 ∧ e.extractLsb 31 25 = 0#7) ∧ ¬(e.extractLsb 14 12 = 4#3 ∧ e.extractLsb 31 25 = 0#7) ∧ ¬(e.extractLsb 14 12 = 5#3 ∧ e.extractLsb 31 25 = 0#7) ∧ ¬(e.extractLsb 14 12 = 5#3 ∧ e.extractLsb 31 25 = 32#7) ∧ ¬(e.extractLsb 14 12 = 6#3 ∧ e.extractLsb 31 25 = 0#7) ∧ ¬(e.extractLsb 14 12 = 7#3 ∧ e.extractLsb 31 25 = 0#7) ∧ ¬(e.extractLsb 14 12 = 0#3 ∧ e.extractLsb 31 25 = 1#7) ∧ ¬(e.extractLsb 14 12 = 1#3 ∧ e.extractLsb 31 25 = 1#7) ∧ ¬(e.extractLsb 14 12 = 2#3 ∧ e.extractLsb 31 25 = 1#7) ∧ ¬(e.extractLsb 14 12 = 3#3 ∧ e.extractLsb 31 25 = 1#7) ∧ ¬(e.extractLsb 14 12 = 4#3 ∧ e.extractLsb 31 25 = 1#7) ∧ ¬(e.extractLsb 14 12 = 5#3 ∧ e.extractLsb 31 25 = 1#7) ∧ ¬(e.extractLsb 14 12 = 6#3 ∧ e.extractLsb 31 25 = 1#7) ∧ ¬(e.extractLsb 14 12 = 7#3 ∧ e.extractLsb 31 25 = 1#7) ∧ ¬(e.extractLsb 14 12 = 2#3 ∧ e.extractLsb 31 25 = 16#7) ∧ ¬(e.extractLsb 14 12 = 4#3 ∧ e.extractLsb 31 25 = 16#7) ∧ ¬(e.extractLsb 14 12 = 6#3 ∧ e.extractLsb 31 25 = 16#7) ∧ ¬(e.extractLsb 14 12 = 1#3 ∧ e.extractLsb 31 25 = 36#7) ∧ ¬(e.extractLsb 14 12 = 5#3 ∧ e.extractLsb 31 25 = 36#7) ∧ ¬(e.extractLsb 14 12 = 1#3 ∧ e.extractLsb 31 25 = 52#7) ∧ ¬(e.extractLsb 14 12 = 1#3 ∧ e.extractLsb 31 25 = 20#7) ∧ ¬(e.extractLsb 14 12 = 7#3 ∧ e.extractLsb 31 25 = 32#7) ∧ ¬(e.extractLsb 14 12 = 6#3 ∧ e.extractLsb 31 25 = 32#7) ∧ ¬(e.extractLsb 14 12 = 4#3 ∧ e.extractLsb 31 25 = 32#7) ∧ ¬(e.extractLsb 14 12 = 6#3 ∧ e.extractLsb 31 25 = 5#7) ∧ ¬(e.extractLsb 14 12 = 7#3 ∧ e.extractLsb 31 25 = 5#7) ∧ ¬(e.extractLsb 14 12 = 4#3 ∧ e.extractLsb 31 25 = 5#7) ∧ ¬(e.extractLsb 14 12 = 5#3 ∧ e.extractLsb 31 25 = 5#7) ∧ ¬(e.extractLsb 14 12 = 1#3 ∧ e.extractLsb 31 25 = 48#7) ∧ ¬(e.extractLsb 14 12 = 5#3 ∧ e.extractLsb 31 25 = 48#7) ∧ ¬(e.extractLsb 14 12 = 4#3 ∧ e.extractLsb 31 25 = 4#7) ∧ ¬(e.extractLsb 14 12 = 7#3 ∧ e.extractLsb 31 25 = 4#7)) := by
  have h3 := (funct3 e).isLt
  have h7 := (funct7 e).isLt
  simp only [decode, ho] at h
  generalize hk : (funct3 e).toNat = k at h h3
  generalize hk7 : (funct7 e).toNat = k7 at h h7
  simp only [opcode] at ho
  simp only [funct3] at hk
  simp only [funct7] at hk7
  try simp only [funct6, rs2] at h
  repeat' split at h
  all_goals (first | (simp at h; done) | skip)
  all_goals (subst_vars; simp only [BitVec.toNat_eq_nat, Bool.or_eq_true, beq_iff_eq, Nat.reducePow, Nat.reduceLT,
    true_and, Nat.reduceAdd, Nat.reduceSub, decide_eq_true_eq, ne_eq] at *; bv_decide)

set_option maxHeartbeats 0 in
/-- An OP-32 word the spec decodes to nothing: its bits. -/
theorem rej_59 {e : BitVec 32} (h : decode e = none) (ho : (opcode e).toNat = 59) :
    (e.extractLsb 6 0 = 0x3B#7 ∧ ¬(e.extractLsb 14 12 = 0#3 ∧ e.extractLsb 31 25 = 0#7) ∧ ¬(e.extractLsb 14 12 = 0#3 ∧ e.extractLsb 31 25 = 32#7) ∧ ¬(e.extractLsb 14 12 = 1#3 ∧ e.extractLsb 31 25 = 0#7) ∧ ¬(e.extractLsb 14 12 = 5#3 ∧ e.extractLsb 31 25 = 0#7) ∧ ¬(e.extractLsb 14 12 = 5#3 ∧ e.extractLsb 31 25 = 32#7) ∧ ¬(e.extractLsb 14 12 = 1#3 ∧ e.extractLsb 31 25 = 48#7) ∧ ¬(e.extractLsb 14 12 = 5#3 ∧ e.extractLsb 31 25 = 48#7) ∧ ¬(e.extractLsb 14 12 = 0#3 ∧ e.extractLsb 31 25 = 4#7) ∧ ¬(e.extractLsb 14 12 = 2#3 ∧ e.extractLsb 31 25 = 16#7) ∧ ¬(e.extractLsb 14 12 = 4#3 ∧ e.extractLsb 31 25 = 16#7) ∧ ¬(e.extractLsb 14 12 = 6#3 ∧ e.extractLsb 31 25 = 16#7) ∧ ¬(e.extractLsb 14 12 = 4#3 ∧ e.extractLsb 31 25 = 4#7) ∧ ¬(e.extractLsb 14 12 = 0#3 ∧ e.extractLsb 31 25 = 1#7) ∧ ¬(e.extractLsb 14 12 = 4#3 ∧ e.extractLsb 31 25 = 1#7) ∧ ¬(e.extractLsb 14 12 = 5#3 ∧ e.extractLsb 31 25 = 1#7) ∧ ¬(e.extractLsb 14 12 = 6#3 ∧ e.extractLsb 31 25 = 1#7) ∧ ¬(e.extractLsb 14 12 = 7#3 ∧ e.extractLsb 31 25 = 1#7)) := by
  have h3 := (funct3 e).isLt
  have h7 := (funct7 e).isLt
  simp only [decode, ho] at h
  generalize hk : (funct3 e).toNat = k at h h3
  generalize hk7 : (funct7 e).toNat = k7 at h h7
  simp only [opcode] at ho
  simp only [funct3] at hk
  simp only [funct7] at hk7
  try simp only [funct6, rs2] at h
  repeat' split at h
  all_goals (first | (simp at h; done) | skip)
  all_goals (subst_vars; simp only [BitVec.toNat_eq_nat, Bool.or_eq_true, beq_iff_eq, Nat.reducePow, Nat.reduceLT,
    true_and, Nat.reduceAdd, Nat.reduceSub, decide_eq_true_eq, ne_eq] at *; bv_decide)

set_option maxHeartbeats 0 in
/-- A branch the spec decodes to nothing: its bits. -/
theorem rej_99 {e : BitVec 32} (h : decode e = none) (ho : (opcode e).toNat = 99) :
    (e.extractLsb 6 0 = 0x63#7 ∧ (e.extractLsb 14 12 = 2#3 ∨ e.extractLsb 14 12 = 3#3)) := by
  have h3 := (funct3 e).isLt
  have h7 := (funct7 e).isLt
  simp only [decode, ho] at h
  generalize hk : (funct3 e).toNat = k at h h3
  generalize hk7 : (funct7 e).toNat = k7 at h h7
  simp only [opcode] at ho
  simp only [funct3] at hk
  simp only [funct7] at hk7
  try simp only [funct6, rs2] at h
  repeat' split at h
  all_goals (first | (simp at h; done) | skip)
  all_goals (subst_vars; simp only [BitVec.toNat_eq_nat, Bool.or_eq_true, beq_iff_eq, Nat.reducePow, Nat.reduceLT,
    true_and, Nat.reduceAdd, Nat.reduceSub, decide_eq_true_eq, ne_eq] at *; bv_decide)

set_option maxHeartbeats 0 in
/-- A jalr the spec decodes to nothing: its bits. -/
theorem rej_103 {e : BitVec 32} (h : decode e = none) (ho : (opcode e).toNat = 103) :
    (e.extractLsb 6 0 = 0x67#7 ∧ e.extractLsb 14 12 ≠ 0#3) := by
  have h3 := (funct3 e).isLt
  have h7 := (funct7 e).isLt
  simp only [decode, ho] at h
  generalize hk : (funct3 e).toNat = k at h h3
  generalize hk7 : (funct7 e).toNat = k7 at h h7
  simp only [opcode] at ho
  simp only [funct3] at hk
  simp only [funct7] at hk7
  try simp only [funct6, rs2] at h
  repeat' split at h
  all_goals (first | (simp at h; done) | skip)
  all_goals (subst_vars; simp only [BitVec.toNat_eq_nat, Bool.or_eq_true, beq_iff_eq, Nat.reducePow, Nat.reduceLT,
    true_and, Nat.reduceAdd, Nat.reduceSub, decide_eq_true_eq, ne_eq] at *; bv_decide)

end Transpiler