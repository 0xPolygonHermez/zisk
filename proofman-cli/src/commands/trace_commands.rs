extern crate env_logger;
use clap::{Args, Parser, Subcommand};

use pilout::{pilout::SymbolType, pilout_proxy::PilOutProxy};
// use std::{fs, path::Path};

#[derive(Parser)]
pub struct Trace {
    #[command(subcommand)]
    pub trace_commands: TraceSubcommands,
}

#[derive(Subcommand)]
pub enum TraceSubcommands {
    Setup(TraceSetupSubcmd),
}

#[derive(Args)]
pub struct TraceSetupSubcmd {}

impl TraceSetupSubcmd {
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let pilout = PilOutProxy::new("./examples/fibv/data/fibv.pilout");

        let witness_cols =
            pilout.symbols.iter().filter(|s| s.r#type == SymbolType::WitnessCol as i32).collect::<Vec<_>>();

        let headers = "use proofman::trace\n".to_owned();
        let headers = headers + "use goldilocks::Goldilocks;\n\n";
        let mut traces = "".to_owned();

        for (subproof_index, subproof) in pilout.subproofs.iter().enumerate() {
            let subproof_name = subproof.name.as_ref().unwrap().clone() + ".";

            for (air_index, air) in pilout.subproofs[subproof_index].airs.iter().enumerate() {
                let mut trace = format!("trace!({} {{\n", air.name.as_ref().unwrap());

                for witness_col in &witness_cols {
                    if witness_col.subproof_id.unwrap() == subproof_index as u32
                        && witness_col.air_id.unwrap() == air_index as u32
                    {
                        let mut name = witness_col.name.clone();
                        if name.starts_with(&subproof_name) {
                            name.replace_range(0..subproof_name.len(), "");
                        }
                        let field_type = if witness_col.dim < 2 {
                            "Goldilocks".to_owned()
                        } else {
                            format!("[Goldilocks; {}]", witness_col.dim)
                        };
                        trace += &format!("\t{}: {},\n", name, field_type);
                    }
                }
                trace += &format!("}}\n\n");

                traces += &trace;
            }
        }

        println!("{}{}", headers, traces);

        Ok(())
    }
}
