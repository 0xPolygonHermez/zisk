/-
  The program's ROM from the spec (Transpiler/Rom.lean), with the BIOS, as
  the theorem's ROM: it holds the BIOS (`BiosAt`) and, at every code word
  the spec describes, its translation (`SpecRom`).

  An address with no instruction is, in the model, an assertion of
  rom_fetch; here, as the ROM is a function, a halt: the run's failure too.
-/
import Equiv.ProgramRom
import Equiv.SpecRom
import Equiv.BiosSim

namespace Equiv.Sim

open Transpiler

/-- The ROM: the BIOS's instruction at an address it has one, else the
    program's, else a halt. -/
def zromOf (segs : List (BitVec 64 × Array (BitVec 32))) (entry : BitVec 64) (a : BitVec 64) :
    _root_.Zisk.zisk_inst :=
  match romLookup (bios entry) a with
  | some i => i
  | none => (romLookup (programRom segs) a).getD halt

theorem bios_addrs (entry : BitVec 64) : (bios entry).map (·.1) = (bios 0).map (·.1) := by
  simp [bios, List.range_succ]

/-- The BIOS's addresses: 0x1000 + 4k, for k < 35. -/
theorem bios_addrs0 : (bios 0).map (·.1) = (List.range 35).map fun k => BitVec.ofNat 64 (0x1000 + 4 * k) := by
  simp [bios, List.range_succ]

theorem bios_nodup (entry : BitVec 64) : ((bios entry).map (·.1)).Nodup := by
  rw [bios_addrs, bios_addrs0]; unfold List.Nodup; rw [List.pairwise_map]
  refine List.Pairwise.imp_of_mem (fun {a b} ha hb hab e => hab ?_) List.nodup_range
  simp only [List.mem_range] at ha hb
  have := congrArg BitVec.toNat e
  simp only [BitVec.toNat_ofNat] at this; omega

theorem bios_low (entry : BitVec 64) : ∀ p ∈ bios entry, p.1.toNat < 0x2000 := by
  intro p hp
  have hm : p.1 ∈ (bios 0).map (·.1) := bios_addrs entry ▸ List.mem_map_of_mem hp
  rw [bios_addrs0] at hm
  simp only [List.mem_map, List.mem_range] at hm
  obtain ⟨k, hk, e⟩ := hm
  rw [← e, BitVec.toNat_ofNat]; omega

theorem romLookup_none {r : List (BitVec 64 × _root_.Zisk.zisk_inst)} {a : BitVec 64}
    (h : ∀ p ∈ r, p.1 ≠ a) : romLookup r a = none := by
  simp only [romLookup, Option.map_eq_none_iff, List.find?_eq_none, beq_iff_eq]
  exact h

theorem zromOf_bios (segs : List (BitVec 64 × Array (BitVec 32))) (entry : BitVec 64) :
    BiosAt (zromOf segs entry) entry := by
  intro p hp
  simp only [zromOf, romLookup_of_mem (bios_nodup entry) hp]

/-- What the program's ROM needs of the program: its code words at distinct
    addresses, 4-aligned, from ROM_ADDR on, and not so many that the
    internal addresses wrap. -/
structure SegsOk (segs : List (BitVec 64 × Array (BitVec 32))) : Prop where
  even : ∀ x ∈ codeWords segs, x.1.getLsbD 0 = false
  nodup : ((codeWords segs).map (·.1)).Nodup
  high : ∀ x ∈ codeWords segs, ROM_ADDR.toNat ≤ x.1.toNat
  count : romCount (codeWords segs) 0 < 2 ^ 62

theorem programRom_high {segs : List (BitVec 64 × Array (BitVec 32))} (hs : SegsOk segs) :
    ∀ p ∈ programRom segs, 0x2000 ≤ p.1.toNat := by
  intro p hp
  rcases romFrom_addr _ 0 hs.even p hp with ⟨x, hx, e⟩ | ⟨m, -, h2, e⟩
  · rw [e]; have := hs.high x hx; rw [rom_addr_toNat] at this; omega
  · have := hs.count
    rw [e]; simp only [internalAddr, BitVec.toNat_add, BitVec.toNat_ofNat, rom_addr_toNat]
    omega

