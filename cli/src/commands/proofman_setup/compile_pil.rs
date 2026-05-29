use anyhow::Result;
use pil2_stark_setup::commands::compile_pil::{run_compile_pil, CompilePilOptions};
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_prover_backend::setup_logger;

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Compile a `.pil` source into a `.pilout` via the JS pil2-compiler.
pub struct ZiskProofmanCompilePil {
    /// Path to the entry `.pil` file
    #[arg(short = 'p', long = "pil")]
    pub pil_path: String,

    /// Output `.pilout` path
    #[arg(short = 'o', long = "output")]
    pub output_path: String,

    /// `-I` include search paths (repeat for multiple, or pass a comma-separated value)
    #[arg(short = 'I', long = "include", num_args = 1.., value_delimiter = ',')]
    pub include_paths: Vec<String>,

    /// `-u` directory for fixed columns
    #[arg(short = 'u', long = "fixed-dir")]
    pub fixed_dir: Option<String>,

    /// Pass `-O fixed-to-file` to write fixed columns to disk
    #[arg(long = "fixed-to-file")]
    pub fixed_to_file: bool,

    /// Pass `-O no-proto-fixed-data` to omit fixed-column values from the pilout
    /// protobuf. Use with `--fixed-dir` + `--fixed-to-file` to avoid the V8 heap
    /// blowup on huge PILs (e.g. zisk.pil).
    #[arg(long = "no-proto-fixed-data")]
    pub no_proto_fixed_data: bool,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

impl ZiskProofmanCompilePil {
    pub fn run(&self) -> Result<()> {
        setup_logger(self.verbose.into());

        let opts = CompilePilOptions {
            pil_path: self.pil_path.clone(),
            output_path: self.output_path.clone(),
            include_paths: self.include_paths.clone(),
            fixed_dir: self.fixed_dir.clone(),
            fixed_to_file: self.fixed_to_file,
            no_proto_fixed_data: self.no_proto_fixed_data,
        };
        run_compile_pil(&opts)
    }
}
