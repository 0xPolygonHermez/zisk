/-
  Totality, the SYSTEM words: `ecall`, `ebreak` and the CSR instructions.
  `ecall` is the exit (an end) or breaks precondition E; `ebreak` is a word
  the transpiler halts on. A CSR instruction is what the machine's decoder
  makes of it: a ZisK call (a step, the call's failure, or precondition I
  broken by it), `csrs ID, x0` (a step), a sequence the transpiler refuses
  (an end), reading marchid (a step), a write to a read-only CSR (a halt);
  or, outside the contract's forms, precondition C or Z broken.
-/
import Equiv.TotalityMem

namespace Equiv.Final

open Equiv.Riscv Equiv.Sim Equiv.Machine Equiv.Top
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Transpiler (decode convert)

/-! ## ecall and ebreak -/

theorem decode_ecall {w : BitVec 32} (h : decode w = some .ecall) : w = ECALL := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first | (simp at h; done) | (simp_all [ECALL]; done)

theorem decode_ebreak {w : BitVec 32} (h : decode w = some .ebreak) : w = EBREAK := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first | (simp at h; done) | (simp_all [EBREAK]; done)

/-! ## A ZisK call -/

/-- A ZisK call's operation, on memory as ZisK starts it: it runs, or it
    fails, on a byte off ZisK's regions if on a missing one, or it fails on
    one of the free input's 8 bytes (`Frame`: a failure on a missing byte
    is one memory lacks). -/
