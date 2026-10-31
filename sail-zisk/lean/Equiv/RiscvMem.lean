/-
  sail-riscv's memory accesses in ZisK's profile, as equations on lean-sail's
  sequential interpreter: the layer under fetches, loads and stores.

  In ZisK's profile (riscv/zisk_profile.py) the PMA regions are ZisK's memory
  map and there is no PMP, so in machine mode with MPRV clear an access is:
  no translation; no PMP check; a PMA check, which an aligned access inside
  a region that permits it passes unsplit; and a plain read of the
  interpreter's bytes. What the state must hold for this is `MemReady`:
  machine mode, MPRV clear, no HTIF, the PMA regions, and the registers the
  checks read (whose values do not matter, but which must be there: the
  interpreter fails on a register it does not have).

  So far reads: `mem_read`, `fetch_bytes`, and `fetch` of a 32-bit
  instruction — in the end the four bytes at pc, little-endian
  (`fetch_word`). Stores, and loads through `vmem_read`, come next.

  lean-sail's free monad has no LawfulMonad instance; it is proved here, so
  the monad laws simplify sail-riscv's code.
-/
import Equiv.Riscv

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- lean-sail's free monad is lawful: so `bind_assoc` and the other monad
    laws apply to sail-riscv's SailM and SailME. -/
instance {Eff : Type v} [Effect Eff] : LawfulMonad (FreeM Eff) := LawfulMonad.mk'
  (id_map := by
    intro α x
    show FreeM.bind x (fun a => FreeM.pure (id a)) = x
    induction x with
    | pure a => rfl
    | impure e k ih => simp only [FreeM.bind]; congr; funext r; exact ih r)
  (pure_bind := by intros; rfl)
  (bind_assoc := by
    intro α β γ x f g
    show FreeM.bind (FreeM.bind x f) g = FreeM.bind x (fun a => FreeM.bind (f a) g)
    induction x with
    | pure a => rfl
    | impure e k ih => simp only [FreeM.bind]; congr; funext r; exact ih r)

@[simp] theorem interp_map (f : α → β) (x : SailM α) :
    interpretSailEffects (f <$> x) = f <$> interpretSailEffects x := by
  rw [map_eq_pure_bind, map_eq_pure_bind, interp_bind]; rfl

@[simp] theorem interp_freePure (a : α) :
    interpretSailEffects (FreeM.pure a : SailM α) = pure a := rfl

@[simp] theorem me_run_map_lift (f : α → β) (x : SailM α) :
    LeanZiskRV.SailME.run (f <$> (monadLift x : SailME β α)) = f <$> x := by
  rw [map_eq_pure_bind, map_eq_pure_bind, me_run_lift_bind]; rfl

@[simp] theorem run_map' (f : α → β) (x : EStateM ε σ α) (s : σ) :
    (f <$> x).run s = match x.run s with
      | .ok a s' => .ok (f a) s'
      | .error e s' => .error e s' := by
  show EStateM.map f x s = _
  simp only [EStateM.map, EStateM.run]; split <;> simp_all

theorem pmpCheck_run (s : RVState) (a : physaddr) (w : Nat) (acc : MemoryAccessType mem_payload) (p : Privilege) :
    (interpretSailEffects (pmpCheck a w acc p)).run s = .ok none s := by
  simp [pmpCheck, sys_pmp_count]
  rfl

theorem effectivePrivilege_run (s : RVState) (acc : MemoryAccessType mem_payload) (m : BitVec 64) (p : Privilege)
    (h : _get_Mstatus_MPRV m = 0) :
    (interpretSailEffects (effectivePrivilege acc m p)).run s = .ok p s := by
  simp [effectivePrivilege, h]

/-- The accesses a ZisK program makes: fetches, data loads and data stores. -/
def PlainAccess (acc : MemoryAccessType mem_payload) : Prop :=
  acc = .InstructionFetch () ∨ acc = .Load .Data ∨ acc = .Store .Data

theorem translateAddr_run (s : RVState) (va : BitVec 64) (acc : MemoryAccessType mem_payload)
    (hacc : PlainAccess acc)
    (m : RegisterType misa) (hm : s.regs.get? misa = some m)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (se : RegisterType senvcfg) (hse : s.regs.get? senvcfg = some se)
    (me : RegisterType menvcfg) (hme : s.regs.get? menvcfg = some me) :
    (interpretSailEffects (translateAddr (.Virtaddr va) acc .Machine)).run s
      = .ok (.Ok (.Physaddr va, .PBMT_PMA, ())) s := by
  rcases hacc with rfl | rfl | rfl <;>
  · simp (config := { decide := true }) [translateAddr, translate_vs_stage, Functions.satp_mode, is_svukte_qualified,
      is_shadow_stack_access, bits_of_virtaddr, zero_extend, Sail.BitVec.zeroExtend, currentlyEnabled,
      read_senvcfg, inVirtualPrivilege, me_run_lift_bind, me_run_pure,
      readReg_run _ _ _ hm, readReg_run _ _ _ hp, readReg_run _ _ _ hse, readReg_run _ _ _ hme]

