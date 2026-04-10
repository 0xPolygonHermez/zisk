use crate::ux::{print_banner, print_banner_command, print_banner_field, print_execution_summary};
use anyhow::Result;

use colored::Colorize;
use std::path::PathBuf;
use tracing::{info, warn};
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_common::io::{StreamSource, ZiskStdin};
use zisk_common::{ProofMode, ZiskProof};
use zisk_prover_backend::GuestProgram;
use zisk_prover_backend::{AsmOptions, ProverClientBuilder, ProverOpts, ZiskProveResult};

// Structure representing the 'prove' subcommand of cargo.
#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
#[command(propagate_version = true)]
#[command(group(
    clap::ArgGroup::new("input_mode")
        .args(["asm", "emulator"])
        .multiple(false)
        .required(false)
))]
pub struct ZiskProve {
    /// ELF file path
    /// This is the path to the ROM file that the witness computation dynamic library will use
    /// to generate the witness.
    #[clap(short = 'e', long)]
    pub elf: PathBuf,

    /// ASM file path
    /// Optional, mutually exclusive with `--emulator`
    #[clap(short = 's', long)]
    pub asm: Option<PathBuf>,

    /// Use prebuilt emulator (mutually exclusive with `--asm`)
    #[clap(short = 'l', long, action = clap::ArgAction::SetTrue)]
    pub emulator: bool,

    /// Input path
    #[clap(short = 'i', long, alias = "input", conflicts_with = "hints")]
    pub inputs: Option<String>,

    /// Precompiles Hints path
    #[clap(short = 'H', long, conflicts_with = "inputs")]
    pub hints: Option<String>,

    /// Setup folder path
    #[clap(short = 'k', long)]
    pub proving_key: Option<PathBuf>,

    /// Setup folder path for SNARK
    #[clap(short = 'w', long)]
    pub proving_key_snark: Option<PathBuf>,

    /// Output dir path
    #[clap(short = 'o', long, default_value = "tmp")]
    pub output_dir: PathBuf,

    #[clap(short = 'a', long, default_value_t = false)]
    pub aggregation: bool,

    #[clap(short = 'c', long, default_value_t = false)]
    pub minimal: bool,

    #[clap(short = 'y', long, default_value_t = false)]
    pub verify_proofs: bool,

    #[clap(short = 'z', long, default_value_t = false)]
    pub preallocate: bool,

    /// Base port for Assembly microservices (default: 23115).
    /// A single execution will use 3 consecutive ports, from this port to port + 2.
    /// If you are running multiple instances of ZisK using mpi on the same machine,
    /// it will use from this base port to base port + 2 * number_of_instances.
    /// For example, if you run 2 mpi instances of ZisK, it will use ports from 23115 to 23117
    /// for the first instance, and from 23118 to 23120 for the second instance.
    #[clap(short = 'p', long, conflicts_with = "emulator")]
    pub port: Option<u16>,

    /// Map unlocked flag
    /// This is used to unlock the memory map for the ROM file.
    /// If you are running ZisK on a machine with limited memory, you may want to enable this option.
    /// This option is mutually exclusive with `--emulator`.
    #[clap(short = 'u', long, conflicts_with = "emulator")]
    pub unlock_mapped_memory: bool,

    /// Redirect ASM emulator output to file
    /// This option is mutually exclusive with `--emulator`
    #[clap(long, conflicts_with = "emulator", default_value_t = false)]
    pub asm_out_file: bool,

    /// Verbosity (-v, -vv)
    #[arg(short ='v', long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8, // Using u8 to hold the number of `-v`

    #[clap(short = 't', long)]
    pub max_streams: Option<usize>,

    #[clap(short = 'h', long)]
    pub number_threads_witness: Option<usize>,

    #[clap(short = 'x', long)]
    pub max_witness_stored: Option<usize>,

    #[clap(short = 'm', long, default_value_t = false)]
    pub minimal_memory: bool,

    #[clap(short = 'j', long, default_value_t = false)]
    pub no_shared_tables_mpi: bool,

    #[clap(short = 'r', long, default_value_t = false)]
    pub no_rma_mpi: bool,

    #[clap(short = 'n', long, default_value_t = false)]
    pub no_auto_setup: bool,

    #[clap(long, default_value_t = false)]
    pub snark: bool,

    #[clap(short = 'g', long, default_value_t = false)]
    pub gpu: bool,
}

