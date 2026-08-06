//! `ziskbin` — compact binary (de)serialization of a [`ZiskRom`].
//!
//! Encodes a fully-built ROM into a small variable-length blob that is embedded
//! in a minimal ELF container (`e_machine = EM_ZISK`, a `.ziskrom` section), so a
//! `.zisk` program can be fed to the existing ELF-based toolchain without RISC-V
//! transpilation. The format is specified in `ziskasm/ziskbin.md`.
//!
//! Only information-bearing fields are stored: fields derived from the opcode are
//! reconstructed on decode via [`ZiskOp::try_from_code`], the instruction address
//! comes from a delta-encoded stream, and the ROM lookup vectors are rebuilt with
//! [`ZiskRom::optimize_instruction_lookup`].

use std::collections::BTreeMap;

use elf::{endian::LittleEndian, ElfBytes};

use crate::{
    zisk_ops::{OpType, ZiskOp},
    DataSection64, ZiskInst, ZiskInstBuilder, ZiskRom,
};

/// Canonical fall-through pc advance (`jmp_offset1`/`jmp_offset2` default). Matches
/// `INST_SIZE` in the ziskasm assembler.
const INST_SIZE: i64 = 4;

/// Container magic tag.
const MAGIC: &[u8; 4] = b"ZKRM";
/// Container format version.
const VERSION: u8 = 1;
/// Private ELF machine id marking a ziskbin ELF ("ZK").
pub const EM_ZISK: u16 = 0x5a4b;
/// Section name holding the ROM container blob.
const SECTION_NAME: &str = ".ziskrom";

// ---------------------------------------------------------------------------
// Varint primitives
// ---------------------------------------------------------------------------

fn put_uvarint(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let byte = (v & 0x7f) as u8;
        v >>= 7;
        if v != 0 {
            out.push(byte | 0x80);
        } else {
            out.push(byte);
            break;
        }
    }
}

fn put_svarint(out: &mut Vec<u8>, v: i64) {
    // zig-zag then LEB128
    put_uvarint(out, ((v << 1) ^ (v >> 63)) as u64);
}

fn put_str(out: &mut Vec<u8>, s: &str) {
    put_uvarint(out, s.len() as u64);
    out.extend_from_slice(s.as_bytes());
}

/// Cursor over a byte slice used by the decoder.
struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Reader { buf, pos: 0 }
    }

    fn u8(&mut self) -> Result<u8, String> {
        let b = *self.buf.get(self.pos).ok_or("ziskbin: unexpected end of data")?;
        self.pos += 1;
        Ok(b)
    }

    fn uvarint(&mut self) -> Result<u64, String> {
        let mut result: u64 = 0;
        let mut shift = 0u32;
        loop {
            let b = self.u8()?;
            if shift >= 64 {
                return Err("ziskbin: uvarint overflow".to_string());
            }
            result |= ((b & 0x7f) as u64) << shift;
            if b & 0x80 == 0 {
                break;
            }
            shift += 7;
        }
        Ok(result)
    }

    fn svarint(&mut self) -> Result<i64, String> {
        let u = self.uvarint()?;
        Ok(((u >> 1) as i64) ^ -((u & 1) as i64))
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self.pos.checked_add(n).ok_or("ziskbin: length overflow")?;
        let slice = self.buf.get(self.pos..end).ok_or("ziskbin: unexpected end of data")?;
        self.pos = end;
        Ok(slice)
    }

    fn string(&mut self) -> Result<String, String> {
        let len = self.uvarint()? as usize;
        let bytes = self.take(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| "ziskbin: invalid utf-8".to_string())
    }
}

// ---------------------------------------------------------------------------
// Instruction codec
// ---------------------------------------------------------------------------

