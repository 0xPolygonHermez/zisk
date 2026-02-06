use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use crate::{commands::get_proving_key, ux::print_banner};
use colored::Colorize;
use fields::Goldilocks;
use proofman_common::{
    initialize_logger, MpiCtx, ParamsGPU, ProofCtx, ProofType, SetupCtx, SetupsVadcop,
};
use rom_setup::gen_assembly;
use rom_setup::rom_merkle_setup;
use std::fs;
use std::sync::Arc;
use zisk_common::ElfBinaryOwned;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct ZiskRomSetup {
    /// ELF file path
    #[clap(short = 'e', long)]
    pub elf: PathBuf,

    /// Setup folder path
    #[clap(short = 'k', long)]
    pub proving_key: Option<PathBuf>,

    /// Setup folder path
    #[clap(short = 'z', long)]
    pub zisk_path: Option<PathBuf>,

    /// Output dir path
    #[clap(short = 'o', long)]
    pub output_dir: Option<PathBuf>,

    /// Enable precompile hints in assembly generation
    #[clap(short = 'n', long, default_value_t = false)]
    pub hints: bool,

    #[arg(short, long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8,
}

impl ZiskRomSetup {
    pub fn run(&self) -> Result<()> {
        initialize_logger(proofman_common::VerboseMode::Info, None);

        tracing::info!(
            "{}",
            format!("{} Rom Setup", format!("{: >12}", "Command").bright_green().bold())
        );
        tracing::info!("");

        print_banner();

        let proving_key = get_proving_key(self.proving_key.as_ref());

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
        let elf_bytes = fs::read(&self.elf)
            .map_err(|e| anyhow::anyhow!("Error reading ELF file {}: {}", self.elf.display(), e))?;
        let elf = ElfBinaryOwned::new(
            elf_bytes,
            self.elf.file_stem().unwrap().to_str().unwrap().to_string(),
            self.hints,
        );
        rom_merkle_setup::<Goldilocks>(&pctx, &elf, &self.output_dir)?;

        gen_assembly(&self.elf, &self.zisk_path, &self.output_dir, self.hints, self.verbose > 0)?;

        println!();
        tracing::info!("{}", "ROM setup successfully completed".bright_green().bold());
        Ok(())
    }
}