theorem zromOf_program {segs : List (BitVec 64 × Array (BitVec 32))} (hs : SegsOk segs) (entry : BitVec 64)
    {p : BitVec 64 × _root_.Zisk.zisk_inst} (hp : p ∈ programRom segs) : zromOf segs entry p.1 = p.2 := by
  have hn : ((programRom segs).map (·.1)).Nodup :=
    romFrom_nodup _ 0 hs.even hs.nodup (Nat.lt_trans hs.count (by decide))
  have hb : romLookup (bios entry) p.1 = none := by
    apply romLookup_none; intro q hq e
    have h1 := bios_low entry q hq; have h2 := programRom_high hs p hp
    rw [e] at h1; omega
  simp only [zromOf, hb, romLookup_of_mem hn hp, Option.getD_some]

/-- The program's ROM holds, at every code word the spec describes, its
    translation, given the words in memory and the words after a segment's
    last ones not changing it. -/
theorem zromOf_spec {segs : List (BitVec 64 × Array (BitVec 32))} (hs : SegsOk segs) (entry : BitVec 64)
    (C : BitVec 64 → Prop) (m : Mem) (hC : ∀ pc, C pc → ∃ x ∈ codeWords segs, x.1 = pc)
    (hw : ∀ x ∈ codeWords segs, ZiskRiscv.readWord m x.1.toNat = some x.2.1)
    (hnext : ∀ x ∈ codeWords segs, ∀ ia,
      convert x.1 x.2.1 (ZiskRiscv.readWord m (x.1 + 4).toNat) ia (ZiskRiscv.readWord m (x.1 + 8).toNat) =
        convert x.1 x.2.1 x.2.2.1 ia x.2.2.2) :
    SpecRom (zromOf segs entry) C m := by
  intro pc w hpc hrw ⟨ia, l, hl⟩
  obtain ⟨x, hx, rfl⟩ := hC pc hpc
  have hwx : w = x.2.1 := by rw [hw x hx] at hrw; exact (Option.some.inj hrw).symm
  subst hwx
  rw [hnext x hx] at hl
  obtain ⟨n, l', hl', hmem⟩ := romFrom_mem _ 0 hx hl
  exact ⟨internalAddr n, l', by rw [hnext x hx]; exact hl', fun p hp => zromOf_program hs entry (hmem p hp)⟩

/-- A program, its code segments `segs` loaded in memory `m`, its code
    addresses C among their words: what the theorems need of the ROM
    (`BiosAt`, `SpecRom`) holds of the ROM the transpiler builds from it,
    `zromOf segs entry`. -/
structure ProgramOk (segs : List (BitVec 64 × Array (BitVec 32))) (C : BitVec 64 → Prop) (m : Mem) : Prop where
  ok : SegsOk segs
  code : ∀ pc, C pc → ∃ x ∈ codeWords segs, x.1 = pc
  words : ∀ x ∈ codeWords segs, ZiskRiscv.readWord m x.1.toNat = some x.2.1
  /-- The words after a segment's last ones, in memory, change no
      translation: the transpiler reads none there. -/
  next : ∀ x ∈ codeWords segs, ∀ ia,
    convert x.1 x.2.1 (ZiskRiscv.readWord m (x.1 + 4).toNat) ia (ZiskRiscv.readWord m (x.1 + 8).toNat) =
      convert x.1 x.2.1 x.2.2.1 ia x.2.2.2

theorem ProgramOk.rom {segs : List (BitVec 64 × Array (BitVec 32))} {C : BitVec 64 → Prop} {m : Mem}
    (h : ProgramOk segs C m) (entry : BitVec 64) :
    BiosAt (zromOf segs entry) entry ∧ SpecRom (zromOf segs entry) C m :=
  ⟨zromOf_bios segs entry, zromOf_spec h.ok entry C m h.code h.words h.next⟩

end Equiv.Sim