/// Writes the 7-bits-per-byte flag bitmap (bit 7 = continuation).
fn put_flags(out: &mut Vec<u8>, flags: u32) {
    let mut k = 0;
    loop {
        let byte = ((flags >> (7 * k)) & 0x7f) as u8;
        let more = (flags >> (7 * (k + 1))) != 0;
        out.push(byte | if more { 0x80 } else { 0 });
        if !more {
            break;
        }
        k += 1;
    }
}

fn read_flags(r: &mut Reader) -> Result<u32, String> {
    let mut flags: u32 = 0;
    let mut k = 0u32;
    loop {
        let b = r.u8()?;
        if k >= 5 {
            return Err("ziskbin: flag bitmap too long".to_string());
        }
        flags |= ((b & 0x7f) as u32) << (7 * k);
        if b & 0x80 == 0 {
            break;
        }
        k += 1;
    }
    Ok(flags)
}

/// Encodes a single [`ZiskInst`] (opcode + flag bitmap + non-default payloads).
/// `paddr` is supplied by the container's address stream and is not stored here.
fn encode_inst(i: &ZiskInst, out: &mut Vec<u8>) {
    out.push(i.op);

    let mut flags: u32 = 0;
    let mut pay: Vec<u8> = Vec::new();
    macro_rules! bit {
        ($n:expr) => {
            flags |= 1u32 << $n
        };
    }

    if i.a_src != 0 {
        bit!(0);
        put_uvarint(&mut pay, i.a_src);
    }
    if i.a_offset_imm0 != 0 {
        bit!(1);
        put_uvarint(&mut pay, i.a_offset_imm0);
    }
    if i.a_use_sp_imm1 != 0 {
        bit!(2);
        put_uvarint(&mut pay, i.a_use_sp_imm1);
    }
    if i.b_src != 0 {
        bit!(3);
        put_uvarint(&mut pay, i.b_src);
    }
    if i.b_offset_imm0 != 0 {
        bit!(4);
        put_uvarint(&mut pay, i.b_offset_imm0);
    }
    if i.b_use_sp_imm1 != 0 {
        bit!(5);
        put_uvarint(&mut pay, i.b_use_sp_imm1);
    }
    if i.store != 0 {
        bit!(6);
        put_uvarint(&mut pay, i.store);
    }
    if i.store_offset != 0 {
        bit!(7);
        put_svarint(&mut pay, i.store_offset);
    }
    if i.store_pc {
        bit!(8);
    }
    if i.store_use_sp {
        bit!(9);
    }
    if i.set_pc {
        bit!(10);
    }
    if i.end {
        bit!(11);
    }
    if i.ind_width != 0 {
        bit!(12);
        put_uvarint(&mut pay, i.ind_width);
    }
    if i.jmp_offset1 != INST_SIZE {
        bit!(13);
        put_svarint(&mut pay, i.jmp_offset1);
    }
    if i.jmp_offset2 != INST_SIZE {
        bit!(14);
        put_svarint(&mut pay, i.jmp_offset2);
    }
    if !i.verbose.is_empty() {
        bit!(15);
        put_str(&mut pay, &i.verbose);
    }
    if let Some(s) = &i.riscv_inst {
        bit!(16);
        put_str(&mut pay, s);
    }
    if i.index != 0 {
        bit!(17);
        put_uvarint(&mut pay, i.index);
    }
    // f18 (sorted_pc_list_index) is intentionally never encoded: it is ROM layout,
    // re-derived by `optimize_instruction_lookup` on decode (see ziskbin.md §6).
    if let Some(v) = i.next_internal_inst {
        bit!(19);
        put_uvarint(&mut pay, v);
    }
    if let Some(v) = i.external_ref_addr {
        bit!(20);
        put_uvarint(&mut pay, v);
    }
    if let Some(v) = i.meta_rs1 {
        bit!(21);
        pay.push(v);
    }
    if let Some(v) = i.meta_rd {
        bit!(22);
        pay.push(v);
    }

    put_flags(out, flags);
    out.extend_from_slice(&pay);
}

