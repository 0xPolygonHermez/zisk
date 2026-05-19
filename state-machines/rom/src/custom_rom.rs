//! Static custom ROM trace construction.
//!
//! [`CustomRom`] builds the `RomRomTrace<F>` commit that holds the program code as field
//! elements. Pure transformation: ELF bytes → `ZiskRom` → trace rows.

use fields::PrimeField64;
use zisk_core::{zisk_ops::ZiskOp, Riscv2zisk, ZiskRom, SRC_IMM};
use zisk_pil::{RomRomTrace, RomRomTraceRow, RomTrace};

use crate::error::{RomError, RomResult};

/// Custom ROM trace construction from ELF bytes.
pub struct CustomRom;

impl CustomRom {
    /// Computes the custom ROM trace from the given ELF bytes.
    ///
    /// # Arguments
    /// * `elf` - The ELF bytes.
    ///
    /// # Errors
    /// Returns [`RomError::ElfTranspile`] if the ELF cannot be transpiled into a `ZiskRom`,
    /// [`RomError::RomTooLarge`] if the resulting program exceeds the maximum number of
    /// rows the custom ROM trace can hold, or [`RomError::TraceConstruction`] if the trace
    /// cannot be constructed.
    pub fn build<F: PrimeField64>(elf: &[u8]) -> RomResult<RomRomTrace<F>> {
        tracing::info!("Computing custom trace ROM");
        let rom = Self::parse_rom::<F>(elf)?;
        Self::build_trace(&rom)
    }

    /// Transpiles `elf` into a `ZiskRom` and validates that its instruction count fits the
    /// PIL ROM trace.
    fn parse_rom<F: PrimeField64>(elf: &[u8]) -> RomResult<ZiskRom> {
        let rom = Riscv2zisk::new(elf).run().map_err(|e| RomError::ElfTranspile(e.to_string()))?;

        let len = rom.insts.len();
        let max_len = RomTrace::<F>::NUM_ROWS;
        if len > max_len {
            return Err(RomError::RomTooLarge { len, max_len });
        }

        Ok(rom)
    }

    /// Builds a trace with one row per ROM instruction, padding the tail with zeros.
    fn build_trace<F: PrimeField64>(rom: &ZiskRom) -> RomResult<RomRomTrace<F>> {
        let buffer = vec![F::ZERO; RomRomTrace::<F>::NUM_ROWS * RomRomTrace::<F>::ROW_SIZE];
        let mut trace = RomRomTrace::new_from_vec(buffer)
            .map_err(|e| RomError::TraceConstruction(e.to_string()))?;

        for (_pc, zib) in rom.insts.iter() {
            let inst = &zib.i;
            let index = inst.index as usize;
            debug_assert!(
                index < RomRomTrace::<F>::NUM_ROWS,
                "ROM instruction index {} out of bounds for ROM trace with {} rows",
                index,
                RomRomTrace::<F>::NUM_ROWS
            );

            trace[index].line = F::from_u64(inst.paddr); // TODO: unify names: pc, paddr, line
            trace[index].a_offset_imm0 = Self::signed_to_field(inst.a_offset_imm0 as i64);
            trace[index].a_imm1 =
                F::from_u64(if inst.a_src == SRC_IMM { inst.a_use_sp_imm1 } else { 0 });
            trace[index].b_offset_imm0 = Self::signed_to_field(inst.b_offset_imm0 as i64);
            trace[index].b_imm1 =
                F::from_u64(if inst.b_src == SRC_IMM { inst.b_use_sp_imm1 } else { 0 });
            trace[index].ind_width = F::from_u64(inst.ind_width);
            // IMPORTANT: the opcodes fcall, fcall_get, and fcall_param are really a variant
            // of the copyb, use to get free-input information
            trace[index].op = if inst.op == ZiskOp::Fcall.code()
                || inst.op == ZiskOp::FcallGet.code()
                || inst.op == ZiskOp::FcallParam.code()
            {
                F::from_u8(ZiskOp::CopyB.code())
            } else {
                F::from_u8(inst.op)
            };
            trace[index].store_offset = Self::signed_to_field(inst.store_offset);
            trace[index].jmp_offset1 = Self::signed_to_field(inst.jmp_offset1);
            trace[index].jmp_offset2 = Self::signed_to_field(inst.jmp_offset2);
            trace[index].flags = F::from_u64(inst.get_flags());
        }

        trace.buffer[rom.insts.len()..].fill(RomRomTraceRow::default());

        Ok(trace)
    }

    /// Converts a signed integer to a field element, mapping negatives through `F::neg`.
    #[inline]
    fn signed_to_field<F: PrimeField64>(v: i64) -> F {
        if v >= 0 {
            F::from_u64(v as u64)
        } else {
            F::neg(F::from_u64((-v) as u64))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fields::Goldilocks;

    type F = Goldilocks;

    #[test]
    fn signed_to_field_maps_zero() {
        assert_eq!(CustomRom::signed_to_field::<F>(0), F::from_u64(0));
    }

    #[test]
    fn signed_to_field_maps_positives_to_from_u64() {
        assert_eq!(CustomRom::signed_to_field::<F>(1), F::from_u64(1));
        assert_eq!(CustomRom::signed_to_field::<F>(42), F::from_u64(42));
        assert_eq!(CustomRom::signed_to_field::<F>(i64::MAX), F::from_u64(i64::MAX as u64));
    }

    #[test]
    fn signed_to_field_maps_negatives_through_neg() {
        assert_eq!(CustomRom::signed_to_field::<F>(-1), -F::from_u64(1));
        assert_eq!(CustomRom::signed_to_field::<F>(-42), -F::from_u64(42));
    }

    #[test]
    fn signed_to_field_negative_round_trips_through_addition() {
        // For any v, signed_to_field(v) + signed_to_field(-v) == 0
        let v = 123_456_789_i64;
        let sum = CustomRom::signed_to_field::<F>(v) + CustomRom::signed_to_field::<F>(-v);
        assert_eq!(sum, F::from_u64(0));
    }

    #[test]
    fn parse_rom_rejects_malformed_elf() {
        let err = CustomRom::parse_rom::<F>(b"not a valid elf file")
            .expect_err("malformed ELF must fail");
        assert!(
            matches!(err, RomError::ElfTranspile(_)),
            "expected RomError::ElfTranspile, got {err:?}"
        );
    }
}
