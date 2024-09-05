// extern crate env_logger;
use clap::Parser;
use pilout::{pilout::SymbolType, pilout_proxy::PilOutProxy};
use serde::Serialize;
use tinytemplate::TinyTemplate;
use std::{fs, path::PathBuf};
use colored::Colorize;
use convert_case::{Case, Casing};

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct PilHelpersCmd {
    #[clap(long)]
    pub pilout: PathBuf,

    #[clap(long)]
    pub path: PathBuf,

    #[clap(short)]
    pub overide: bool,
}

#[derive(Serialize)]
struct ProofCtx {
    project_name: String,
    num_stages: u32,
    pilout_filename: String,
    air_groups: Vec<AirGroupsCtx>,
    constant_subproofs: Vec<(String, usize)>,
    constant_airs: Vec<(String, Vec<usize>, String)>,
}

#[derive(Debug, Serialize)]
struct AirGroupsCtx {
    subproof_id: usize,
    name: String,
    snake_name: String,
    airs: Vec<AirCtx>,
}

#[derive(Debug, Serialize)]
struct AirCtx {
    id: usize,
    name: String,
    num_rows: u32,
    columns: Vec<ColumnCtx>,
}

#[derive(Debug, Serialize)]
struct ColumnCtx {
    name: String,
    r#type: String,
}

impl PilHelpersCmd {
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("{} Pil-helpers", format!("{: >12}", "Command").bright_green().bold());
        println!();

        // Check if the pilout file exists
        if !self.pilout.exists() {
            return Err(format!("Pilout file '{}' does not exist", self.pilout.display()).into());
        }

        // Check if the path exists
        let pil_helpers_path = self.path.join("pil_helpers");
        if !pil_helpers_path.exists() {
            std::fs::create_dir_all(&pil_helpers_path)?;
        } else if !pil_helpers_path.is_dir() {
            return Err(format!("Path '{}' already exists and is not a folder", pil_helpers_path.display()).into());
        }

        let files = ["mod.rs", "pilout.rs"];

        if !self.overide {
            // Check if the files already exist and launch an error if they do
            for file in files.iter() {
                let dst = pil_helpers_path.join(file);
                if dst.exists() {
                    return Err(format!("{} already exists, skipping", dst.display()).into());
                }
            }
        }

        // Read the pilout file
        let pilout = PilOutProxy::new(&self.pilout.display().to_string())?;

        let mut wcctxs = Vec::new();
        let mut constant_subproofs: Vec<(String, usize)> = Vec::new();
        let mut constant_airs: Vec<(String, Vec<usize>, String)> = Vec::new();

        for (subproof_id, subproof) in pilout.subproofs.iter().enumerate() {
            wcctxs.push(AirGroupsCtx {
                subproof_id,
                name: subproof.name.as_ref().unwrap().clone().to_case(Case::Pascal),
                snake_name: subproof.name.as_ref().unwrap().clone().to_case(Case::Snake).to_uppercase(),
                airs: subproof
                    .airs
                    .iter()
                    .enumerate()
                    .map(|(air_id, air)| AirCtx {
                        id: air_id,
                        name: air.name.as_ref().unwrap().clone(),
                        num_rows: air.num_rows.unwrap(),
                        columns: Vec::new(),
                    })
                    .collect(),
            });

            // Prepare constants
            constant_subproofs
                .push((subproof.name.as_ref().unwrap().clone().to_case(Case::Snake).to_uppercase(), subproof_id));

            for (air_idx, air) in subproof.airs.iter().enumerate() {
                let air_name = air.name.as_ref().unwrap().clone().to_case(Case::Snake).to_uppercase();
                let contains_key = constant_airs.iter().position(|(name, _, _)| name == &air_name);

                let idx = contains_key.unwrap_or_else(|| {
                    constant_airs.push((air_name.clone(), Vec::new(), "".to_owned()));
                    constant_airs.len() - 1
                });

                constant_airs[idx].1.push(air_idx);
            }

            for constant in constant_airs.iter_mut() {
                constant.2 = constant.1.iter().map(|&num| num.to_string()).collect::<Vec<String>>().join(",");
            }
        }

        // Build columns data for traces
        for (subproof_id, subproof) in pilout.subproofs.iter().enumerate() {
            for (air_id, _) in subproof.airs.iter().enumerate() {
                // Search symbols where subproof_id == subproof_id && air_id == air_id && type == WitnessCol
                pilout
                    .symbols
                    .iter()
                    .filter(|symbol| {
                        symbol.subproof_id.is_some()
                            && symbol.subproof_id.unwrap() == subproof_id as u32
                            && symbol.air_id.is_some()
                            && symbol.air_id.unwrap() == air_id as u32
                            && symbol.stage.is_some()
                            && symbol.stage.unwrap() == 1
                            && symbol.r#type == SymbolType::WitnessCol as i32
                    })
                    .for_each(|symbol| {
                        let air = wcctxs[subproof_id].airs.get_mut(air_id).unwrap();
                        let name = symbol.name.split_once('.').map(|x| x.1).unwrap_or(&symbol.name);
                        let r#type =
                            if symbol.dim == 0 { "F".to_string() } else { format!("[F; {}]", symbol.lengths[0]) };
                        air.columns.push(ColumnCtx { name: name.to_owned(), r#type });
                    });
            }
        }

        let context = ProofCtx {
            project_name: pilout.name.clone().unwrap().to_case(Case::Pascal),
            num_stages: pilout.num_stages(),
            pilout_filename: self.pilout.file_name().unwrap().to_str().unwrap().to_string(),
            air_groups: wcctxs,
            constant_airs,
            constant_subproofs,
        };

        const MOD_RS: &str = include_str!("../../assets/templates/pil_helpers_mod.rs.tt");

        let mut tt = TinyTemplate::new();
        tt.add_template("mod.rs", MOD_RS)?;
        tt.add_template("pilout.rs", include_str!("../../assets/templates/pil_helpers_pilout.rs.tt"))?;
        tt.add_template("traces.rs", include_str!("../../assets/templates/pil_helpers_trace.rs.tt"))?;

        // Write the files
        // --------------------------------------------
        // Write mod.rs
        fs::write(pil_helpers_path.join("mod.rs"), MOD_RS)?;

        // Write pilout.rs
        fs::write(pil_helpers_path.join("pilout.rs"), tt.render("pilout.rs", &context)?)?;

        // Write traces.rs
        fs::write(pil_helpers_path.join("traces.rs"), tt.render("traces.rs", &context)?)?;

        Ok(())
    }
}
