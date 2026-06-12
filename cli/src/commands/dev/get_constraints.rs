use anyhow::Result;
use colored::Colorize;
use std::path::PathBuf;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_common::ZiskPaths;

use fields::Goldilocks;

use proofman_common::{
    get_constraints_lines_str, get_global_constraints_lines_str, GlobalInfo, ProofType, SetupCtx,
};

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// List every constraint (index + PIL source line) of every AIR in the
/// proving key, plus the global constraints. The indices are the
/// `constraint_id`s reported by constraint verification, so this is the
/// lookup table for writing unit tests that pin a specific constraint.
pub(crate) struct GetConstraintsCmd {
    /// Path to a precomputed proving key
    #[arg(short = 'k', long)]
    proving_key: Option<PathBuf>,

    /// Only print the named AIRs (exact name, case-insensitive). Repeat the
    /// flag or separate with commas: `--air ArithEq --air Binary`,
    /// `--air ArithEq,Binary`
    #[arg(short = 'a', long, value_delimiter = ',')]
    air: Vec<String>,
}

impl GetConstraintsCmd {
    pub(crate) fn run(&self) -> Result<()> {
        println!("{} GetConstraints", format!("{: >12}", "Command").bright_green().bold());
        println!();

        let proving_key = ZiskPaths::get_proving_key(self.proving_key.as_ref());

        let global_info = GlobalInfo::new(&proving_key)
            .map_err(|e| anyhow::anyhow!("Error loading global info: {}", e))?;
        let sctx: SetupCtx<Goldilocks> =
            SetupCtx::new(&global_info, &ProofType::Basic, false, &[], false)
                .map_err(|e| anyhow::anyhow!("Error loading setup: {}", e))?;

        for airgroup_id in 0..global_info.air_groups.len() {
            for air_id in 0..global_info.airs[airgroup_id].len() {
                let air_name = &global_info.airs[airgroup_id][air_id].name;
                if !self.air.is_empty()
                    && !self.air.iter().any(|f| f.eq_ignore_ascii_case(air_name))
                {
                    continue;
                }

                println!(
                    "{}",
                    format!(
                        "    ► Constraints of {} - {} [{airgroup_id}:{air_id}]",
                        global_info.air_groups[airgroup_id], air_name,
                    )
                    .bright_white()
                    .bold()
                );
                let constraints_lines = get_constraints_lines_str(&sctx, airgroup_id, air_id)
                    .map_err(|e| anyhow::anyhow!("Error getting constraints: {}", e))?;
                for (idx, line) in constraints_lines.iter().enumerate() {
                    println!("        · Constraint #{idx} : {line}");
                }
            }
        }

        // The global constraints are AIR-independent; skip them when the
        // user asked for specific AIRs.
        if self.air.is_empty() {
            let global_constraints_lines = get_global_constraints_lines_str(&sctx);

            println!("{}", "    ► Global Constraints".bright_white().bold());
            for (idx, line) in global_constraints_lines.iter().enumerate() {
                println!("        · Global Constraint #{idx} -> {line}");
            }
        }

        Ok(())
    }
}
