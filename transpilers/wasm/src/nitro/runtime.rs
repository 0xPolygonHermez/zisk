//! Explicit native helper linking and lifecycle for the Nitro adapter.
use crate::{
    emit::Code,
    layout::*,
    module::ValKind::{I32, I64},
    Import,
};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, error::Error};
use zisk_core::{
    mem::DataSection, rom_layout::normalize_rw_data_sections, zisk_rom::DataSection64, ZiskRom,
};
use zisk_riscv::elf_extraction::{
    collect_elf_payload_from_bytes, merge_ro_sections, validate_entry_point, ElfPayload,
};

pub(crate) const ROM_BASE: u64 = 0x9700_0000;
const RAM_BASE: u64 = 0xa060_0000;
const ARGS: u64 = WASM_CTRL_ADDR + 0x200;
const MODULE_HASH: u64 = WASM_CTRL_ADDR + 0x300;

pub(crate) struct Runtime {
    payload: ElfPayload,
    pub entry: u64,
    hash: [u8; 32],
}

impl Runtime {
    pub fn new(wasm: &[u8], elf: &[u8]) -> Result<Self, Box<dyn Error>> {
        let payload = collect_elf_payload_from_bytes(elf)?;
        validate_entry_point(&payload)?;
        for (sections, lo, hi) in [
            (&payload.exec, ROM_BASE, zisk_core::ZISKLIB_ROM_ADDR),
            (&payload.ro, ROM_BASE, zisk_core::ZISKLIB_ROM_ADDR),
            (&payload.rw, RAM_BASE, WASM_MEM_BASE),
        ] {
            for section in sections {
                if section.addr < lo
                    || section
                        .addr
                        .checked_add(section.data.len() as u64)
                        .is_none_or(|end| end > hi)
                {
                    return Err(format!(
                        "nitro: native section at {:#x} exceeds its reserved region",
                        section.addr
                    )
                    .into());
                }
            }
        }
        Ok(Self { entry: payload.entry_point, payload, hash: Sha256::digest(wasm).into() })
    }

    pub fn link(
        &self,
        rom: &mut ZiskRom,
        symbols: &HashMap<String, u64>,
    ) -> Result<(), Box<dyn Error>> {
        let mut calls = HashMap::new();
        for section in &self.payload.exec {
            for id in zisk_riscv::zkvmcall_ids(section.addr, &section.data)? {
                let call = zisk_definitions::zkvmcall_by_id(id).ok_or("nitro: unknown zkvmcall")?;
                if !matches!(call.name, "zkvm_keccak256" | "zkvm_sha256") {
                    return Err(
                        format!("nitro: unsupported native runtime call {}", call.name).into()
                    );
                }
                calls.insert(
                    id,
                    *symbols.get(call.target).ok_or("nitro: missing native crypto symbol")?,
                );
                rom.zkvmcalls.insert(id);
            }
        }
        for section in &self.payload.exec {
            zisk_riscv::add_zisk_code(rom, section.addr, &section.data, &calls, &HashMap::new());
        }
        for section in merge_ro_sections(self.payload.ro.clone())? {
            rom.ro_data_64.push(section64(section));
        }
        for section in normalize_rw_data_sections(self.payload.rw.clone()) {
            rom.rw_data_64.push(section64(section));
        }
        Ok(())
    }

    pub fn initialize(&self, code: &mut Code) {
        for (i, word) in self.hash.chunks_exact(8).enumerate() {
            code.store_imm_to_abs(
                MODULE_HASH + i as u64 * 8,
                u64::from_le_bytes(word.try_into().unwrap()),
            );
        }
        code.store_imm_to_abs(ARGS, zisk_core::INPUT_ADDR + 16);
        code.load_abs_to_reg(REG_T0, zisk_core::INPUT_ADDR + 8);
        code.store_reg_to_abs(ARGS + 8, REG_T0, 8);
        code.store_imm_to_abs(ARGS + 16, MODULE_HASH);
        dispatch(code, 0);
        require_success(code);
    }
}

fn section64(section: DataSection) -> DataSection64 {
    DataSection64 {
        addr: section.addr,
        data: section
            .data
            .chunks_exact(8)
            .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
            .collect(),
    }
}