fn decode_inst(r: &mut Reader, paddr: u64) -> Result<ZiskInst, String> {
    let op = r.u8()?;
    let flags = read_flags(r)?;
    let has = |n: u32| flags & (1u32 << n) != 0;

    let mut i = ZiskInst { paddr, op, ..Default::default() };

    // (1) reconstruct opcode-derived fields
    let z = ZiskOp::try_from_code(op).map_err(|_| format!("ziskbin: invalid opcode 0x{op:02x}"))?;
    i.func = z.get_call_function();
    i.op_str = z.name();
    let ot = z.op_type();
    i.op_type = ot.into();
    i.input_size = z.input_size();
    i.is_precompiled = i.input_size > 0;
    i.is_external_op = ot != OpType::Internal && ot != OpType::Fcall;
    i.m32 = z.name().contains("_w");

    // (2) non-zero canonical defaults
    i.jmp_offset1 = INST_SIZE;
    i.jmp_offset2 = INST_SIZE;

    // (3) overlay present fields, in ascending flag order
    if has(0) {
        i.a_src = r.uvarint()?;
    }
    if has(1) {
        i.a_offset_imm0 = r.uvarint()?;
    }
    if has(2) {
        i.a_use_sp_imm1 = r.uvarint()?;
    }
    if has(3) {
        i.b_src = r.uvarint()?;
    }
    if has(4) {
        i.b_offset_imm0 = r.uvarint()?;
    }
    if has(5) {
        i.b_use_sp_imm1 = r.uvarint()?;
    }
    if has(6) {
        i.store = r.uvarint()?;
    }
    if has(7) {
        i.store_offset = r.svarint()?;
    }
    i.store_pc = has(8);
    i.store_use_sp = has(9);
    i.set_pc = has(10);
    i.end = has(11);
    if has(12) {
        i.ind_width = r.uvarint()?;
    }
    if has(13) {
        i.jmp_offset1 = r.svarint()?;
    }
    if has(14) {
        i.jmp_offset2 = r.svarint()?;
    }
    if has(15) {
        i.verbose = r.string()?;
    }
    if has(16) {
        i.riscv_inst = Some(r.string()?);
    }
    if has(17) {
        i.index = r.uvarint()?;
    }
    if has(18) {
        i.sorted_pc_list_index = r.uvarint()? as usize;
    }
    if has(19) {
        i.next_internal_inst = Some(r.uvarint()?);
    }
    if has(20) {
        i.external_ref_addr = Some(r.uvarint()?);
    }
    if has(21) {
        i.meta_rs1 = Some(r.u8()?);
    }
    if has(22) {
        i.meta_rd = Some(r.u8()?);
    }

    Ok(i)
}

// ---------------------------------------------------------------------------
// Data section codec
// ---------------------------------------------------------------------------

fn encode_section(s: &DataSection64, out: &mut Vec<u8>) {
    put_uvarint(out, s.addr);
    put_uvarint(out, s.data.len() as u64);
    for w in &s.data {
        out.extend_from_slice(&w.to_le_bytes());
    }
}

fn decode_section(r: &mut Reader) -> Result<DataSection64, String> {
    let addr = r.uvarint()?;
    let n = r.uvarint()? as usize;
    let mut data = Vec::with_capacity(n);
    for _ in 0..n {
        let bytes = r.take(8)?;
        data.push(u64::from_le_bytes(bytes.try_into().unwrap()));
    }
    Ok(DataSection64 { addr, data })
}

// ---------------------------------------------------------------------------
// ROM container
// ---------------------------------------------------------------------------

