/-
  The machine's decoder on ZisK calls (ZiskRiscv/Decode.lean): every call
  it makes is well formed (`CallWF`): registers from 5-bit fields, an
  operation that does not halt and leaves its flag clear, a precompile no
  fcall, a count's call no extended argument.
-/
import Equiv.CallStep

namespace Equiv.Machine

open ZiskRiscv

/-- A decoded instruction's register fields, 5 bits each. -/
def FB (f : Fields) : Prop := f.rd < 32 ∧ f.rs1 < 32 ∧ f.rs2 < 32

theorem fields_fb (w : BitVec 32) : FB (fields w) := by
  simp only [FB, fields]
  refine ⟨?_, ?_, ?_⟩ <;> exact BitVec.isLt _

/-- What `CallWF` needs of a call's operation. -/
structure OpOk (op : Zisk.zisk_op) : Prop where
  halt : ∀ u, op ≠ .ZOP_HALT u
  pflag : ∀ a b, (Zisk.Functions.zisk_exec op a b).2 = false
  pre : Zisk.Functions.zisk_op_is_precompiled op = true → Transpiler.fcallOp op = false

theorem twoPointers_ok {csr : Nat} {op : Zisk.zisk_op} (h : twoPointers csr = some op) : OpOk op := by
  simp only [twoPointers] at h
  split at h <;> cases h <;>
    exact ⟨by (intro u hu; cases hu), by (intro a b; simp [Zisk.Functions.zisk_exec]), by decide⟩

theorem onePointer_ok {csr : Nat} {op : Zisk.zisk_op} (h : onePointer csr = some op) : OpOk op := by
  simp only [onePointer] at h
  split at h <;> cases h <;>
    exact ⟨by (intro u hu; cases hu), by (intro a b; simp [Zisk.Functions.zisk_exec]), by decide⟩

/-- A register argument from a bounded field: bounded. -/
macro "reg_wf" : tactic => `(tactic| (intro r hr; first
  | (cases hr; done)
  | (injection hr with hr; subst hr; first | assumption | omega)))

/-- An operation fact for `call_wf`: by computation, or from the
    precompile tables. -/
macro "op_halt" : tactic => `(tactic| first
  | (intro u hu; cases hu; done)
  | exact (twoPointers_ok ‹twoPointers _ = some _›).halt
  | exact (onePointer_ok ‹onePointer _ = some _›).halt)
macro "op_pflag" : tactic => `(tactic| first
  | (intro a b; simp [Zisk.Functions.zisk_exec]; done)
  | exact (twoPointers_ok ‹twoPointers _ = some _›).pflag
  | exact (onePointer_ok ‹onePointer _ = some _›).pflag)
macro "op_pre" : tactic => `(tactic| first
  | (dsimp only; decide)
  | exact (twoPointers_ok ‹twoPointers _ = some _›).pre
  | exact (onePointer_ok ‹onePointer _ = some _›).pre)
macro "count_wf" : tactic => `(tactic| (intro r hr; first
  | (cases hr; done)
  | (injection hr with hr; subst hr; exact ⟨by first | assumption | omega, rfl⟩)))

/-- `CallWF` of a concrete call, its registers bounded by hypotheses. -/
macro "call_wf" : tactic => `(tactic| exact ⟨by (dsimp only; first | assumption | omega), by reg_wf,
  by (intro h; cases h), by reg_wf, by op_halt, by op_pflag, by op_pre, by count_wf⟩)

theorem dmaPair_wf {csr rd src : Nat} {n0 : Fields} (hrd : rd < 32) (hsrc : src < 32) (hn : FB n0)
    {c : Call} (h : dmaPair csr rd src n0 = .call c) : CallWF c := by
  obtain ⟨h1, h2, h3⟩ := hn
  simp only [dmaPair] at h
  split at h <;> (try split at h) <;> (try split at h) <;> (try split at h)
  all_goals (first | cases h | (injection h with h; subst h))
  all_goals call_wf

theorem memsetPair_wf {dst : Nat} {n0 : Option Fields} (hdst : dst < 32) (hn : ∀ n, n0 = some n → FB n)
    {c : Call} (h : memsetPair dst n0 = .call c) : CallWF c := by
  simp only [memsetPair] at h
  split at h
  · next n =>
    obtain ⟨h1, h2, h3⟩ := hn n rfl
    split at h
    · injection h with h; subst h; call_wf
    · cases h
  · cases h