fn dispatch(code: &mut Code, op: u64) {
    code.load_imm_to_reg(10, op);
    code.load_imm_to_reg(11, ARGS);
    code.mov_reg(12, REG_MEM_END);
    code.call_runtime();
}

fn require_success(code: &mut Code) {
    let done = code.new_label();
    code.cmp_imm_branch("eq", REG_RET, 0, done, true);
    code.trap();
    code.bind(done);
}

pub(crate) fn emit_finalize(code: &mut Code) {
    code.store_imm_to_abs(ARGS, zisk_core::OUTPUT_ADDR);
    dispatch(code, 1);
    require_success(code);
}

fn emit<const OP: u64, const COUNT: u32, const MASK: u32>(code: &mut Code) {
    for index in 0..COUNT {
        code.load_slot_to_reg(REG_T0, local_offset(index));
        if MASK & (1 << index) != 0 {
            code.alu_ri("and", REG_T0, REG_T0, 0xffff_ffff);
        }
        code.store_reg_to_abs(ARGS + index as u64 * 8, REG_T0, 8);
    }
    dispatch(code, OP);
}

pub(crate) fn imports<'a>() -> Vec<Import<'a>> {
    let mut imports = Vec::from(super::crypto_imports());
    imports.extend([
        Import {
            module: "wavmio",
            name: "getGlobalStateBytes32",
            params: &[I32, I32],
            results: &[],
            emit: emit::<10, 2, 3>,
        },
        Import {
            module: "wavmio",
            name: "setGlobalStateBytes32",
            params: &[I32, I32],
            results: &[],
            emit: emit::<11, 2, 3>,
        },
        Import {
            module: "wavmio",
            name: "getGlobalStateU64",
            params: &[I32],
            results: &[I64],
            emit: emit::<12, 1, 1>,
        },
        Import {
            module: "wavmio",
            name: "setGlobalStateU64",
            params: &[I32, I64],
            results: &[],
            emit: emit::<13, 2, 1>,
        },
        Import {
            module: "wavmio",
            name: "readInboxMessage",
            params: &[I64, I32, I32],
            results: &[I32],
            emit: emit::<14, 3, 6>,
        },
        Import {
            module: "wavmio",
            name: "readDelayedInboxMessage",
            params: &[I64, I32, I32],
            results: &[I32],
            emit: emit::<15, 3, 6>,
        },
        Import {
            module: "wavmio",
            name: "resolveTypedPreimage",
            params: &[I32, I32, I32, I32],
            results: &[I32],
            emit: emit::<16, 4, 15>,
        },
        Import {
            module: "wavmio",
            name: "validateCertificate",
            params: &[I32, I32],
            results: &[I32],
            emit: emit::<17, 2, 3>,
        },
        Import {
            module: "wavmio",
            name: "greedyResolveTypedPreimage",
            params: &[I32, I32, I32, I32, I32],
            results: &[I32],
            emit: emit::<18, 5, 31>,
        },
        Import {
            module: "wavmio",
            name: "resolvePreImage",
            params: &[I32, I32, I32],
            results: &[I32],
            emit: |code| {
                code.store_imm_to_abs(ARGS, 0);
                for i in 0..3 {
                    code.load_slot_to_reg(REG_T0, local_offset(i));
                    code.alu_ri("and", REG_T0, REG_T0, 0xffff_ffff);
                    code.store_reg_to_abs(ARGS + (i as u64 + 1) * 8, REG_T0, 8);
                }
                dispatch(code, 16);
            },
        },
        Import {
            module: "arbcompress",
            name: "brotli_decompress",
            params: &[I32, I32, I32, I32, I32],
            results: &[I32],
            emit: emit::<20, 5, 31>,
        },
        Import {
            module: "arbcompress",
            name: "brotli_compress",
            params: &[I32, I32, I32, I32, I32, I32, I32],
            results: &[I32],
            emit: emit::<21, 7, 127>,
        },
        Import { module: "hooks", name: "beforeFirstIO", params: &[], results: &[], emit: |_| {} },
    ]);
    imports
}
