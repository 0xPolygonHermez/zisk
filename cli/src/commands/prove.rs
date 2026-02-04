use crate::ux::{print_banner, print_banner_field};
use anyhow::Result;

use colored::Colorize;
use proofman::{get_vadcop_final_proof_vkey, SnarkProof, SnarkProtocol};
use proofman_common::ParamsGPU;
use proofman_util::VadcopFinalProof;
use std::fs;
use std::path::PathBuf;
use tracing::warn;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_common::io::{StreamSource, ZiskStdin};
use zisk_common::ElfBinaryOwned;
#[cfg(feature = "stats")]
use zisk_common::ExecutorStatsEvent;
use zisk_sdk::ZiskProgramVK;
use zisk_sdk::{get_proving_key, ProofOpts, ProverClient, ZiskProof, ZiskProveResult};

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
    #[clap(short = 'i', long, alias = "input")]
    pub inputs: Option<String>,

    /// Precompiles Hints path
    #[clap(long)]
    pub hints: Option<String>,

    /// Setup folder path
    #[clap(short = 'k', long)]
    pub proving_key: Option<PathBuf>,

    /// Output dir path
    #[clap(short = 'o', long, default_value = "tmp")]
    pub output_dir: PathBuf,

    #[clap(short = 'a', long, default_value_t = false)]
    pub aggregation: bool,

    #[clap(short = 'c', long, default_value_t = false)]
    pub compressed: bool,

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

    /// Verbosity (-v, -vv)
    #[arg(short ='v', long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8, // Using u8 to hold the number of `-v`

    #[clap(short = 't', long)]
    pub max_streams: Option<usize>,

    #[clap(short = 'n', long)]
    pub number_threads_witness: Option<usize>,

    #[clap(short = 'x', long)]
    pub max_witness_stored: Option<usize>,

    #[clap(short = 'b', long, default_value_t = false)]
    pub save_proofs: bool,

    #[clap(short = 'm', long, default_value_t = false)]
    pub minimal_memory: bool,

    #[clap(short = 'j', long, default_value_t = false)]
    pub shared_tables: bool,

    #[clap(short = 'r', long, default_value_t = false)]
    pub rma: bool,
}

