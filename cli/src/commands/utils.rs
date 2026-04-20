use anyhow::Result;
use zisk_build::ZISK_VERSION_MESSAGE;

mod clean_cache;
mod convert_input;

pub use clean_cache::ZiskCleanCache;
pub use convert_input::ZiskConvertInput;

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Utility commands
pub struct ZiskUtils {
    #[command(subcommand)]
    pub command: ZiskUtilsCommand,
}

#[derive(clap::Subcommand)]
pub enum ZiskUtilsCommand {
    /// Clean the zisk cache for a specific program or remove the entire cache
    CleanCache(ZiskCleanCache),
    /// Convert old input files to new ZiskStdin format
    ConvertInput(ZiskConvertInput),
}

impl ZiskUtils {
    pub fn run(&mut self) -> Result<()> {
        match &mut self.command {
            ZiskUtilsCommand::CleanCache(cmd) => cmd.run(),
            ZiskUtilsCommand::ConvertInput(cmd) => cmd.run(),
        }
    }
}
