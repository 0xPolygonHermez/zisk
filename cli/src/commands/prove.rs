// extern crate env_logger;
use clap::{Parser, ValueEnum};
use std::{
    error::Error,
    fmt::Display,
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};
use colored::Colorize;

use p3_goldilocks::Goldilocks;

use proofman::ProofMan;

use std::str::FromStr;

#[derive(Parser, Debug, Clone, ValueEnum)]
pub enum Field {
    Goldilocks,
    // Add other variants here as needed
}

impl FromStr for Field {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Goldilocks" => Ok(Field::Goldilocks),
            // Add parsing for other variants here
            _ => Err(format!("'{}' is not a valid value for Field", s)),
        }
    }
}

impl Display for Field {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Field::Goldilocks => write!(f, "goldilocks"),
        }
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct ProveCmd {
    /// Witness computation dynamic library path
    #[clap(short, long)]
    pub wc_lib: PathBuf,

    /// Public inputs path
    #[clap(short, long)]
    pub public_inputs: Option<PathBuf>,

    /// Setup folder path
    #[clap(long)]
    pub proving_key: PathBuf,

    /// Output file path
    #[clap(short, long, default_value = "proof.json")]
    pub output: PathBuf,

    #[clap(long, default_value_t = Field::Goldilocks)]
    pub field: Field,
}

impl ProveCmd {
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("{} {}", format!("{: >12}", "Command").bright_green().bold(), "Prove");
        println!("");

        type GL = Goldilocks;

        let mut public_inputs_u8 = Vec::new();
        if self.public_inputs.is_some() {
            public_inputs_u8 = Self::read_hex_values_from_file(self.public_inputs.as_ref().unwrap().to_str().unwrap())?;
        }

        match self.field {
            Field::Goldilocks => {
                let _proof = ProofMan::generate_proof::<GL>(self.wc_lib.clone(), self.proving_key.clone(), public_inputs_u8);
            }
        }

        Ok(())
    }

    fn read_hex_values_from_file(filename: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        let file = File::open(filename)?;
        let reader = BufReader::new(file);

        let mut hex_values = Vec::new();

        for line_result in reader.lines() {
            let line = line_result?;

            if line.starts_with("0x") {
                let hex_digits = &line[2..]; // Skip "0x" prefix
                let mut chars = hex_digits.chars();

                while let Some(char1) = chars.next() {
                    if let Some(char2) = chars.next() {
                        let hex_str = format!("{}{}", char1, char2);
                        if let Ok(hex_value) = u8::from_str_radix(&hex_str, 16) {
                            hex_values.push(hex_value);
                        } else {
                            eprintln!("Error parsing hexadecimal value: {}", hex_str);
                        }
                    } else {
                        eprintln!("Odd number of hexadecimal digits: {}", hex_digits);
                    }
                }
            } else {
                eprintln!("Invalid line format: {}", line);
            }
        }

        Ok(hex_values)
    }
}
