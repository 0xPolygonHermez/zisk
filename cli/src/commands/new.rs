// use clap::Parser;
// use colored::Colorize;
// use proofman::command_handlers::trace_setup_handler::trace_setup_handler;
// use tinytemplate::TinyTemplate;
// use std::path::{Path, PathBuf};
// use std::fs;
// use serde::Serialize;
// use pilout::pilout_proxy::PilOutProxy;
// use convert_case::{Case, Casing};

// #[derive(Parser)]
// #[command(version, about, long_about = None)]
// #[command(propagate_version = true)]
// pub struct NewCmd {
//     /// Name of the new project
//     pub name: String,

//     /// Proofman configuration file path
//     #[clap(short, long)]
//     pub pilout: PathBuf,
// }

// #[derive(Debug, Serialize)]
// struct Context {
//     project_name: String,
//     pilout_filename: String,
//     wc: Vec<WCContext>,
// }

// #[derive(Debug, Serialize)]
// struct WCContext {
//     subproof_id: usize,
//     name: String,
//     snake_name: String,
//     airs: Vec<AirContext>,
// }

// #[derive(Debug, Serialize)]
// struct AirContext {
//     name: String,
// }

// impl NewCmd {
//     pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
//         println!("{} {}", format!("{: >12}", "Command").bright_green().bold(), "New project");
//         println!("");

//         if let Ok(metadata) = fs::metadata(&self.pilout) {
//             if !metadata.is_file() {
//                 println!("Path exists, but it is not a file: {}", self.pilout.display());
//                 Err("Path exists, but it is not a file")?;
//             }
//         } else {
//             println!("Pilout file does not exist: {}", self.pilout.display());
//             Err("Pilout file does not exist")?;
//         }

//         let pilout = PilOutProxy::new(&self.pilout.display().to_string(), false)?;

//         let root_folder = Path::new(&self.name);
//         if root_folder.exists() {
//             print!("Path already exists. Aborting...");
//             return Err("Path already exists")?;
//         }

//         println!("Creating new proofman project: {}", self.name);
//         println!("Using pilout file: {:?}", self.pilout);

//         let data_folder = root_folder.join("data");
//         let src_folder = root_folder.join("src");

//         // Create all the folders
//         fs::create_dir(&root_folder)?;
//         fs::create_dir(&data_folder)?;
//         fs::create_dir(&src_folder)?;
//         fs::create_dir(&src_folder.join("witness_computation"))?;

//         // Create the project root directory
//         const GIT_IGNORE: &str = include_str!("../../assets/templates/.gitignore");
//         const CARGO_TOML: &str = include_str!("../../assets/templates/Cargo.toml.tt");
//         const PROOFMAN_CONFIG_JSON: &str = include_str!("../../assets/templates/proofman.config.json.tt");
//         const MAIN_RS: &str = include_str!("../../assets/templates/main.rs.tt");
//         const MOD_RS: &str = include_str!("../../assets/templates/mod.rs.tt");
//         const WC_RS: &str = include_str!("../../assets/templates/witness_computation.rs.tt");

//         let mut tt = TinyTemplate::new();
//         tt.add_template("cargo.toml", CARGO_TOML)?;
//         tt.add_template("proofman.config.json", PROOFMAN_CONFIG_JSON)?;
//         tt.add_template("main.rs", MAIN_RS)?;
//         tt.add_template("wc.rs", WC_RS)?;

//         // get the wc, fill subproof_id with the  index inside subproofs
//         let mut wc = Vec::new();

//         for (index, subproof) in pilout.subproofs.iter().enumerate() {
//             let subproof_id = index;
//             wc.push(WCContext {
//                 subproof_id,
//                 name: subproof.name.as_ref().unwrap().clone().to_case(Case::Pascal),
//                 snake_name: subproof.name.as_ref().unwrap().clone().to_case(Case::Snake),
//                 airs: subproof.airs.iter().map(|air| AirContext { name: air.name.as_ref().unwrap().clone() }).collect(),
//             });
//         }

//         let context = Context {
//             project_name: self.name.clone(),
//             pilout_filename: self.pilout.file_name().unwrap().to_str().unwrap().to_string(),
//             wc,
//         };

//         // Create the root folder content
//         fs::write(root_folder.join(".gitignore"), GIT_IGNORE)?;
//         fs::write(root_folder.join("Cargo.toml"), tt.render("cargo.toml", &context)?)?;
//         fs::write(root_folder.join("proofman.config.json"), tt.render("proofman.config.json", &context)?)?;

//         // Create the data folder content
//         fs::copy(&self.pilout, data_folder.join(&context.pilout_filename))?;

//         // Create the src folder content
//         fs::write(src_folder.join("main.rs"), tt.render("main.rs", &context)?)?;
//         fs::write(src_folder.join("mod.rs"), MOD_RS)?;

//         // create src/witness_computation assets
//         let mut module = "".to_owned();
//         for wc in &context.wc {
//             let mut data = std::collections::HashMap::new();

//             data.insert("wc", wc);
//             fs::write(
//                 src_folder.join("witness_computation").join(format!("{}_wc.rs", wc.snake_name)),
//                 tt.render("wc.rs", &data)?,
//             )?;

//             let traces_content = trace_setup_handler(&pilout, wc.subproof_id)?;
//             fs::write(
//                 src_folder.join("witness_computation").join(format!("{}_traces.rs", wc.snake_name)),
//                 traces_content,
//             )?;

//             module += &format!("pub mod {}_wc;\n", wc.name.to_lowercase());
//             module += &format!("pub mod {}_traces;\n", wc.name.to_lowercase());
//         }
//         fs::write(src_folder.join("witness_computation").join("mod.rs".to_owned()), module)?;

//         Ok(())
//     }
// }
