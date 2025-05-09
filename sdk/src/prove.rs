//! Configuration and context for the proving process.
//!
use crate::common::{
    print_banner, Field, OutputPath, ProvingKeyPath, Sha256fScriptPath, WitnessLibPath,
};
use anyhow::Result;
use colored::Colorize;
use log::info;
use proofman_common::{json_to_debug_instances_map, DebugInfo, ModeName};
use serde::Serialize;
use std::fs::File;
use std::io::Write;
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
    pub sha256f_script: Sha256fScriptPath,

    /// Only verify constraints (no proof generation).
    pub only_verify_constraints: bool,
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

    pub fn sha256f_script(mut self, path: Option<impl Into<PathBuf>>) -> Self {
        self.sha256f_script = Sha256fScriptPath::new(path);
        self
    }

    pub fn only_verify_constraints(mut self, enabled: bool) -> Self {
        self.only_verify_constraints = enabled;
        self
    }

    // TODO: Add function to check if all paths exists
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
            sha256f_script: Sha256fScriptPath::default(),
            only_verify_constraints: false,
        }
    }
}

/// ProveResult holds the result of the proving process.
#[derive(Serialize)]
pub struct ProveResult {
    /// Proof ID. Only available if the proof is generated (not only verifying constraints).
    pub proof_id: Option<String>,

    /// Number of cycles used
    pub cycles: u64,

    /// Proving time in seconds
    pub time: f64,
}

impl ProveResult {
    pub fn new(proof_id: Option<String>, cycles: u64, time: f64) -> Self {
        ProveResult { proof_id, cycles, time }
    }

    pub fn print(&self) {
        println!();

        if self.proof_id.is_some() {
            info!("{}", "Zisk: --- PROVE SUMMARY ------------------------".bright_green().bold());
        } else {
            info!(
                "{}",
                "Zisk: --- VERIFY CONSTRAINTS SUMMARY ------------------------"
                    .bright_green()
                    .bold()
            );
        };

        if let Some(proof_id) = &self.proof_id {
            info!("                Proof ID: {}", proof_id);
        }
        info!("              ► Statistics");
        info!("                time: {} seconds, steps: {}", self.time, self.cycles);
    }

    pub fn save(&self, path: PathBuf) -> Result<()> {
        // Save the proof result only if the proof was generated and we have a proof ID
        if self.proof_id.is_some() {
            let prove_result_json = serde_json::to_string_pretty(&self)?;

            let mut file = File::create(&path)?;
            file.write_all(prove_result_json.as_bytes())
                .map_err(|e| anyhow::anyhow!("Failed to save prove result: {}", e))?;
        }

        Ok(())
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

    /// Path to the ASM_MT file (optional)
    pub asm_mt_path: Option<PathBuf>,

    /// Path to the ASM_ROM file (optional)
    pub asm_rom_path: Option<PathBuf>,

    /// Path to the ELF binary file
    pub elf_bin_path: PathBuf,
}

impl ProveContext {
    pub fn print(&self) {
        print_banner();

        let command =
            if self.config.only_verify_constraints { "Verify constraints" } else { "Prove" };

        println!("{} {}", format!("{: >12}", "Command").bright_green().bold(), command);
        println!(
            "{: >12} {}",
            "Witness Lib".bright_green().bold(),
            self.config.witness_lib.as_ref().display()
        );

        println!("{: >12} {}", "Elf".bright_green().bold(), self.elf.display());

        if let Some(asm_path) = &self.asm_mt_path {
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
            "Sha256f".bright_green().bold(),
            self.config.sha256f_script.as_ref().display()
        );

        println!();
    }
}
