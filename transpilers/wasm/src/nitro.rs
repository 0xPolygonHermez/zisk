//! Explicit Nitro crypto ABI registrations; this is not a complete Nitro runtime.
//!
//! Results are computed by the production library's constrained instructions in the ROM.
//! Recovery IDs 2/3 trap until the native library implements their x = r + n branch.
use super::{
    emit::{Code, LabelId},
    layout::*,
    module::ValKind::I32,
    Import,
};

pub(crate) mod runtime;

/// Compile Nitro with an explicitly supplied, trusted native runtime ELF.
/// The resulting ROM/VK binds both artifacts; stdin remains untrusted witness.
pub fn wasm2rom(
    bytes: &[u8],
    runtime_elf: &[u8],
) -> Result<zisk_core::ZiskRom, Box<dyn std::error::Error>> {
    if !cfg!(feature = "float") {
        return Err("nitro: build with the float feature".into());
    }
    let runtime = runtime::Runtime::new(bytes, runtime_elf)?;
    super::wasm2rom_impl(bytes, &runtime::imports(), Some(&runtime))
}

/// Registers Nitro's `arbcrypto` imports without changing the generic compiler or CLI defaults.
pub fn crypto_imports() -> [Import<'static>; 2] {
    [
        Import {
            module: "arbcrypto",
            name: "keccak256",
            params: &[I32, I32, I32],
            results: &[],
            emit: keccak256,
        },
        Import {
            module: "arbcrypto",
            name: "ecrecovery",
            params: &[I32, I32, I32, I32, I32],
            results: &[I32],
            emit: ecrecovery,
        },
    ]
}

fn local_u32(code: &mut Code, reg: u64, index: u32) {
    code.load_slot_to_reg(reg, local_offset(index));
    code.alu_ri("and", reg, reg, 0xffff_ffff);
}

// The unsigned wasm32 sum is evaluated in u64, so wrapping/negative i32 inputs cannot escape.
fn checked_pointer(code: &mut Code, reg: u64, len: u64, trap: LabelId) {
    code.alu_ri("add", reg, reg, WASM_MEM_BASE as i64);
    code.alu_rr("add", REG_T0, reg, len);
    code.cmp_reg_branch("ltu", REG_MEM_END, REG_T0, trap, true);
}

fn keccak256(code: &mut Code) {
    let trap = code.new_label();
    let done = code.new_label();
    local_u32(code, 10, 0);
    local_u32(code, 11, 1);
    checked_pointer(code, 10, 11, trap);
    local_u32(code, 12, 2);
    code.load_imm_to_reg(13, 32);
    checked_pointer(code, 12, 13, trap);
    // The sponge consumes the entire input before squeezing, so overlapping buffers are safe.
    code.call_library("ziskasm_zkvm_keccak256");
    code.cmp_imm_branch("eq", REG_RET, 0, trap, false);
    code.jump(done);
    code.bind(trap);
    code.trap();
    code.bind(done);
}

fn copy_to_scratch(code: &mut Code, source: u64, dest: u64, len: u64) {
    for off in (0..len).step_by(8) {
        code.load_mem_to_reg("copyb", REG_T0, source, off as i64, 8);
        code.store_reg_to_abs(dest + off, REG_T0, 8);
    }
}

// Compare a big-endian scalar at signature register 11 without changing its pointer.
fn scalar_below(code: &mut Code, offset: i64, limit: [u64; 4], rejected: LabelId) {
    let accepted = code.new_label();
    for (word, bound) in limit.into_iter().enumerate() {
        code.load_mem_to_reg("rev8", REG_T0, 11, offset + 8 * word as i64, 8);
        code.cmp_imm_branch("ltu", REG_T0, bound as i64, accepted, true);
        code.cmp_imm_branch("eq", REG_T0, bound as i64, rejected, false);
    }
    code.jump(rejected);
    code.bind(accepted);
}

fn ecrecovery(code: &mut Code) {
    let bad_hash = code.new_label();
    let bad_sig = code.new_label();
    let bad_id = code.new_label();
    let failed = code.new_label();
    let trap = code.new_label();
    let done = code.new_label();
    local_u32(code, REG_T0, 1);
    code.cmp_imm_branch("eq", REG_T0, 32, bad_hash, false);
    local_u32(code, REG_T0, 3);
    code.cmp_imm_branch("eq", REG_T0, 65, bad_sig, false);

    local_u32(code, 10, 0);
    code.load_imm_to_reg(11, 32);
    checked_pointer(code, 10, 11, trap);
    copy_to_scratch(code, 10, WASM_CRYPTO_HASH_ADDR, 32);
    local_u32(code, 11, 2);
    code.load_imm_to_reg(12, 65);
    checked_pointer(code, 11, 12, trap);
    // Pinned Nitro parses nonzero, in-range scalars before checking the recovery ID.
    for offset in [0, 32] {
        code.load_imm_to_reg(REG_T1, 0);
        for word in 0..4 {
            code.load_mem_to_reg("copyb", REG_T0, 11, offset + 8 * word, 8);
            code.alu_rr("or", REG_T1, REG_T1, REG_T0);
        }
        code.cmp_imm_branch("eq", REG_T1, 0, trap, true);
        scalar_below(
            code,
            offset,
            [0xffffffffffffffff, 0xfffffffffffffffe, 0xbaaedce6af48a03b, 0xbfd25e8cd0364141],
            trap,
        );
    }
    code.load_mem_to_reg("copyb", 12, 11, 64, 1);
    code.cmp_imm_branch("ltu", 12, 4, bad_id, false);
    // shortcut: native recovery supports IDs 0/1 only, remove this trap when IDs 2/3 are proved.
    code.cmp_imm_branch("ltu", 12, 2, trap, false);
    // Nitro's k256 recovery rejects high-s before accessing output; native recovery does not.
    scalar_below(
        code,
        32,
        [0x7fffffffffffffff, 0xffffffffffffffff, 0x5d576e7357a4501d, 0xdfe92f46681b20a1],
        failed,
    );
    copy_to_scratch(code, 11, WASM_CRYPTO_SIG_ADDR, 64);
    code.load_imm_to_reg(10, WASM_CRYPTO_HASH_ADDR);
    code.load_imm_to_reg(11, WASM_CRYPTO_SIG_ADDR);
    code.load_imm_to_reg(13, WASM_CRYPTO_OUTPUT_ADDR);
    code.call_library("ziskasm_zkvm_secp256k1_ecrecover");
    code.cmp_imm_branch("eq", REG_RET, 0, failed, false);

    // Nitro accesses output only on success. Scratch also preserves aliased hash/signature input.
    local_u32(code, REG_T1, 4);
    code.load_imm_to_reg(REG_T2, 65);
    checked_pointer(code, REG_T1, REG_T2, trap);
    code.load_imm_to_reg(REG_T0, 4);
    code.store_reg_to_mem(REG_T1, 0, REG_T0, 1);
    for off in (0..64).step_by(8) {
        code.load_abs_to_reg(REG_T0, WASM_CRYPTO_OUTPUT_ADDR + off);
        code.store_reg_to_mem(REG_T1, 1 + off as i64, REG_T0, 8);
    }
    code.load_imm_to_reg(REG_RET, 0);
    code.jump(done);
    for (label, status) in [(bad_hash, 1), (bad_sig, 2), (bad_id, 3), (failed, 4)] {
        code.bind(label);
        code.load_imm_to_reg(REG_RET, status);
        code.jump(done);
    }
    code.bind(trap);
    code.trap();
    code.bind(done);
}