impl ZiskProve {
    pub fn run(&mut self) -> Result<()> {
        // Check if the deprecated alias was used
        if std::env::args().any(|arg| arg == "--input") {
            eprintln!("{}", "Warning: --input is deprecated, use --inputs instead".yellow().bold());
        }

        print_banner();

        print_banner_command("Prove");

        print_banner_field("Elf", self.elf.display());

        let inputs_str = self.inputs.clone().unwrap_or_else(|| "None".dimmed().to_string());
        print_banner_field("Input", inputs_str);

        if let Some(hints) = &self.hints {
            print_banner_field("Prec. Hints", hints);
        }

        if self.snark && self.minimal {
            anyhow::bail!("Minimal proofs are not supported for SNARK generation.");
        }

        // Build ProverOpts once with all configuration
        let mut prover_options = ProverOpts::default()
            .aggregation(self.aggregation)
            .rma(!self.no_rma_mpi)
            .output_dir(self.output_dir.clone())
            .shared_tables(!self.no_shared_tables_mpi)
            .verbose(self.verbose);

        if self.minimal_memory {
            prover_options = prover_options.minimal_memory();
        }
        if self.verify_proofs {
            prover_options = prover_options.verify_proofs();
        }
        if self.preallocate {
            prover_options = prover_options.preallocate();
        }
        if self.gpu {
            prover_options = prover_options.gpu();
        }
        if self.snark {
            prover_options = prover_options.preload_plonk();
        }
        if let Some(ref path) = self.proving_key {
            prover_options = prover_options.proving_key(path.clone());
        }
        if let Some(ref path) = self.proving_key_snark {
            prover_options = prover_options.proving_key_snark(path.clone());
        }
        if let Some(max) = self.max_witness_stored {
            prover_options = prover_options.max_witness_stored(max);
        }
        if let Some(threads) = self.number_threads_witness {
            prover_options = prover_options.number_threads_witness(threads);
        }
        if let Some(max) = self.max_streams {
            prover_options = prover_options.max_streams(max);
        }

        // ASM-specific options (only used if not emulator)
        let mut asm_options = AsmOptions::default();
        if let Some(ref path) = self.asm {
            asm_options = asm_options.asm_path(path.clone());
        }
        if let Some(port) = self.port {
            asm_options = asm_options.base_port(port);
        }
        if self.no_auto_setup {
            asm_options = asm_options.no_auto_setup();
        }
        if self.unlock_mapped_memory {
            asm_options = asm_options.unlock_mapped_memory();
        }
        if self.asm_out_file {
            asm_options = asm_options.asm_out_file();
        }
        prover_options = prover_options.with_asm_options(asm_options);

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
            self.run_emu(stdin, prover_options)?
        } else {
            self.run_asm(stdin, hints_stream, prover_options)?
        };

        if world_rank == 0 {
            info!("{}", "--- PROVE SUMMARY ------------------------".bright_green().bold());

            if let Some(proof_id) = &result.get_proof_id() {
                let output_dir = match result.get_proof().proof {
                    ZiskProof::VadcopFinal(_) | ZiskProof::VadcopFinalMinimal(_) => {
                        self.output_dir.join("vadcop_final_proof.bin")
                    }
                    ZiskProof::Plonk(_) => self.output_dir.join("final_snark_proof.bin"),
                    _ => {
                        return Err(anyhow::anyhow!("Unsupported proof type for saving proof file"))
                    }
                };
                result.save_proof(output_dir)?;
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
        prover_options: ProverOpts,
    ) -> Result<(ZiskProveResult, i32)> {
        let prover =
            ProverClientBuilder::new().emu().with_prover_options(prover_options).build()?;

        let guest_program = GuestProgram::from_uri(self.elf.to_str().unwrap())?;
        prover.setup(&guest_program).run()?;

        let world_rank = prover.world_rank();

        let mut prover = prover.prove(&guest_program, stdin);
        if self.snark {
            prover = prover.wrap(ProofMode::Plonk);
        }
        if self.minimal {
            prover = prover.wrap(ProofMode::VadcopFinalMinimal);
        }
        let result = prover.run()?;

        Ok((result, world_rank))
    }

    pub fn run_asm(
        &mut self,
        stdin: ZiskStdin,
        hints_stream: Option<StreamSource>,
        prover_options: ProverOpts,
    ) -> Result<(ZiskProveResult, i32)> {
        let prover =
            ProverClientBuilder::new().asm().with_prover_options(prover_options).build()?;

        let guest_program = GuestProgram::from_uri(self.elf.to_str().unwrap())?;
        if hints_stream.is_some() {
            prover.setup(&guest_program).with_hints().run()?;
        } else {
            prover.setup(&guest_program).run()?;
        }

        if let Some(hints_stream) = hints_stream {
            prover.register_hints_stream(hints_stream)?;
        }

        let world_rank = prover.world_rank();

        let mut prover = prover.prove(&guest_program, stdin);
        if self.snark {
            prover = prover.wrap(ProofMode::Plonk);
        }
        if self.minimal {
            prover = prover.wrap(ProofMode::VadcopFinalMinimal);
        }

        let result = prover.run()?;

        Ok((result, world_rank))
    }
}