set_option hygiene false in
/-- Split every `if` and `match` of a decoder at h, and close each call. -/
macro "dec_wf" : tactic => `(tactic| (
  split at h <;> (try split at h) <;> (try split at h) <;> (try split at h) <;> (try split at h) <;>
    (try split at h) <;> (try split at h) <;>
  first
    | (cases h; done)
    | (injection h with h; subst h; call_wf)
    | (injection h with h; subst h; simp only [Option.some.injEq, reduceCtorEq] at *; first | done | (subst_vars; call_wf))
    | (exact dmaPair_wf (by assumption) (by assumption) ⟨by assumption, by assumption, by assumption⟩ h)
    | (exact memsetPair_wf (by assumption) (by assumption) h)))

theorem decodeCsrrs_wf {f : Fields} {n0 : Option Fields} (hf : FB f) (hn : ∀ n, n0 = some n → FB n)
    {c : Call} (h : decodeCsrrs f n0 = .call c) : CallWF c := by
  obtain ⟨f1, f2, f3⟩ := hf
  cases n0 with
  | none =>
    have hn' : ∀ n, (none : Option Fields) = some n → FB n := hn
    simp only [decodeCsrrs] at h
    dec_wf
  | some n =>
    have hb : FB n := hn n rfl
    have hn' : ∀ m, (some n : Option Fields) = some m → FB m := hn
    obtain ⟨n1', n2', n3'⟩ := hb
    simp only [decodeCsrrs] at h
    dec_wf

theorem decodeCsrrsi_wf {f : Fields} {n0 n1 : Option Fields} (hf : FB f) (hn : ∀ n, n0 = some n → FB n)
    (hn1 : ∀ n, n1 = some n → FB n) {c : Call} (h : decodeCsrrsi f n0 n1 = .call c) : CallWF c := by
  obtain ⟨f1, f2, f3⟩ := hf
  have h0 : (0 : Nat) < 32 := by omega
  cases n0 with
  | none =>
    simp only [decodeCsrrsi] at h
    dec_wf
  | some n =>
    obtain ⟨n1', n2', n3'⟩ := hn n rfl
    cases n1 with
    | none => simp only [decodeCsrrsi] at h; dec_wf
    | some m =>
      obtain ⟨m1, m2, m3⟩ := hn1 m rfl
      simp only [decodeCsrrsi] at h
      dec_wf

theorem decodeCsrrwi_wf {f : Fields} (hf : FB f) {c : Call} (h : decodeCsrrwi f = .call c) : CallWF c := by
  obtain ⟨f1, f2, f3⟩ := hf
  simp only [decodeCsrrwi] at h
  split at h
  · injection h with h; subst h; call_wf
  · cases h

/-- Every call the machine's decoder makes is well formed. -/
theorem decode_call_wf {fetch : Nat → Option (BitVec 32)} {a : Nat} {c : Call} (h : decode fetch a = .call c) :
    CallWF c := by
  have hn : ∀ k n, (fetch k).map fields = some n → FB n := by
    intro k n hk
    cases e : fetch k with
    | none => rw [e] at hk; cases hk
    | some w' => rw [e] at hk; cases hk; exact fields_fb w'
  simp only [decode] at h
  split at h
  · cases h
  · next w hw =>
    have hf := fields_fb w
    split at h <;> first
      | (cases h; done)
      | exact decodeCsrrs_wf hf (hn _) h
      | exact decodeCsrrsi_wf hf (hn _) (hn _) h
      | exact decodeCsrrwi_wf hf h

/-- A call comes from a word of `csrrs`, `csrrsi` or `csrrwi`. -/
theorem decode_call_kind {fetch : Nat → Option (BitVec 32)} {a : Nat} {c : Call} (h : decode fetch a = .call c) :
    ∃ w, fetch a = some w ∧
      (kind (fields w) = .csrrs ∨ kind (fields w) = .csrrsi ∨ kind (fields w) = .csrrwi) := by
  simp only [decode] at h
  split at h
  · cases h
  · next w hw =>
    refine ⟨w, hw, ?_⟩
    split at h <;> simp_all

/-- Those words the spec decodes as CSR instructions. -/
theorem spec_decode_csr (w : BitVec 32)
    (hk : kind (fields w) = .csrrs ∨ kind (fields w) = .csrrsi ∨ kind (fields w) = .csrrwi) :
    Transpiler.decode w = some .csr := by
  have ho : (fields w).opcode = 0x73 ∧ ((fields w).funct3 = 2 ∨ (fields w).funct3 = 6 ∨ (fields w).funct3 = 5) := by
    revert hk
    simp only [kind]
    split <;> simp_all <;> split <;> simp_all <;> split <;> simp_all
  obtain ⟨h1, h3⟩ := ho
  have e1 : (Transpiler.opcode w).toNat = (fields w).opcode := by
    simp [Transpiler.opcode, fields, BitVec.extractLsb, BitVec.extractLsb'_toNat]
  have e3 : (Transpiler.funct3 w).toNat = (fields w).funct3 := by
    simp [Transpiler.funct3, fields, BitVec.extractLsb, BitVec.extractLsb'_toNat]
  have hne : (w == 0x00000073#32) = false := by
    rcases h3 with h3 | h3 | h3 <;>
    · simp only [beq_eq_false_iff_ne]; intro e; subst e; simp [fields] at h3
  have hne' : (w == 0x00100073#32) = false := by
    rcases h3 with h3 | h3 | h3 <;>
    · simp only [beq_eq_false_iff_ne]; intro e; subst e; simp [fields] at h3
  simp only [Transpiler.decode, e1, e3, h1, hne, hne']
  rcases h3 with h3 | h3 | h3 <;> simp [h3]

/-- The length of a call the pairs make: two words. -/
theorem dmaPair_len {csr rd src : Nat} {n0 : Fields} {c : Call} (h : dmaPair csr rd src n0 = .call c) : c.len = 2 := by
  simp only [dmaPair] at h
  split at h <;> (try split at h) <;> first | (injection h with h; subst h; rfl) | cases h

theorem memsetPair_len {dst : Nat} {n0 : Option Fields} {c : Call} (h : memsetPair dst n0 = .call c) : c.len = 2 := by
  simp only [memsetPair] at h
  split at h <;> (try split at h) <;> first | (injection h with h; subst h; rfl) | cases h

/-- `csrrs`'s call of one word does not depend on the word after it. -/
theorem decodeCsrrs_len1 {f : Fields} {n0 : Option Fields} {c : Call} (h : decodeCsrrs f n0 = .call c)
    (hl : c.len ≠ 2) (n0' : Option Fields) : decodeCsrrs f n0' = .call c := by
  simp only [decodeCsrrs] at h ⊢
  split at h <;> first
    | exact h
    | (cases n0 with
        | none => cases h
        | some n => exact absurd (dmaPair_len (n0 := n) h) hl)
    | exact absurd (memsetPair_len h) hl
    | skip
  by_cases c1 : (f.rd != 0) = true
  · simp only [c1, if_true] at h ⊢; exact h
  simp only [c1, if_false, Bool.false_eq_true] at h ⊢
  by_cases c2 : (f.rs1 == 0) = true
  · simp only [c2, if_true] at h ⊢; exact h
  simp only [c2, if_false, Bool.false_eq_true] at h ⊢
  split at h <;> first
    | exact h
    | (cases n0 <;> simp only [id] at h <;> (try split at h) <;> (try split at h) <;>
        first | (injection h with h; subst h; exact absurd rfl hl) | (cases h; done))
    | skip
  all_goals (split at h <;> first | exact h | (cases n0 <;> simp only [id] at h <;> (try split at h) <;> (try split at h) <;> first | (injection h with h; subst h; exact absurd rfl hl) | (cases h; done)))

/-- `csrrsi`'s calls are of more than one word. -/
theorem decodeCsrrsi_len {f : Fields} {n0 n1 : Option Fields} {c : Call} (h : decodeCsrrsi f n0 n1 = .call c) :
    c.len ≠ 1 := by
  simp only [decodeCsrrsi] at h
  split at h
  · split at h
    · split at h
      · injection h with h; subst h; dsimp only; decide
      · cases h
    · rw [memsetPair_len h]; decide
    · cases h
  · cases h

/-- `csrrs`'s calls are of one or two words. -/
theorem decodeCsrrs_len3 {f : Fields} {n0 : Option Fields} {c : Call} (h : decodeCsrrs f n0 = .call c) :
    c.len ≠ 3 := by
  by_cases h2 : c.len = 2
  · rw [h2]; decide
  · have h1 := decodeCsrrs_len1 h h2 none
    simp only [decodeCsrrs] at h1
    split at h1 <;> (try split at h1) <;> (try split at h1) <;> (try split at h1) <;> (try split at h1) <;>
      (try split at h1) <;> first
      | (cases h1; done)
      | (injection h1 with h1; subst h1; simp)
      | (exact absurd (memsetPair_len h1) h2)
      | skip

/-- `csrrsi`'s call of three words: the memset port, the immediate 2, a
    word after it. -/
theorem decodeCsrrsi_len3 {f : Fields} {n0 n1 : Option Fields} {c : Call} (h : decodeCsrrsi f n0 n1 = .call c)
    (hl : c.len = 3) : f.csr = 0x816 ∧ f.rs1 = 2 ∧ n0.isSome := by
  simp only [decodeCsrrsi] at h
  split at h
  · next hc =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hc
    split at h
    · exact ⟨hc.2, by assumption, rfl⟩
    · rw [memsetPair_len h] at hl; cases hl
    · cases h
  · cases h

/-- `csrrsi`'s call of fewer than three words does not depend on the second
    word after it. -/
theorem decodeCsrrsi_n1' {f : Fields} {n0 n1 : Option Fields} {c : Call} (h : decodeCsrrsi f n0 n1 = .call c)
    (hl : c.len ≠ 3) (n1' : Option Fields) : decodeCsrrsi f n0 n1' = .call c := by
  simp only [decodeCsrrsi] at h ⊢
  split at h
  · next hc =>
    rw [if_pos hc]
    split at h <;> (try split at h) <;> (try split) <;> first | (simp only [Decoded.call.injEq] at h; subst h; simp at hl; done) | simp_all
  · cases h

/-- A call of three words: the memset sequence's first word at pc, the next
    word there. -/
theorem decode_len3 {fetch : Nat → Option (BitVec 32)} {a : Nat} {c : Call} (h : decode fetch a = .call c)
    (hl : c.len = 3) : ∃ w, fetch a = some w ∧ kind (fields w) = .csrrsi ∧ (fields w).csr = 0x816 ∧
      (fields w).rs1 = 2 ∧ (fetch (a + 4)).isSome := by
  simp only [decode] at h
  split at h
  · cases h
  · next w hw =>
    refine ⟨w, hw, ?_⟩
    revert h
    generalize hk : kind (fields w) = k
    cases k <;> intro h <;> first
      | (cases h; done)
      | exact absurd hl (decodeCsrrs_len3 h)
      | (obtain ⟨h1, h2, h3⟩ := decodeCsrrsi_len3 h hl
         refine ⟨rfl, h1, h2, ?_⟩
         cases e : fetch (a + 4) <;> simp_all)
      | (simp only [decodeCsrrwi] at h; split at h
         · injection h with h; subst h; cases hl
         · cases h)

theorem fetch3_none (pc : BitVec 64) (w : BitVec 32) (next : Option (BitVec 32)) :
    Transpiler.fetch3 pc w next none = Transpiler.fetch2 pc w next := by
  funext a; simp only [Transpiler.fetch3, Transpiler.fetch2]; split <;> (try rfl); split <;> (try rfl); split <;> rfl

/-- A call decoded from the word at pc and the two after it is the
    decoder's on any words that agree with them: at pc; at pc + 4, for a
    call of more than one word; at pc + 8, for one of three. -/
theorem decode_any3 {pc : BitVec 64} {w : BitVec 32} {next next2 : Option (BitVec 32)} {c : Call}
    (h : decode (Transpiler.fetch3 pc w next next2) pc.toNat = .call c) (fetch : Nat → Option (BitVec 32))
    (h0 : fetch pc.toNat = some w) (h4 : c.len ≠ 1 → fetch (pc.toNat + 4) = next)
    (h8 : c.len = 3 → fetch (pc.toNat + 8) = next2) :
    decode fetch pc.toNat = .call c := by
  have f0 : Transpiler.fetch3 pc w next next2 pc.toNat = some w := by simp [Transpiler.fetch3]
  have f4 : Transpiler.fetch3 pc w next next2 (pc.toNat + 4) = next := by simp [Transpiler.fetch3]
  have f8 : Transpiler.fetch3 pc w next next2 (pc.toNat + 8) = next2 := by simp [Transpiler.fetch3]
  simp only [decode, f0, f4, f8] at h
  simp only [decode, h0]
  by_cases hl : c.len = 1
  · generalize (fetch (pc.toNat + 4)).map fields = n0'
    generalize (fetch (pc.toNat + 8)).map fields = n1'
    revert h
    cases kind (fields w) <;> first
      | exact id
      | exact fun h => decodeCsrrs_len1 h (by rw [hl]; decide) n0'
      | exact fun h => absurd hl (decodeCsrrsi_len h)
  · rw [h4 hl]
    by_cases h3 : c.len = 3
    · rw [h8 h3]; exact h
    · generalize (fetch (pc.toNat + 8)).map fields = n1
      revert h
      cases kind (fields w) <;> first | exact id | exact fun h => decodeCsrrsi_n1' h h3 n1

/-- A call the spec decodes, from the word at pc and the two after it: the
    spec describes the word, its translation the call's instructions. -/
theorem convert_call3 {pc : BitVec 64} {w : BitVec 32} {next next2 : Option (BitVec 32)} {c : Call}
    (h : decode (Transpiler.fetch3 pc w next next2) pc.toNat = .call c) (ia : BitVec 64) :
    Transpiler.convert pc w next ia next2 = some (Transpiler.callInsts c pc ia) := by
  obtain ⟨w', hw', hk⟩ := decode_call_kind h
  have : w' = w := by simp [Transpiler.fetch3] at hw'; exact hw'.symm
  subst this
  have hd := spec_decode_csr w' hk
  simp only [Transpiler.convert, hd]
  have h2 : c.len ≠ 3 → decode (Transpiler.fetch2 pc w' next) pc.toNat = .call c := fun h3 =>
    decode_any3 h _ (by simp [Transpiler.fetch2]) (fun _ => by simp [Transpiler.fetch2])
      (fun e => absurd e h3)
  cases next2 with
  | none =>
    rw [fetch3_none] at h
    simp only [Transpiler.csr3, Transpiler.csr, h]
  | some n2 =>
    simp only [Transpiler.csr3]
    split
    · simp only [h]
    · next hg =>
      have h3 : c.len ≠ 3 := by
        intro e
        obtain ⟨w2, hw2, k1, k2, k3, k4⟩ := decode_len3 h e
        simp [Transpiler.fetch3] at hw2; subst hw2
        apply hg
        simp only [k1, k2, k3, Bool.and_eq_true, beq_iff_eq, and_true]
        exact ⟨rfl, by simpa [Transpiler.fetch3] using k4⟩
      simp only [Transpiler.csr, h2 h3]

/-- A call the spec decodes, from the word at pc and the two after it: the
    ZisK program holds its translation. -/
theorem transpiled_call3 {zrom : BitVec 64 → Zisk.zisk_inst} {pc : BitVec 64} {w : BitVec 32}
    {next next2 : Option (BitVec 32)} (ht : Equiv.Sim.Transpiled zrom pc w next next2) {c : Call}
    (h : decode (Transpiler.fetch3 pc w next next2) pc.toNat = .call c) :
    ∃ ia, ∀ p ∈ Transpiler.callInsts c pc ia, zrom p.1 = p.2 := by
  obtain ⟨ia, l, hl, hz⟩ := ht
  obtain ⟨w', hw', hk⟩ := decode_call_kind h
  have : w' = w := by simp [Transpiler.fetch3] at hw'; exact hw'.symm
  subst this
  have hd := spec_decode_csr w' hk
  simp only [Transpiler.convert, hd] at hl
  -- the call as the two-word decoder has it, when the call is not memset's three words
  have h2 : c.len ≠ 3 → decode (Transpiler.fetch2 pc w' next) pc.toNat = .call c := fun h3 =>
    decode_any3 h _ (by simp [Transpiler.fetch2]) (fun _ => by simp [Transpiler.fetch2])
      (fun e => absurd e h3)
  cases next2 with
  | none =>
    rw [fetch3_none] at h
    simp only [Transpiler.csr3, Transpiler.csr, h] at hl
    cases hl; exact ⟨ia, hz⟩
  | some n2 =>
    simp only [Transpiler.csr3] at hl
    split at hl
    · simp only [h] at hl; cases hl; exact ⟨ia, hz⟩
    · next hg =>
      have h3 : c.len ≠ 3 := by
        intro e
        obtain ⟨w2, hw2, k1, k2, k3, k4⟩ := decode_len3 h e
        simp [Transpiler.fetch3] at hw2; subst hw2
        apply hg
        simp only [k1, k2, k3, Bool.and_eq_true, beq_iff_eq, and_true]
        exact ⟨rfl, by simpa [Transpiler.fetch3] using k4⟩
      simp only [Transpiler.csr, h2 h3] at hl
      cases hl; exact ⟨ia, hz⟩

end Equiv.Machine
