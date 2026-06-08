//! Validate dashboard JSON against the coordinator contract.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod contract;
mod rules;
mod validator;

use contract::Contract;

fn print_usage() {
    eprintln!("usage: validate-dashboard --dashboard <path> [--contract <path>]");
}

fn parse_args(args: &[String]) -> Option<(PathBuf, Option<PathBuf>)> {
    let mut dashboard: Option<PathBuf> = None;
    let mut contract: Option<PathBuf> = None;
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--dashboard" => {
                dashboard = Some(PathBuf::from(iter.next()?));
            }
            "--contract" => {
                contract = Some(PathBuf::from(iter.next()?));
            }
            _ => return None,
        }
    }
    let dashboard = dashboard?;
    Some((dashboard, contract))
}

fn default_contract_path(dashboard: &Path) -> PathBuf {
    dashboard
        .parent()
        .map(|d| d.join("..").join("known-contract.json"))
        .unwrap_or_else(|| PathBuf::from("../known-contract.json"))
}

fn run(args: Vec<String>) -> ExitCode {
    let (dashboard_path, contract_path) = match parse_args(&args) {
        Some(v) => v,
        None => {
            print_usage();
            return ExitCode::from(2);
        }
    };

    let contract_path = contract_path.unwrap_or_else(|| default_contract_path(&dashboard_path));

    let contract = match Contract::load(&contract_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e);
            return ExitCode::from(1);
        }
    };

    let dashboard_text = match std::fs::read_to_string(&dashboard_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("failed to read dashboard {}: {}", dashboard_path.display(), e);
            return ExitCode::from(1);
        }
    };

    let dashboard: serde_json::Value = match serde_json::from_str(&dashboard_text) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("failed to parse dashboard JSON {}: {}", dashboard_path.display(), e);
            return ExitCode::from(1);
        }
    };

    let errors = validator::validate(&dashboard, &contract);

    if !errors.is_empty() {
        eprintln!("dashboard validation failed:");
        for error in &errors {
            eprintln!("- {}", error);
        }
        return ExitCode::from(1);
    }

    println!("dashboard validation passed: {}", dashboard_path.display());
    ExitCode::from(0)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    run(args)
}
