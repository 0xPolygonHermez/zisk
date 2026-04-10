use anyhow::Result;
use std::path::PathBuf;

use crate::ux::print_banner_field;
use crate::{commands::get_proving_key, ux::print_banner};
use colored::Colorize;
use fields::Goldilocks;
use proofman_common::{MpiCtx, ParamsGPU, ProofCtx, ProofType, SetupCtx, SetupsVadcop};
use rom_setup::gen_assembly;
use rom_setup::rom_merkle_setup;
use std::sync::Arc;
use zisk_common::ElfBinaryFromFile;
use zisk_sdk::{ZISK_VERSION_MESSAGE, setup_logger};

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Setup guest program
pub struct ZiskProgramSetup {
    /// Path to the program ELF file
    #[arg(short = 'e', long)] //Optional, mirar si existe Cargo.toml
    pub elf: PathBuf,

    /// Path to a precomputed proving key
    #[arg(short = 'k', long)]
    pub proving_key: Option<PathBuf>,

    /// Enable precompile hints in assembly generation
    #[arg(short = 'n', long, default_value_t = false)]
    pub hints: bool,

    /// Enable GPU acceleration in assembly generation
    #[clap(long, default_value_t = false)]
    pub gpu: bool,

    /// Verbose (-v, -vv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    // Hidden flags

    /// Output dir path
    #[arg(short = 'o', long, hide = true)]
    pub output_dir: Option<PathBuf>,
}

impl ZiskProgramSetup {
    pub fn run(&self) -> Result<()> {
        setup_logger(self.verbose.into());

        print_banner();

        print_banner_field("Command", "Rom Setup");
        print_banner_field("Elf", self.elf.display());
        if self.hints {
            print_banner_field("Hints", "Enabled".yellow());
        }

        let proving_key = get_proving_key(self.proving_key.as_ref());

        print_banner_field("Proving Key", proving_key.display());

        println!();

        let mpi_ctx = Arc::new(MpiCtx::new());
        let mut pctx = ProofCtx::create_ctx(proving_key, false, self.verbose.into(), mpi_ctx)?;

        let mut params_gpu = ParamsGPU::new(false);
        params_gpu.with_max_number_streams(1);

        let sctx = Arc::new(SetupCtx::<Goldilocks>::new(
            &pctx.global_info,
            &ProofType::Basic,
            false,
            &params_gpu,
            &[],
        ));
        let setups_vadcop =
            Arc::new(SetupsVadcop::new(&pctx.global_info, false, false, &params_gpu, &[]));
        pctx.set_device_buffers(&sctx, &setups_vadcop, false, &params_gpu)?;
        let pctx = Arc::new(pctx);

        tracing::info!("Computing setup for ROM {}", self.elf.display());

        tracing::info!("Computing merkle root");
        let elf = ElfBinaryFromFile::new(&self.elf, self.hints)?;
        rom_merkle_setup::<Goldilocks>(&pctx, &elf, &self.output_dir)?;

        gen_assembly(&self.elf, &self.output_dir, self.hints, self.verbose > 0)?;

        println!();
        tracing::info!("{}", "ROM setup successfully completed".bright_green().bold());
        Ok(())
    }
}