/// Serializes a fully-built [`ZiskRom`] into the ziskbin container blob.
pub fn encode_rom(rom: &ZiskRom) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    out.push(0); // profile: faithful
    put_uvarint(&mut out, rom.insts.len() as u64);
    put_uvarint(&mut out, rom.ro_data_64.len() as u64);
    put_uvarint(&mut out, rom.rw_data_64.len() as u64);

    // Instruction stream: ascending addresses (BTreeMap order), delta-encoded.
    let mut prev = 0u64;
    for (addr, zib) in &rom.insts {
        put_uvarint(&mut out, addr - prev);
        prev = *addr;
        encode_inst(&zib.i, &mut out);
    }

    for s in &rom.ro_data_64 {
        encode_section(s, &mut out);
    }
    for s in &rom.rw_data_64 {
        encode_section(s, &mut out);
    }

    out
}

/// Decodes a ziskbin container blob into a finalized [`ZiskRom`] (with lookup
/// vectors rebuilt via [`ZiskRom::optimize_instruction_lookup`]).
pub fn decode_rom(blob: &[u8]) -> Result<ZiskRom, String> {
    let mut r = Reader::new(blob);
    if r.take(4)? != MAGIC {
        return Err("ziskbin: bad magic".to_string());
    }
    let version = r.u8()?;
    if version != VERSION {
        return Err(format!("ziskbin: unsupported version {version}"));
    }
    let _profile = r.u8()?;
    let inst_count = r.uvarint()?;
    let ro_count = r.uvarint()?;
    let rw_count = r.uvarint()?;

    let mut rom = ZiskRom::default();

    let mut prev = 0u64;
    let mut insts = BTreeMap::new();
    for _ in 0..inst_count {
        let addr = prev + r.uvarint()?;
        prev = addr;
        let inst = decode_inst(&mut r, addr)?;
        insts.insert(addr, ZiskInstBuilder { i: inst });
    }
    rom.insts = insts;

    for _ in 0..ro_count {
        rom.ro_data_64.push(decode_section(&mut r)?);
    }
    for _ in 0..rw_count {
        rom.rw_data_64.push(decode_section(&mut r)?);
    }

    rom.optimize_instruction_lookup().map_err(|e| format!("ziskbin: {e}"))?;
    Ok(rom)
}

// ---------------------------------------------------------------------------
// ELF container
// ---------------------------------------------------------------------------

fn w16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn w32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn w64(out: &mut Vec<u8>, v: u64) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// Wraps a built [`ZiskRom`] in a minimal ELF64 carrying the ziskbin blob in a
/// `.ziskrom` section, marked with `e_machine = EM_ZISK`.
pub fn rom_to_elf(rom: &ZiskRom) -> Vec<u8> {
    let blob = encode_rom(rom);

    // Section-header string table: "\0.ziskrom\0.shstrtab\0"
    let mut shstrtab = vec![0u8];
    let name_ziskrom = shstrtab.len() as u32;
    shstrtab.extend_from_slice(SECTION_NAME.as_bytes());
    shstrtab.push(0);
    let name_shstrtab = shstrtab.len() as u32;
    shstrtab.extend_from_slice(b".shstrtab");
    shstrtab.push(0);

    const EHSIZE: u64 = 64;
    const SHENTSIZE: u64 = 64;
    let off_blob = EHSIZE;
    let off_shstr = off_blob + blob.len() as u64;
    let off_sh = off_shstr + shstrtab.len() as u64;

    let mut out = Vec::new();

    // ELF header (Elf64, little-endian)
    out.extend_from_slice(&[0x7f, b'E', b'L', b'F', 2, 1, 1, 0]); // magic, ELF64, LE, v1, SysV
    out.extend_from_slice(&[0; 8]); // e_ident padding
    w16(&mut out, 2); // e_type = ET_EXEC
    w16(&mut out, EM_ZISK); // e_machine
    w32(&mut out, 1); // e_version
    w64(&mut out, crate::mem::ROM_ENTRY); // e_entry (nonzero; real entry is in the stream)
    w64(&mut out, 0); // e_phoff
    w64(&mut out, off_sh); // e_shoff
    w32(&mut out, 0); // e_flags
    w16(&mut out, EHSIZE as u16); // e_ehsize
    w16(&mut out, 0); // e_phentsize
    w16(&mut out, 0); // e_phnum
    w16(&mut out, SHENTSIZE as u16); // e_shentsize
    w16(&mut out, 3); // e_shnum (null, .ziskrom, .shstrtab)
    w16(&mut out, 2); // e_shstrndx

    // Section data
    out.extend_from_slice(&blob);
    out.extend_from_slice(&shstrtab);

    // Section headers
    let shdr = |out: &mut Vec<u8>, name, typ, off, size| {
        w32(out, name); // sh_name
        w32(out, typ); // sh_type
        w64(out, 0); // sh_flags
        w64(out, 0); // sh_addr
        w64(out, off); // sh_offset
        w64(out, size); // sh_size
        w32(out, 0); // sh_link
        w32(out, 0); // sh_info
        w64(out, 1); // sh_addralign
        w64(out, 0); // sh_entsize
    };
    shdr(&mut out, 0, 0, 0, 0); // [0] null
    shdr(&mut out, name_ziskrom, 1 /* SHT_PROGBITS */, off_blob, blob.len() as u64);
    shdr(&mut out, name_shstrtab, 3 /* SHT_STRTAB */, off_shstr, shstrtab.len() as u64);

    out
}

