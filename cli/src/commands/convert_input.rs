use anyhow::{bail, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::ux::print_banner;
use crate::ux::print_banner_field;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_common::io::ZiskStdin;
use zisk_prover_backend::setup_logger;

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Convert old input files to new ZiskStdin format
pub struct ZiskConvertInput {
    /// Input file to convert
    #[arg(short = 'i', long)]
    pub input_file: Option<PathBuf>,

    /// Output file path
    #[arg(short = 'o', long)]
    pub output_file: Option<PathBuf>,

    /// Input directory containing files to convert
    #[arg(short = 'd', long)]
    pub input_dir: Option<PathBuf>,

    /// Output directory for converted files
    #[arg(short = 't', long)]
    pub output_dir: Option<PathBuf>,

    /// Process subdirectories recursively
    #[arg(short = 'r', long)]
    pub recursive: bool,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

impl ZiskConvertInput {
    pub fn run(&self) -> Result<()> {
        setup_logger(self.verbose.into());

        print_banner();
        print_banner_field("Command", "Convert Input");

        // Validate arguments
        let use_files = self.input_file.is_some() || self.output_file.is_some();
        let use_dirs = self.input_dir.is_some() || self.output_dir.is_some();

        if use_files && use_dirs {
            bail!("Cannot use both file and directory modes. Use either -i/-o or --input-dir/--output-dir");
        }

        if use_files {
            // File mode - both input and output files must be provided
            let input_file = self.input_file.as_ref().ok_or_else(|| {
                anyhow::anyhow!("Input file (-i) is required when using file mode")
            })?;
            let output_file = self.output_file.as_ref().ok_or_else(|| {
                anyhow::anyhow!("Output file (-o) is required when using file mode")
            })?;

            print_banner_field("Input File", input_file.display());
            print_banner_field("Output File", output_file.display());

            self.convert_file(input_file, output_file)?;
        } else if use_dirs {
            // Directory mode - both input and output dirs must be provided
            let input_dir = self.input_dir.as_ref().ok_or_else(|| {
                anyhow::anyhow!(
                    "Input directory (--input-dir) is required when using directory mode"
                )
            })?;
            let output_dir = self.output_dir.as_ref().ok_or_else(|| {
                anyhow::anyhow!(
                    "Output directory (--output-dir) is required when using directory mode"
                )
            })?;

            print_banner_field("Input Directory", input_dir.display());
            print_banner_field("Output Directory", output_dir.display());
            print_banner_field("Recursive", if self.recursive { "Yes" } else { "No" });

            self.convert_directory(input_dir, output_dir)?;
        } else {
            bail!(
                "Either file mode (-i/-o) or directory mode (--input-dir/--output-dir) is required"
            );
        }

        println!("\n✓ Input conversion completed successfully!");

        Ok(())
    }

    fn convert_file(&self, input_path: &PathBuf, output_path: &Path) -> Result<()> {
        println!("Converting: {} -> {}", input_path.display(), output_path.display());

        let input = std::fs::read(input_path)?;
        let zisk_stdin = ZiskStdin::new();
        zisk_stdin.write_slice(&input);
        zisk_stdin.save(output_path)?;

        Ok(())
    }

    fn convert_directory(&self, input_dir: &PathBuf, output_dir: &PathBuf) -> Result<()> {
        if !input_dir.is_dir() {
            bail!("Input directory does not exist or is not a directory: {}", input_dir.display());
        }

        fs::create_dir_all(output_dir)?;

        let mut files_converted = 0;

        if self.recursive {
            self.convert_directory_recursive(
                input_dir,
                output_dir,
                input_dir,
                &mut files_converted,
            )?;
        } else {
            self.convert_directory_flat(input_dir, output_dir, &mut files_converted)?;
        }

        println!("\n✓ Converted {} file(s)", files_converted);

        Ok(())
    }

    fn convert_directory_flat(
        &self,
        input_dir: &PathBuf,
        output_dir: &Path,
        files_converted: &mut usize,
    ) -> Result<()> {
        for entry in fs::read_dir(input_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let file_name = path.file_name().ok_or_else(|| {
                    anyhow::anyhow!("Failed to get filename for: {}", path.display())
                })?;
                let output_path = output_dir.join(file_name);

                self.convert_file(&path, &output_path)?;
                *files_converted += 1;
            }
        }

        Ok(())
    }

    fn convert_directory_recursive(
        &self,
        current_dir: &PathBuf,
        output_base: &PathBuf,
        input_base: &PathBuf,
        files_converted: &mut usize,
    ) -> Result<()> {
        for entry in fs::read_dir(current_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                // Compute relative path from input base
                let relative_path = path
                    .strip_prefix(input_base)
                    .map_err(|_| anyhow::anyhow!("Failed to compute relative path"))?;
                let output_path = output_base.join(relative_path);

                // Create parent directory if needed
                if let Some(parent) = output_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                self.convert_file(&path, &output_path)?;
                *files_converted += 1;
            } else if path.is_dir() {
                self.convert_directory_recursive(&path, output_base, input_base, files_converted)?;
            }
        }

        Ok(())
    }
}