/-- Whether a region's attributes permit an access ZisK programs make. -/
def permits (a : PMA) : MemoryAccessType mem_payload → Bool
  | .InstructionFetch () => a.executable
  | .Load .Data => a.readable
  | .Store .Data => a.writable
  | _ => false

/-- An access the PMA check passes unsplit: aligned, or a load or store
    within the region's misaligned-atomicity granule, of 2^4 bytes as in
    ZisK's profile. -/
def AlignOk (r : PMA_Region) (acc : MemoryAccessType mem_payload) (a : BitVec 64) (w : Nat) : Prop :=
  Int.tmod (a.toNat : Int) w = 0 ∨
    (acc ≠ .InstructionFetch () ∧ r.attributes.misaligned_atomicity_granule_size_exp = 4 ∧ w ≤ 8 ∧
      allowed_misaligned a w 4 = true)

theorem extract_full (a : BitVec 64) :
    Sail.BitVec.extractLsb a ((Functions.xlen : Int) - 1).toNat 0 = a := by
  simp only [Sail.BitVec.extractLsb, Functions.xlen]
  apply BitVec.eq_of_getLsbD_eq; intro i hi; simp
  intro _; omega

/-- An access inside a region that permits it, aligned or within its
    granule, passes the PMA check, unsplit. -/
theorem pmaCheck_run (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (a : BitVec 64) (w : Nat) (acc : MemoryAccessType mem_payload) (hacc : PlainAccess acc)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) w = some r)
    (hperm : permits r.attributes acc = true) (hal : AlignOk r acc a w) :
    (interpretSailEffects (pmaCheck (.Physaddr a) w acc .PBMT_PMA false)).run s
      = .ok (.Ok { splittable := .CannotSplit, granule_size_exp := 0 }) s := by
  obtain ⟨base, size, attrs, dt⟩ := r
  rcases hal with hal | ⟨hf, hg, hw8, hall⟩
  · rcases hacc with rfl | rfl | rfl <;>
    · simp only [permits] at hperm
      simp (config := { decide := true }) [pmaCheck, hm, override_PMA, mag_pma_check, is_aligned_paddr,
        BitVec.toNatInt, hal, hperm, me_run_lift_bind, me_run_pure, readReg_run _ _ _ hr,
        LeanZiskRV.assert, Sail.ArchSem.PreSail.assert, is_mag_applicable_access, bind_assoc]
      all_goals rfl
  · have hw8' : (w ≤b 8) = true := by simp; omega
    rcases hacc with rfl | rfl | rfl
    · exact absurd rfl hf
    all_goals
      simp only [permits] at hperm
      simp only at hg
      simp (config := { decide := true }) [pmaCheck, hm, override_PMA, mag_pma_check, within_pma_mag,
        mag_of_pma, hg, hall, hperm, hw8, hw8', Functions.xlen_bytes, me_run_lift_bind, me_run_pure,
        readReg_run _ _ _ hr, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert, is_mag_applicable_access,
        bind_assoc, is_vector_access, extract_full]
      all_goals rfl

/-- A plain read of RAM: the interpreter's bytes, little-endian. -/
theorem read_ram_run (s : RVState) (acc : MemoryAccessType mem_payload) (a : BitVec 64) (w : Nat)
    (v : BitVec (8 * w)) (hv : (Sail.ArchSem.readBytes w a.toNat : Sail.ArchSem.ArchSemSeqM exception _).run s = .ok v s) :
    (interpretSailEffects (read_ram .Read_plain acc (.Physaddr a) w false)).run s
      = .ok (from_bytes_le (n := w) (bitvec_to_vecbytes v), default_meta) s := by
  simp [read_ram, Sail.ArchSem.PreSail.sail_mem_read, interpretSailEffects, FreeM.liftM,
    interpretInstructionEffect, Mem_request.toArchSem]
  erw [hv]; rfl

/-- Writing a value over the whole of a vector, as checked_mem_read places
    an unsplit read, gives the value. -/