/// If `elf` is a ziskbin ELF (`e_machine == EM_ZISK`), extracts and decodes the
/// `.ziskrom` section into a [`ZiskRom`]. Returns `Ok(None)` for a non-ziskbin
/// input, so a caller can fall through to RISC-V transpilation.
pub fn try_elf_to_rom(elf: &[u8]) -> Result<Option<ZiskRom>, String> {
    if elf.len() < 20 || elf[0..4] != [0x7f, b'E', b'L', b'F'] {
        return Ok(None);
    }
    if u16::from_le_bytes([elf[18], elf[19]]) != EM_ZISK {
        return Ok(None);
    }

    let file = ElfBytes::<LittleEndian>::minimal_parse(elf)
        .map_err(|e| format!("ziskbin: ELF parse error: {e}"))?;
    let shdr = file
        .section_header_by_name(SECTION_NAME)
        .map_err(|e| format!("ziskbin: section lookup error: {e}"))?
        .ok_or_else(|| format!("ziskbin: missing `{SECTION_NAME}` section"))?;
    let (data, _) =
        file.section_data(&shdr).map_err(|e| format!("ziskbin: section data error: {e}"))?;

    Ok(Some(decode_rom(data)?))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Populates the opcode-derived fields, mirroring what the builder/decoder do,
    /// so a hand-built sample matches a real assembled ROM.
    fn derive(i: &mut ZiskInst) {
        let z = ZiskOp::try_from_code(i.op).unwrap();
        i.func = z.get_call_function();
        i.op_str = z.name();
        let ot = z.op_type();
        i.op_type = ot.into();
        i.input_size = z.input_size();
        i.is_precompiled = i.input_size > 0;
        i.is_external_op = ot != OpType::Internal && ot != OpType::Fcall;
        i.m32 = z.name().contains("_w");
    }

    /// Builds a small ROM by hand using real opcodes.
    fn sample_rom() -> ZiskRom {
        let code = ZiskOp::try_from_name("copyb").unwrap().code();
        let add = ZiskOp::try_from_name("add").unwrap().code();
        let mut rom = ZiskRom::default();

        // instruction at ROM_ENTRY: copyb(imm) -> reg, fall-through
        let mut a = ZiskInst { paddr: crate::mem::ROM_ENTRY, op: code, ..Default::default() };
        a.b_src = crate::SRC_IMM;
        a.b_offset_imm0 = 0x1234;
        a.store = crate::STORE_REG;
        a.store_offset = 80;
        a.jmp_offset1 = INST_SIZE;
        a.jmp_offset2 = INST_SIZE;
        a.index = 0;
        derive(&mut a);
        rom.insts.insert(a.paddr, ZiskInstBuilder { i: a });

        // instruction at ROM_ADDR: add(reg, imm) -> reg with a real branch target
        let mut b = ZiskInst { paddr: crate::mem::ROM_ADDR, op: add, ..Default::default() };
        b.a_src = crate::SRC_REG;
        b.a_offset_imm0 = 40;
        b.b_src = crate::SRC_IMM;
        b.b_offset_imm0 = 8;
        b.store = crate::STORE_REG;
        b.store_offset = 80;
        b.jmp_offset1 = 0x40; // non-default
        b.jmp_offset2 = INST_SIZE;
        b.index = 1;
        derive(&mut b);
        rom.insts.insert(b.paddr, ZiskInstBuilder { i: b });

        rom.ro_data_64.push(DataSection64 { addr: 0x9000_0000, data: vec![0xdead, 0xbeef] });
        rom.rw_data_64.push(DataSection64 { addr: 0xa043_0000, data: vec![0, 1, 2] });
        rom
    }

    fn assert_inst_eq(a: &ZiskInst, b: &ZiskInst) {
        assert_eq!(a.op, b.op);
        assert_eq!(a.paddr, b.paddr);
        assert_eq!(a.a_src, b.a_src);
        assert_eq!(a.a_offset_imm0, b.a_offset_imm0);
        assert_eq!(a.a_use_sp_imm1, b.a_use_sp_imm1);
        assert_eq!(a.b_src, b.b_src);
        assert_eq!(a.b_offset_imm0, b.b_offset_imm0);
        assert_eq!(a.b_use_sp_imm1, b.b_use_sp_imm1);
        assert_eq!(a.store, b.store);
        assert_eq!(a.store_offset, b.store_offset);
        assert_eq!(a.store_pc, b.store_pc);
        assert_eq!(a.set_pc, b.set_pc);
        assert_eq!(a.ind_width, b.ind_width);
        assert_eq!(a.end, b.end);
        assert_eq!(a.jmp_offset1, b.jmp_offset1);
        assert_eq!(a.jmp_offset2, b.jmp_offset2);
        assert_eq!(a.index, b.index);
        assert_eq!(a.op_str, b.op_str);
        assert_eq!(a.input_size, b.input_size);
        assert_eq!(a.is_external_op, b.is_external_op);
        assert_eq!(a.m32, b.m32);
    }

    #[test]
    fn rom_round_trip() {
        let rom = sample_rom();
        let decoded = decode_rom(&encode_rom(&rom)).unwrap();

        assert_eq!(decoded.insts.len(), rom.insts.len());
        for (addr, zib) in &rom.insts {
            assert_inst_eq(&decoded.insts[addr].i, &zib.i);
        }
        assert_eq!(decoded.ro_data_64.len(), 1);
        assert_eq!(decoded.ro_data_64[0].addr, 0x9000_0000);
        assert_eq!(decoded.ro_data_64[0].data, vec![0xdead, 0xbeef]);
        assert_eq!(decoded.rw_data_64[0].data, vec![0, 1, 2]);
    }

    #[test]
    fn encode_is_stable() {
        let rom = sample_rom();
        let first = encode_rom(&rom);
        let second = encode_rom(&decode_rom(&first).unwrap());
        assert_eq!(first, second, "re-encoding a decoded ROM must be byte-identical");
    }

    #[test]
    fn elf_round_trip() {
        let rom = sample_rom();
        let elf = rom_to_elf(&rom);
        assert_eq!(&elf[0..4], b"\x7fELF");
        assert_eq!(u16::from_le_bytes([elf[18], elf[19]]), EM_ZISK);

        let decoded = try_elf_to_rom(&elf).unwrap().expect("should be a ziskbin ELF");
        assert_eq!(decoded.insts.len(), rom.insts.len());

        // A non-ziskbin buffer yields None (fall through to RISC-V).
        assert!(try_elf_to_rom(b"not an elf").unwrap().is_none());
    }
}
