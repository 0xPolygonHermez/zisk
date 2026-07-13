//! Sample constant definitions demonstrating the `#[constants]`/`#[emit]` system.
//!
//! This module exists only to prove the codegen loop end-to-end (rendered by the
//! `zisk-definitions-generator` engine). During integration the real ZisK constants
//! replace it, one module at a time. Every attribute feature is exercised here:
//! inheritance, `#[emit(internal)]`, `skip(..)`, target restriction, a derived
//! value, a per-target prefix, a radix override, and a `fits` override.

// The emission schema, used to type `ZISK_CONSTANTS` below. The `#[constants]`
// macro references `zisk_definitions_generator::meta::*` directly, so no re-export
// is needed.
use zisk_definitions_generator::meta;

pub const ZISK_CONSTANTS: &[(&meta::GroupMeta, &[meta::Export])] = &[];

#[cfg(test)]
mod tests {
    use zisk_definitions_generator::{meta, render, GenFile};
    use zisk_definitions_macros::constants;

    /// Every constants group the generator renders. Adding a group is a one-line edit
    /// here; the engine (`zisk-definitions-generator`) stays generic over this registry.
    #[allow(dead_code)]
    pub const CONSTANT_SAMPLES: &[(&meta::GroupMeta, &[meta::Export])] = &[
        (&memory::GROUP, memory::EXPORTS),
        (&opcodes::GROUP, opcodes::EXPORTS),
        (&execution::GROUP, execution::EXPORTS),
    ];

    /// Program memory map. 32-bit address space, shared by Rust, the C emulator, and PIL.
    #[constants(group = "memory", to(rust, c, pil), hex, fits = 32)]
    pub mod memory {
        /// First global RW memory address.
        pub const RAM_ADDR: u64 = 0xa000_0000;

        /// Program stack size — derives `SYS_ADDR`; itself emitted nowhere.
        #[emit(internal)]
        pub const STACK_SIZE: u64 = 0x40_0000;

        /// First system RW memory address.
        pub const SYS_ADDR: u64 = RAM_ADDR + STACK_SIZE;

        /// Extra precompile parameters (256 B → 32 params). Rust + PIL only.
        #[emit(skip(c))]
        pub const EXTRA_PARAMS_ADDR: u64 = SYS_ADDR + 0x0F00;
    }

    /// Operation codes. Shared by Rust and PIL; PIL wants an `OP_` prefix.
    #[constants(group = "opcodes", to(rust, pil), hex, pil_prefix = "OP_")]
    pub mod opcodes {
        /// Addition.
        pub const ADD: u8 = 0x0a;
        /// Subtraction.
        pub const SUB: u8 = 0x0b;
    }

    /// Execution-size parameters shared by the executor and the constraints.
    #[constants(group = "execution", to(rust, c, pil), hex)]
    pub mod execution {
        /// log2 of the maximum step count. Reads better in decimal.
        #[emit(dec)]
        pub const MAIN_STEP_BITS: u32 = 36;

        /// Maximum number of execution steps (a hard PIL constraint).
        /// Exceeds 32 bits, so widen the inherited fit check.
        #[emit(fits = 64)]
        pub const MAX_STEPS: u64 = 1u64 << MAIN_STEP_BITS;
    }

    fn contents<'a>(files: &'a [GenFile], name: &str) -> Option<&'a str> {
        files.iter().find(|f| f.name == name).map(|f| f.contents.as_str())
    }

    #[test]
    fn sample_round_trips() {
        let files = render(CONSTANT_SAMPLES, "test").expect("render");

        let mem_h = contents(&files, "memory.h").expect("memory.h missing");
        let mem_pil = contents(&files, "memory.pil").expect("memory.pil missing");

        // `internal` is emitted as its own definition nowhere (it may still appear
        // inside a derived constant's provenance comment, which is intended).
        assert!(!mem_h.contains("#define STACK_SIZE"));
        assert!(!mem_pil.contains("const int STACK_SIZE"));

        // Derived value stamped as a literal, with its expression as provenance.
        assert!(mem_h.contains("#define SYS_ADDR"));
        assert!(mem_h.contains("(uint64_t)0xa0400000"));
        assert!(mem_h.contains("RAM_ADDR + STACK_SIZE"));

        // `skip(c)`: EXTRA_PARAMS_ADDR reaches PIL but not the C header.
        assert!(!mem_h.contains("#define EXTRA_PARAMS_ADDR"));
        assert!(mem_pil.contains("EXTRA_PARAMS_ADDR"));
        assert!(mem_pil.contains("0xA0400F00"));

        // Opcodes: PIL-only, with the `OP_` prefix.
        assert!(contents(&files, "opcodes.h").is_none());
        let op_pil = contents(&files, "opcodes.pil").expect("opcodes.pil missing");
        assert!(op_pil.contains("const int OP_ADD = 0xA;"));
        assert!(op_pil.contains("const int OP_SUB = 0xB;"));

        // Execution: decimal radix override, and a 2^36 value in hex.
        let ex_pil = contents(&files, "execution.pil").expect("execution.pil missing");
        assert!(ex_pil.contains("= 36;"));
        assert!(ex_pil.contains("= 0x1000000000;"));
    }
}