theorem full_update (w : Nat) (hw : 0 < w) (x z : BitVec (8 * w)) :
    BitVec.setWidth (8 * w) (BitVec.setWidth (8 * (w : Int)).toNat
      (Sail.BitVec.updateSubrange z (8 * (w : Int) - 1).toNat (0 * (w : Int)).toNat
        (BitVec.setWidth ((8 * (w : Int) - 1).toNat - (0 * (w : Int)).toNat + 1) x))) = x := by
  apply BitVec.eq_of_getLsbD_eq
  intro i hi
  have e1 : (8 * (w : Int)).toNat = 8 * w := by omega
  have e2 : (8 * (w : Int) - 1).toNat = 8 * w - 1 := by omega
  have e3 : (0 * (w : Int)).toNat = 0 := by omega
  have e4 : 8 * w - 1 + 1 = 8 * w := by omega
  simp [Sail.BitVec.updateSubrange, Sail.BitVec.updateSubrange', e1, e2, e4, hi]

/-- An aligned plain read inside a region that permits it, of memory nobody
    configured as MMIO: the bytes, little-endian. -/
theorem checked_mem_read_run (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (w : Nat) (hw : 0 < w) (acc : MemoryAccessType mem_payload) (hacc : PlainAccess acc)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) w = some r)
    (hperm : permits r.attributes acc = true) (hal : AlignOk r acc a w)
    (v : BitVec (8 * w)) (hv : (Sail.ArchSem.readBytes w a.toNat : Sail.ArchSem.ArchSemSeqM exception _).run s = .ok v s) :
    (interpretSailEffects (checked_mem_read acc .PBMT_PMA .Machine (.Physaddr a) w false false false false)).run s
      = .ok (.Ok (from_bytes_le (n := w) (bitvec_to_vecbytes v), default_meta)) s := by
  have hp := pmaCheck_run s rs hr a w acc hacc r hm hperm hal
  have hram := read_ram_run s acc a w v hv
  simp (config := { decide := true }) [checked_mem_read, check_pma_with_pmp_priority, hp, split_misaligned,
    misaligned_order, read_kind_of_flags, me_run_lift_bind, bind_assoc, untilFuelM, untilFuelM.go,
    pmpCheck_run, within_mmio_readable, within_clint, within_sig, within_htif_readable,
    within_htif_writable, readReg_run _ _ _ hh, bits_of_physaddr, BitVec.addInt, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert]
  have h0 : a + 0#64 = a := by simp
  erw [h0, hram]
  exact congrArg (fun x => EStateM.Result.ok (Ok (x, default_meta)) s) (full_update w hw _ _)

/-- The same read through mem_read, which drops the metadata. -/
theorem mem_read_run (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (w : Nat) (hw : 0 < w) (acc : MemoryAccessType mem_payload) (hacc : PlainAccess acc)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) w = some r)
    (hperm : permits r.attributes acc = true) (hal : AlignOk r acc a w)
    (v : BitVec (8 * w)) (hv : (Sail.ArchSem.readBytes w a.toNat : Sail.ArchSem.ArchSemSeqM exception _).run s = .ok v s) :
    (interpretSailEffects (mem_read acc .PBMT_PMA .Machine (.Physaddr a) w false false false)).run s
      = .ok (.Ok (from_bytes_le (n := w) (bitvec_to_vecbytes v))) s := by
  have h := checked_mem_read_run s rs hr hh a w hw acc hacc r hm hperm hal v hv
  simp [mem_read, mem_read_priv_meta, h, MemoryOpResult_drop_meta]

/-- What sail-riscv's memory accesses need of the state, in ZisK's profile:
    machine mode, MPRV clear, no HTIF, the PMA regions `rs`, and the
    registers translation reads. -/
structure MemReady (s : RVState) (rs : List PMA_Region) : Prop where
  priv : s.regs.get? cur_privilege = some .Machine
  mstatus : ∃ m, s.regs.get? mstatus = some m ∧ _get_Mstatus_MPRV m = 0
  misa : ∃ m, s.regs.get? misa = some m
  senvcfg : ∃ v, s.regs.get? senvcfg = some v
  menvcfg : ∃ v, s.regs.get? menvcfg = some v
  htif : s.regs.get? htif_tohost_base = some none
  pma : s.regs.get? pma_regions = some rs

