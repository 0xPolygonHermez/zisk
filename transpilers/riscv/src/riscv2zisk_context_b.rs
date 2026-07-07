//! Provides Riscv2ZiskContext software implementations to implement B-extension instructions using
//! a set of native ZisK instructions.

use crate::{Riscv2ZiskContext, RiscvInst};

use zisk_core::ZiskInstBuilder;

#[cfg(feature = "float")]
use zisk_core::{FLOAT_LIB_ROM_ADDR, FLOAT_LIB_SP, FREG_F0, FREG_INST, FREG_RA, FREG_X0, REG_X0};

/*

RISC-V B extensions.  Some instructions appear in multiple extensions, but they are only implemented
once in ZisK.

Zba — address generation (accelerates array indexing via shift-add)
    sh1add, sh2add, sh3add
    sh2add.uw, sh3add.uw, slli.uw
    add.uw, sh1add.uw,
    zext.w is a pseudoinstruction here (it maps to add.uw rd, rs, zero).

Zbb — basic bit manipulation (the largest of the four)
    Logical-with-negate: andn, orn, xnor
    Count zeros: clz, ctz (plus RV64 clzw, ctzw)
    Population count: cpop (plus RV64 cpopw)
    Min/max: min, minu, max, maxu
    Sign/zero extend: sext.b, sext.h, zext.h
    Byte reverse: rev8
    OR-combine byte: orc.b
    Rotate: rol, ror, rori (plus RV64 rolw, rorw, roriw)

Zbc — carry-less multiplication
    clmul, clmulh, clmulr

Zbs — single-bit instructions (set/clear/invert/extract one bit)
    bclr, bclri, bext, bexti, binv, binvi, bset, bseti

Zbkb — bit manipulation for cryptography
    rol, ror, rori, andn, orn, xnor (all shared with Zbb; RV64 also has rolw, rorw, roriw)
    pack, packh, packw — pack low halves of two registers
    brev8 — bit-reverse within each byte

Zbkc — carry-less multiplication for cryptography
    clmul, clmulh

Zbkx — crossbar permutation
    xperm8 — byte-wise crossbar permutation
    xperm4 — nibble-wise crossbar permutation

*/

// B extensions operations cost in number of ZisK instructions:
//     andn: 2 instructions
//     orn: 2 instructions
//     xnor: 2 instructions
//     add_u_w: 2 instructions
//     sh1add: 2 instructions
//     sh2add: 2 instructions
//     sh3add: 2 instructions
//     sh1add_u_w: 3 instructions
//     sh2add_u_w: 3 instructions
//     sh3add_u_w: 3 instructions
//     bclr: 3 instructions
//     bclri: 1 instruction
//     bset: 2 instructions
//     bseti: 1 instruction
//     binv: 2 instructions
//     binvi: 1 instruction
//     bext: 2 instructions
//     bexti: 2 instructions
//     sll_u_2: 2 instructions
//     rol: 5 instructions
//     rol_w: 7 instructions
//     ror: 5 instructions
//     rori: 3 instructions
//     ror_w: 7 instructions
//     rev8: 13 instructions
//     brev8: 15 instructions
//     pack: 4 instructions
//     pack_h: 4 instructions
//     pack_w: 5 instructions
//     clz: 25 instructions
//     ctz: 15 instructions
//     clz_w: 24 instructions
//     ctz_w: 16 instructions
//     cpop: 12 instructions
//     cpop_w: 13 instructions
//     orc_b: 7 instructions
//     clmul: 321 instructions
//     clmul_h: 316 instructions
//     clmul_r: 321 instructions
//     xperm4: 114 instructions
//     xperm8: 90 instructions