impl ZiskProve {
    pub fn run(&mut self) -> Result<()> {
        // Check if the deprecated alias was used
        if std::env::args().any(|arg| arg == "--input") {
            eprintln!("{}", "Warning: --input is deprecated, use --inputs instead".yellow().bold());
        }

        print_banner();

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

        if let Some(inputs) = &self.inputs {
            print_banner_field("Input", inputs);
        }

        if let Some(hints) = &self.hints {
            print_banner_field("Prec. Hints", hints);
        }

        let stdin = ZiskStdin::from_uri(self.inputs.as_ref())?;

        let hints_stream = match self.hints.as_ref() {
            Some(uri) => {
                let stream = StreamSource::from_uri(uri)?;
                if matches!(stream, StreamSource::Quic(_)) {
                    anyhow::bail!("QUIC hints source is not supported for execution.");
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

        let (vk, result, world_rank) = if emulator {
            self.run_emu(stdin, gpu_params)?
        } else {
            self.run_asm(stdin, hints_stream, gpu_params)?
        };

        if world_rank == 0 {
            let elapsed = result.duration.as_secs_f64();
            tracing::info!("");
            tracing::info!(
                "{}",
                "--- PROVE SUMMARY ------------------------".bright_green().bold()
            );

            match result.proof {
                ZiskProof::VadcopFinal(ref proof) | ZiskProof::VadcopFinalCompressed(ref proof) => {
                    let compressed = matches!(result.proof, ZiskProof::VadcopFinalCompressed(_));
                    let vadcop_final_proof = VadcopFinalProof::new(
                        proof.clone(),
                        result.publics.bytes_u64(&vk),
                        compressed,
                    );
                    vadcop_final_proof
                        .save(self.output_dir.join("vadcop_final_proof.bin"))
                        .map_err(|e| {
                            anyhow::anyhow!(
                                "Failed to save VadcopFinalProof to output dir {:?}: {}",
                                self.output_dir.join("vadcop_final_proof.bin").display(),
                                e
                            )
                        })?;
                }
                ZiskProof::Plonk(ref proof) | ZiskProof::Fflonk(ref proof) => {
                    let protocol_id = match result.proof {
                        ZiskProof::Plonk(_) => SnarkProtocol::Plonk.protocol_id(),
                        ZiskProof::Fflonk(_) => SnarkProtocol::Fflonk.protocol_id(),
                        _ => unreachable!(),
                    };
                    let proving_key = get_proving_key(self.proving_key.as_ref());
                    let vadcop_verkey = get_vadcop_final_proof_vkey(&proving_key, false)?;

                    let snark_proof = SnarkProof {
                        proof_bytes: proof.clone(),
                        public_bytes: result.publics.bytes_solidity(&vk, &vadcop_verkey),
                        public_snark_bytes: result.publics.hash_solidity(&vk, &vadcop_verkey),
                        protocol_id,
                    };
                    snark_proof.save(self.output_dir.join("snark_proof.bin")).map_err(|e| {
                        anyhow::anyhow!(
                            "Failed to save SnarkProof to output dir {:?}: {}",
                            self.output_dir.join("snark_proof.bin").display(),
                            e
                        )
                    })?;
                }
                ZiskProof::Null() => {}
            }

            if let Some(proof_id) = &result.proof_id {
                tracing::info!("      Proof ID: {}", proof_id);
            }
            tracing::info!("    ► Statistics");
            tracing::info!("      time: {} seconds, steps: {}", elapsed, result.execution.steps);
        }

        Ok(())
    }

    pub fn run_emu(
        &mut self,
        stdin: ZiskStdin,
        gpu_params: Option<ParamsGPU>,
    ) -> Result<(ZiskProgramVK, ZiskProveResult, i32)> {
        let prover = ProverClient::builder()
            .aggregation(self.aggregation)
            .proving_key_path_opt(self.proving_key.clone())
            .verbose(self.verbose)
            .shared_tables(self.shared_tables)
            .gpu(gpu_params)
            .print_command_info()
            .build()?;

        let elf_bin = fs::read(&self.elf)
            .map_err(|e| anyhow::anyhow!("Error reading ELF file {}: {}", self.elf.display(), e))?;
        let elf = ElfBinaryOwned::new(
            elf_bin,
            self.elf.file_stem().unwrap().to_str().unwrap().to_string(),
            false,
        );
        let vk = prover.setup(&elf)?;

        let proof_options = ProofOpts {
            aggregation: self.aggregation,
            rma: self.rma,
            minimal_memory: self.minimal_memory,
            verify_proofs: self.verify_proofs,
            save_proofs: self.save_proofs,
            output_dir_path: Some(self.output_dir.clone()),
        };

        let result = prover.prove(stdin).with_proof_options(proof_options).run()?;
        let world_rank = prover.world_rank();

        Ok((vk, result, world_rank))
    }

    pub fn run_asm(
        &mut self,
        stdin: ZiskStdin,
        hints_stream: Option<StreamSource>,
        gpu_params: Option<ParamsGPU>,
    ) -> Result<(ZiskProgramVK, ZiskProveResult, i32)> {
        let prover = ProverClient::builder()
            .aggregation(self.aggregation)
            .asm()
            .proving_key_path_opt(self.proving_key.clone())
            .verbose(self.verbose)
            .shared_tables(self.shared_tables)
            .asm_path_opt(self.asm.clone())
            .base_port_opt(self.port)
            .unlock_mapped_memory(self.unlock_mapped_memory)
            .gpu(gpu_params)
            .print_command_info()
            .build()?;

        let elf_bin = fs::read(&self.elf)
            .map_err(|e| anyhow::anyhow!("Error reading ELF file {}: {}", self.elf.display(), e))?;
        let elf = ElfBinaryOwned::new(
            elf_bin,
            self.elf.file_stem().unwrap().to_str().unwrap().to_string(),
            hints_stream.is_some(),
        );
        let vk = prover.setup(&elf)?;

        let proof_options = ProofOpts {
            aggregation: self.aggregation,
            rma: self.rma,
            minimal_memory: self.minimal_memory,
            verify_proofs: self.verify_proofs,
            save_proofs: self.save_proofs,
            output_dir_path: Some(self.output_dir.clone()),
        };

        if let Some(hints_stream) = hints_stream {
            prover.set_hints_stream(hints_stream)?;
        }
        let result = prover.prove(stdin).with_proof_options(proof_options).run()?;
        let world_rank = prover.world_rank();

        Ok((vk, result, world_rank))
    }
}
