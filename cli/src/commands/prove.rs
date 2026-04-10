use crate::ux::{print_banner, print_banner_command, print_banner_field, print_execution_summary};
use anyhow::Result;

use colored::Colorize;
use proofman_common::ParamsGPU;
use std::path::PathBuf;
use tracing::{info, warn};
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_common::io::{StreamSource, ZiskStdin};
use zisk_common::ElfBinaryFromFile;
use zisk_sdk::{ProofOpts, ProverClient, ZiskProof, ZiskProveResult};

use super::{detect_current_project_elf, resolve_elf_path};

// Structure representing the 'prove' subcommand of cargo.
#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Generate a proof from the execution of the guest program
pub struct ZiskProve {
    /// Path to the program ELF file
    #[arg(short = 'e', long)]
    pub elf: Option<PathBuf>,

    /// Id of your program generated during setup
    #[arg(short = 'p', long, conflicts_with = "elf")]
    program_id: Option<String>,

    /// Use prebuilt emulator (mutually exclusive with `--asm`)
    #[arg(short = 'l', long, conflicts_with = "asm")]
    pub emulator: bool,

    /// Input file path for the guest. Accepts a string literal or a path to a binary file
    #[arg(alias = "input", short = 'i', long, conflicts_with = "hints")]
    pub inputs: Option<String>,

    /// Save the input to the specified file path. Only used if `--inputs` is a string literal and not a file path
    #[arg(long, requires = "inputs")]
    pub save_inputs: bool,

    /// Precompiles hints file path for the guest
    #[arg(long, conflicts_with = "inputs")]
    pub hints: Option<String>,

    /// Path to a precomputed proving key
    #[arg(short = 'k', long)]
    pub proving_key: Option<PathBuf>,

    /// Path to a precomputed PLONK proving key
    #[arg(short = 'w', long)]
    pub proving_key_plonk: Option<PathBuf>,

    /// Save the generated proof to the specified file path
    #[arg(short = 'o', long)]
    pub output: Option<PathBuf>,

    /// Enable proofs aggregation
    #[arg(short = 'a', long, default_value_t = false)]
    pub aggregation: bool,

    /// Smaller STARK proof with reduced size at the cost of longer proving time. Mutually exclusive with plonk
    #[arg(short = 'c', long, default_value_t = false, conflicts_with = "plonk")]
    pub minimal: bool,

    /// PLONK proof. Required for on-chain verification via the EVM verifier. Mutually exclusive with minimal
    #[arg(long, default_value_t = false, conflicts_with = "minimal")]
    pub plonk: bool,

    /// Verify proofs after generation
    #[arg(short = 'y', long, default_value_t = false)]
    pub verify_proofs: bool,

    /// Base port for Assembly microservices (default: 23115).
    /// A single execution will use 3 consecutive ports, from this port to port + 2.
    /// If you are running multiple instances of ZisK using mpi on the same machine,
    /// it will use from this base port to base port + 2 * number_of_instances.
    /// For example, if you run 2 mpi instances of ZisK, it will use ports from 23115 to 23117
    /// for the first instance, and from 23118 to 23120 for the second instance.
    //TODO: Remove
    #[arg(short = 'p', long, conflicts_with = "emulator")]
    pub port: Option<u16>,

    /// This is used to unlock the memory map for the ROM file. Mutually exclusive with --emulator
    #[arg(short = 'u', long, conflicts_with = "emulator")]
    pub unlock_mapped_memory: bool,

    /// Maximum memory (bytes) for witness storage during proving
    // TODO: Review default value
    #[arg(short = 'x', long)]
    pub max_witness_stored: Option<usize>,

    /// Reduce memory footprint during proving at the cost of speed
    #[arg(short = 'm', long, default_value_t = false)]
    pub minimal_memory: bool,

    //TODO: Review if we want to keep this flag
    #[arg(short = 'r', long, default_value_t = false)]
    pub rma: bool,

    /// Use GPU acceleration
    #[clap(long, default_value_t = false)]
    pub gpu: bool,