impl Riscv2ZiskContext<'_> {
    /// Implements the andn function, which computes the bitwise AND of the first source register
    /// and the bitwise NOT of the second source register.
    pub fn andn(&mut self, i: &RiscvInst) {
        // Get addresses of the required instructions to implement this function
        let rom_address = i.rom_address;
        let internal_address_1 = self.rom.get_internal_address();

        // reg32 = rs2 XOR 0xFFFFFFFFFFFFFFFF = NOT rs2
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("imm", 0xFFFFFFFFFFFFFFFF, false);
            zib.src_b("reg", i.rs2 as u64, false);
            zib.op("xor").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address_1);
            let jump_address = internal_address_1 as i64 - i.rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 1/2", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // rd = rs1 AND reg32 = rs1 AND NOT rs2
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_1, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("and").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address_1 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 2/2", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }
    }

    /// Implements the orn function, which computes the bitwise OR of the first source register and
    /// the bitwise NOT of the second source register.
    pub fn orn(&mut self, i: &RiscvInst) {
        // Get addresses of the required instructions to implement this function
        let rom_address = i.rom_address;
        let internal_address_1 = self.rom.get_internal_address();

        // reg32 = rs2 XOR 0xFFFFFFFFFFFFFFFF = NOT rs2
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("imm", 0xFFFFFFFFFFFFFFFF, false);
            zib.src_b("reg", i.rs2 as u64, false);
            zib.op("xor").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address_1);
            let jump_address = internal_address_1 as i64 - i.rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 1/2", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // rd = rs1 OR reg32 = rs1 OR NOT rs2
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_1, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("or").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address_1 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 2/2", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }
    }

    /// Implements the xnor function, which computes the bitwise XOR of the first source register
    /// and the second source register, and then negates the result.
    pub fn xnor(&mut self, i: &RiscvInst) {
        // Get addresses of the required instructions to implement this function
        let rom_address = i.rom_address;
        let internal_address_1 = self.rom.get_internal_address();

        // rd = rs1 XOR rs2
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("reg", i.rs2 as u64, false);
            zib.op("xor").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address_1);
            let jump_address = internal_address_1 as i64 - i.rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 1/2", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // rd = NOT rd = NOT (rs1 XOR rs2)
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_1, i.inst_name.to_string());
            zib.src_a("imm", 0xFFFFFFFFFFFFFFFF, false);
            zib.src_b("reg", i.rd as u64, false);
            zib.op("xor").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address_1 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 2/2", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }
    }

    /// Implements add_u_w, which computes the sum of the first source register and the second
    /// source register modulo 2^32
    pub fn add_u_w(&mut self, i: &RiscvInst) {
        // Get addresses of the required instructions to implement this function
        let rom_address = i.rom_address;
        let internal_address_1 = self.rom.get_internal_address();

        // reg32 = rs1 & 0xFFFFFFFF
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", 0xFFFFFFFF, false);
            zib.op("and").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address_1);
            let jump_address = internal_address_1 as i64 - i.rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 1/2", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // rd = rs2 + reg32 = rs2 + (rs1 & 0xFFFFFFFF)
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_1, i.inst_name.to_string());
            zib.src_a("reg", i.rs2 as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("add").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address_1 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 2/2", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }
    }

    /// Implements sh1add, which computes the sum of the first source register and the second source
    /// register shifted left by 1
    pub fn sh1add(&mut self, i: &RiscvInst) {
        // Get addresses of the required instructions to implement this function
        let rom_address = i.rom_address;
        let internal_address_1 = self.rom.get_internal_address();

        // reg32 = rs1 << 1
        // Use scratch register 32 (not rd) to avoid clobbering rs2 when rd == rs2
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", 1, false);
            zib.op("sll").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address_1);
            let jump_address = internal_address_1 as i64 - i.rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 1/2", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // rd = rs2 + reg32 = rs2 + (rs1 << 1)
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_1, i.inst_name.to_string());
            zib.src_a("reg", i.rs2 as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("add").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address_1 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 2/2", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }
    }

    /// Implements sh2add, which computes the sum of the first source register and the second source
    /// register shifted left by 2
    pub fn sh2add(&mut self, i: &RiscvInst) {
        // Get addresses of the required instructions to implement this function
        let rom_address = i.rom_address;
        let internal_address_1 = self.rom.get_internal_address();

        // reg32 = rs1 << 2
        // Use scratch register 32 (not rd) to avoid clobbering rs2 when rd == rs2
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", 2, false);
            zib.op("sll").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address_1);
            let jump_address = internal_address_1 as i64 - i.rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 1/2", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // rd = rs2 + reg32 = rs2 + (rs1 << 2)
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_1, i.inst_name.to_string());
            zib.src_a("reg", i.rs2 as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("add").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address_1 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 2/2", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }
    }

    /// Implements sh3add, which computes the sum of the second source register and the first source
    /// register shifted left by 3
    pub fn sh3add(&mut self, i: &RiscvInst) {
        // Get addresses of the required instructions to implement this function
        let rom_address = i.rom_address;
        let internal_address_1 = self.rom.get_internal_address();

        // reg32 = rs1 << 3
        // Use scratch register 32 (not rd) to avoid clobbering rs2 when rd == rs2
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", 3, false);
            zib.op("sll").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address_1);
            let jump_address = internal_address_1 as i64 - i.rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 1/2", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // rd = rs2 + reg32 = rs2 + (rs1 << 3)
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_1, i.inst_name.to_string());
            zib.src_a("reg", i.rs2 as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("add").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address_1 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 2/2", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }
    }

    /// Implements sh1add_u_w, which computes the sum of the first source register and the second
    /// source register shifted left by 1, modulo 2^32
    pub fn sh1add_u_w(&mut self, i: &RiscvInst) {
        // Get addresses of the required instructions to implement this function
        let rom_address = i.rom_address;
        let internal_address_1 = self.rom.get_internal_address();
        let internal_address_2 = self.rom.get_internal_address();

        // reg32 = rs1 & 0xFFFFFFFF
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", 0xFFFFFFFF, false);
            zib.op("and").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address_1);
            let jump_address = internal_address_1 as i64 - i.rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 1/3", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // reg32 = reg32 << 1 = (rs1 & 0xFFFFFFFF) << 1
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_1, i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("imm", 1, false);
            zib.op("sll").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address_2);
            let jump_address = internal_address_2 as i64 - internal_address_1 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 2/3", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // rd = rs2 + reg32 = rs2 + (rs1 & 0xFFFFFFFF) << 1
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_2, i.inst_name.to_string());
            zib.src_a("reg", i.rs2 as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("add").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address_2 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 3/3", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }
    }

    /// Implements sh2add_u_w, which computes the sum of the first source register and the second
    /// source register shifted left by 2, modulo 2^32
    pub fn sh2add_u_w(&mut self, i: &RiscvInst) {
        // Get addresses of the required instructions to implement this function
        let rom_address = i.rom_address;
        let internal_address_1 = self.rom.get_internal_address();
        let internal_address_2 = self.rom.get_internal_address();

        // reg32 = rs1 & 0xFFFFFFFF
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", 0xFFFFFFFF, false);
            zib.op("and").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address_1);
            let jump_address = internal_address_1 as i64 - i.rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 1/3", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // reg32 = reg32 << 2 = (rs1 & 0xFFFFFFFF) << 2
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_1, i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("imm", 2, false);
            zib.op("sll").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address_2);
            let jump_address = internal_address_2 as i64 - internal_address_1 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 2/3", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // rd = rs2 + reg32 = rs2 + (rs1 & 0xFFFFFFFF) << 2
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_2, i.inst_name.to_string());
            zib.src_a("reg", i.rs2 as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("add").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address_2 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 3/3", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }
    }

    /// Implements sh3add_u_w, which computes the sum of the first source register and the second
    /// source register shifted left by 3, modulo 2^32
    pub fn sh3add_u_w(&mut self, i: &RiscvInst) {
        // Get addresses of the required instructions to implement this function
        let rom_address = i.rom_address;
        let internal_address_1 = self.rom.get_internal_address();
        let internal_address_2 = self.rom.get_internal_address();

        // reg32 = rs1 & 0xFFFFFFFF
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", 0xFFFFFFFF, false);
            zib.op("and").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address_1);
            let jump_address = internal_address_1 as i64 - i.rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 1/3", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // reg32 = reg32 << 3 = (rs1 & 0xFFFFFFFF) << 3
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_1, i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("imm", 3, false);
            zib.op("sll").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address_2);
            let jump_address = internal_address_2 as i64 - internal_address_1 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 2/3", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // rd = rs2 + reg32 = rs2 + (rs1 & 0xFFFFFFFF) << 3
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_2, i.inst_name.to_string());
            zib.src_a("reg", i.rs2 as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("add").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address_2 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 3/3", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }
    }

    /// Implements bclr, which clears the bit of the first source register specified by the second
    /// source register
    pub fn bclr(&mut self, i: &RiscvInst) {
        // Get addresses of the required instructions to implement this function
        let rom_address = i.rom_address;
        let internal_address_1 = self.rom.get_internal_address();
        let internal_address_2 = self.rom.get_internal_address();

        // reg32 = 1 << rs2 = 1 << (rs2 & 0x3F)
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("imm", 1, false);
            zib.src_b("reg", i.rs2 as u64, false);
            zib.op("sll").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address_1);
            let jump_address = internal_address_1 as i64 - i.rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 1/3", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // reg32 = reg32 XOR 0xFFFFFFFFFFFFFFFF = NOT reg32 = NOT (1 << (rs2 & 0x3F))
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_1, i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("imm", 0xFFFFFFFFFFFFFFFF, false);
            zib.op("xor").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address_2);
            let jump_address = internal_address_2 as i64 - internal_address_1 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 2/3", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // rd = rs1 AND reg32 = rs1 AND NOT (1 << (rs2 & 0x3F))
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_2, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("and").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address_2 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 3/3", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }
    }

    /// Implements bclri, which clears the bit of the first source register specified by the
    /// immediate
    pub fn bclri(&mut self, i: &RiscvInst) {
        // Get addresses of the required instructions to implement this function
        let rom_address = i.rom_address;

        // rd = rs1 AND NOT (1 << (imm & 0x3F))
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", !(1 << (i.imm as u64 & 0x3F)), false);
            zib.op("and").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.j(4, 4);
            zib.verbose(&format!("{} r{}, r{}, r{}", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }
    }

    /// Implements bset, which sets the bit of the first source register specified by the second
    /// source register
    pub fn bset(&mut self, i: &RiscvInst) {
        // Get addresses of the required instructions to implement this function
        let rom_address = i.rom_address;
        let internal_address_1 = self.rom.get_internal_address();

        // reg32 = 1 << rs2 = 1 << (rs2 & 0x3F)
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("imm", 1, false);
            zib.src_b("reg", i.rs2 as u64, false);
            zib.op("sll").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address_1);
            let jump_address = internal_address_1 as i64 - i.rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 1/2", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // rd = rs1 OR reg32 = rs1 OR (1 << (rs2 & 0x3F))
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_1, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("or").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address_1 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 2/2", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }
    }

    /// Implements bseti, which sets the bit of the first source register specified by the immediate
    pub fn bseti(&mut self, i: &RiscvInst) {
        // Get addresses of the required instructions to implement this function
        let rom_address = i.rom_address;

        // rd = rs1 OR (1 << (i.imm & 0x3F))
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", 1 << (i.imm & 0x3F) as u64, false);
            zib.op("or").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.j(4, 4);
            zib.verbose(&format!("{} r{}, r{}, r{}", i.inst_name, i.rd, i.rs1, i.imm));
            zib.build(self.rom);
        }
    }

    /// Implements binv, which inverts the bit of the first source register specified by the second
    /// source register
    pub fn binv(&mut self, i: &RiscvInst) {
        // Get addresses of the required instructions to implement this function
        let rom_address = i.rom_address;
        let internal_address_1 = self.rom.get_internal_address();

        // reg32 = 1 << rs2 = 1 << (rs2 & 0x3F)
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("imm", 1, false);
            zib.src_b("reg", i.rs2 as u64, false);
            zib.op("sll").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address_1);
            let jump_address = internal_address_1 as i64 - i.rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 1/2", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // rd = rs1 XOR reg32 = rs1 XOR (1 << (rs2 & 0x3F))
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_1, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("xor").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address_1 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 2/2", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }
    }

    /// Implements binvi, which inverts the bit of the first source register specified by the
    /// immediate
    pub fn binvi(&mut self, i: &RiscvInst) {
        // Get addresses of the required instructions to implement this function
        let rom_address = i.rom_address;

        // rd = rs1 XOR (1 << (imm & 0x3F))
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", 1 << (i.imm as u64 & 0x3F), false);
            zib.op("xor").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.j(4, 4);
            zib.verbose(&format!("{} r{}, r{}, imm{}", i.inst_name, i.rd, i.rs1, i.imm));
            zib.build(self.rom);
        }
    }

    /// Implements bext, which extracts the bit of the first source register specified by the second
    /// source register, i.e. it sets the destination register to 1 if the bit is set; 0 otherwise
    pub fn bext(&mut self, i: &RiscvInst) {
        // Get addresses of the required instructions to implement this function
        let rom_address = i.rom_address;
        let internal_address_1 = self.rom.get_internal_address();

        // rd = rs1 >> rs2 = rs1 >> (rs2 & 0x3F)
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("reg", i.rs2 as u64, false);
            zib.op("srl").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address_1);
            let jump_address = internal_address_1 as i64 - i.rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 1/2", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // rd = rd AND 1 = (rs1 >> (rs2 & 0x3F)) & 1
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_1, i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 1, false);
            zib.op("and").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address_1 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 2/2", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }
    }

    /// Implements bexti, which extracts the bit of the first source register specified by the
    /// immediate, i.e. it sets the destination register to 1 if the bit is set; 0 otherwise
    pub fn bexti(&mut self, i: &RiscvInst) {
        // Get addresses of the required instructions to implement this function
        let rom_address = i.rom_address;
        let internal_address_1 = self.rom.get_internal_address();

        // rd = rs1 >> imm = rs1 >> (imm & 0x3F)
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", i.imm as u64, false);
            zib.op("srl").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address_1);
            let jump_address = internal_address_1 as i64 - i.rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 1/2", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // rd = rd AND 1 = (rs1 >> (imm & 0x3F)) & 1
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_1, i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 1, false);
            zib.op("and").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address_1 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, imm {} 2/2", i.inst_name, i.rd, i.rs1, i.imm));
            zib.build(self.rom);
        }
    }

    /// Implements sll_u_w, which shifts the first source register modulo 32 by the second source
    /// register
    pub fn sll_u_w(&mut self, i: &RiscvInst, is_imm: bool) {
        // Get addresses of the required instructions to implement this function
        let rom_address = i.rom_address;
        let internal_address_1 = self.rom.get_internal_address();

        // reg32 = rs1 & 0xFFFFFFFF
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", 0xFFFFFFFF, false);
            zib.op("and").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address_1);
            let jump_address = internal_address_1 as i64 - i.rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 1/2", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // rd = reg32 << rs2 = (rs1 & 0xFFFFFFFF) << rs2
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_1, i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            if is_imm {
                zib.src_b("imm", i.imm as u64, false);
            } else {
                zib.src_b("reg", i.rs2 as u64, false);
            }
            zib.op("sll").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address_1 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 2/2", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }
    }

    /// Implements rol, which rotates the first source register left by the second source register
    pub fn rol(&mut self, i: &RiscvInst) {
        // Get addresses of the required instructions to implement this function
        let rom_address = i.rom_address;
        let internal_address_1 = self.rom.get_internal_address();
        let internal_address_2 = self.rom.get_internal_address();
        let internal_address_3 = self.rom.get_internal_address();
        let internal_address_4 = self.rom.get_internal_address();

        // reg32 = rs1 << rs2 = rs1 << (rs2 & 0x3F)
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("reg", i.rs2 as u64, false);
            zib.op("sll").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address_1);
            let jump_address = internal_address_1 as i64 - i.rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 1/5", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // reg33 = rs2 & 0x3F
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_1, i.inst_name.to_string());
            zib.src_a("reg", i.rs2 as u64, false);
            zib.src_b("imm", 0x3F, false);
            zib.op("and").unwrap();
            zib.store("reg", 33, false, false);
            zib.set_next_internal_address(internal_address_2);
            let jump_address = internal_address_2 as i64 - internal_address_1 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 2/5", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // reg33 = 64 - reg33 = 64 - (rs2 & 0x3F)
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_2, i.inst_name.to_string());
            zib.src_a("imm", 64, false);
            zib.src_b("reg", 33, false);
            zib.op("sub").unwrap();
            zib.store("reg", 33, false, false);
            zib.set_next_internal_address(internal_address_3);
            let jump_address = internal_address_3 as i64 - internal_address_2 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 3/5", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // rd = rs1 >> reg33 = rs1 >> (64 - (rs2 & 0x3F))
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_3, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("reg", 33, false);
            zib.op("srl").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address_4);
            let jump_address = internal_address_4 as i64 - internal_address_3 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 4/5", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // rd = reg32 OR rd = (rs1 << (rs2 & 0x3F)) OR (rs1 >> (64 - (rs2 & 0x3F)))
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_4, i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("reg", i.rd as u64, false);
            zib.op("or").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address_4 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 5/5", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }
    }

    /// Implements rol_w, which rotates the first source register modulo 32 left by the second source
    /// register
    /// TODO: optimize using sll_w and srl_w, so no mask is needed
    pub fn rol_w(&mut self, i: &RiscvInst) {
        // Get addresses of the required instructions to implement this function
        let rom_address = i.rom_address;
        let internal_address_1 = self.rom.get_internal_address();
        let internal_address_2 = self.rom.get_internal_address();
        let internal_address_3 = self.rom.get_internal_address();
        let internal_address_4 = self.rom.get_internal_address();
        let internal_address_5 = self.rom.get_internal_address();
        let internal_address_6 = self.rom.get_internal_address();

        // reg32 = rs1 & 0xFFFFFFFF
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", 0xFFFFFFFF, false);
            zib.op("and").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address_1);
            let jump_address = internal_address_1 as i64 - i.rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 1/7", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // reg32 = reg32 << rs2 = (rs1 & 0xFFFFFFFF) << (rs2 & 0x1F)
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_1, i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("reg", i.rs2 as u64, false);
            zib.op("sll_w").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address_2);
            let jump_address = internal_address_2 as i64 - internal_address_1 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 2/7", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // reg33 = rs2 & 0x1F
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_2, i.inst_name.to_string());
            zib.src_a("reg", i.rs2 as u64, false);
            zib.src_b("imm", 0x1F, false);
            zib.op("and").unwrap();
            zib.store("reg", 33, false, false);
            zib.set_next_internal_address(internal_address_3);
            let jump_address = internal_address_3 as i64 - internal_address_2 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 3/7", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // reg33 = 32 - reg33 = 32 - (rs2 & 0x1F)
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_3, i.inst_name.to_string());
            zib.src_a("imm", 32, false);
            zib.src_b("reg", 33, false);
            zib.op("sub").unwrap();
            zib.store("reg", 33, false, false);
            zib.set_next_internal_address(internal_address_4);
            let jump_address = internal_address_4 as i64 - internal_address_3 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 4/7", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // rd = rs1 >> reg33 = rs1 >> (32 - (rs2 & 0x1F))
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_4, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("reg", 33, false);
            zib.op("srl_w").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address_5);
            let jump_address = internal_address_5 as i64 - internal_address_4 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 5/7", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // rd = reg32 OR rd = (rs1 << (rs2 & 0x1F)) OR (rs1 >> (32 - (rs2 & 0x1F)))
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_5, i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("reg", i.rd as u64, false);
            zib.op("or").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address_6);
            let jump_address = internal_address_6 as i64 - internal_address_5 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 6/7", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // sign-extend rd from 32 to 64 bits
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_6, i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.op("signextend_w").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address_6 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 7/7", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }
    }

    /// Implements ror, which rotates the first source register right by the second source register
    pub fn ror(&mut self, i: &RiscvInst) {
        // Get addresses of the required instructions to implement this function
        let rom_address = i.rom_address;
        let internal_address_1 = self.rom.get_internal_address();
        let internal_address_2 = self.rom.get_internal_address();
        let internal_address_3 = self.rom.get_internal_address();
        let internal_address_4 = self.rom.get_internal_address();

        // reg32 = rs1 >> rs2 = rs1 >> (rs2 & 0x3F)
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("reg", i.rs2 as u64, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address_1);
            let jump_address = internal_address_1 as i64 - i.rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 1/5", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // reg33 = rs2 & 0x3F
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_1, i.inst_name.to_string());
            zib.src_a("reg", i.rs2 as u64, false);
            zib.src_b("imm", 0x3F, false);
            zib.op("and").unwrap();
            zib.store("reg", 33, false, false);
            zib.set_next_internal_address(internal_address_2);
            let jump_address = internal_address_2 as i64 - internal_address_1 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 2/5", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // reg33 = 64 - reg33 = 64 - (rs2 & 0x3F)
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_2, i.inst_name.to_string());
            zib.src_a("imm", 64, false);
            zib.src_b("reg", 33, false);
            zib.op("sub").unwrap();
            zib.store("reg", 33, false, false);
            zib.set_next_internal_address(internal_address_3);
            let jump_address = internal_address_3 as i64 - internal_address_2 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 3/5", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // rd = rs1 << reg33 = rs1 << (64 - (rs2 & 0x3F))
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_3, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("reg", 33, false);
            zib.op("sll").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address_4);
            let jump_address = internal_address_4 as i64 - internal_address_3 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 4/5", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // rd = reg32 OR rd = (rs1 >> (rs2 & 0x3F)) OR (rs1 << (64 - (rs2 & 0x3F)))
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_4, i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("reg", i.rd as u64, false);
            zib.op("or").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address_4 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 5/5", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }
    }

    /// Implements rori, which rotates the first source register right by the immediate
    pub fn rori(&mut self, i: &RiscvInst) {
        // Get addresses of the required instructions to implement this function
        let rom_address = i.rom_address;
        let internal_address_1 = self.rom.get_internal_address();
        let internal_address_2 = self.rom.get_internal_address();

        // reg32 = rs1 >> (imm & 0x3F)
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", i.imm as u64, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address_1);
            let jump_address = internal_address_1 as i64 - i.rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, imm{} 1/3", i.inst_name, i.rd, i.rs1, i.imm));
            zib.build(self.rom);
        }

        // rd = rs1 << (64 - (imm & 0x3F))
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_1, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", 64 - (i.imm as u64 & 0x3F), false);
            zib.op("sll").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address_2);
            let jump_address = internal_address_2 as i64 - internal_address_1 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, imm{} 2/3", i.inst_name, i.rd, i.rs1, i.imm));
            zib.build(self.rom);
        }

        // rd = reg32 OR rd = (rs1 >> (imm & 0x3F)) OR (rs1 << (64 - (imm & 0x3F)))
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_2, i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("reg", i.rd as u64, false);
            zib.op("or").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address_2 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, imm{} 3/3", i.inst_name, i.rd, i.rs1, i.imm));
            zib.build(self.rom);
        }
    }

    /// Implements ror_w, which rotates the first source register modulo 32 right by the second
    /// source register
    //
    // TODO: optimize using sll_w and srl_w, so no mask is needed
    pub fn ror_w(&mut self, i: &RiscvInst) {
        // Get addresses of the required instructions to implement this function
        let rom_address = i.rom_address;
        let internal_address_1 = self.rom.get_internal_address();
        let internal_address_2 = self.rom.get_internal_address();
        let internal_address_3 = self.rom.get_internal_address();
        let internal_address_4 = self.rom.get_internal_address();
        let internal_address_5 = self.rom.get_internal_address();
        let internal_address_6 = self.rom.get_internal_address();

        // reg32 = rs1 & 0xFFFFFFFF
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", 0xFFFFFFFF, false);
            zib.op("and").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address_1);
            let jump_address = internal_address_1 as i64 - i.rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 1/7", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // reg32 = reg32 >> rs2 = (rs1 & 0xFFFFFFFF) >> (rs2 & 0x1F)
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_1, i.inst_name.to_string());
            zib.src_a("reg", 32 as u64, false);
            zib.src_b("reg", i.rs2 as u64, false);
            zib.op("srl_w").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address_2);
            let jump_address = internal_address_2 as i64 - internal_address_1 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 2/7", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // reg33 = rs2 & 0x1F
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_2, i.inst_name.to_string());
            zib.src_a("reg", i.rs2 as u64, false);
            zib.src_b("imm", 0x1F, false);
            zib.op("and").unwrap();
            zib.store("reg", 33, false, false);
            zib.set_next_internal_address(internal_address_3);
            let jump_address = internal_address_3 as i64 - internal_address_2 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 3/7", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // reg33 = 32 - reg33 = 32 - (rs2 & 0x1F)
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_3, i.inst_name.to_string());
            zib.src_a("imm", 32, false);
            zib.src_b("reg", 33, false);
            zib.op("sub").unwrap();
            zib.store("reg", 33, false, false);
            zib.set_next_internal_address(internal_address_4);
            let jump_address = internal_address_4 as i64 - internal_address_3 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 4/7", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // reg33 = rs1 << reg33 = rs1 << (32 - (rs2 & 0x1F))
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_4, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("reg", 33, false);
            zib.op("sll_w").unwrap();
            zib.store("reg", 33, false, false);
            zib.set_next_internal_address(internal_address_5);
            let jump_address = internal_address_5 as i64 - internal_address_4 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 5/7", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // rd = reg32 OR reg33 = (rs1 >> (rs2 & 0x1F)) OR (rs1 << (32 - (rs2 & 0x1F)))
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_5, i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("reg", 33, false);
            zib.op("or").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address_6);
            let jump_address = internal_address_6 as i64 - internal_address_5 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 6/7", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }

        // sign-extend rd from 32 to 64 bits
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_6, i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.op("signextend_w").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address_6 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, r{} 7/7", i.inst_name, i.rd, i.rs1, i.rs2));
            zib.build(self.rom);
        }
    }

    /// Implements rori_w, which rotates the first source register modulo 32 right by the immediate
    //
    // TODO: optimize using sll_w and srl_w, so no mask is needed
    pub fn rori_w(&mut self, i: &RiscvInst) {
        // Get addresses of the required instructions to implement this function
        let rom_address = i.rom_address;
        let internal_address_1 = self.rom.get_internal_address();
        let internal_address_2 = self.rom.get_internal_address();
        let internal_address_3 = self.rom.get_internal_address();
        let internal_address_4 = self.rom.get_internal_address();

        // reg32 = rs1 & 0xFFFFFFFF
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", 0xFFFFFFFF, false);
            zib.op("and").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address_1);
            let jump_address = internal_address_1 as i64 - i.rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, imm {} 1/5", i.inst_name, i.rd, i.rs1, i.imm));
            zib.build(self.rom);
        }

        // reg32 = reg32 >> imm = (rs1 & 0xFFFFFFFF) >> (imm & 0x1F)
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_1, i.inst_name.to_string());
            zib.src_a("reg", 32 as u64, false);
            zib.src_b("imm", i.imm as u64, false);
            zib.op("srl_w").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address_2);
            let jump_address = internal_address_2 as i64 - internal_address_1 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, imm {} 2/5", i.inst_name, i.rd, i.rs1, i.imm));
            zib.build(self.rom);
        }

        // rd = rs1 << (32 - (imm & 0x1F))
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_2, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", 32 - (i.imm as u64 & 0x1F), false);
            zib.op("sll_w").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address_3);
            let jump_address = internal_address_3 as i64 - internal_address_2 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, imm {} 3/5", i.inst_name, i.rd, i.rs1, i.imm));
            zib.build(self.rom);
        }

        // rd = reg32 OR rd = (rs1 >> (imm & 0x1F)) OR (rs1 << (32 - (imm & 0x1F)))
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_3, i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("reg", i.rd as u64, false);
            zib.op("or").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address_4);
            let jump_address = internal_address_4 as i64 - internal_address_3 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, imm {} 4/5", i.inst_name, i.rd, i.rs1, i.imm));
            zib.build(self.rom);
        }

        // sign-extend rd from 32 to 64 bits
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address_4, i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.op("signextend_w").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address_4 as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{}, imm {} 5/5", i.inst_name, i.rd, i.rs1, i.imm));
            zib.build(self.rom);
        }
    }

    /// Implement rev8: switch endianness of a 64-bit register (i.e. reverse the order of the bytes)
    //
    // M1 = 0x00FF00FF00FF00FF
    // M2 = 0x0000FFFF0000FFFF
    //
    // # stage 1 — swap adjacent bytes
    // x = ((x >> 8)  & M1) | ((x & M1) << 8)
    // # stage 2 — swap adjacent 16-bit lanes
    // x = ((x >> 16) & M2) | ((x & M2) << 16)
    // # stage 3 — swap the two 32-bit halves (no mask needed)
    // x = (x >> 32) | (x << 32)
    //
    // ALU op count (RV64I, excluding constant loads):
    // stage 1: srli, and, and, slli, or → 5
    // stage 2: srli, and, and, slli, or → 5
    // stage 3: srli, slli, or → 3
    //
    pub fn rev8(&mut self, i: &RiscvInst) {
        // Define constants for the masks used in the rev8 implementation
        const M1: u64 = 0x00FF00FF00FF00FF;
        const M2: u64 = 0x0000FFFF0000FFFF;

        // Get addresses for the required instructions to implement this function
        let rom_address = i.rom_address;
        let mut internal_address = [0u64; 12];
        for i in 0..12 {
            internal_address[i] = self.rom.get_internal_address();
        }

        // reg32 = rs1 >> 8
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", 8, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[0]);
            let jump_address = internal_address[0] as i64 - i.rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 1/14", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // reg32 = reg32 & M1
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[0], i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("imm", M1, false);
            zib.op("and").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[1]);
            let jump_address = internal_address[1] as i64 - internal_address[0] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 2/14", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // reg33 = rs1 & M1
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[1], i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", M1, false);
            zib.op("and").unwrap();
            zib.store("reg", 33, false, false);
            zib.set_next_internal_address(internal_address[2]);
            let jump_address = internal_address[2] as i64 - internal_address[1] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 3/14", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // reg33 = reg33 << 8
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[2], i.inst_name.to_string());
            zib.src_a("reg", 33, false);
            zib.src_b("imm", 8, false);
            zib.op("sll").unwrap();
            zib.store("reg", 33, false, false);
            zib.set_next_internal_address(internal_address[3]);
            let jump_address = internal_address[3] as i64 - internal_address[2] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 4/14", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = reg32 | reg33
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[3], i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("reg", 33, false);
            zib.op("or").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[4]);
            let jump_address = internal_address[4] as i64 - internal_address[3] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 5/14", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // reg32 = rd >> 16
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[4], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 16, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[5]);
            let jump_address = internal_address[5] as i64 - internal_address[4] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 6/14", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // reg32 = reg32 & M2
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[5], i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("imm", M2, false);
            zib.op("and").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[6]);
            let jump_address = internal_address[6] as i64 - internal_address[5] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 7/14", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // reg33 = rd & M2
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[6], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", M2, false);
            zib.op("and").unwrap();
            zib.store("reg", 33, false, false);
            zib.set_next_internal_address(internal_address[7]);
            let jump_address = internal_address[7] as i64 - internal_address[6] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 8/14", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // reg33 = reg33 << 16
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[7], i.inst_name.to_string());
            zib.src_a("reg", 33, false);
            zib.src_b("imm", 16, false);
            zib.op("sll").unwrap();
            zib.store("reg", 33, false, false);
            zib.set_next_internal_address(internal_address[8]);
            let jump_address = internal_address[8] as i64 - internal_address[7] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 9/14", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = reg32 | reg33
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[8], i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("reg", 33, false);
            zib.op("or").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[9]);
            let jump_address = internal_address[9] as i64 - internal_address[8] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 10/14", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // reg32 = rd >> 32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[9], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 32, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[10]);
            let jump_address = internal_address[10] as i64 - internal_address[9] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 11/14", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd << 32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[10], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 32, false);
            zib.op("sll").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[11]);
            let jump_address = internal_address[11] as i64 - internal_address[10] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 12/14", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = reg32 | rd
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[11], i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("reg", i.rd as u64, false);
            zib.op("or").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address[11] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 13/14", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }
    }

    /// Implements brev8, which reverses the order of the bits of every byte in a 64-bit register
    //
    // m1 = 0x5555555555555555   # 0x55 per byte — swap adjacent bits
    // m2 = 0x3333333333333333   # 0x33 per byte — swap adjacent bit-pairs
    // m4 = 0x0F0F0F0F0F0F0F0F   # 0x0F per byte — swap adjacent nibbles
    //
    // x = ((x >> 1) & m1) | ((x & m1) << 1)   # stage 1
    // x = ((x >> 2) & m2) | ((x & m2) << 2)   # stage 2
    // x = ((x >> 4) & m4) | ((x & m4) << 4)   # stage 3
    //
    // ALU op count (RV64I, excluding constant loads): each stage is srli, and, and, slli, or → 5, so 15 instructions. Also the log-optimum (log₂8 = 3 stages).
    //
    pub fn brev8(&mut self, i: &RiscvInst) {
        // Define constants for the masks used in the brev8 implementation
        const M1: u64 = 0x5555555555555555;
        const M2: u64 = 0x3333333333333333;
        const M4: u64 = 0x0F0F0F0F0F0F0F0F;

        // Get addresses for the required instructions to implement this function
        let rom_address = i.rom_address;
        let mut internal_address = [0u64; 14];
        for i in 0..14 {
            internal_address[i] = self.rom.get_internal_address();
        }

        // reg32 = rs1 >> 1
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", 1, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[0]);
            let jump_address = internal_address[0] as i64 - i.rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 1/15", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // reg32 = reg32 & M1
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[0], i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("imm", M1, false);
            zib.op("and").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[1]);
            let jump_address = internal_address[1] as i64 - internal_address[0] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 2/15", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // reg33 = rs1 & M1
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[1], i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", M1, false);
            zib.op("and").unwrap();
            zib.store("reg", 33, false, false);
            zib.set_next_internal_address(internal_address[2]);
            let jump_address = internal_address[2] as i64 - internal_address[1] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 3/15", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // reg33 = reg33 << 1
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[2], i.inst_name.to_string());
            zib.src_a("reg", 33, false);
            zib.src_b("imm", 1, false);
            zib.op("sll").unwrap();
            zib.store("reg", 33, false, false);
            zib.set_next_internal_address(internal_address[3]);
            let jump_address = internal_address[3] as i64 - internal_address[2] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 4/15", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = reg32 | reg33
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[3], i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("reg", 33, false);
            zib.op("or").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[4]);
            let jump_address = internal_address[4] as i64 - internal_address[3] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 5/15", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // reg32 = rd >> 2
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[4], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 2, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[5]);
            let jump_address = internal_address[5] as i64 - internal_address[4] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 6/15", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // reg32 = reg32 & M2
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[5], i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("imm", M2, false);
            zib.op("and").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[6]);
            let jump_address = internal_address[6] as i64 - internal_address[5] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 7/15", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // reg33 = rd & M2
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[6], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", M2, false);
            zib.op("and").unwrap();
            zib.store("reg", 33, false, false);
            zib.set_next_internal_address(internal_address[7]);
            let jump_address = internal_address[7] as i64 - internal_address[6] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 8/15", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // reg33 = reg33 << 2
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[7], i.inst_name.to_string());
            zib.src_a("reg", 33, false);
            zib.src_b("imm", 2, false);
            zib.op("sll").unwrap();
            zib.store("reg", 33, false, false);
            zib.set_next_internal_address(internal_address[8]);
            let jump_address = internal_address[8] as i64 - internal_address[7] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 9/15", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = reg32 | reg33
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[8], i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("reg", 33, false);
            zib.op("or").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[9]);
            let jump_address = internal_address[9] as i64 - internal_address[8] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 10/15", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // reg32 = rd >> 4
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[9], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 4, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[10]);
            let jump_address = internal_address[10] as i64 - internal_address[9] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 11/15", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // reg32 = reg32 & M4
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[10], i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("imm", M4, false);
            zib.op("and").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[11]);
            let jump_address = internal_address[11] as i64 - internal_address[10] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 12/15", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // reg33 = rd & M4
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[11], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", M4, false);
            zib.op("and").unwrap();
            zib.store("reg", 33, false, false);
            zib.set_next_internal_address(internal_address[12]);
            let jump_address = internal_address[12] as i64 - internal_address[11] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 13/15", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // reg33 = reg33 << 4
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[12], i.inst_name.to_string());
            zib.src_a("reg", 33, false);
            zib.src_b("imm", 4, false);
            zib.op("sll").unwrap();
            zib.store("reg", 33, false, false);
            zib.set_next_internal_address(internal_address[13]);
            let jump_address = internal_address[13] as i64 - internal_address[12] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 14/15", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = reg32 | reg33
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[13], i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("reg", 33, false);
            zib.op("or").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address[13] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 15/15", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }
    }

    /// Implements pack, which packs the lower half of a 64-bit register with the lower half of
    /// another 64-bit register
    //
    // reg32 = rs1 & 0xFFFFFFFF
    // rd = rs2 & 0xFFFFFFFF
    // rd = rd << 32 = (rs2 & 0xFFFFFFFF) << 32
    // rd = reg32 | rd = (rs1 & 0xFFFFFFFF) | ((rs2 & 0xFFFFFFFF) << 32)
    //
    pub fn pack(&mut self, i: &RiscvInst) {
        // Get addresses for the required instructions to implement this function
        let rom_address = i.rom_address;
        let mut internal_address = [0u64; 3];
        for i in 0..3 {
            internal_address[i] = self.rom.get_internal_address();
        }

        // reg32 = rs1 & 0xFFFFFFFF
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", 0xFFFFFFFF, false);
            zib.op("and").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[0]);
            let jump_address = internal_address[0] as i64 - rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 1/4", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rs2 & 0xFFFFFFFF
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[0], i.inst_name.to_string());
            zib.src_a("reg", i.rs2 as u64, false);
            zib.src_b("imm", 0xFFFFFFFF, false);
            zib.op("and").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[1]);
            let jump_address = internal_address[1] as i64 - internal_address[0] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 2/4", i.inst_name, i.rd, i.rs2));
            zib.build(self.rom);
        }

        // rd = rd << 32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[1], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 32, false);
            zib.op("sll").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[2]);
            let jump_address = internal_address[2] as i64 - internal_address[1] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 3/4", i.inst_name, i.rd, i.rs2));
            zib.build(self.rom);
        }

        // rd = reg32 | rd
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[2], i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("reg", i.rd as u64, false);
            zib.op("or").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address[2] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 4/4", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }
    }

    /// Implements pack_h, which packs the lower byte of a 64-bit register with the lower byte of
    /// another 64-bit register
    //
    // reg32 = rs1 & 0xFF
    // rd = rs2 & 0xFF
    // rd = rd << 8 = (rs2 & 0xFF) << 8
    // rd = reg32 | rd = (rs1 & 0xFF) | ((rs2 & 0xFF) << 8)
    //
    pub fn pack_h(&mut self, i: &RiscvInst) {
        // Get addresses for the required instructions to implement this function
        let rom_address = i.rom_address;
        let mut internal_address = [0u64; 3];
        for i in 0..3 {
            internal_address[i] = self.rom.get_internal_address();
        }

        // reg32 = rs1 & 0xFF
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", 0xFF, false);
            zib.op("and").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[0]);
            let jump_address = internal_address[0] as i64 - rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 1/4", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rs2 & 0xFF
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[0], i.inst_name.to_string());
            zib.src_a("reg", i.rs2 as u64, false);
            zib.src_b("imm", 0xFF, false);
            zib.op("and").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[1]);
            let jump_address = internal_address[1] as i64 - internal_address[0] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 2/4", i.inst_name, i.rd, i.rs2));
            zib.build(self.rom);
        }

        // rd = rd << 8
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[1], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 8, false);
            zib.op("sll").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[2]);
            let jump_address = internal_address[2] as i64 - internal_address[1] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 3/4", i.inst_name, i.rd, i.rs2));
            zib.build(self.rom);
        }

        // rd = reg32 | rd
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[2], i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("reg", i.rd as u64, false);
            zib.op("or").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address[2] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 4/4", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }
    }

    /// Implement pack_w, which packs the lower 16 bits of a 64-bit register with the lower 16 bits
    /// of another 64-bit register
    //
    // reg32 = rs1 & 0xFFFF
    // rd = rs2 & 0xFFFF
    // rd = rd << 16 = (rs2 & 0xFFFF) << 16
    // rd = reg32 | rd = (rs1 & 0xFFFF) | ((rs2 & 0xFFFF) << 16)
    // sign-extend rd to 64 bits
    //
    pub fn pack_w(&mut self, i: &RiscvInst) {
        // Get addresses for the required instructions to implement this function
        let rom_address = i.rom_address;
        let mut internal_address = [0u64; 4];
        for i in 0..4 {
            internal_address[i] = self.rom.get_internal_address();
        }

        // reg32 = rs1 & 0xFFFF
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", 0xFFFF, false);
            zib.op("and").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[0]);
            let jump_address = internal_address[0] as i64 - rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 1/5", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rs2 & 0xFFFF
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[0], i.inst_name.to_string());
            zib.src_a("reg", i.rs2 as u64, false);
            zib.src_b("imm", 0xFFFF, false);
            zib.op("and").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[1]);
            let jump_address = internal_address[1] as i64 - internal_address[0] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 2/5", i.inst_name, i.rd, i.rs2));
            zib.build(self.rom);
        }

        // rd = rd << 16
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[1], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 16, false);
            zib.op("sll").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[2]);
            let jump_address = internal_address[2] as i64 - internal_address[1] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 3/5", i.inst_name, i.rd, i.rs2));
            zib.build(self.rom);
        }

        // rd = rd | reg32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[2], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("or").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[3]);
            let jump_address = internal_address[3] as i64 - internal_address[2] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 4/5", i.inst_name, i.rd, i.rs2));
            zib.build(self.rom);
        }

        // Sign-extend rd to 64 bits
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[3], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.op("signextend_w").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address[3] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 5/5", i.inst_name, i.rd, i.rs2));
            zib.build(self.rom);
        }
    }

    /// Implement clz, which counts the number of leading zeros in a 64-bit register
    //
    // Smear-right all ones:
    // x |= x >> 1
    // x |= x >> 2
    // x |= x >> 4
    // x |= x >> 8
    // x |= x >> 16
    // x |= x >> 32       # x is now 0…01…1  (all bits from MSB down to 0 set)
    //
    // clz = 64 - popcount(x)
    //
    // Popcount the result to get the number of ones, and subtract from 64 to get the number of
    // leading zeros:
    // x = x - ((x >> 1) & 0x5555555555555555)
    // x = (x & 0x3333333333333333) + ((x >> 2) & 0x3333333333333333)
    // x = (x + (x >> 4)) & 0x0F0F0F0F0F0F0F0F
    // cnt = (x * 0x0101010101010101) >> 56
    pub fn clz(&mut self, i: &RiscvInst) {
        // Get addresses for the required instructions to implement this function
        let rom_address = i.rom_address;
        let mut internal_address = [0u64; 24];
        for i in 0..24 {
            internal_address[i] = self.rom.get_internal_address();
        }

        // reg32 = rs1 >> 1
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", 1, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[0]);
            let jump_address = internal_address[0] as i64 - rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 1/25", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rs1 | (reg32 >> 1)
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[0], i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("or").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[1]);
            let jump_address = internal_address[1] as i64 - internal_address[0] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 2/25", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // reg32 = rd >> 2
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[1], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 2, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[2]);
            let jump_address = internal_address[2] as i64 - internal_address[1] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 3/25", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd | reg32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[2], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("or").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[3]);
            let jump_address = internal_address[3] as i64 - internal_address[2] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 4/25", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // reg32 = rd >> 4
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[3], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 4, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[4]);
            let jump_address = internal_address[4] as i64 - internal_address[3] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 5/25", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd | reg32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[4], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("or").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[5]);
            let jump_address = internal_address[5] as i64 - internal_address[4] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 6/25", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // reg32 = rd >> 8
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[5], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 8, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[6]);
            let jump_address = internal_address[6] as i64 - internal_address[5] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 7/25", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd | reg32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[6], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("or").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[7]);
            let jump_address = internal_address[7] as i64 - internal_address[6] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 8/25", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // reg32 = rd >> 16
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[7], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 16, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[8]);
            let jump_address = internal_address[8] as i64 - internal_address[7] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 9/25", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd | reg32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[8], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("or").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[9]);
            let jump_address = internal_address[9] as i64 - internal_address[8] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 10/25", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // reg32 = rd >> 32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[9], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 32, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[10]);
            let jump_address = internal_address[10] as i64 - internal_address[9] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 11/25", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd | reg32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[10], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("or").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[11]);
            let jump_address = internal_address[11] as i64 - internal_address[10] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 12/25", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // x = x - ((x >> 1) & 0x5555555555555555)

        // reg32 = rd >> 1
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[11], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 1, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[12]);
            let jump_address = internal_address[12] as i64 - internal_address[11] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 13/25", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // reg32 = reg32 & 0x5555555555555555
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[12], i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("imm", 0x5555555555555555, false);
            zib.op("and").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[13]);
            let jump_address = internal_address[13] as i64 - internal_address[12] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 14/25", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd - reg32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[13], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("sub").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[14]);
            let jump_address = internal_address[14] as i64 - internal_address[13] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 15/25", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // x = (x & 0x3333333333333333) + ((x >> 2) & 0x3333333333333333)

        // reg32 = rd >> 2
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[14], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 2, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[15]);
            let jump_address = internal_address[15] as i64 - internal_address[14] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 16/25", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // reg32 = reg32 & 0x3333333333333333
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[15], i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("imm", 0x3333333333333333, false);
            zib.op("and").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[16]);
            let jump_address = internal_address[16] as i64 - internal_address[15] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 17/25", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd & 0x3333333333333333
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[16], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 0x3333333333333333, false);
            zib.op("and").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[17]);
            let jump_address = internal_address[17] as i64 - internal_address[16] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 18/25", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd + reg32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[17], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("add").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[18]);
            let jump_address = internal_address[18] as i64 - internal_address[17] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 19/25", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // x = (x + (x >> 4)) & 0x0F0F0F0F0F0F0F0F

        // reg32 = rd >> 4
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[18], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 4, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[19]);
            let jump_address = internal_address[19] as i64 - internal_address[18] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 20/25", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd + reg32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[19], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("add").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[20]);
            let jump_address = internal_address[20] as i64 - internal_address[19] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 21/25", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd & 0x0F0F0F0F0F0F0F0F
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[20], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 0x0F0F0F0F0F0F0F0F, false);
            zib.op("and").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[21]);
            let jump_address = internal_address[21] as i64 - internal_address[20] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 22/25", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // cnt = (x * 0x0101010101010101) >> 56

        // rd = rd * 0x0101010101010101
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[21], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 0x0101010101010101, false);
            zib.op("mul").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[22]);
            let jump_address = internal_address[22] as i64 - internal_address[21] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 23/25", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd >> 56
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[22], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 56, false);
            zib.op("srl").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[23]);
            let jump_address = internal_address[23] as i64 - internal_address[22] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 24/25", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // clz = 64 - popcount(x)

        // rd = 64 - rd
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[23], i.inst_name.to_string());
            zib.src_a("imm", 64, false);
            zib.src_b("reg", i.rd as u64, false);
            zib.op("sub").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address[23] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 25/25", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }
    }

    /// Implements ctz, which counts the number of trailing zeros in a 64-bit register
    //
    //  ctz(x) = popcount( ~x & (x − 1) )
    //
    // # build the trailing-zero mask  (scratch reg = r32, like clz)
    // r32 = rs1 - 1                          # sub, imm 1     (x=0 wraps to all-ones ✓)
    // rd  = rs1 XOR 0xFFFFFFFFFFFFFFFF        # xor  → ~x
    // rd  = rd & r32                          # and  → ~x & (x-1)
    //
    // # popcount(rd)  — identical to the clz popcount block
    // r32 = rd >> 1;  r32 &= 0x5555555555555555;  rd -= r32
    // r32 = rd >> 2;  r32 &= 0x3333333333333333;  rd &= 0x3333333333333333;  rd += r32
    // r32 = rd >> 4;  rd += r32;  rd &= 0x0F0F0F0F0F0F0F0F
    // rd  = rd * 0x0101010101010101
    // rd  = rd >> 56                          # ← final result, no "64 -" step
    //
    pub fn ctz(&mut self, i: &RiscvInst) {
        // Get addresses for the required instructions to implement this function
        let rom_address = i.rom_address;
        let mut internal_address = [0u64; 14];
        for i in 0..14 {
            internal_address[i] = self.rom.get_internal_address();
        }

        // r32 = rs1 - 1
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", 1, false);
            zib.op("sub").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[0]);
            let jump_address = internal_address[0] as i64 - rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 1/15", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rs1 XOR 0xFFFFFFFFFFFFFFFF
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[0], i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", 0xFFFFFFFFFFFFFFFF, false);
            zib.op("xor").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[1]);
            let jump_address = internal_address[1] as i64 - internal_address[0] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 2/15", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd & r32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[1], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("and").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[2]);
            let jump_address = internal_address[2] as i64 - internal_address[1] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 3/15", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // popcount block

        // r32 = rd >> 1
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[2], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 1, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[3]);
            let jump_address = internal_address[3] as i64 - internal_address[2] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 4/15", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // r32 = r32 & 0x5555555555555555
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[3], i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("imm", 0x5555555555555555, false);
            zib.op("and").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[4]);
            let jump_address = internal_address[4] as i64 - internal_address[3] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 5/15", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd - r32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[4], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("sub").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[5]);
            let jump_address = internal_address[5] as i64 - internal_address[4] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 6/15", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // r32 = rd >> 2
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[5], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 2, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[6]);
            let jump_address = internal_address[6] as i64 - internal_address[5] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 7/15", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // r32 = r32 & 0x3333333333333333
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[6], i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("imm", 0x3333333333333333, false);
            zib.op("and").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[7]);
            let jump_address = internal_address[7] as i64 - internal_address[6] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 8/15", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd & 0x3333333333333333
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[7], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 0x3333333333333333, false);
            zib.op("and").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[8]);
            let jump_address = internal_address[8] as i64 - internal_address[7] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 9/15", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd + r32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[8], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("add").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[9]);
            let jump_address = internal_address[9] as i64 - internal_address[8] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 10/15", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // r32 = rd >> 4
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[9], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 4, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[10]);
            let jump_address = internal_address[10] as i64 - internal_address[9] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 11/15", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd + r32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[10], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("add").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[11]);
            let jump_address = internal_address[11] as i64 - internal_address[10] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 12/15", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd & 0x0F0F0F0F0F0F0F0F
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[11], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 0x0F0F0F0F0F0F0F0F, false);
            zib.op("and").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[12]);
            let jump_address = internal_address[12] as i64 - internal_address[11] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 13/15", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd * 0x0101010101010101
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[12], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 0x0101010101010101, false);
            zib.op("mul").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[13]);
            let jump_address = internal_address[13] as i64 - internal_address[12] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 14/15", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd >> 56
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[13], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 56, false);
            zib.op("srl").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address[13] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 15/15", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }
    }

    /// Implements clz_w, which counts the number of leading zeros in a 32-bit register
    //
    // # 0. confine to the low 32 bits — upper bits MUST be zero before smearing
    // rd = rs1 & 0x00000000FFFFFFFF        # and
    //
    // # 1. smear-right within 32 bits (5 steps, NOT 6 — there is no >>32)
    // rd |= rd >> 1
    // rd |= rd >> 2
    // rd |= rd >> 4
    // rd |= rd >> 8
    // rd |= rd >> 16                        # rd is now 0…0 0…01…1, all bits MSB→0 set
    //
    // # 2. popcount(rd) — byte-for-byte identical to the clz popcount block
    // r32 = rd >> 1;  r32 &= 0x5555555555555555;  rd -= r32
    // r32 = rd >> 2;  r32 &= 0x3333333333333333;  rd &= 0x3333333333333333;  rd += r32
    // r32 = rd >> 4;  rd += r32;  rd &= 0x0F0F0F0F0F0F0F0F
    // rd  = rd * 0x0101010101010101
    // rd  = rd >> 56
    //
    // # 3. clz.w = 32 − popcount   (note: 32, not 64)
    // rd = 32 - rd                          # src_a imm 32, src_b reg rd, sub
    //
    pub fn clz_w(&mut self, i: &RiscvInst) {
        // Get addresses for the required instructions to implement this function
        let rom_address = i.rom_address;
        let mut internal_address = [0u64; 23];
        for i in 0..23 {
            internal_address[i] = self.rom.get_internal_address();
        }

        // reg32 = rs1 & 0x00000000FFFFFFFF
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", 0x00000000FFFFFFFF, false);
            zib.op("and").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[0]);
            let jump_address = internal_address[0] as i64 - rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 1/24", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // smear-right block

        // reg32 = reg32 >> 1 = (rs1 & 0x00000000FFFFFFFF) >> 1
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[0], i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("imm", 1, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[1]);
            let jump_address = internal_address[1] as i64 - internal_address[0] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 2/24", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd | reg32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[1], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("or").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[2]);
            let jump_address = internal_address[2] as i64 - internal_address[1] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 3/24", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // reg32 = rd >> 2
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[2], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 2, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[3]);
            let jump_address = internal_address[3] as i64 - internal_address[2] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 4/24", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd | reg32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[3], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("or").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[4]);
            let jump_address = internal_address[4] as i64 - internal_address[3] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 5/24", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // reg32 = rd >> 4
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[4], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 4, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[5]);
            let jump_address = internal_address[5] as i64 - internal_address[4] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 6/24", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd | reg32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[5], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("or").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[6]);
            let jump_address = internal_address[6] as i64 - internal_address[5] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 7/24", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // reg32 = rd >> 8
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[6], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 8, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[7]);
            let jump_address = internal_address[7] as i64 - internal_address[6] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 8/24", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd | reg32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[7], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("or").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[8]);
            let jump_address = internal_address[8] as i64 - internal_address[7] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 9/24", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // reg32 = rd >> 16
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[8], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 16, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[9]);
            let jump_address = internal_address[9] as i64 - internal_address[8] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 10/24", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd | reg32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[9], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("or").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[10]);
            let jump_address = internal_address[10] as i64 - internal_address[9] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 11/24", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // popcount block

        // r32 = rd >> 1
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[10], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 1, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[11]);
            let jump_address = internal_address[11] as i64 - internal_address[10] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 12/24", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // r32 = r32 & 0x5555555555555555
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[11], i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("imm", 0x5555555555555555, false);
            zib.op("and").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[12]);
            let jump_address = internal_address[12] as i64 - internal_address[11] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 13/24", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd - r32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[12], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("sub").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[13]);
            let jump_address = internal_address[13] as i64 - internal_address[12] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 14/24", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // r32 = rd >> 2
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[13], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 2, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[14]);
            let jump_address = internal_address[14] as i64 - internal_address[13] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 15/24", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // r32 = r32 & 0x3333333333333333
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[14], i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("imm", 0x3333333333333333, false);
            zib.op("and").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[15]);
            let jump_address = internal_address[15] as i64 - internal_address[14] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 16/24", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd & 0x3333333333333333
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[15], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 0x3333333333333333, false);
            zib.op("and").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[16]);
            let jump_address = internal_address[16] as i64 - internal_address[15] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 17/24", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd + r32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[16], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("add").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[17]);
            let jump_address = internal_address[17] as i64 - internal_address[16] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 18/24", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // r32 = rd >> 4
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[17], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 4, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[18]);
            let jump_address = internal_address[18] as i64 - internal_address[17] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 19/24", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd + r32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[18], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("add").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[19]);
            let jump_address = internal_address[19] as i64 - internal_address[18] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 20/24", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd & 0x0F0F0F0F0F0F0F0F
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[19], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 0x0F0F0F0F0F0F0F0F, false);
            zib.op("and").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[20]);
            let jump_address = internal_address[20] as i64 - internal_address[19] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 21/24", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd * 0x0101010101010101
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[20], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 0x0101010101010101, false);
            zib.op("mul").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[21]);
            let jump_address = internal_address[21] as i64 - internal_address[20] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 22/24", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd >> 56
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[21], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 56, false);
            zib.op("srl").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[22]);
            let jump_address = internal_address[22] as i64 - internal_address[21] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 23/24", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = 32 - rd
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[22], i.inst_name.to_string());
            zib.src_a("imm", 32, false);
            zib.src_b("reg", i.rd as u64, false);
            zib.op("sub").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address[22] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 24/24", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }
    }

    /// Implements ctz_w, which counts the number of trailing zeros in a 32-bit register
    //
    // # 0. plant sentinel at bit 32 so a zero low-word yields 32 (upper garbage is bounded out for free)
    // rd  = rs1 | 0x0000000100000000        # or
    //
    // # 1. build the trailing-zero mask  ~rd & (rd - 1)   (r32 = scratch)
    // r32 = rd - 1                          # sub, imm 1
    // rd  = rd XOR 0xFFFFFFFFFFFFFFFF        # xor  → ~rd
    // rd  = rd & r32                         # and  → ~rd & (rd-1)
    //
    // # 2. popcount(rd) — identical to the clz/ctz popcount block
    // r32 = rd >> 1;  r32 &= 0x5555555555555555;  rd -= r32
    // r32 = rd >> 2;  r32 &= 0x3333333333333333;  rd &= 0x3333333333333333;  rd += r32
    // r32 = rd >> 4;  rd += r32;  rd &= 0x0F0F0F0F0F0F0F0F
    // rd  = rd * 0x0101010101010101
    // rd  = rd >> 56                         # ← result; no "N -" correction step
    //
    pub fn ctz_w(&mut self, i: &RiscvInst) {
        // Get addresses for the required instructions to implement this function
        let rom_address = i.rom_address;
        let mut internal_address = [0u64; 15];
        for i in 0..15 {
            internal_address[i] = self.rom.get_internal_address();
        }

        // rd = rs1 | 0x0000000100000000
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", 0x0000000100000000, false);
            zib.op("or").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[0]);
            let jump_address = internal_address[0] as i64 - rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 1/16", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // build the trailing-zero mask block

        // r32 = rd - 1
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[0], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 1, false);
            zib.op("sub").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[1]);
            let jump_address = internal_address[1] as i64 - internal_address[0] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 2/16", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd XOR 0xFFFFFFFFFFFFFFFF
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[1], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 0xFFFFFFFFFFFFFFFF, false);
            zib.op("xor").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[2]);
            let jump_address = internal_address[2] as i64 - internal_address[1] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 3/16", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd & r32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[2], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("and").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[3]);
            let jump_address = internal_address[3] as i64 - internal_address[2] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 4/16", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // popcount block

        // r32 = rd >> 1
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[3], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 1, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[4]);
            let jump_address = internal_address[4] as i64 - internal_address[3] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 5/16", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // r32 = r32 & 0x5555555555555555
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[4], i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("imm", 0x5555555555555555, false);
            zib.op("and").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[5]);
            let jump_address = internal_address[5] as i64 - internal_address[4] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 6/16", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd - r32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[5], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("sub").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[6]);
            let jump_address = internal_address[6] as i64 - internal_address[5] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 7/16", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // r32 = rd >> 2
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[6], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 2, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[7]);
            let jump_address = internal_address[7] as i64 - internal_address[6] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 8/16", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // r32 = r32 & 0x3333333333333333
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[7], i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("imm", 0x3333333333333333, false);
            zib.op("and").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[8]);
            let jump_address = internal_address[8] as i64 - internal_address[7] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 9/16", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd & 0x3333333333333333
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[8], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 0x3333333333333333, false);
            zib.op("and").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[9]);
            let jump_address = internal_address[9] as i64 - internal_address[8] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 10/16", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd + r32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[9], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("add").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[10]);
            let jump_address = internal_address[10] as i64 - internal_address[9] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 11/16", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // r32 = rd >> 4
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[10], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 4, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[11]);
            let jump_address = internal_address[11] as i64 - internal_address[10] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 12/16", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd + r32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[11], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("add").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[12]);
            let jump_address = internal_address[12] as i64 - internal_address[11] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 13/16", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd & 0x0F0F0F0F0F0F0F0F
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[12], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 0x0F0F0F0F0F0F0F0F, false);
            zib.op("and").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[13]);
            let jump_address = internal_address[13] as i64 - internal_address[12] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 14/16", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd * 0x0101010101010101
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[13], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 0x0101010101010101, false);
            zib.op("mul").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[14]);
            let jump_address = internal_address[14] as i64 - internal_address[13] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 15/16", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd >> 56
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[14], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 56, false);
            zib.op("srl").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address[14] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 16/16", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }
    }

    /// Implements cpop, which counts the number of set bits (population count) in the source
    /// register.
    //
    //   x  = rs1
    //   x  = x - ((x >> 1) & 0x5555555555555555)
    //   x  = (x & 0x3333333333333333) + ((x >> 2) & 0x3333333333333333)
    //   x  = (x + (x >> 4)) & 0x0F0F0F0F0F0F0F0F
    //   rd = (x * 0x0101010101010101) >> 56
    //
    // 12 ZisK instructions (scratch = reg 32).
    //
    pub fn cpop(&mut self, i: &RiscvInst) {
        let rom_address = i.rom_address;
        let mut internal_address = [0u64; 11];
        for k in 0..11 {
            internal_address[k] = self.rom.get_internal_address();
        }

        // r32 = rs1 >> 1
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", 1, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[0]);
            let jump_address = internal_address[0] as i64 - rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 1/12", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // r32 = r32 & 0x5555555555555555
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[0], i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("imm", 0x5555555555555555, false);
            zib.op("and").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[1]);
            let jump_address = internal_address[1] as i64 - internal_address[0] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 2/12", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rs1 - r32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[1], i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("sub").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[2]);
            let jump_address = internal_address[2] as i64 - internal_address[1] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 3/12", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // r32 = rd >> 2
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[2], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 2, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[3]);
            let jump_address = internal_address[3] as i64 - internal_address[2] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 4/12", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // r32 = r32 & 0x3333333333333333
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[3], i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("imm", 0x3333333333333333, false);
            zib.op("and").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[4]);
            let jump_address = internal_address[4] as i64 - internal_address[3] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 5/12", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd & 0x3333333333333333
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[4], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 0x3333333333333333, false);
            zib.op("and").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[5]);
            let jump_address = internal_address[5] as i64 - internal_address[4] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 6/12", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd + r32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[5], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("add").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[6]);
            let jump_address = internal_address[6] as i64 - internal_address[5] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 7/12", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // r32 = rd >> 4
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[6], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 4, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[7]);
            let jump_address = internal_address[7] as i64 - internal_address[6] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 8/12", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd + r32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[7], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("add").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[8]);
            let jump_address = internal_address[8] as i64 - internal_address[7] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 9/12", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd & 0x0F0F0F0F0F0F0F0F
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[8], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 0x0F0F0F0F0F0F0F0F, false);
            zib.op("and").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[9]);
            let jump_address = internal_address[9] as i64 - internal_address[8] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 10/12", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd * 0x0101010101010101
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[9], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 0x0101010101010101, false);
            zib.op("mul").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[10]);
            let jump_address = internal_address[10] as i64 - internal_address[9] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 11/12", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd >> 56
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[10], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 56, false);
            zib.op("srl").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address[10] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 12/12", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }
    }

    /// Implements cpop_w, which counts the number of set bits (population count) in the lower 32 bits
    /// of the source register.
    //
    // Mask to 32 bits, then SWAR / parallel popcount (Hacker's Delight):
    //   x  = rs1 & 0xFFFFFFFF
    //   x  = x - ((x >> 1) & 0x5555555555555555)
    //   x  = (x & 0x3333333333333333) + ((x >> 2) & 0x3333333333333333)
    //   x  = (x + (x >> 4)) & 0x0F0F0F0F0F0F0F0F
    //   rd = (x * 0x0101010101010101) >> 56
    //
    // 13 ZisK instructions (scratch = reg 32).
    pub fn cpop_w(&mut self, i: &RiscvInst) {
        let rom_address = i.rom_address;
        let mut internal_address = [0u64; 12];
        for k in 0..12 {
            internal_address[k] = self.rom.get_internal_address();
        }

        // rd = rs1 & 0xFFFFFFFF
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", 0xFFFFFFFF, false);
            zib.op("and").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[0]);
            let jump_address = internal_address[0] as i64 - rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 1/13", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // r32 = rd >> 1
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[0], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 1, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[1]);
            let jump_address = internal_address[1] as i64 - internal_address[0] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 2/13", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // r32 = r32 & 0x5555555555555555
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[1], i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("imm", 0x5555555555555555, false);
            zib.op("and").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[2]);
            let jump_address = internal_address[2] as i64 - internal_address[1] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 3/13", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd - r32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[2], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("sub").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[3]);
            let jump_address = internal_address[3] as i64 - internal_address[2] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 4/13", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // r32 = rd >> 2
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[3], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 2, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[4]);
            let jump_address = internal_address[4] as i64 - internal_address[3] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 5/13", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // r32 = r32 & 0x3333333333333333
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[4], i.inst_name.to_string());
            zib.src_a("reg", 32, false);
            zib.src_b("imm", 0x3333333333333333, false);
            zib.op("and").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[5]);
            let jump_address = internal_address[5] as i64 - internal_address[4] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 6/13", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd & 0x3333333333333333
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[5], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 0x3333333333333333, false);
            zib.op("and").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[6]);
            let jump_address = internal_address[6] as i64 - internal_address[5] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 7/13", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd + r32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[6], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("add").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[7]);
            let jump_address = internal_address[7] as i64 - internal_address[6] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 8/13", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // r32 = rd >> 4
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[7], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 4, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[8]);
            let jump_address = internal_address[8] as i64 - internal_address[7] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 9/13", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd + r32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[8], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("add").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[9]);
            let jump_address = internal_address[9] as i64 - internal_address[8] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 10/13", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd & 0x0F0F0F0F0F0F0F0F
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[9], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 0x0F0F0F0F0F0F0F0F, false);
            zib.op("and").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[10]);
            let jump_address = internal_address[10] as i64 - internal_address[9] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 11/13", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd * 0x0101010101010101
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[10], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 0x0101010101010101, false);
            zib.op("mul").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[11]);
            let jump_address = internal_address[11] as i64 - internal_address[10] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 12/13", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd >> 56
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[11], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 56, false);
            zib.op("srl").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address[11] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 13/13", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }
    }

    /// Implements orc_b, which sets each output byte to 0xFF if the corresponding source byte is
    /// nonzero, or 0x00 otherwise (bitwise OR-combine within each byte).
    //
    // SWAR, with M = 0x7F7F7F7F7F7F7F7F, H = 0x8080808080808080:
    //   t  = rs1 & M
    //   t  = t + M          # bit7 of each byte set iff low 7 bits nonzero (no inter-byte carry)
    //   t  = t | rs1        # include original bit7 → bit7 set iff byte nonzero
    //   t  = t & H          # 0x80 per nonzero byte
    //   s  = t >> 7         # 0x01 per nonzero byte
    //   s  = t - s          # 0x7F per nonzero byte (no inter-byte borrow)
    //   rd = t | s          # 0xFF per nonzero byte
    //
    // 7 ZisK instructions (scratch = reg 32).
    pub fn orc_b(&mut self, i: &RiscvInst) {
        const M: u64 = 0x7F7F7F7F7F7F7F7F;
        const H: u64 = 0x8080808080808080;

        let rom_address = i.rom_address;
        let mut internal_address = [0u64; 6];
        for k in 0..6 {
            internal_address[k] = self.rom.get_internal_address();
        }

        // rd = rs1 & M
        {
            let mut zib = ZiskInstBuilder::new_from_riscv(rom_address, i.inst_name.to_string());
            zib.src_a("reg", i.rs1 as u64, false);
            zib.src_b("imm", M, false);
            zib.op("and").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[0]);
            let jump_address = internal_address[0] as i64 - rom_address as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 1/7", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd + M
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[0], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", M, false);
            zib.op("add").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[1]);
            let jump_address = internal_address[1] as i64 - internal_address[0] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 2/7", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd | rs1
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[1], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", i.rs1 as u64, false);
            zib.op("or").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[2]);
            let jump_address = internal_address[2] as i64 - internal_address[1] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 3/7", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd & H
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[2], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", H, false);
            zib.op("and").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            zib.set_next_internal_address(internal_address[3]);
            let jump_address = internal_address[3] as i64 - internal_address[2] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 4/7", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // r32 = rd >> 7
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[3], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("imm", 7, false);
            zib.op("srl").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[4]);
            let jump_address = internal_address[4] as i64 - internal_address[3] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 5/7", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // r32 = rd - r32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[4], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("sub").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(internal_address[5]);
            let jump_address = internal_address[5] as i64 - internal_address[4] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 6/7", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }

        // rd = rd | r32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(internal_address[5], i.inst_name.to_string());
            zib.src_a("reg", i.rd as u64, false);
            zib.src_b("reg", 32, false);
            zib.op("or").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - internal_address[5] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} 7/7", i.inst_name, i.rd, i.rs1));
            zib.build(self.rom);
        }
    }

    /// Implements clmul, the low 64 bits of the carry-less product of rs1 and rs2.
    //
    //   clmul(a,b) = XOR over i in 0..64 of (a << i) for each set bit i of b
    //
    // Branchless, 5 ZisK ops per bit (sll, sra, sll, and, xor) using sra to build a
    // per-bit all-ones/all-zeros mask, plus one instruction to zero rd:
    //   1 + 64*5 = 321 ZisK instructions.  scratch = regs 32 (mask) and 33 (term).
    //
    // rd = 0
    // for i in 0..64:
    //     m  = rs2 << (63 - i)     # sll  — move bit i to bit 63
    //     m  = m sra 63            # sra  — 0xFFFF..FF if bit i set, else 0
    //     t  = rs1 << i            # sll
    //     t  = t & m               # and
    //     rd = rd ^ t              # xor
    //
    pub fn clmul(&mut self, i: &RiscvInst) {
        let rom_address = i.rom_address;

        const N: usize = 1 + 64 * 5; // 321
        let mut ia = [0u64; N - 1];
        for k in 0..(N - 1) {
            ia[k] = self.rom.get_internal_address();
        }
        // Address of instruction k: rom_address for k==0, else ia[k-1].
        let addr = |k: usize| if k == 0 { rom_address } else { ia[k - 1] };

        for k in 0..N {
            let this = addr(k);
            let mut zib = ZiskInstBuilder::new_from_riscv(this, i.inst_name.to_string());

            if k == 0 {
                // rd = rs1 & 0  → 0
                zib.src_a("reg", i.rs1 as u64, false);
                zib.src_b("imm", 0, false);
                zib.op("and").unwrap();
                zib.store("reg", i.rd as i64, false, false);
            } else {
                let j = k - 1; // 0..320
                let bit = (j / 5) as u64; // 0..63
                match j % 5 {
                    0 => {
                        // reg32 = rs2 << (63 - bit)
                        zib.src_a("reg", i.rs2 as u64, false);
                        zib.src_b("imm", 63 - bit, false);
                        zib.op("sll").unwrap();
                        zib.store("reg", 32, false, false);
                    }
                    1 => {
                        // reg32 = reg32 sra 63  → all-ones iff bit set
                        zib.src_a("reg", 32, false);
                        zib.src_b("imm", 63, false);
                        zib.op("sra").unwrap();
                        zib.store("reg", 32, false, false);
                    }
                    2 => {
                        // reg33 = rs1 << bit
                        zib.src_a("reg", i.rs1 as u64, false);
                        zib.src_b("imm", bit, false);
                        zib.op("sll").unwrap();
                        zib.store("reg", 33, false, false);
                    }
                    3 => {
                        // reg33 = reg33 & reg32
                        zib.src_a("reg", 33, false);
                        zib.src_b("reg", 32, false);
                        zib.op("and").unwrap();
                        zib.store("reg", 33, false, false);
                    }
                    _ => {
                        // rd = rd ^ reg33
                        zib.src_a("reg", i.rd as u64, false);
                        zib.src_b("reg", 33, false);
                        zib.op("xor").unwrap();
                        zib.store("reg", i.rd as i64, false, false);
                    }
                }
            }

            let next = if k + 1 < N { addr(k + 1) } else { rom_address + 4 };
            if k + 1 < N {
                zib.set_next_internal_address(next);
            }
            let jump_address = next as i64 - this as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!(
                "{} r{}, r{}, r{} {}/{}",
                i.inst_name,
                i.rd,
                i.rs1,
                i.rs2,
                k + 1,
                N
            ));
            zib.build(self.rom);
        }
    }

    /// Implements clmul_h, the high 64 bits of the carry-less product of rs1 and rs2.
    //
    //   clmulh(a,b) = XOR over i in 1..64 of (a >> (64 - i)) for each set bit i of b
    //
    // Branchless, 5 ZisK ops per bit (sll, sra, srl, and, xor): sll+sra build a
    // per-bit all-ones/all-zeros mask, srl is the (logical) partial product term,
    // plus one instruction to zero rd:  1 + 63*5 = 316 ZisK instructions.
    // scratch = regs 32 (mask) and 33 (term).
    pub fn clmul_h(&mut self, i: &RiscvInst) {
        let rom_address = i.rom_address;

        const N: usize = 1 + 63 * 5; // 316
        let mut ia = [0u64; N - 1];
        for k in 0..(N - 1) {
            ia[k] = self.rom.get_internal_address();
        }
        // Address of instruction k: rom_address for k==0, else ia[k-1].
        let addr = |k: usize| if k == 0 { rom_address } else { ia[k - 1] };

        for k in 0..N {
            let this = addr(k);
            let mut zib = ZiskInstBuilder::new_from_riscv(this, i.inst_name.to_string());

            if k == 0 {
                // rd = rs1 & 0  → 0
                zib.src_a("reg", i.rs1 as u64, false);
                zib.src_b("imm", 0, false);
                zib.op("and").unwrap();
                zib.store("reg", i.rd as i64, false, false);
            } else {
                let j = k - 1; // 0..314
                let bit = (j / 5) as u64 + 1; // 1..63
                match j % 5 {
                    0 => {
                        // reg32 = rs2 << (63 - bit)
                        zib.src_a("reg", i.rs2 as u64, false);
                        zib.src_b("imm", 63 - bit, false);
                        zib.op("sll").unwrap();
                        zib.store("reg", 32, false, false);
                    }
                    1 => {
                        // reg32 = reg32 sra 63  → all-ones iff bit set
                        zib.src_a("reg", 32, false);
                        zib.src_b("imm", 63, false);
                        zib.op("sra").unwrap();
                        zib.store("reg", 32, false, false);
                    }
                    2 => {
                        // reg33 = rs1 >> (64 - bit)   (logical)
                        zib.src_a("reg", i.rs1 as u64, false);
                        zib.src_b("imm", 64 - bit, false);
                        zib.op("srl").unwrap();
                        zib.store("reg", 33, false, false);
                    }
                    3 => {
                        // reg33 = reg33 & reg32
                        zib.src_a("reg", 33, false);
                        zib.src_b("reg", 32, false);
                        zib.op("and").unwrap();
                        zib.store("reg", 33, false, false);
                    }
                    _ => {
                        // rd = rd ^ reg33
                        zib.src_a("reg", i.rd as u64, false);
                        zib.src_b("reg", 33, false);
                        zib.op("xor").unwrap();
                        zib.store("reg", i.rd as i64, false, false);
                    }
                }
            }

            let next = if k + 1 < N { addr(k + 1) } else { rom_address + 4 };
            if k + 1 < N {
                zib.set_next_internal_address(next);
            }
            let jump_address = next as i64 - this as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!(
                "{} r{}, r{}, r{} {}/{}",
                i.inst_name,
                i.rd,
                i.rs1,
                i.rs2,
                k + 1,
                N
            ));
            zib.build(self.rom);
        }
    }

    /// Implements clmul_r, the "reversed" carry-less product of rs1 and rs2
    /// (bits 63..126 of the full 127-bit product).
    //
    //   clmulr(a,b) = XOR over i in 0..64 of (a >> (63 - i)) for each set bit i of b
    //
    // Branchless, 5 ZisK ops per bit (sll, sra, srl, and, xor): sll+sra build a
    // per-bit all-ones/all-zeros mask, srl is the (logical) partial product term,
    // plus one instruction to zero rd:  1 + 64*5 = 321 ZisK instructions.
    // scratch = regs 32 (mask) and 33 (term).
    pub fn clmul_r(&mut self, i: &RiscvInst) {
        let rom_address = i.rom_address;

        const N: usize = 1 + 64 * 5; // 321
        let mut ia = [0u64; N - 1];
        for k in 0..(N - 1) {
            ia[k] = self.rom.get_internal_address();
        }
        // Address of instruction k: rom_address for k==0, else ia[k-1].
        let addr = |k: usize| if k == 0 { rom_address } else { ia[k - 1] };

        for k in 0..N {
            let this = addr(k);
            let mut zib = ZiskInstBuilder::new_from_riscv(this, i.inst_name.to_string());

            if k == 0 {
                // rd = rs1 & 0  → 0
                zib.src_a("reg", i.rs1 as u64, false);
                zib.src_b("imm", 0, false);
                zib.op("and").unwrap();
                zib.store("reg", i.rd as i64, false, false);
            } else {
                let j = k - 1; // 0..319
                let bit = (j / 5) as u64; // 0..63
                match j % 5 {
                    0 => {
                        // reg32 = rs2 << (63 - bit)
                        zib.src_a("reg", i.rs2 as u64, false);
                        zib.src_b("imm", 63 - bit, false);
                        zib.op("sll").unwrap();
                        zib.store("reg", 32, false, false);
                    }
                    1 => {
                        // reg32 = reg32 sra 63  → all-ones iff bit set
                        zib.src_a("reg", 32, false);
                        zib.src_b("imm", 63, false);
                        zib.op("sra").unwrap();
                        zib.store("reg", 32, false, false);
                    }
                    2 => {
                        // reg33 = rs1 >> (63 - bit)   (logical)
                        zib.src_a("reg", i.rs1 as u64, false);
                        zib.src_b("imm", 63 - bit, false);
                        zib.op("srl").unwrap();
                        zib.store("reg", 33, false, false);
                    }
                    3 => {
                        // reg33 = reg33 & reg32
                        zib.src_a("reg", 33, false);
                        zib.src_b("reg", 32, false);
                        zib.op("and").unwrap();
                        zib.store("reg", 33, false, false);
                    }
                    _ => {
                        // rd = rd ^ reg33
                        zib.src_a("reg", i.rd as u64, false);
                        zib.src_b("reg", 33, false);
                        zib.op("xor").unwrap();
                        zib.store("reg", i.rd as i64, false, false);
                    }
                }
            }

            let next = if k + 1 < N { addr(k + 1) } else { rom_address + 4 };
            if k + 1 < N {
                zib.set_next_internal_address(next);
            }
            let jump_address = next as i64 - this as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!(
                "{} r{}, r{}, r{} {}/{}",
                i.inst_name,
                i.rd,
                i.rs1,
                i.rs2,
                k + 1,
                N
            ));
            zib.build(self.rom);
        }
    }

    /// Implements xperm4, a nibble crossbar: each nibble of rs2 indexes a nibble of rs1.
    //
    //   for j in 0..16:
    //       idx = (rs2 >> (4*j)) & 0xF        # 0..15  (always in range on RV64)
    //       nib = (rs1 >> (4*idx)) & 0xF      # selected nibble
    //       rd |= nib << (4*j)
    //
    // Per output nibble (7 ops): srl, and, sll (→ 4*idx), srl(reg, data-dependent),
    // and, sll (→ place), or.  Plus one instruction to zero the accumulator and one to
    // copy it into rd:
    //   1 + 16*7 + 1 = 114 ZisK instructions.  scratch = reg 32 (accumulator),
    //   reg 33 (shift amount) and reg 34 (nibble).  rd is written only at the end so it
    //   may safely alias rs1 or rs2.
    //
    //   reg32 = 0
    //   for i in 0..16
    //      reg33 = rs2 >> (4*i) (if i!=0, else reg33 = rs2)
    //      reg33 = reg33 & 0xF
    //      reg33 = reg33 << 2 = 4*reg33 = number of bits to shift rs1 to select the nibble
    //      reg34 = rs1 >> reg33
    //      reg34 = reg34 & 0xF
    //      reg34 = reg34 << (4*i) (if i!=0, else nothing)
    //      r32 = r32 | reg34
    //   rd = r32
    //
    pub fn xperm4(&mut self, i: &RiscvInst) {
        // The first instruction is at rom_address, the rest are internal addresses
        let rom_address = i.rom_address;

        // Calculate the number of ZisK instructions needed for xperm4
        const N: usize = 1 + 16 * 7 + 1; // 114

        // Address of instruction k: rom_address for k==0, else internal address
        let mut addr = [0u64; N];
        for k in 0..N {
            addr[k] = if k == 0 { rom_address } else { self.rom.get_internal_address() };
        }
        let mut addr_index = 0;

        // Reset reg32

        // r32 = 0
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(addr[addr_index], i.inst_name.to_string());
            zib.src_a("imm", 0, false);
            zib.src_b("imm", 0, false);
            zib.op("copyb").unwrap();
            zib.store("reg", 32, false, false);
            zib.set_next_internal_address(addr[addr_index + 1]);
            let jump_address = addr[addr_index + 1] as i64 - addr[addr_index] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} {}/{}", i.inst_name, i.rd, i.rs1, addr_index + 1, N));
            zib.build(self.rom);
            addr_index += 1;
        }

        for j in 0..16 {
            // reg33 = rs2 >> (4*i) (if i!=0, else reg33 = rs2)
            {
                let mut zib =
                    ZiskInstBuilder::new_from_riscv(addr[addr_index], i.inst_name.to_string());
                zib.src_a("reg", i.rs2 as u64, false);
                zib.src_b("imm", j * 4, false);
                zib.op("srl").unwrap();
                zib.store("reg", 33, false, false);
                zib.set_next_internal_address(addr[addr_index + 1]);
                let jump_address = addr[addr_index + 1] as i64 - addr[addr_index] as i64;
                zib.j(jump_address, jump_address);
                zib.verbose(&format!(
                    "{} r{}, r{} {}/{}",
                    i.inst_name,
                    i.rd,
                    i.rs1,
                    addr_index + 1,
                    N
                ));
                zib.build(self.rom);
                addr_index += 1;
            }

            // reg33 = reg33 & 0xF
            {
                let mut zib =
                    ZiskInstBuilder::new_from_riscv(addr[addr_index], i.inst_name.to_string());
                zib.src_a("reg", 33, false);
                zib.src_b("imm", 0xF, false);
                zib.op("and").unwrap();
                zib.store("reg", 33, false, false);
                zib.set_next_internal_address(addr[addr_index + 1]);
                let jump_address = addr[addr_index + 1] as i64 - addr[addr_index] as i64;
                zib.j(jump_address, jump_address);
                zib.verbose(&format!(
                    "{} r{}, r{} {}/{}",
                    i.inst_name,
                    i.rd,
                    i.rs1,
                    addr_index + 1,
                    N
                ));
                zib.build(self.rom);
                addr_index += 1;
            }

            // reg33 = reg33 << 2 = 4*reg33 = number of bits to shift rs1 to select the nibble
            {
                let mut zib =
                    ZiskInstBuilder::new_from_riscv(addr[addr_index], i.inst_name.to_string());
                zib.src_a("reg", 33, false);
                zib.src_b("imm", 2, false);
                zib.op("sll").unwrap();
                zib.store("reg", 33, false, false);
                zib.set_next_internal_address(addr[addr_index + 1]);
                let jump_address = addr[addr_index + 1] as i64 - addr[addr_index] as i64;
                zib.j(jump_address, jump_address);
                zib.verbose(&format!(
                    "{} r{}, r{} {}/{}",
                    i.inst_name,
                    i.rd,
                    i.rs1,
                    addr_index + 1,
                    N
                ));
                zib.build(self.rom);
                addr_index += 1;
            }

            // reg34 = rs1 >> reg33
            {
                let mut zib =
                    ZiskInstBuilder::new_from_riscv(addr[addr_index], i.inst_name.to_string());
                zib.src_a("reg", i.rs1 as u64, false);
                zib.src_b("reg", 33, false);
                zib.op("srl").unwrap();
                zib.store("reg", 34, false, false);
                zib.set_next_internal_address(addr[addr_index + 1]);
                let jump_address = addr[addr_index + 1] as i64 - addr[addr_index] as i64;
                zib.j(jump_address, jump_address);
                zib.verbose(&format!(
                    "{} r{}, r{} {}/{}",
                    i.inst_name,
                    i.rd,
                    i.rs1,
                    addr_index + 1,
                    N
                ));
                zib.build(self.rom);
                addr_index += 1;
            }

            // reg34 = reg34 & 0xF
            {
                let mut zib =
                    ZiskInstBuilder::new_from_riscv(addr[addr_index], i.inst_name.to_string());
                zib.src_a("reg", 34, false);
                zib.src_b("imm", 0xF, false);
                zib.op("and").unwrap();
                zib.store("reg", 34, false, false);
                zib.set_next_internal_address(addr[addr_index + 1]);
                let jump_address = addr[addr_index + 1] as i64 - addr[addr_index] as i64;
                zib.j(jump_address, jump_address);
                zib.verbose(&format!(
                    "{} r{}, r{} {}/{}",
                    i.inst_name,
                    i.rd,
                    i.rs1 as u64,
                    addr_index + 1,
                    N
                ));
                zib.build(self.rom);
                addr_index += 1;
            }

            // reg34 = reg34 << (4*i) (if i!=0, else nothing)
            {
                let mut zib =
                    ZiskInstBuilder::new_from_riscv(addr[addr_index], i.inst_name.to_string());
                zib.src_a("reg", 34, false);
                zib.src_b("imm", 4 * j, false);
                zib.op("sll").unwrap();
                zib.store("reg", 34, false, false);
                zib.set_next_internal_address(addr[addr_index + 1]);
                let jump_address = addr[addr_index + 1] as i64 - addr[addr_index] as i64;
                zib.j(jump_address, jump_address);
                zib.verbose(&format!(
                    "{} r{}, r{} {}/{}",
                    i.inst_name,
                    i.rd,
                    i.rs1,
                    addr_index + 1,
                    N
                ));
                zib.build(self.rom);
                addr_index += 1;
            }

            // r32 = r32 | reg34 (if i==0, just copy)
            {
                let mut zib =
                    ZiskInstBuilder::new_from_riscv(addr[addr_index], i.inst_name.to_string());
                zib.src_a("reg", 32, false);
                zib.src_b("reg", 34, false);
                zib.op("or").unwrap();
                zib.store("reg", 32, false, false);
                zib.set_next_internal_address(addr[addr_index + 1]);
                let jump_address = addr[addr_index + 1] as i64 - addr[addr_index] as i64;
                zib.j(jump_address, jump_address);
                zib.verbose(&format!(
                    "{} r{}, r{} {}/{}",
                    i.inst_name,
                    i.rd,
                    i.rs1 as u64,
                    addr_index + 1,
                    N
                ));
                zib.build(self.rom);
                addr_index += 1;
            }
        }
        // rd = r32
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(addr[addr_index], i.inst_name.to_string());
            // copyb stores src_b into c, so the accumulator must be in src_b (not src_a).
            zib.src_a("imm", 0, false);
            zib.src_b("reg", 32, false);
            zib.op("copyb").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - addr[addr_index] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} {}/{}", i.inst_name, i.rd, i.rs1, addr_index + 1, N));
            zib.build(self.rom);
        }
    }

    /// Implements xperm8, a byte crossbar: each byte of rs2 indexes a byte of rs1.
    /// On RV64, byte indices >= 8 are out of range and produce a zero byte.
    //
    //   for j in 0..8:
    //       idx  = (rs2 >> (8*j)) & 0xFF
    //       byte = (idx < 8) ? ((rs1 >> (8*idx)) & 0xFF) : 0
    //       rd  |= byte << (8*j)
    //
    // Range check is branchless: mask = ((idx & 0xF8) - 1) sra 63  → all-ones iff idx < 8.
    // 1 to zero the accumulator + 11 ops per byte + 1 to copy accumulator into rd
    // = 1 + 8*11 + 1 = 90 ZisK instructions.
    // scratch = reg 32 (index / shift-amount / byte), reg 33 (range mask) and reg 34
    // (accumulator).  rd is written only at the end so it may safely alias rs1 or rs2.
    //
    pub fn xperm8(&mut self, i: &RiscvInst) {
        // The first instruction is at rom_address, the rest are internal addresses
        let rom_address = i.rom_address;

        // Calculate the number of ZisK instructions needed for xperm8
        const N: usize = 1 + 8 * 11 + 1; // 90

        // Address of instruction k: rom_address for k==0, else internal address
        let mut addr = [0u64; N];
        for k in 0..N {
            addr[k] = if k == 0 { rom_address } else { self.rom.get_internal_address() };
        }
        let mut addr_index = 0;

        // reg34 = 0 (accumulator; copyb stores src_b into c)
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(addr[addr_index], i.inst_name.to_string());
            zib.src_a("imm", 0, false);
            zib.src_b("imm", 0, false);
            zib.op("copyb").unwrap();
            zib.store("reg", 34, false, false);
            zib.set_next_internal_address(addr[addr_index + 1]);
            let jump_address = addr[addr_index + 1] as i64 - addr[addr_index] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} {}/{}", i.inst_name, i.rd, i.rs1, addr_index + 1, N));
            zib.build(self.rom);
            addr_index += 1;
        }

        for j in 0..8 {
            // reg32 = rs2 >> (8*j)   (idx in low byte, garbage above)
            {
                let mut zib =
                    ZiskInstBuilder::new_from_riscv(addr[addr_index], i.inst_name.to_string());
                zib.src_a("reg", i.rs2 as u64, false);
                zib.src_b("imm", 8 * j, false);
                zib.op("srl").unwrap();
                zib.store("reg", 32, false, false);
                zib.set_next_internal_address(addr[addr_index + 1]);
                let jump_address = addr[addr_index + 1] as i64 - addr[addr_index] as i64;
                zib.j(jump_address, jump_address);
                zib.verbose(&format!(
                    "{} r{}, r{} {}/{}",
                    i.inst_name,
                    i.rd,
                    i.rs1,
                    addr_index + 1,
                    N
                ));
                zib.build(self.rom);
                addr_index += 1;
            }

            // reg33 = reg32 & 0xF8   (0 iff idx < 8)
            {
                let mut zib =
                    ZiskInstBuilder::new_from_riscv(addr[addr_index], i.inst_name.to_string());
                zib.src_a("reg", 32, false);
                zib.src_b("imm", 0xF8, false);
                zib.op("and").unwrap();
                zib.store("reg", 33, false, false);
                zib.set_next_internal_address(addr[addr_index + 1]);
                let jump_address = addr[addr_index + 1] as i64 - addr[addr_index] as i64;
                zib.j(jump_address, jump_address);
                zib.verbose(&format!(
                    "{} r{}, r{} {}/{}",
                    i.inst_name,
                    i.rd,
                    i.rs1,
                    addr_index + 1,
                    N
                ));
                zib.build(self.rom);
                addr_index += 1;
            }

            // reg33 = reg33 - 1
            {
                let mut zib =
                    ZiskInstBuilder::new_from_riscv(addr[addr_index], i.inst_name.to_string());
                zib.src_a("reg", 33, false);
                zib.src_b("imm", 1, false);
                zib.op("sub").unwrap();
                zib.store("reg", 33, false, false);
                zib.set_next_internal_address(addr[addr_index + 1]);
                let jump_address = addr[addr_index + 1] as i64 - addr[addr_index] as i64;
                zib.j(jump_address, jump_address);
                zib.verbose(&format!(
                    "{} r{}, r{} {}/{}",
                    i.inst_name,
                    i.rd,
                    i.rs1,
                    addr_index + 1,
                    N
                ));
                zib.build(self.rom);
                addr_index += 1;
            }

            // reg33 = reg33 sra 63   → all-ones iff idx < 8, else 0
            {
                let mut zib =
                    ZiskInstBuilder::new_from_riscv(addr[addr_index], i.inst_name.to_string());
                zib.src_a("reg", 33, false);
                zib.src_b("imm", 63, false);
                zib.op("sra").unwrap();
                zib.store("reg", 33, false, false);
                zib.set_next_internal_address(addr[addr_index + 1]);
                let jump_address = addr[addr_index + 1] as i64 - addr[addr_index] as i64;
                zib.j(jump_address, jump_address);
                zib.verbose(&format!(
                    "{} r{}, r{} {}/{}",
                    i.inst_name,
                    i.rd,
                    i.rs1,
                    addr_index + 1,
                    N
                ));
                zib.build(self.rom);
                addr_index += 1;
            }

            // reg32 = reg32 & 0x7    (low 3 bits of idx)
            {
                let mut zib =
                    ZiskInstBuilder::new_from_riscv(addr[addr_index], i.inst_name.to_string());
                zib.src_a("reg", 32, false);
                zib.src_b("imm", 0x7, false);
                zib.op("and").unwrap();
                zib.store("reg", 32, false, false);
                zib.set_next_internal_address(addr[addr_index + 1]);
                let jump_address = addr[addr_index + 1] as i64 - addr[addr_index] as i64;
                zib.j(jump_address, jump_address);
                zib.verbose(&format!(
                    "{} r{}, r{} {}/{}",
                    i.inst_name,
                    i.rd,
                    i.rs1,
                    addr_index + 1,
                    N
                ));
                zib.build(self.rom);
                addr_index += 1;
            }

            // reg32 = reg32 << 3     → 8*(idx & 7), a valid shift amount 0..56
            {
                let mut zib =
                    ZiskInstBuilder::new_from_riscv(addr[addr_index], i.inst_name.to_string());
                zib.src_a("reg", 32, false);
                zib.src_b("imm", 3, false);
                zib.op("sll").unwrap();
                zib.store("reg", 32, false, false);
                zib.set_next_internal_address(addr[addr_index + 1]);
                let jump_address = addr[addr_index + 1] as i64 - addr[addr_index] as i64;
                zib.j(jump_address, jump_address);
                zib.verbose(&format!(
                    "{} r{}, r{} {}/{}",
                    i.inst_name,
                    i.rd,
                    i.rs1,
                    addr_index + 1,
                    N
                ));
                zib.build(self.rom);
                addr_index += 1;
            }

            // reg32 = rs1 >> reg32   (data-dependent shift)
            {
                let mut zib =
                    ZiskInstBuilder::new_from_riscv(addr[addr_index], i.inst_name.to_string());
                zib.src_a("reg", i.rs1 as u64, false);
                zib.src_b("reg", 32, false);
                zib.op("srl").unwrap();
                zib.store("reg", 32, false, false);
                zib.set_next_internal_address(addr[addr_index + 1]);
                let jump_address = addr[addr_index + 1] as i64 - addr[addr_index] as i64;
                zib.j(jump_address, jump_address);
                zib.verbose(&format!(
                    "{} r{}, r{} {}/{}",
                    i.inst_name,
                    i.rd,
                    i.rs1,
                    addr_index + 1,
                    N
                ));
                zib.build(self.rom);
                addr_index += 1;
            }

            // reg32 = reg32 & 0xFF   → selected byte
            {
                let mut zib =
                    ZiskInstBuilder::new_from_riscv(addr[addr_index], i.inst_name.to_string());
                zib.src_a("reg", 32, false);
                zib.src_b("imm", 0xFF, false);
                zib.op("and").unwrap();
                zib.store("reg", 32, false, false);
                zib.set_next_internal_address(addr[addr_index + 1]);
                let jump_address = addr[addr_index + 1] as i64 - addr[addr_index] as i64;
                zib.j(jump_address, jump_address);
                zib.verbose(&format!(
                    "{} r{}, r{} {}/{}",
                    i.inst_name,
                    i.rd,
                    i.rs1,
                    addr_index + 1,
                    N
                ));
                zib.build(self.rom);
                addr_index += 1;
            }

            // reg32 = reg32 & reg33  → zero if idx out of range
            {
                let mut zib =
                    ZiskInstBuilder::new_from_riscv(addr[addr_index], i.inst_name.to_string());
                zib.src_a("reg", 32, false);
                zib.src_b("reg", 33, false);
                zib.op("and").unwrap();
                zib.store("reg", 32, false, false);
                zib.set_next_internal_address(addr[addr_index + 1]);
                let jump_address = addr[addr_index + 1] as i64 - addr[addr_index] as i64;
                zib.j(jump_address, jump_address);
                zib.verbose(&format!(
                    "{} r{}, r{} {}/{}",
                    i.inst_name,
                    i.rd,
                    i.rs1,
                    addr_index + 1,
                    N
                ));
                zib.build(self.rom);
                addr_index += 1;
            }

            // reg32 = reg32 << (8*j) → place into output byte j
            {
                let mut zib =
                    ZiskInstBuilder::new_from_riscv(addr[addr_index], i.inst_name.to_string());
                zib.src_a("reg", 32, false);
                zib.src_b("imm", 8 * j, false);
                zib.op("sll").unwrap();
                zib.store("reg", 32, false, false);
                zib.set_next_internal_address(addr[addr_index + 1]);
                let jump_address = addr[addr_index + 1] as i64 - addr[addr_index] as i64;
                zib.j(jump_address, jump_address);
                zib.verbose(&format!(
                    "{} r{}, r{} {}/{}",
                    i.inst_name,
                    i.rd,
                    i.rs1,
                    addr_index + 1,
                    N
                ));
                zib.build(self.rom);
                addr_index += 1;
            }

            // reg34 = reg34 | reg32  (accumulate; rd written only at the end)
            {
                let mut zib =
                    ZiskInstBuilder::new_from_riscv(addr[addr_index], i.inst_name.to_string());
                zib.src_a("reg", 34, false);
                zib.src_b("reg", 32, false);
                zib.op("or").unwrap();
                zib.store("reg", 34, false, false);
                zib.set_next_internal_address(addr[addr_index + 1]);
                let jump_address = addr[addr_index + 1] as i64 - addr[addr_index] as i64;
                zib.j(jump_address, jump_address);
                zib.verbose(&format!(
                    "{} r{}, r{} {}/{}",
                    i.inst_name,
                    i.rd,
                    i.rs1,
                    addr_index + 1,
                    N
                ));
                zib.build(self.rom);
                addr_index += 1;
            }
        }

        // rd = reg34 (copyb stores src_b into c)
        {
            let mut zib =
                ZiskInstBuilder::new_from_riscv(addr[addr_index], i.inst_name.to_string());
            zib.src_a("imm", 0, false);
            zib.src_b("reg", 34, false);
            zib.op("copyb").unwrap();
            zib.store("reg", i.rd as i64, false, false);
            let jump_address = rom_address as i64 + 4 - addr[addr_index] as i64;
            zib.j(jump_address, jump_address);
            zib.verbose(&format!("{} r{}, r{} {}/{}", i.inst_name, i.rd, i.rs1, addr_index + 1, N));
            zib.build(self.rom);
        }
    }
} // impl Riscv2ZiskContext
