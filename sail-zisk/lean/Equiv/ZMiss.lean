/-
  Where a ZisK instruction can fail on a byte its memory lacks
  (`execute_miss`): its a source from memory (`A_MEM`), its b source from
  memory or through a register (`B_MEM`, `B_IND`), at the bytes they read,
  or its operation, one of those `zisk_exec_mem` gives a body. The rest of
  a step — the registers, the store, the next pc — fails on no byte.
-/
import Equiv.ZKeep

namespace Equiv.Zisk

open _root_.Zisk Sail Sail.ConcurrencyInterfaceV1

/-- A run of P fails on a missing byte only at a byte S holds. -/
def MissIn {α : Type} (S : Nat → Prop) (P : SailM α) : Prop :=
  ∀ σ k σ', P.run σ = .error (.OutOfMemoryRange k) σ' → S k

/-- P fails on no missing byte. -/
abbrev NoMiss {α : Type} (P : SailM α) : Prop := MissIn (fun _ => False) P

theorem MissIn.mono {α : Type} {S T : Nat → Prop} {P : SailM α} (h : MissIn S P) (hST : ∀ k, S k → T k) :
    MissIn T P := fun σ k σ' hr => hST k (h σ k σ' hr)

theorem MissIn.pure {α : Type} (S : Nat → Prop) (a : α) : MissIn S (pure a : SailM α) := by
  intro σ k σ' hr; cases hr