    /// Verbose (-v, -vv)
    #[arg(short ='v', long, action = clap::ArgAction::Count)]
    pub verbose: u8, // Using u8 to hold the number of `-v`

    /// Hidden flags

    /// ASM file path
    #[arg(short = 's', long, hide = true, conflicts_with = "emulator")]
    pub asm: Option<PathBuf>,

    /// Redirect ASM emulator output to file
    #[arg(long, default_value_t = false, hide = true, conflicts_with = "emulator")]
    pub asm_out_file: bool,

    /// Disable automatic ROM setup
    #[arg(short = 'n', long, default_value_t = false, hide = true)]
    pub no_auto_setup: bool,

    /// Use shared tables for execution
    #[arg(short = 'j', long, default_value_t = false, hide = true)]
    pub shared_tables: bool,

    #[arg(short = 'b', long, default_value_t = false, hide = true)]
    // TODO: Review, we can remove this flag since now we can use the optional `--output` flag
    pub save_proofs: bool,

    #[arg(short = 'z', long, default_value_t = false, hide = true)]
    pub preallocate: bool,

    #[arg(short = 't', long, hide = true)]
    pub max_streams: Option<usize>,

    #[arg(long, hide = true)]
    pub number_threads_witness: Option<usize>,
}

impl ZiskProve {
    pub fn run(&mut self) -> Result<()> {
        if self.elf.is_none() && self.program_id.is_none() {
            self.elf = detect_current_project_elf()?;
        }

        // Check if the deprecated alias was used
        if std::env::args().any(|arg| arg == "--input") {
            eprintln!("{}", "Warning: --input is deprecated, use --inputs instead".yellow().bold());
        }

        print_banner();

        print_banner_command("Prove");

        if let Some(elf) = &self.elf {
            print_banner_field("Elf", elf.display());
        } else if let Some(program_id) = &self.program_id {
            print_banner_field("Program ID", program_id);
        }

        let mut gpu_params = None;
        if self.preallocate
            || self.max_streams.is_some()
            || self.number_threads_witness.is_some()
            || self.max_witness_stored.is_some()
        {
            let mut gpu_params_new = ParamsGPU::new(self.preallocate);
            if let Some(max_witness_stored) = self.max_witness_stored {
                gpu_params_new.with_max_witness_stored(max_witness_stored);
            }
            gpu_params = Some(gpu_params_new);
        }

        let inputs_str = self.inputs.clone().unwrap_or_else(|| "None".dimmed().to_string());
        print_banner_field("Input", inputs_str);

        if let Some(hints) = &self.hints {
            print_banner_field("Prec. Hints", hints);
        }

        if self.plonk && self.minimal {
            anyhow::bail!("Compressed proofs are not supported for PLONK generation.");
        }

        let stdin = ZiskStdin::from_uri(self.inputs.as_ref())?;

        let hints_stream = match self.hints.as_ref() {
            Some(uri) => {
                let stream = StreamSource::from_uri(uri)?;
                if matches!(stream, StreamSource::Quic(_)) {
                    anyhow::bail!("QUIC hints source is not supported in CLI mode.");
                }
                Some(stream)
            }
            None => None,
        };

        let emulator = if cfg!(target_os = "macos") {
            if !self.emulator {
                warn!("Emulator mode is forced on macOS due to lack of ASM support.");
            }
            true
        } else {
            self.emulator
        };

        let (result, world_rank) = if emulator {
            self.run_emu(stdin, gpu_params)?
        } else {
            self.run_asm(stdin, hints_stream, gpu_params)?
        };

        if world_rank == 0 {
            info!("{}", "--- PROVE SUMMARY ------------------------".bright_green().bold());

            if let Some(proof_id) = &result.get_proof_id() {
                let output_dir = match result.get_proof() {
                    ZiskProof::VadcopFinal(_) | ZiskProof::VadcopFinalCompressed(_) => {
                        match self.output.clone() {
                            Some(path) => path,
                            None => PathBuf::from("vadcop_final_proof.bin"),
                        }
                    }
                    ZiskProof::Plonk(_) | ZiskProof::Fflonk(_) => {
                        match self.output.clone() {
                            Some(path) => path,
                            None => PathBuf::from("final_plonk_proof.bin"),
                        }
                    }
                    _ => {
                        return Err(anyhow::anyhow!("Unsupported proof type for saving proof file"))
                    }
                };
                result.save_proof_with_publics(output_dir)?;
                info!("Proof ID: {}", proof_id);
                info!("Proof Time: {:.3} seconds", result.duration.as_secs_f64());
            }
            print_execution_summary(
                &result.executor_summary.executor_time,
                result.duration,
                result.executor_summary.steps,
            );
        }

        Ok(())
    }

