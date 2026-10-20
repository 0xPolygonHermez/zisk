/-
  The model's step with memory sources and stores.

  The model's memory is a map from byte addresses to bytes; `read_ram` reads
  `n` of them little-endian and fails on a byte the map lacks, `write_ram`
  inserts them. The memory bus works with 8-byte words, so memory is seen
  here as words: `MemOk σ W D`, the model's bytes hold the word `W a` at
  every address `a` of a set `D` of multiples of 8.

  * `read_ram_word`, `write_ram_word`: a held word is what an 8-byte read
    returns; a write holds the word it writes and keeps every other byte.
  * `execute_mem`: as `execute_gen` (`ModelRegs.lean`), with memory sources —
    `A_MEM`, `B_MEM`, and `B_IND` of width 8 — and memory stores — `ST_MEM`,
    and `ST_IND` of width 8 — at addresses of `D` away from the register
    file: the step reads the words memory holds and leaves memory as the
    store's update of them (`storeW`).
-/
import Proofs.ModelRegs

open Out.Functions PreSail Sail

namespace Correspondence

/-- The eight bytes at `a` hold the word `w`, little-endian. -/
def HoldsWord (m : Std.HashMap Nat (BitVec 8)) (a : Nat) (w : BitVec 64) : Prop :=
  ∀ k, k < 8 → m.get? (a + k) = some (w.extractLsb' (8 * k) 8)

theorem read_ram_word (σ : State) (addr : BitVec 64) (w : BitVec 64)
    (h : HoldsWord σ.mem addr.toNat w) :
    (Sail.read_ram 64 8 (zeros (n := 64)) addr : SailM (BitVec (8 * 8))) σ = .ok w σ := by
  have h0 := h 0 (by omega); have h1 := h 1 (by omega); have h2 := h 2 (by omega)
  have h3 := h 3 (by omega); have h4 := h 4 (by omega); have h5 := h 5 (by omega)
  have h6 := h 6 (by omega); have h7 := h 7 (by omega)
  simp only [Nat.add_zero, Std.HashMap.get?_eq_getElem?] at h0 h1 h2 h3 h4 h5 h6 h7
  simp [Sail.read_ram, PreSail.read_ram, PreSail.readBytes, PreSail.readByte, bind, EStateM.bind, get,
    getThe, MonadStateOf.get, EStateM.get, pure, EStateM.pure, h0, h1, h2, h3, h4, h5, h6, h7,
    Nat.add_assoc, EStateM.pure, throw, throwThe, MonadExceptOf.throw]
  apply BitVec.eq_of_getLsbD_eq
  intro i hi
  repeat rw [BitVec.getLsbD_append]
  simp only [BitVec.getLsbD_extractLsb', cond_eq_if]
  repeat' split
  all_goals first
    | omega
    | (simp only [Bool.and_eq_true, decide_eq_true_eq] at *; omega)
    | (simp only [decide_eq_true (show _ < 8 by omega), Bool.true_and]; congr 1; omega)
    | done

/-- Memory after writing the word `v` at `a`: its eight bytes, little-endian. -/
def writeWord (m : Std.HashMap Nat (BitVec 8)) (a : Nat) (v : BitVec 64) : Std.HashMap Nat (BitVec 8) :=
  ((((((((m.insert a (v.extractLsb' 0 8)).insert (a + 1) (v.extractLsb' 8 8)).insert (a + 2)
    (v.extractLsb' 16 8)).insert (a + 3) (v.extractLsb' 24 8)).insert (a + 4) (v.extractLsb' 32 8)).insert
    (a + 5) (v.extractLsb' 40 8)).insert (a + 6) (v.extractLsb' 48 8)).insert (a + 7) (v.extractLsb' 56 8))

theorem write_ram_word (σ : State) (addr : BitVec 64) (v : BitVec 64) :
    (Sail.write_ram 64 8 (zeros (n := 64)) addr v : SailM Unit) σ =
      .ok () { σ with mem := writeWord σ.mem addr.toNat v } := by
  simp [Sail.write_ram, PreSail.write_ram, PreSail.writeBytes, PreSail.writeByte, List.ofFn,
    Fin.foldr, Fin.foldr.loop, bind, EStateM.bind, pure, EStateM.pure, modify, modifyGet,
    MonadStateOf.modifyGet, EStateM.modifyGet, List.forM, writeWord]
  done

theorem ins_ne (m : Std.HashMap Nat (BitVec 8)) {k b : Nat} (v : BitVec 8) (h : k ≠ b) :
    (m.insert k v)[b]? = m[b]? := by
  rw [Std.HashMap.getElem?_insert, if_neg (by simpa using h)]

theorem ins_self (m : Std.HashMap Nat (BitVec 8)) (k : Nat) (v : BitVec 8) :
    (m.insert k v)[k]? = some v := Std.HashMap.getElem?_insert_self

theorem writeWord_holds (m : Std.HashMap Nat (BitVec 8)) (a : Nat) (v : BitVec 64) :
    HoldsWord (writeWord m a v) a v := by
  intro k hk
  simp only [writeWord, Std.HashMap.get?_eq_getElem?]
  rcases k with _ | _ | _ | _ | _ | _ | _ | _ | k
  all_goals first
    | omega
    | (repeat (first | rw [ins_self] | rw [ins_ne _ _ (by omega)]))
  all_goals first | rfl | (simp only [Nat.add_zero]; repeat (first | rw [ins_self] | rw [ins_ne _ _ (by omega)]))

theorem writeWord_other (m : Std.HashMap Nat (BitVec 8)) (a : Nat) (v : BitVec 64) {b : Nat}
    (hb : b < a ∨ a + 8 ≤ b) : (writeWord m a v).get? b = m.get? b := by
  simp only [writeWord, Std.HashMap.get?_eq_getElem?]
  repeat rw [ins_ne _ _ (by omega)]

/-- A word written elsewhere leaves a held word held. -/
theorem HoldsWord.write {m : Std.HashMap Nat (BitVec 8)} {b : Nat} {w : BitVec 64}
    (h : HoldsWord m b w) (a : Nat) (v : BitVec 64) (hab : b + 8 ≤ a ∨ a + 8 ≤ b) :
    HoldsWord (writeWord m a v) b w := by
  intro k hk
  rw [writeWord_other m a v (by omega)]
  exact h k hk

/-! ### Reading and writing words -/

/-- Reading a held word, away from the register file. -/
theorem read_mem_word (σ : State) (addr : BitVec 64) (w : BitVec 64)
    (hr : hits_registers addr 8 = false) (h : HoldsWord σ.mem addr.toNat w) :
    read_mem addr 8 σ = .ok w σ := by
  unfold read_mem
  simp only [hr, Bool.not_false, PreSail.assert, bind, EStateM.bind, pure, EStateM.pure, if_true]
  exact read_ram_word σ addr w h

/-- Writing a word, away from the register file. -/
theorem write_mem_word (σ : State) (addr : BitVec 64) (v : BitVec 64)
    (hr : hits_registers addr 8 = false) :
    write_mem addr 8 v σ = .ok () { σ with mem := writeWord σ.mem addr.toNat v } := by
  unfold write_mem
  simp only [hr, Bool.not_false, PreSail.assert, bind, EStateM.bind, pure, EStateM.pure, if_true]
  exact write_ram_word σ addr v

theorem zero_extend_self (x : BitVec 64) : zero_extend x 64 = x := by
  simp [zero_extend, Sail.BitVec.zeroExtend]

/-! ### Memory as words -/

/-- The model's memory holds the word `W a` at every address `a` of `D`. -/
def MemOk (σ : State) (W : Nat → BitVec 64) (D : Nat → Prop) : Prop :=
  ∀ a, D a → HoldsWord σ.mem a (W a)

/-- An address the step may read or write a word at: in `D`, away from the
register file. -/
def MemAt (D : Nat → Prop) (addr : BitVec 64) : Prop := D addr.toNat ∧ hits_registers addr 8 = false

/-- Source `a`'s value, memory included. -/
def aValM (inst : zisk_inst) (zc : BitVec 64) (xs : Vector (BitVec 64) 32) (W : Nat → BitVec 64) :
    BitVec 64 :=
  match inst.a_src with
  | .A_MEM => W inst.a_imm.toNat
  | _ => aVal inst zc xs

/-- Source `b`'s value, memory included; `a` is source `a`'s. -/
def bValM (inst : zisk_inst) (zc : BitVec 64) (xs : Vector (BitVec 64) 32) (W : Nat → BitVec 64)
    (a : BitVec 64) : BitVec 64 :=
  match inst.b_src with
  | .B_MEM => W inst.b_imm.toNat
  | .B_IND => W (a + inst.b_imm).toNat
  | _ => bVal inst zc xs

def ASrcOkM (D : Nat → Prop) (inst : zisk_inst) : Prop :=
  ASrcOk inst ∨ (inst.a_src = .A_MEM ∧ MemAt D inst.a_imm)

def BSrcOkM (D : Nat → Prop) (inst : zisk_inst) (a : BitVec 64) : Prop :=
  BSrcOk inst ∨ (inst.b_src = .B_MEM ∧ MemAt D inst.b_imm) ∨
    (inst.b_src = .B_IND ∧ inst.ind_width = 8 ∧ MemAt D (a + inst.b_imm))

def StoreOkM (D : Nat → Prop) (inst : zisk_inst) (a : BitVec 64) : Prop :=
  StoreOk inst ∨ (inst.store = .ST_MEM ∧ MemAt D inst.store_off) ∨
    (inst.store = .ST_IND ∧ inst.ind_width = 8 ∧ MemAt D (a + inst.store_off))

/-- The address a store writes a word at, if it stores to memory. -/
def storeAddr (inst : zisk_inst) (a : BitVec 64) : Option (BitVec 64) :=
  match inst.store with
  | .ST_MEM => some inst.store_off
  | .ST_IND => some (a + inst.store_off)
  | _ => none

/-- Memory, as words, after the store. -/
def storeW (inst : zisk_inst) (pc c a : BitVec 64) (W : Nat → BitVec 64) : Nat → BitVec 64 :=
  match storeAddr inst a with
  | some s => fun b => if b = s.toNat then storeVal inst pc c else W b
  | none => W

theorem source_a_okM (inst : zisk_inst) (σ : State) (zc : BitVec 64) (xs : Vector (BitVec 64) 32)
    {W : Nat → BitVec 64} {D : Nat → Prop} (hm : MemOk σ W D)
    (hzc : σ.regs.get? Register.Zc = some zc) (hxs : σ.regs.get? Register.Xs = some xs)
    (ha : ASrcOkM D inst) : source_a inst σ = .ok (aValM inst zc xs W) σ := by
  rcases ha with ha | ⟨ha, hd, hr⟩
  · have e := source_a_ok inst σ zc xs hzc hxs ha
    have : aValM inst zc xs W = aVal inst zc xs := by
      unfold aValM
      rcases ha with h | h | ⟨h, -⟩ <;> rw [h]
    rw [this]; exact e
  · simp only [source_a, aValM, ha]
    exact read_mem_word σ _ _ hr (hm _ hd)

theorem source_b_okM (inst : zisk_inst) (a : BitVec 64) (σ : State) (zc : BitVec 64)
    (xs : Vector (BitVec 64) 32) {W : Nat → BitVec 64} {D : Nat → Prop} (hm : MemOk σ W D)
    (hzc : σ.regs.get? Register.Zc = some zc) (hxs : σ.regs.get? Register.Xs = some xs)
    (hb : BSrcOkM D inst a) : source_b inst a σ = .ok (bValM inst zc xs W a) σ := by
  rcases hb with hb | ⟨hb, hd, hr⟩ | ⟨hb, hw, hd, hr⟩
  · have e := source_b_ok inst a σ zc xs hzc hxs hb
    have : bValM inst zc xs W a = bVal inst zc xs := by
      unfold bValM
      rcases hb with h | h | ⟨h, -⟩ <;> rw [h]
    rw [this]; exact e
  · simp only [source_b, bValM, hb]
    exact read_mem_word σ _ _ hr (hm _ hd)
  · simp only [source_b, bValM, hb, hw]
    have e := read_mem_word σ _ _ hr (hm _ hd)
    have h8 : inst.ind_width.toNat = 8 := by rw [hw]; rfl
    generalize inst.ind_width.toNat = n at h8 ⊢
    subst h8
    simp only [bind, EStateM.bind, e, pure, EStateM.pure, zero_extend_self]

/-- Writing word `v` at `s`, an address of `D`, keeps memory as words: `v` at
`s`, the rest as before. -/
theorem MemOk.write {σ : State} {W : Nat → BitVec 64} {D : Nat → Prop} (hm : MemOk σ W D)
    (hD : ∀ b, D b → b % 8 = 0) {s : Nat} (hs : D s) (v : BitVec 64) :
    MemOk { σ with mem := writeWord σ.mem s v } (fun b => if b = s then v else W b) D := by
  intro b hb
  by_cases e : b = s
  · subst e; simp only [if_true]; exact writeWord_holds _ _ _
  · simp only [e, if_false]
    have := hD b hb; have := hD s hs
    exact (hm b hb).write s v (by omega)

theorem store_c_okM (inst : zisk_inst) (a c : BitVec 64) (σ : State) (pc : BitVec 64)
    (xs : Vector (BitVec 64) 32) {W : Nat → BitVec 64} {D : Nat → Prop} (hm : MemOk σ W D)
    (hD : ∀ b, D b → b % 8 = 0)
    (hpc : σ.regs.get? Register.PC = some pc) (hxs : σ.regs.get? Register.Xs = some xs)
    (hs : StoreOkM D inst a) :
    ∃ σ₁, store_c inst a c σ = .ok () σ₁ ∧
      σ₁.regs.get? Register.PC = σ.regs.get? Register.PC ∧
      σ₁.regs.get? Register.Zc = σ.regs.get? Register.Zc ∧
      σ₁.regs.get? Register.Step = σ.regs.get? Register.Step ∧
      σ₁.regs.get? Register.Xs = some (storeXs inst pc c xs) ∧
      MemOk σ₁ (storeW inst pc c a W) D := by
  have hv : value_to_store inst c σ = .ok (storeVal inst pc c) σ := by
    unfold value_to_store storeVal
    cases inst.store_pc <;> simp [readReg_eq, hpc, bind, EStateM.bind, pure, EStateM.pure]
  rcases hs with hs | ⟨hs, hd, hr⟩ | ⟨hs, hw, hd, hr⟩
  · rcases hs with hs | ⟨hs, hr⟩
    · refine ⟨σ, ?_, rfl, rfl, rfl, by simp [storeXs, hs, hxs], ?_⟩
      · unfold store_c
        rw [bind_ok hv]
        simp [hs, pure, EStateM.pure]
      · simpa [storeW, storeAddr, hs] using hm
    · have e : store_c inst a c σ = .ok () { σ with regs := σ.regs.insert Register.Xs (xs.set! (regIdx inst.store_off).toNat (storeVal inst pc c)) } := by
        unfold store_c
        rw [bind_ok hv]
        simp only [hs]
        exact wX_ok _ _ σ xs hxs hr
      refine ⟨_, e, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert],
        by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, storeXs, hs], ?_⟩
      simpa [storeW, storeAddr, hs, MemOk] using hm
  · have e : store_c inst a c σ =
        .ok () { σ with mem := writeWord σ.mem inst.store_off.toNat (storeVal inst pc c) } := by
      unfold store_c
      rw [bind_ok hv]
      simp only [hs]
      exact write_mem_word σ _ _ hr
    refine ⟨_, e, rfl, rfl, rfl, by simp [storeXs, hs, hxs], ?_⟩
    simpa [storeW, storeAddr, hs] using hm.write hD hd (storeVal inst pc c)
  · have e : store_c inst a c σ =
        .ok () { σ with mem := writeWord σ.mem (a + inst.store_off).toNat (storeVal inst pc c) } := by
      unfold store_c
      rw [bind_ok hv]
      simp only [hs]
      have h8 : inst.ind_width.toNat = 8 := by rw [hw]; rfl
      have h64 : ((fun x y => x * y) (8 : Int) inst.ind_width).toNat = 64 := by
        simp only [hw]; rfl
      rw [h64]
      generalize inst.ind_width.toNat = n at h8 ⊢
      subst h8
      have : BitVec.setWidth (8 * 8) (Sail.BitVec.truncate (storeVal inst pc c) 64) = storeVal inst pc c := by
        simp [Sail.BitVec.truncate]
      rw [this]
      exact write_mem_word σ _ _ hr
    refine ⟨_, e, rfl, rfl, rfl, by simp [storeXs, hs, hxs], ?_⟩
    simpa [storeW, storeAddr, hs] using hm.write hD hd (storeVal inst pc c)

/-- **One step with memory.** As `execute_gen`, with memory sources (`A_MEM`,
`B_MEM`, and `B_IND` of width 8) and memory stores (`ST_MEM`, and `ST_IND` of
width 8) at word addresses of `D`, away from the register file: the step
computes `zisk_exec inst.op a b` of its sources, stores, and moves on, and
memory, as words, is the store's update. -/
theorem execute_mem (inst : zisk_inst) (σ : State) (pc zc st : BitVec 64)
    (xs : Vector (BitVec 64) 32) {W : Nat → BitVec 64} {D : Nat → Prop} (hm : MemOk σ W D)
    (hD : ∀ b, D b → b % 8 = 0)
    (hpc : σ.regs.get? Register.PC = some pc) (hzc : σ.regs.get? Register.Zc = some zc)
    (hst : σ.regs.get? Register.Step = some st) (hxs : σ.regs.get? Register.Xs = some xs)
    (ha : ASrcOkM D inst) (hb : BSrcOkM D inst (aValM inst zc xs W))
    (hs : StoreOkM D inst (aValM inst zc xs W))
    (hop : inst.op ≠ .ZOP_HALT ()) (hend : inst.is_end = false) :
    ∃ σ', zisk_execute inst σ = .ok false σ' ∧
      σ'.regs.get? Register.Zc = some (zisk_exec inst.op (aValM inst zc xs W)
        (bValM inst zc xs W (aValM inst zc xs W))).1 ∧
      σ'.regs.get? Register.PC = some (nextPcV inst pc
        (zisk_exec inst.op (aValM inst zc xs W) (bValM inst zc xs W (aValM inst zc xs W))).1
        (zisk_exec inst.op (aValM inst zc xs W) (bValM inst zc xs W (aValM inst zc xs W))).2) ∧
      σ'.regs.get? Register.Step = some (BitVec.addInt st 1) ∧
      σ'.regs.get? Register.Xs = some (storeXs inst pc
        (zisk_exec inst.op (aValM inst zc xs W) (bValM inst zc xs W (aValM inst zc xs W))).1 xs) ∧
      MemOk σ' (storeW inst pc
        (zisk_exec inst.op (aValM inst zc xs W) (bValM inst zc xs W (aValM inst zc xs W))).1
        (aValM inst zc xs W) W) D := by
  generalize hr : zisk_exec inst.op (aValM inst zc xs W) (bValM inst zc xs W (aValM inst zc xs W)) = r
  obtain ⟨c, flag⟩ := r
  obtain ⟨σ₁, hs₁, p₁, z₁, t₁, x₁, m₁⟩ :=
    store_c_okM inst (aValM inst zc xs W) c σ pc xs hm hD hpc hxs hs
  have ea := source_a_okM inst σ zc xs hm hzc hxs ha
  have eb := source_b_okM inst (aValM inst zc xs W) σ zc xs hm hzc hxs hb
  simp only [zisk_execute, bind, EStateM.bind, ea, eb, hr, hs₁, pure, EStateM.pure]
  rw [hpc] at p₁
  rw [hzc] at z₁
  rw [hst] at t₁
  unfold MemOk at m₁
  cases hsp : inst.set_pc <;> cases flag <;>
  · simp [writeReg_run, readReg_eq, Std.ExtDHashMap.get?_insert, t₁, x₁, hend, nextPcV, next_pc,
      p₁, hsp, bind, EStateM.bind, pure, EStateM.pure, MemOk]
    exact m₁

end Correspondence