theorem MissIn.bind {α β : Type} {S : Nat → Prop} {P : SailM α} {f : α → SailM β} (hP : MissIn S P)
    (hf : ∀ a, MissIn S (f a)) : MissIn S (P >>= f) := by
  intro σ k σ' hr
  rw [run_bind'] at hr
  cases h1 : P.run σ with
  | ok a σ1 => rw [h1] at hr; exact hf a σ1 k σ' hr
  | error e σ1 => rw [h1] at hr; cases hr; exact hP σ k _ h1

theorem MissIn.ite {α : Type} {S : Nat → Prop} {c : Prop} [Decidable c] {P Q : SailM α} (hP : MissIn S P)
    (hQ : MissIn S Q) : MissIn S (if c then P else Q) := by
  split
  · exact hP
  · exact hQ

theorem MissIn.dite {α : Type} {S : Nat → Prop} {c : Prop} [Decidable c] {P : c → SailM α} {Q : ¬ c → SailM α}
    (hP : ∀ h, MissIn S (P h)) (hQ : ∀ h, MissIn S (Q h)) : MissIn S (if h : c then P h else Q h) := by
  split
  · exact hP _
  · exact hQ _

theorem MissIn.throw {α : Type} (S : Nat → Prop) (e : Sail.Error exception) (he : ∀ k, e ≠ .OutOfMemoryRange k) :
    MissIn S (MonadExcept.throw e : SailM α) := by
  intro σ k σ' hr; cases hr; exact absurd rfl (he k)

theorem MissIn.assert (S : Nat → Prop) (p : Bool) (msg : String) :
    MissIn S (Sail.ConcurrencyInterfaceV1.PreSail.assert p msg : SailM Unit) := by
  unfold Sail.ConcurrencyInterfaceV1.PreSail.assert
  split
  · exact MissIn.pure _ _
  · exact MissIn.throw _ _ (fun _ h => by cases h)

theorem MissIn.readReg (S : Nat → Prop) (r : Register) : MissIn S (Zisk.readReg r) := by
  intro σ k σ' hr
  rw [readReg_run'] at hr
  split at hr <;> cases hr

theorem MissIn.writeReg (S : Nat → Prop) (r : Register) (v : RegisterType r) : MissIn S (Zisk.writeReg r v) := by
  intro σ k σ' hr; rw [writeReg_run] at hr; cases hr

theorem MissIn.write_mem (S : Nat → Prop) (addr : BitVec 64) (n : Nat) (v : BitVec (8 * n)) :
    MissIn S (Functions.write_mem addr n v) := by
  unfold Functions.write_mem
  refine MissIn.bind (MissIn.assert _ _ _) (fun _ => MissIn.bind (MissIn.assert _ _ _) (fun _ => ?_))
  intro σ k σ' hr
  simp only [Sail.ConcurrencyInterfaceV1.PreSail.write_ram, run_bind', Equiv.zisk_writeBytes, run_pure'] at hr
  cases hr

theorem MissIn.readByte (a : Nat) :
    MissIn (fun k => k = a) (Sail.ConcurrencyInterfaceV1.PreSail.readByte a : SailM (BitVec 8)) := by
  intro σ k σ' hr
  rw [readByte_run'] at hr
  split at hr
  · cases hr
  · cases hr; rfl

theorem MissIn.readBytes : ∀ (n a : Nat),
    MissIn (fun k => a ≤ k ∧ k < a + n)
      (Sail.ConcurrencyInterfaceV1.PreSail.readBytes n a : SailM ((BitVec (8 * n)) × Option Bool))
  | 0, _ => by simp only [Sail.ConcurrencyInterfaceV1.PreSail.readBytes]; exact MissIn.pure _ _
  | 1, a => by
    simp only [Sail.ConcurrencyInterfaceV1.PreSail.readBytes]
    exact MissIn.bind ((MissIn.readByte a).mono (fun k h => by omega)) (fun _ => MissIn.pure _ _)
  | n + 2, a => by
    simp only [Sail.ConcurrencyInterfaceV1.PreSail.readBytes]
    exact MissIn.bind ((MissIn.readByte a).mono (fun k h => by omega))
      (fun _ => MissIn.bind ((MissIn.readBytes (n + 1) (a + 1)).mono (fun k h => by omega))
        (fun _ => MissIn.pure _ _))

/-- A read of n bytes at addr fails only on one of them. -/
theorem MissIn.read_mem (addr : BitVec 64) (n : Nat) :
    MissIn (fun k => addr.toNat ≤ k ∧ k < addr.toNat + n) (Functions.read_mem addr n) := by
  unfold Functions.read_mem
  refine MissIn.bind (MissIn.assert _ _ _) (fun _ => MissIn.bind (MissIn.assert _ _ _) (fun _ =>
    MissIn.ite (MissIn.bind (MissIn.readReg _ _) (fun _ => MissIn.pure _ _)) ?_))
  simp only [Sail.ConcurrencyInterfaceV1.PreSail.read_ram]
  exact MissIn.bind (MissIn.readBytes _ _) (fun _ => MissIn.pure _ _)

/-- Push `NoMiss` through code with no memory read. -/
syntax "nomiss" : tactic
macro_rules
  | `(tactic| nomiss) => `(tactic| first
      | (refine MissIn.bind ?_ (fun _ => ?_) <;> nomiss)
      | exact MissIn.pure _ _
      | exact MissIn.assert _ _ _
      | exact MissIn.readReg _ _
      | exact MissIn.writeReg _ _ _
      | exact MissIn.write_mem _ _ _ _
      | (refine MissIn.ite ?_ ?_ <;> nomiss)
      | (split <;> nomiss)
      | (dsimp only; nomiss)
      | (frame_unfold; nomiss))

/-- The operations `zisk_exec_mem` gives a body: the precompiles, the DMA,
    the fcalls, `jump_dest`. -/
def memOp : zisk_op → Bool
  | .ZOP_ADD256 () | .ZOP_ARITH256 () | .ZOP_ARITH256_MOD () | .ZOP_ARITH384_MOD () | .ZOP_BABYJUBJUB_ADD ()
  | .ZOP_BLAKE2B () | .ZOP_BLAKE2S () | .ZOP_BLAKE3 () | .ZOP_BLS12_381_COMPLEX_ADD ()
  | .ZOP_BLS12_381_COMPLEX_MUL () | .ZOP_BLS12_381_COMPLEX_SUB () | .ZOP_BLS12_381_CURVE_ADD ()
  | .ZOP_BLS12_381_CURVE_DBL () | .ZOP_BN254_COMPLEX_ADD () | .ZOP_BN254_COMPLEX_MUL ()
  | .ZOP_BN254_COMPLEX_SUB () | .ZOP_BN254_CURVE_ADD () | .ZOP_BN254_CURVE_DBL () | .ZOP_DMA_INPUTCPY ()
  | .ZOP_DMA_MEMCMP () | .ZOP_DMA_MEMCPY () | .ZOP_DMA_XMEMCMP () | .ZOP_DMA_XMEMCPY () | .ZOP_DMA_XMEMSET ()
  | .ZOP_ED25519_ADD () | .ZOP_FCALL () | .ZOP_FCALL_GET () | .ZOP_FCALL_PARAM () | .ZOP_JUMP_DEST ()
  | .ZOP_KECCAK () | .ZOP_POSEIDON1 () | .ZOP_POSEIDON2 () | .ZOP_POSEIDON2_KOALABEAR () | .ZOP_SECP256K1_ADD ()
  | .ZOP_SECP256K1_DBL () | .ZOP_SECP256R1_ADD () | .ZOP_SECP256R1_DBL () | .ZOP_SHA256 () => true
  | _ => false

theorem exec_mem_pure (op : zisk_op) (h : memOp op = false) (a b ext : BitVec 64) :
    Functions.zisk_exec_mem op a b ext = pure none := by
  cases op <;> first | (simp [memOp] at h; done) | rfl

theorem source_a_miss (inst : zisk_inst) :
    MissIn (fun k => inst.a_src = .A_MEM ∧ inst.a_imm.toNat ≤ k ∧ k < inst.a_imm.toNat + 8)
      (Functions.source_a inst) := by
  unfold Functions.source_a
  split
  · exact MissIn.readReg _ _
  · unfold Functions.rX; nomiss
  · next h => exact (MissIn.read_mem _ _).mono (fun k hk => ⟨h, hk⟩)
  · exact MissIn.pure _ _
  · exact MissIn.readReg _ _

/-- The bytes an instruction's b source reads, a the value of its a source. -/
def bReads (inst : zisk_inst) (a : BitVec 64) (k : Nat) : Prop :=
  (inst.b_src = .B_MEM ∧ inst.b_imm.toNat ≤ k ∧ k < inst.b_imm.toNat + 8) ∨
  (inst.b_src = .B_IND ∧ (a + inst.b_imm).toNat ≤ k ∧ k < (a + inst.b_imm).toNat + inst.ind_width)

theorem source_b_miss (inst : zisk_inst) (a : BitVec 64) : MissIn (bReads inst a) (Functions.source_b inst a) := by
  unfold Functions.source_b
  split
  · exact MissIn.readReg _ _
  · unfold Functions.rX; nomiss
  · next h => exact (MissIn.read_mem _ _).mono (fun k hk => .inl ⟨h, hk⟩)
  · exact MissIn.pure _ _
  · next h => exact MissIn.bind ((MissIn.read_mem _ _).mono (fun k hk => .inr ⟨h, hk⟩)) (fun _ => MissIn.pure _ _)

set_option maxHeartbeats 0 in
set_option maxRecDepth 100000 in
/-- Where an instruction fails on a missing byte: where its a source reads,
    where its b source reads, or in its operation, one with a body. -/
theorem execute_miss (inst : zisk_inst) (σ : ZState) (k : Nat) (σ' : ZState)
    (h : (Functions.zisk_execute inst).run σ = .error (.OutOfMemoryRange k) σ') :
    (inst.a_src = .A_MEM ∧ inst.a_imm.toNat ≤ k ∧ k < inst.a_imm.toNat + 8) ∨
    (∃ a σ1, (Functions.source_a inst).run σ = .ok a σ1 ∧ bReads inst a k) ∨ memOp inst.op = true := by
  unfold Functions.zisk_execute at h
  rw [run_bind'] at h
  cases ha : (Functions.source_a inst).run σ with
  | error e σ1 =>
    rw [ha] at h; cases h
    exact .inl (source_a_miss inst σ k _ ha)
  | ok a σ1 =>
    rw [ha] at h; dsimp only at h
    rw [run_bind'] at h
    cases hb : (Functions.source_b inst a).run σ1 with
    | error e σ2 =>
      rw [hb] at h; cases h
      exact .inr (.inl ⟨a, σ1, rfl, source_b_miss inst a σ1 k _ hb⟩)
    | ok b σ2 =>
      rw [hb] at h; dsimp only at h
      cases hm : memOp inst.op
      · rw [exec_mem_pure _ hm] at h
        exfalso
        revert h
        generalize (Functions.zisk_exec inst.op a b) = r
        simp only [pure_bind]
        intro h
        refine (show NoMiss _ from ?_) σ2 k σ' h
        refine MissIn.bind ?_ (fun _ => ?_)
        · unfold Functions.store_c
          refine MissIn.bind ?_ (fun _ => ?_)
          · unfold Functions.value_to_store
            exact MissIn.ite (MissIn.bind (MissIn.readReg _ _) (fun _ => MissIn.pure _ _)) (MissIn.pure _ _)
          split
          · exact MissIn.pure _ _
          · unfold Functions.wX
            exact MissIn.bind (MissIn.assert _ _ _) (fun _ => MissIn.bind (MissIn.assert _ _ _) (fun _ =>
              MissIn.bind (MissIn.readReg _ _) (fun _ => MissIn.writeReg _ _ _)))
          · exact MissIn.write_mem _ _ _ _
          · exact MissIn.write_mem _ _ _ _
        have hj : ∀ y : Unit, NoMiss (do
            writeReg Register.Zc r.fst
            writeReg Register.Zflag r.snd
            let st ← readReg Register.Step
            writeReg Register.Step (BitVec.addInt st 1)
            if inst.is_end = true then pure true
            else do
              let p ← Functions.next_pc inst r.fst r.snd
              writeReg Register.PC p
              pure false) := by
          intro _
          refine MissIn.bind (MissIn.writeReg _ _ _) (fun _ => MissIn.bind (MissIn.writeReg _ _ _) (fun _ =>
            MissIn.bind (MissIn.readReg _ _) (fun _ => MissIn.bind (MissIn.writeReg _ _ _) (fun _ =>
              MissIn.ite (MissIn.pure _ _) ?_))))
          unfold Functions.next_pc
          refine MissIn.bind ?_ (fun _ => MissIn.bind (MissIn.writeReg _ _ _) (fun _ => MissIn.pure _ _))
          exact MissIn.ite (MissIn.pure _ _) (MissIn.ite (MissIn.bind (MissIn.readReg _ _) (fun _ => MissIn.pure _ _))
            (MissIn.bind (MissIn.readReg _ _) (fun _ => MissIn.pure _ _)))
        try dsimp only
        split
        · exact MissIn.bind (MissIn.bind (MissIn.writeReg _ _ _) (fun _ => MissIn.writeReg _ _ _)) hj
        · exact hj ()
      · exact .inr (.inr rfl)

end Equiv.Zisk