theorem call_cases (S : ZiskRiscv.State model) (c : ZiskRiscv.Call) (hf : Equiv.Full S.rv.mem) :
    CallOk S c ∨ CallFail S c ∨
      ∃ k σ', (_root_.Zisk.Functions.zisk_exec_mem c.op (S.argValue c.a) (S.argValue c.b)
        (if _root_.Zisk.Functions.zisk_op_is_precompiled c.op then c.ext else 0)).run
        (ZiskRiscv.ziskState (Equiv.normIn (callMem S c)) S.fcall) = .error (.OutOfMemoryRange k) σ' ∧
        Equiv.IN_LO ≤ k ∧ k < Equiv.IN_LO + 8 := by
  cases hr : (_root_.Zisk.Functions.zisk_exec_mem c.op (S.argValue c.a) (S.argValue c.b)
      (if _root_.Zisk.Functions.zisk_op_is_precompiled c.op then c.ext else 0)).run
      (ZiskRiscv.ziskState (Equiv.normIn (callMem S c)) S.fcall) with
  | ok r σ' => exact .inl ⟨r, σ', hr⟩
  | error e σ' =>
    have hab := ((Equiv.Zisk.Frame.exec_mem _ _ _ _).2 _ _ (Equiv.Zisk.FR.ref_machine _ _) e σ' hr).2
    by_cases hin : ∃ k, e = .OutOfMemoryRange k ∧ Equiv.IN_LO ≤ k ∧ k < Equiv.IN_LO + 8
    · obtain ⟨k, rfl, hk⟩ := hin
      exact .inr (.inr ⟨k, σ', rfl, hk⟩)
    · refine .inr (.inl ⟨e, σ', hr, fun k hk => ⟨fun h => hin ⟨k, hk, h⟩, ?_⟩⟩)
      have hn : (Equiv.normIn (callMem S c)).get? k = none := hab k hk
      rw [Equiv.normIn_get, if_neg (fun h => hin ⟨k, hk, h⟩)] at hn
      have hfc : Equiv.Full (callMem S c) := by
        unfold callMem; split
        · exact hf.grow (by rw [writeDword_eq]; exact memWrite_grows' _ _ _)
        · exact hf
      cases hreg : ZiskRiscv.inRegions k
      · rfl
      · have := hfc k hreg; rw [hn] at this; cases this

/-! ## The machine's decoder on a CSR instruction -/

theorem csrrs_nop {f : ZiskRiscv.Fields} {n0 n0' : Option ZiskRiscv.Fields}
    (h : ZiskRiscv.decodeCsrrs f n0 = .nop) : ZiskRiscv.decodeCsrrs f n0' = .nop := by
  revert h
  unfold ZiskRiscv.decodeCsrrs
  split
  all_goals simp only [ZiskRiscv.memsetPair, ZiskRiscv.dmaPair]
  all_goals intro h
  all_goals first
    | exact h
    | (exfalso; revert h; (repeat' split) <;> simp <;> done)
    | (revert h; (repeat' split) <;> simp_all)

/-- The machine's nop reads the word at pc alone. -/
theorem nop_any {fetch fetch' : Nat → Option (BitVec 32)} {a : Nat} (h : ZiskRiscv.decode fetch a = .nop)
    (hf : fetch' a = fetch a) : ZiskRiscv.decode fetch' a = .nop := by
  cases hw : fetch a with
  | none => simp [ZiskRiscv.decode, hw] at h
  | some w =>
    have hk := nop_kind hw h
    simp only [ZiskRiscv.decode, hw, hk] at h
    simp only [ZiskRiscv.decode, hf, hw, hk]
    exact csrrs_nop h

theorem csrrsi_riscv {f : ZiskRiscv.Fields} {n0 n1 n1' : Option ZiskRiscv.Fields}
    (h : ZiskRiscv.decodeCsrrsi f n0 n1 = .riscv) : ZiskRiscv.decodeCsrrsi f n0 n1' = .riscv := by
  revert h
  unfold ZiskRiscv.decodeCsrrsi
  split
  · intro h; exfalso; revert h; (repeat' split) <;> simp [ZiskRiscv.memsetPair] <;> (repeat' split) <;> simp
  · intro _; rfl

/-- Plain RISC-V to the machine's decoder, given the two words after pc:
    so given the next one only (the word after it settles memset's
    sequence alone). -/
theorem riscv_fetch2 {pc : BitVec 64} {w : BitVec 32} {n n2 : Option (BitVec 32)}
    (h : ZiskRiscv.decode (Transpiler.fetch3 pc w n n2) pc.toNat = .riscv) :
    ZiskRiscv.decode (Transpiler.fetch2 pc w n) pc.toNat = .riscv := by
  have e4 : pc.toNat + 4 ≠ pc.toNat := by omega
  have e8 : pc.toNat + 8 ≠ pc.toNat := by omega
  have e84 : pc.toNat + 8 ≠ pc.toNat + 4 := by omega
  simp only [ZiskRiscv.decode, Transpiler.fetch3, Transpiler.fetch2, if_true, e4, e8, e84, if_false] at h ⊢
  revert h
  cases ZiskRiscv.kind (ZiskRiscv.fields w) <;> simp
  exact csrrsi_riscv

/-- A CSR instruction the machine leaves to sail-riscv, which the spec
    translates: `csr`'s translation, with the next word alone. -/
theorem csr_riscv {pc : BitVec 64} {w : BitVec 32} {next next2 : Option (BitVec 32)} {ia : BitVec 64}
    {l : List (BitVec 64 × _root_.Zisk.zisk_inst)} (hd : decode w = some .csr)
    (hD : ZiskRiscv.decode (Transpiler.fetch3 pc w next next2) pc.toNat = .riscv)
    (hl : convert pc w next ia next2 = some l) : Transpiler.csr w pc next ia = some l := by
  simp only [convert, hd, Transpiler.csr3] at hl
  cases next2 with
  | none => exact hl
  | some n2 =>
    simp only at hl
    split at hl
    · rw [hD] at hl; cases hl
    · exact hl

/-- `csr`'s translation of what the machine leaves to sail-riscv: reading
    marchid, or a halt on a write to a read-only CSR. -/
theorem csr_riscv_cases {pc : BitVec 64} {w : BitVec 32} {next : Option (BitVec 32)} {ia : BitVec 64}
    {l : List (BitVec 64 × _root_.Zisk.zisk_inst)}
    (hD : ZiskRiscv.decode (Transpiler.fetch2 pc w next) pc.toNat = .riscv)
    (hl : Transpiler.csr w pc next ia = some l) :
    (ZiskRiscv.kind (ZiskRiscv.fields w) = .csrrs ∧ (ZiskRiscv.fields w).csr = 0xF12 ∧
      (ZiskRiscv.fields w).rs1 = 0 ∧ (ZiskRiscv.fields w).rd ≠ 0) ∨
    Transpiler.writesReadOnly (ZiskRiscv.fields w) = true := by
  simp only [Transpiler.csr, hD] at hl
  split at hl
  · next h =>
    simp only [Bool.and_eq_true, bne_iff_ne, ne_eq, beq_iff_eq] at h
    obtain ⟨⟨⟨hk, hc⟩, hr⟩, hd⟩ := h
    refine .inl ⟨?_, hc, hr, hd⟩
    revert hk; cases ZiskRiscv.kind (ZiskRiscv.fields w) <;> decide
  · split at hl
    · next h => exact .inr h
    · cases hl

/-! ## Reading marchid, writing a read-only CSR -/

def marchidOfWord (w : BitVec 32) : MarchidInstr := ⟨w.extractLsb' 7 5⟩

theorem marchidOfWord_encode {w : BitVec 32} (hk : ZiskRiscv.kind (ZiskRiscv.fields w) = .csrrs)
    (hc : (ZiskRiscv.fields w).csr = 0xF12) (hr : (ZiskRiscv.fields w).rs1 = 0) : (marchidOfWord w).encode = w := by
  obtain ⟨h1, h2⟩ := csrrs_fields hk
  have hc' := bv_of_toNat (x := w.extractLsb' 20 12) hc
  have hr' := bv_of_toNat (x := w.extractLsb' 15 5) hr
  simp only [marchidOfWord, MarchidInstr.encode]
  revert h1 h2 hc' hr'; bv_decide

theorem marchidOfWord_rd {w : BitVec 32} (hd : (ZiskRiscv.fields w).rd ≠ 0) : (marchidOfWord w).rd ≠ 0 := by
  intro e; apply hd; simp only [ZiskRiscv.fields]; simp only [marchidOfWord] at e; rw [e]; rfl

/-- A CSR instruction's kind, from its funct3. -/
theorem kind_csr {f : ZiskRiscv.Fields} {k : ZiskRiscv.Kind} (h : ZiskRiscv.kind f = k)
    (hk : k = .csrrw ∨ k = .csrrs ∨ k = .csrrc ∨ k = .csrrwi ∨ k = .csrrsi ∨ k = .csrrci) :
    f.opcode = 115 ∧
      (k = .csrrw → f.funct3 = 1) ∧ (k = .csrrs → f.funct3 = 2) ∧ (k = .csrrc → f.funct3 = 3) ∧
      (k = .csrrwi → f.funct3 = 5) ∧ (k = .csrrsi → f.funct3 = 6) ∧ (k = .csrrci → f.funct3 = 7) := by
  subst h
  unfold ZiskRiscv.kind at hk ⊢
  split at hk <;> (try split at hk) <;> (try split at hk) <;> simp_all

/-- What `writesReadOnly` reads of a word: SYSTEM's opcode, a CSR number
    of a read-only CSR, an instruction that writes it. -/
theorem ro_fields {w : BitVec 32} (h : Transpiler.writesReadOnly (ZiskRiscv.fields w) = true) :
    w.extractLsb' 0 7 = 0x73#7 ∧ (w.extractLsb' 20 12).extractLsb 11 10 = 3#2 ∧
      (w.extractLsb' 12 3 = 1 ∨ w.extractLsb' 12 3 = 5 ∨
        ((w.extractLsb' 12 3 = 2 ∨ w.extractLsb' 12 3 = 3 ∨ w.extractLsb' 12 3 = 6 ∨ w.extractLsb' 12 3 = 7) ∧
          w.extractLsb' 15 5 ≠ 0)) := by
  simp only [Transpiler.writesReadOnly, Bool.and_eq_true] at h
  obtain ⟨hw, hc⟩ := h
  have hro : (w.extractLsb' 20 12).extractLsb 11 10 = 3#2 := by
    apply BitVec.eq_of_toNat_eq
    simp only [ZiskRiscv.fields, beq_iff_eq] at hc
    simp only [BitVec.extractLsb, BitVec.extractLsb'_toNat, Nat.shiftRight_eq_div_pow] at hc ⊢
    simp; omega
  cases hk : ZiskRiscv.kind (ZiskRiscv.fields w) <;> simp only [hk] at hw
  all_goals first | (cases hw; done) | skip
  all_goals
    obtain ⟨ho, h1, h2, h3, h5, h6, h7⟩ := kind_csr hk (by simp)
    simp only [ZiskRiscv.fields] at ho h1 h2 h3 h5 h6 h7
    refine ⟨bv_of_toNat ho, hro, ?_⟩
    simp only [forall_const, reduceCtorEq, false_implies] at h1 h2 h3 h5 h6 h7
    first
      | exact .inl (bv_of_toNat (by assumption))
      | exact .inr (.inl (bv_of_toNat (by assumption)))
      | (simp only [ZiskRiscv.fields, bne_iff_ne, ne_eq] at hw
         refine .inr (.inr ⟨?_, fun e => hw (by rw [e]; rfl)⟩)
         first
           | exact .inl (bv_of_toNat (by assumption))
           | exact .inr (.inl (bv_of_toNat (by assumption)))
           | exact .inr (.inr (.inl (bv_of_toNat (by assumption))))
           | exact .inr (.inr (.inr (bv_of_toNat (by assumption)))))

def roOfWord (w : BitVec 32) (h : Transpiler.writesReadOnly (ZiskRiscv.fields w) = true) : RoInstr :=
  ⟨w.extractLsb' 20 12, w.extractLsb' 15 5, w.extractLsb' 12 3, w.extractLsb' 7 5, (ro_fields h).2.1,
    (ro_fields h).2.2⟩

theorem roOfWord_encode {w : BitVec 32} (h : Transpiler.writesReadOnly (ZiskRiscv.fields w) = true) :
    (roOfWord w h).encode = w := by
  have h1 := (ro_fields h).1
  simp only [roOfWord, RoInstr.encode]; revert h1; bv_decide

/-! ## What the machine's decoder makes of a CSR instruction -/

set_option maxHeartbeats 2000000 in
theorem decode_csr_fields {w : BitVec 32} (h : decode w = some .csr) :
    (ZiskRiscv.fields w).opcode = 115 ∧ (ZiskRiscv.fields w).funct3 ≠ 0 ∧ (ZiskRiscv.fields w).funct3 ≠ 4 := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first
    | (simp at h; done)
    | (simp only [Transpiler.opcode, Transpiler.funct3, BitVec.extractLsb, beq_iff_eq, Bool.or_eq_true,
         Nat.reduceSub, Nat.reduceAdd] at *
       simp only [ZiskRiscv.fields]; cases h; omega)

theorem decode_csr_kind {w : BitVec 32} (h : decode w = some .csr) :
    ZiskRiscv.kind (ZiskRiscv.fields w) = .csrrw ∨ ZiskRiscv.kind (ZiskRiscv.fields w) = .csrrs ∨
      ZiskRiscv.kind (ZiskRiscv.fields w) = .csrrc ∨ ZiskRiscv.kind (ZiskRiscv.fields w) = .csrrwi ∨
      ZiskRiscv.kind (ZiskRiscv.fields w) = .csrrsi ∨ ZiskRiscv.kind (ZiskRiscv.fields w) = .csrrci := by
  obtain ⟨ho, h0, h4⟩ := decode_csr_fields h
  have h8 : (ZiskRiscv.fields w).funct3 < 8 := by simp only [ZiskRiscv.fields]; exact BitVec.isLt _
  unfold ZiskRiscv.kind
  rw [ho]
  generalize (ZiskRiscv.fields w).funct3 = f at h0 h4 h8
  rcases (by omega : f = 1 ∨ f = 2 ∨ f = 3 ∨ f = 5 ∨ f = 6 ∨ f = 7) with rfl | rfl | rfl | rfl | rfl | rfl <;> simp

/-- A CSR instruction is no `ecall` and no lr or sc to the machine. -/
theorem csr_not_ecall_subst {w : BitVec 32} (h : decode w = some .csr) {fetch : Nat → Option (BitVec 32)} {a : Nat}
    (hf : fetch a = some w) :
    ZiskRiscv.decode fetch a ≠ .ecall ∧ ∀ w' zr, ZiskRiscv.decode fetch a ≠ .subst w' zr := by
  have hk := decode_csr_kind h
  simp only [ZiskRiscv.decode, hf]
  rcases hk with hk | hk | hk | hk | hk | hk <;> simp only [hk] <;>
    (try simp only [ZiskRiscv.decodeCsrrs, ZiskRiscv.decodeCsrrsi, ZiskRiscv.decodeCsrrwi]) <;>
    refine ⟨?_, fun _ _ => ?_⟩ <;> (repeat' split) <;>
    (try simp [ZiskRiscv.memsetPair, ZiskRiscv.dmaPair]) <;> (repeat' split) <;> (try simp)

/-! ## The step's facts from the context -/

/-- The word at pc, where the spec translates it reading the next word
    only: the facts every location structure takes. -/
theorem ctx_words {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {S : ZiskRiscv.State model} {pc : BitVec 64} {w : BitVec 32} (hx : Ctx zrom C S pc w)
    (hdesc : ∃ ia l, convert pc w (nextWord S.rv pc) ia = some l) :
    WordsAt S.rv ziskRegions pc zrom w (nextWord S.rv pc) := by
  have hb := readWord_bytes hx.word
  obtain ⟨ia, l, hl⟩ := hdesc
  have ht := hx.spec pc w hx.code hx.word ⟨ia, l, by
    have := convert_next2 ⟨ia, l, hl⟩ ia (nextWord2 S.rv pc); exact this.trans hl⟩
  exact ⟨hx.aligned pc hx.code, hx.exec pc hx.code, exec_add2 (hx.exec pc hx.code) (hx.aligned pc hx.code),
    hb.1, hb.2.1, hb.2.2.1, hb.2.2.2, ht.drop2 ⟨ia, l, hl⟩⟩

/-- The step's cases, from one of the ends. -/
theorem StepCase.ofEnd {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {S : ZiskRiscv.State model} (h : EndAt zrom C S) : StepCase zrom C S :=
  .inr (.inr (.inr (.inl h)))

/-! ## The SYSTEM group, assembled -/

theorem kind_ecall {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {S : ZiskRiscv.State model} {pc : BitVec 64} {w : BitVec 32}
    (hx : Ctx zrom C S pc w) (hd : decode w = some .ecall) : StepCase zrom C S := by
  have e := decode_ecall hd; subst e
  by_cases h93 : getX S.rv 17 = some 93
  · exact StepCase.ofEnd (.inl ⟨pc, hx.atPc, hx.code, hx.word, h93⟩)
  · exact StepCase.ofBroken hx (.inl ⟨rfl, h93⟩)

theorem kind_ebreak {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {S : ZiskRiscv.State model} {pc : BitVec 64} {w : BitVec 32}
    (hx : Ctx zrom C S pc w) (hd : decode w = some .ebreak) : StepCase zrom C S := by
  have hw := ctx_words hx ⟨0, _, by simp only [convert, hd]; rfl⟩
  have e := decode_ebreak hd; subst e
  exact StepCase.ofEnd (.inr (.inr (.inr (.inl ⟨pc, EBREAK, hx.atPc, .inl rfl, hw⟩))))

set_option maxHeartbeats 1000000 in
theorem kind_csrw {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {S : ZiskRiscv.State model} {pc : BitVec 64} {w : BitVec 32}
    (hx : Ctx zrom C S pc w) (hd : decode w = some .csr) : StepCase zrom C S := by
  by_cases hb : ∀ ia, convert pc w (nextWord S.rv pc) ia (nextWord2 S.rv pc) = none
  · exact StepCase.ofBroken hx (.inr (.inr (.inl ⟨hd, hb⟩)))
  obtain ⟨ia, hne⟩ := Classical.not_forall.1 hb
  obtain ⟨l, hl⟩ := Option.ne_none_iff_exists'.1 hne
  have htr : Transpiled zrom pc w (nextWord S.rv pc) (nextWord2 S.rv pc) :=
    hx.spec pc w hx.code hx.word ⟨ia, l, hl⟩
  have hex := hx.exec pc hx.code
  have hrom := exec_rom hex
  have h4 : (pc + 4).toNat = pc.toNat + 4 := by rw [BitVec.toNat_add]; simp; omega
  have h8 : (pc + 8).toNat = pc.toNat + 8 := by rw [BitVec.toNat_add]; simp; omega
  have hC4 : C (pc + 4) := hx.next1 (by simp [Transpiler.usesNext, hd])
  have hC8 : C (pc + 8) := hx.next2 (by simp [Transpiler.usesNext2, hd])
  have hf3 : Transpiler.fetch3 pc w (nextWord S.rv pc) (nextWord2 S.rv pc) pc.toNat = some w := by
    simp [Transpiler.fetch3]
  have hns := csr_not_ecall_subst hd hf3
  cases hD : ZiskRiscv.decode (Transpiler.fetch3 pc w (nextWord S.rv pc) (nextWord2 S.rv pc)) pc.toNat with
  | call c =>
    rcases call_cases S c hx.full with hok | hfail | ⟨k, σ', hk, hin⟩
    · exact .inl (.inr (.inl ⟨pc, w, c, hx.atPc, hex, hx.word, htr, h4, h8, hD, fun _ => hC4, fun _ => hC8, hok⟩))
    · exact StepCase.ofEnd (.inr (.inl ⟨pc, w, c, hx.atPc, hx.code, hx.word, hD, fun _ => hC4, fun _ => hC8,
        hfail⟩))
    · exact .inr (.inr (.inr (.inr (.inr (.inr ⟨pc, w, c, hx.atPc, hx.word, hD, k, σ', hk, hin⟩)))))
  | nop =>
    exact .inl (.inr (.inr (.inr ⟨pc, w, hx.atPc, hex, hx.word, htr,
      fun fetch hf => nop_any hD (by rw [hf, hf3])⟩)))
  | malformed why =>
    exact StepCase.ofEnd (.inr (.inr (.inr (.inr (.inl ⟨pc, w, why, hx.atPc, hx.code, hx.word, hD,
      ⟨ia, l, hl⟩, hC4, hC8⟩)))))
  | riscv =>
    have hl' := csr_riscv hd hD hl
    have hw := ctx_words hx ⟨ia, l, by simp only [convert, hd, Transpiler.csr3]; exact hl'⟩
    rcases csr_riscv_cases (riscv_fetch2 hD) hl' with ⟨hk, hc, hr, hrd⟩ | hro
    · have e := marchidOfWord_encode hk hc hr
      rw [← e] at hw
      exact StepCase.ofOneOkT ⟨pc, hx.atPc, .inr (.inr (.inr (.inr (.inr ⟨marchidOfWord w, hw,
        marchidOfWord_rd hrd, hx.marchid⟩))))⟩
    · exact StepCase.ofEnd (.inr (.inr (.inr (.inl ⟨pc, w, hx.atPc, .inr (.inl ⟨roOfWord w hro,
        roOfWord_encode hro⟩), hw⟩))))
  | ecall => exact absurd hD hns.1
  | subst w' zr => exact absurd hD (hns.2 w' zr)

/-- `KindCases.sys`: `ecall`, `ebreak` or a CSR instruction at pc, the
    step's cases. -/
theorem kind_sys {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop} :
    ∀ S pc w i, Ctx zrom C S pc w → decode w = some i → group i = .sys → StepCase zrom C S := by
  intro S pc w i hx hd hg
  cases i with
  | ecall => exact kind_ecall hx hd
  | ebreak => exact kind_ebreak hx hd
  | csr => exact kind_csrw hx hd
  | _ => simp [group] at hg

end Equiv.Final