    pub fn run_emu(
        &mut self,
        stdin: ZiskStdin,
        gpu_params: Option<ParamsGPU>,
    ) -> Result<(ZiskProveResult, i32)> {
        let prover = ProverClient::builder()
            .aggregation(self.aggregation)
            .proving_key_path_opt(self.proving_key.clone())
            .proving_key_snark_path_opt(self.proving_key_plonk.clone())
            .verbose(self.verbose)
            .shared_tables(self.shared_tables)
            .with_snark(self.plonk)
            .gpu(gpu_params)
            .print_command_info()
            .build()?;

        let elf_path = resolve_elf_path(&self.elf)?;
        let elf = ElfBinaryFromFile::new(elf_path, false)?;
        let (pk, _) = prover.setup(&elf)?;

        let proof_options = ProofOpts {
            aggregation: self.aggregation,
            rma: self.rma,
            minimal_memory: self.minimal_memory,
            verify_proofs: self.verify_proofs,
            save_proofs: self.save_proofs,
            output_dir_path: None, // TODO: Review this
        };

        let world_rank = prover.world_rank();

        let mut prover = prover.prove(&pk, stdin).with_proof_options(proof_options);
        if self.plonk {
            prover = prover.plonk();
        }
        if self.minimal {
            prover = prover.compressed();
        }
        let result = prover.run()?;

        Ok((result, world_rank))
    }

    pub fn run_asm(
        &mut self,
        stdin: ZiskStdin,
        hints_stream: Option<StreamSource>,
        gpu_params: Option<ParamsGPU>,
    ) -> Result<(ZiskProveResult, i32)> {
        let prover = ProverClient::builder()
            .aggregation(self.aggregation)
            .asm()
            .proving_key_path_opt(self.proving_key.clone())
            .proving_key_snark_path_opt(self.proving_key_plonk.clone())
            .verbose(self.verbose)
            .with_snark(self.plonk)
            .shared_tables(self.shared_tables)
            .asm_path_opt(self.asm.clone())
            .base_port_opt(self.port)
            .no_auto_setup(self.no_auto_setup)
            .unlock_mapped_memory(self.unlock_mapped_memory)
            .asm_out_file(self.asm_out_file)
            .gpu(gpu_params)
            .print_command_info()
            .build()?;

        let elf_path = resolve_elf_path(&self.elf)?;
        let elf = ElfBinaryFromFile::new(elf_path, hints_stream.is_some())?;
        let (pk, _) = prover.setup(&elf)?;

        let proof_options = ProofOpts {
            aggregation: self.aggregation,
            rma: self.rma,
            minimal_memory: self.minimal_memory,
            verify_proofs: self.verify_proofs,
            save_proofs: self.save_proofs,
            output_dir_path: None, // TODO: Review this
        };

        if let Some(hints_stream) = hints_stream {
            pk.register_hints_stream(hints_stream)?;
        }

        let world_rank = prover.world_rank();

        let mut prover = prover.prove(&pk, stdin).with_proof_options(proof_options);
        if self.plonk {
            prover = prover.plonk();
        }
        if self.minimal {
            prover = prover.compressed();
        }

        let result = prover.run()?;

        Ok((result, world_rank))
    }
}
