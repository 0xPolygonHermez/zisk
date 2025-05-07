//! This module provides the configuration and context for the proving process.
//!
use crate::common::{
    print_banner, Field, KeccakScriptPath, OutputPath, ProvingKeyPath, WitnessLibPath,
};
use colored::Colorize;
use proofman_common::{json_to_debug_instances_map, DebugInfo, ModeName};
use std::path::PathBuf;

/// Prove command configuration options.
#[derive(Clone)]
pub struct ProveConfig {
    /// Witness computation dynamic library path.
    pub witness_lib: WitnessLibPath,

    /// ASM file path (optional, mutually exclusive with emulator option).
    pub asm: Option<PathBuf>,

    /// Use prebuilt emulator instead of ASM.
    pub emulator: bool,

    /// Proving key path.
    pub proving_key: ProvingKeyPath,

    /// Output path.
    pub output_dir: OutputPath,

    /// Field type to use.
    pub field: Field,

    /// Enable aggregation.
    pub aggregation: bool,

    /// Enable final SNARK generation.
    pub final_snark: bool,

    /// Enable proof verification.
    pub verify_proofs: bool,

    /// Verbosity level (0 = silent, 1 = verbose, 2 = very verbose, etc.).
    pub verbose: u8,

    /// Debug information.
    pub debug_info: DebugInfo,

    /// Keccak script file path.
    pub keccak_script: KeccakScriptPath,
}

impl ProveConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn witness_lib(mut self, path: Option<impl Into<PathBuf>>) -> Self {
        self.witness_lib = WitnessLibPath::new(path);
        self
    }

    pub fn asm(mut self, path: Option<impl Into<PathBuf>>) -> Self {
        self.asm = path.map(|p| p.into());
        self
    }

    pub fn emulator(mut self, enabled: bool) -> Self {
        self.emulator = enabled;
        self
    }

    pub fn proving_key(mut self, path: Option<impl Into<PathBuf>>) -> Self {
        self.proving_key = ProvingKeyPath::new(path);
        self
    }

    pub fn output_dir(mut self, path: Option<impl Into<PathBuf>>) -> Self {
        self.output_dir = OutputPath::new(path);

        self
    }

    pub fn field(mut self, field: Field) -> Self {
        self.field = field;
        self
    }

    pub fn aggregation(mut self, enabled: bool) -> Self {
        self.aggregation = enabled;
        self
    }

    pub fn final_snark(mut self, enabled: bool) -> Self {
        self.final_snark = enabled;
        self
    }

    pub fn verify_proofs(mut self, enabled: bool) -> Self {
        self.verify_proofs = enabled;
        self
    }

    pub fn verbose(mut self, level: u8) -> Self {
        self.verbose = level;
        self
    }

    pub fn debug(mut self, debug: Option<Option<String>>) -> Self {
        self.debug_info = match debug {
            None => DebugInfo::default(),
            Some(None) => DebugInfo::new_debug(),
            Some(Some(debug_value)) => {
                json_to_debug_instances_map(self.proving_key.clone().into(), debug_value.clone())
            }
        };
        self
    }

    pub fn keccak_script(mut self, path: Option<impl Into<PathBuf>>) -> Self {
        self.keccak_script = KeccakScriptPath::new(path);
        self
    }
}

impl Default for ProveConfig {
    fn default() -> Self {
        ProveConfig {
            witness_lib: WitnessLibPath::default(),
            asm: None,
            emulator: false,
            proving_key: ProvingKeyPath::default(),
            output_dir: OutputPath::default(),
            field: Field::default(),
            aggregation: true,
            final_snark: false,
            verify_proofs: false,
            verbose: 0,
            debug_info: DebugInfo::default(),
            keccak_script: KeccakScriptPath::default(),
        }
    }
}

/// ProveContext holds the context for the proving process.
#[derive(Clone, Default)]
pub struct ProveContext {
    /// Path to the ELF file
    pub elf: PathBuf,

    /// Path to the input file (optional)
    pub input: Option<PathBuf>,

    /// Prove configuration options
    pub config: ProveConfig,

    /// Path to the ASM file (optional)
    pub asm_path: Option<PathBuf>,

    /// Path to the ELF binary file
    pub elf_bin_path: PathBuf,
}

impl ProveContext {
    pub fn print(&self) {
        print_banner();

        println!("{} Prove", format!("{: >12}", "Command").bright_green().bold());
        println!(
            "{: >12} {}",
            "Witness Lib".bright_green().bold(),
            self.config.witness_lib.as_ref().display()
        );

        println!("{: >12} {}", "Elf".bright_green().bold(), self.elf.display());

        if let Some(asm_path) = &self.asm_path {
            println!("{: >12} {}", "ASM runner".bright_green().bold(), asm_path.display());
        } else {
            println!(
                "{: >12} {}",
                "Emulator".bright_green().bold(),
                "Running in emulator mode".bright_yellow()
            );
        }

        if let Some(input) = &self.input {
            println!("{: >12} {}", "Inputs".bright_green().bold(), input.display());
        }

        println!(
            "{: >12} {}",
            "Proving key".bright_green().bold(),
            self.config.proving_key.as_ref().display()
        );

        let std_mode = if self.config.debug_info.std_mode.name == ModeName::Debug {
            "Debug mode"
        } else {
            "Standard mode"
        };
        println!("{: >12} {}", "STD".bright_green().bold(), std_mode);
        println!(
            "{: >12} {}",
            "Keccak".bright_green().bold(),
            self.config.keccak_script.as_ref().display()
        );

        println!();
    }
}
