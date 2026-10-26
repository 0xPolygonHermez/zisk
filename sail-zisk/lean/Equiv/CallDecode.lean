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
  simp only [Transpiler.decode, e1, e3, h1, hne]
  rcases h3 with h3 | h3 | h3 <;> simp [h3]

/-- `csrrsi`'s call does not depend on the second word after it, when it is
    one without it. -/
theorem decodeCsrrsi_n1 {f : Fields} {n0 : Option Fields} {c : Call} (h : decodeCsrrsi f n0 none = .call c)
    (n1 : Option Fields) : decodeCsrrsi f n0 n1 = .call c := by
  simp only [decodeCsrrsi] at h ⊢
  split at h
  · next hc =>
    rw [if_pos hc]
    split at h <;> split <;> simp_all
  · cases h

/-- A call decoded from the word at pc and the one after it is the
    machine's, on memory holding them. -/
theorem decode_of_fetch2 {m : Equiv.Mem} {pc : BitVec 64} {w : BitVec 32} {next : Option (BitVec 32)}
    (hw : readWord m pc.toNat = some w) (hn : readWord m (pc.toNat + 4) = next) {c : Call}
    (h : decode (Transpiler.fetch2 pc w next) pc.toNat = .call c) : decode (readWord m) pc.toNat = .call c := by
  have f0 : Transpiler.fetch2 pc w next pc.toNat = some w := by simp [Transpiler.fetch2]
  have f4 : Transpiler.fetch2 pc w next (pc.toNat + 4) = next := by simp [Transpiler.fetch2]
  have f8 : Transpiler.fetch2 pc w next (pc.toNat + 8) = none := by simp [Transpiler.fetch2]
  simp only [decode, f0, f4, f8] at h
  simp only [decode, hw, hn]
  generalize (readWord m (pc.toNat + 8)).map fields = n1
  revert h
  cases kind (fields w) <;> first | exact id | exact fun h => decodeCsrrsi_n1 h n1

/-- A call the spec decodes: the ZisK program holds its translation. -/
theorem transpiled_call {zrom : BitVec 64 → Zisk.zisk_inst} {pc : BitVec 64} {w : BitVec 32}
    {next : Option (BitVec 32)} (ht : Equiv.Sim.Transpiled zrom pc w next) {c : Call}
    (h : decode (Transpiler.fetch2 pc w next) pc.toNat = .call c) :
    ∃ ia, ∀ p ∈ Transpiler.callInsts c pc ia, zrom p.1 = p.2 := by
  obtain ⟨ia, l, hl, hz⟩ := ht
  obtain ⟨w', hw', hk⟩ := decode_call_kind h
  have : w' = w := by simp [Transpiler.fetch2] at hw'; exact hw'.symm
  subst this
  have hd := spec_decode_csr w' hk
  simp only [Transpiler.convert, hd, Transpiler.csr, h] at hl
  cases hl
  exact ⟨ia, hz⟩


end Equiv.Machine