/-- A fetch of `w` bytes at `pc`, aligned, inside an executable region. -/
theorem fetch_bytes_run (s : RVState) (rs : List PMA_Region) (hs : MemReady s rs)
    (start pc : BitVec 64) (w : Nat) (hw : 0 < w)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr pc) w = some r)
    (hx : r.attributes.executable = true) (hal : Int.tmod (pc.toNat : Int) w = 0)
    (v : BitVec (8 * w)) (hv : (Sail.ArchSem.readBytes w pc.toNat : Sail.ArchSem.ArchSemSeqM exception _).run s = .ok v s) :
    (interpretSailEffects (fetch_bytes start pc w)).run s
      = .ok (.FetchBytes_Success (from_bytes_le (n := w) (bitvec_to_vecbytes v))) s := by
  obtain ⟨hp, ⟨ms, hms, hmprv⟩, ⟨mi, hmi⟩, ⟨se, hse⟩, ⟨me, hme⟩, hh, hr⟩ := hs
  have hacc : PlainAccess (.InstructionFetch ()) := Or.inl rfl
  have ht := translateAddr_run s pc _ hacc mi hmi hp se hse me hme
  have hrd := mem_read_run s rs hr hh pc w hw _ hacc r hm (by simpa [permits] using hx) (Or.inl hal) v hv
  have he := effectivePrivilege_run s (.InstructionFetch ()) ms .Machine hmprv
  simp [fetch_bytes, ext_fetch_check_pc, me_run_lift_bind, me_run_pure, bind_assoc,
    readReg_run _ _ _ hms, readReg_run _ _ _ hp, he, ht, hrd]

theorem vec_getElem_int (v : Vector α n) (i : Int) (h : i.toNat < n) :
    v[i]'h = v[i.toNat]'h := rfl

macro "bytes_rt" : tactic => `(tactic| (
  simp [from_bytes_le, bitvec_to_vecbytes, Sail.BitVec.updateSubrange, Sail.BitVec.updateSubrange',
    Sail.BitVec.slice, Id.run, forIn, ForIn'.forIn', IntRange.forIn']
  repeat (rw [IntRange.forIn'.loop]; simp (config := { decide := true }) [Membership.mem])
  try simp [vec_getElem_int, Vector.getElem_ofFn, Id.run, pure]
  show _ = _
  bv_decide))

/-- sail-riscv's bytes to bits and back: the identity, at the widths of
    RISC-V's accesses. -/
theorem rt1 (v : BitVec 8) : from_bytes_le (n := 1) (bitvec_to_vecbytes v) = v := by bytes_rt



theorem rt2 (v : BitVec 16) : from_bytes_le (n := 2) (bitvec_to_vecbytes v) = v := by bytes_rt
theorem rt4 (v : BitVec 32) : from_bytes_le (n := 4) (bitvec_to_vecbytes v) = v := by bytes_rt
theorem rt8 (v : BitVec 64) : from_bytes_le (n := 8) (bitvec_to_vecbytes v) = v := by bytes_rt

/-- The interpreter's read of two bytes: little-endian. -/
theorem readBytes2_run (s : RVState) (a : Nat) (b0 b1 : BitVec 8)
    (h0 : s.mem.get? a = some b0) (h1 : s.mem.get? (a + 1) = some b1) :
    (Sail.ArchSem.readBytes 2 a : Sail.ArchSem.ArchSemSeqM exception _).run s = .ok (b1 ++ b0) s := by
  simp only [Std.ExtHashMap.get?_eq_getElem?] at h0 h1
  simp [Sail.ArchSem.readBytes, Sail.ArchSem.readByte, h0, h1]

theorem tmod_nat (n m : Nat) : Int.tmod (n : Int) (m : Int) = ((n % m : Nat) : Int) := rfl

theorem access_low (pc : BitVec 64) (hal : pc.toNat % 4 = 0) :
    Sail.BitVec.access pc 0 = 0#1 ∧ Sail.BitVec.access pc 1 = 0#1 := by
  have h0 : pc.getLsbD 0 = false := by
    simp [BitVec.getLsbD, Nat.testBit, Nat.shiftRight_zero]; omega
  have h1 : pc.getLsbD 1 = false := by
    simp [BitVec.getLsbD, Nat.testBit, Nat.shiftRight_succ]; omega
  constructor <;> apply BitVec.eq_of_getLsbD_eq <;> intro i hi <;>
    (have : i = 0 := by omega
     subst this; simp [Sail.BitVec.access]; rw [← BitVec.getLsbD_eq_getElem]; assumption)

/-- sail-riscv's fetch of a 32-bit instruction at a 4-byte aligned pc in an
    executable region: two halves, as without Ziccif, the low one not
    compressed. -/
theorem fetch_run (s : RVState) (rs : List PMA_Region) (hs : MemReady s rs)
    (pc : BitVec 64) (hpc : s.regs.get? PC = some pc) (hal : pc.toNat % 4 = 0)
    (r1 r2 : PMA_Region) (hm1 : matching_pma_region rs (.Physaddr pc) 2 = some r1)
    (hm2 : matching_pma_region rs (.Physaddr (pc + 2)) 2 = some r2)
    (hx1 : r1.attributes.executable = true) (hx2 : r2.attributes.executable = true)
    (lo hi : BitVec 16)
    (hlo : (Sail.ArchSem.readBytes 2 pc.toNat : Sail.ArchSem.ArchSemSeqM exception _).run s = .ok lo s)
    (hhi : (Sail.ArchSem.readBytes 2 (pc + 2).toNat : Sail.ArchSem.ArchSemSeqM exception _).run s = .ok hi s)
    (hrvc : isRVC (from_bytes_le (n := 2) (bitvec_to_vecbytes lo)) = false) :
    (interpretSailEffects (fetch ())).run s
      = .ok (.F_Base (from_bytes_le (n := 2) (bitvec_to_vecbytes hi) ++ from_bytes_le (n := 2) (bitvec_to_vecbytes lo))) s := by
  have ha1 : Int.tmod (pc.toNat : Int) 2 = 0 := by
    rw [show (2 : Int) = ((2 : Nat) : Int) from rfl, tmod_nat]; omega
  have ha2 : Int.tmod ((pc + 2).toNat : Int) 2 = 0 := by
    rw [show (2 : Int) = ((2 : Nat) : Int) from rfl, tmod_nat, BitVec.toNat_add]; simp; omega
  obtain ⟨b0, b1⟩ := access_low pc hal
  have f1 := fetch_bytes_run s rs hs pc pc 2 (by omega) r1 hm1 hx1 ha1 lo hlo
  have f2 := fetch_bytes_run s rs hs pc (pc + 2) 2 (by omega) r2 hm2 hx2 ha2 hi hhi
  obtain ⟨hp, _, ⟨mi, hmi⟩, -, -, -, -⟩ := hs
  simp (config := { decide := true }) [fetch, ext_fetch_check_pc, me_run_lift_bind,
    readReg_run _ _ _ hpc, readReg_run _ _ _ hmi, currentlyEnabled, hartSupports,
    is_aligned_vaddr, BitVec.addInt, f1, hrvc, b0, b1]
  erw [f2]; rfl

/-- The fetch in terms of memory: the four bytes at pc, little-endian, when
    the low two bits of the first are 11 (not a compressed instruction). -/
theorem fetch_word (s : RVState) (rs : List PMA_Region) (hs : MemReady s rs)
    (pc : BitVec 64) (hpc : s.regs.get? PC = some pc) (hal : pc.toNat % 4 = 0)
    (r1 r2 : PMA_Region) (hm1 : matching_pma_region rs (.Physaddr pc) 2 = some r1)
    (hm2 : matching_pma_region rs (.Physaddr (pc + 2)) 2 = some r2)
    (hx1 : r1.attributes.executable = true) (hx2 : r2.attributes.executable = true)
    (b0 b1 b2 b3 : BitVec 8)
    (h0 : s.mem.get? pc.toNat = some b0) (h1 : s.mem.get? (pc.toNat + 1) = some b1)
    (h2 : s.mem.get? (pc.toNat + 2) = some b2) (h3 : s.mem.get? (pc.toNat + 3) = some b3)
    (hc : b0.extractLsb 1 0 = 3#2) :
    (interpretSailEffects (fetch ())).run s = .ok (.F_Base ((b3 ++ b2) ++ (b1 ++ b0))) s := by
  have e2 : (pc + 2).toNat = pc.toNat + 2 := by rw [BitVec.toNat_add]; simp; omega
  have lo := readBytes2_run s pc.toNat b0 b1 h0 h1
  have hi := readBytes2_run s (pc + 2).toNat b2 b3 (by rw [e2]; exact h2) (by rw [e2]; exact h3)
  have hr : isRVC (from_bytes_le (n := 2) (bitvec_to_vecbytes (b1 ++ b0))) = false := by
    rw [rt2]; simp [isRVC, Sail.BitVec.extractLsb, LeanZiskRV.not]
    have : (b1 ++ b0).extractLsb 1 0 = b0.extractLsb 1 0 := by bv_decide
    rw [this, hc]
  have := fetch_run s rs hs pc hpc hal r1 r2 hm1 hm2 hx1 hx2 _ _ lo hi hr
  rwa [rt2, rt2] at this

end Equiv.Riscv
